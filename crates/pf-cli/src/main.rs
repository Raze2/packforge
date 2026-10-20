//! `pf` — PackForge command-line tool.

use clap::{Args, Parser, Subcommand};
use pf_core::{Board, BoardRef, Format, Project, Units, Value};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "pf", version, about = "PackForge packaging design tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List built-in components.
    List,
    /// Show the parameter schema of a component (JSON).
    Describe { component: String },
    /// List board presets.
    Boards,
    /// Render a design and print stats (or the full result with --json).
    Render {
        #[command(flatten)]
        design: DesignArgs,
        /// Print the full render result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Export a design to a file.
    Export {
        #[command(flatten)]
        design: DesignArgs,
        /// Output format: svg | dxf
        #[arg(short, long)]
        format: String,
        /// Output file ("-" for stdout). Default: <component>.<ext>
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Args)]
struct DesignArgs {
    /// Component id, e.g. fefco-0201
    component: String,
    /// Parameter as KEY=VALUE (repeatable), lengths in mm: -p L=300 -p W=200 -p H=150
    #[arg(short = 'p', long = "param", value_name = "KEY=VALUE")]
    params: Vec<String>,
    /// Board preset id (see `pf boards`)
    #[arg(short, long, default_value = "C-flute", conflicts_with = "caliper")]
    board: String,
    /// Custom board caliper in mm (instead of a preset)
    #[arg(long)]
    caliper: Option<f64>,
}

impl DesignArgs {
    fn project(&self) -> Result<Project, String> {
        let mut params = BTreeMap::new();
        for kv in &self.params {
            let (k, v) = kv
                .split_once('=')
                .ok_or_else(|| format!("parameter '{kv}' must look like KEY=VALUE"))?;
            params.insert(k.trim().to_string(), parse_value(v.trim()));
        }
        let board = match self.caliper {
            Some(c) => BoardRef::Custom(Board {
                id: "custom".into(),
                name: "Custom".into(),
                caliper_mm: c,
                flute: None,
            }),
            None => BoardRef::Preset(self.board.clone()),
        };
        Ok(Project {
            component_id: self.component.clone(),
            component_version: None,
            params,
            board,
            units: Units::Mm,
        })
    }
}

fn parse_value(v: &str) -> Value {
    if let Ok(n) = v.parse::<f64>() {
        Value::Number(n)
    } else if v == "true" || v == "false" {
        Value::Bool(v == "true")
    } else {
        Value::Text(v.to_string())
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::List => {
            for c in pf_core::list_components().map_err(|e| e.to_string())? {
                println!(
                    "{:<14} {:<8} {:<8} {}",
                    c.id,
                    c.version,
                    format!("{:?}", c.tier).to_lowercase(),
                    c.name
                );
            }
        }
        Command::Describe { component } => {
            let info = pf_core::describe(&component).map_err(|e| e.to_string())?;
            println!("{}", pretty(&info)?);
        }
        Command::Boards => {
            for b in pf_core::boards() {
                println!("{:<10} {:>5} mm  {}", b.id, b.caliper_mm, b.name);
            }
        }
        Command::Render {
            design,
            json: as_json,
        } => {
            let r = pf_core::render(&design.project()?).map_err(|e| e.to_string())?;
            if as_json {
                println!("{}", pretty(&r)?);
            } else {
                let s = r.stats;
                println!("component     {} v{}", r.component_id, r.component_version);
                println!("board         {} ({} mm)", r.board.id, r.board.caliper_mm);
                println!(
                    "blank         {:.1} × {:.1} mm",
                    s.blank_width, s.blank_height
                );
                println!("blank area    {:.4} m²", s.blank_area_m2);
                println!("outline area  {:.4} m²", s.outline_area_m2);
                println!("cut length    {:.0} mm", s.cut_length);
                println!("crease length {:.0} mm", s.crease_length);
            }
            for d in &r.diagnostics {
                eprintln!("{:?}: {}", d.severity, d.message);
            }
        }
        Command::Export {
            design,
            format,
            output,
        } => {
            let f = Format::parse(&format)
                .ok_or_else(|| format!("unsupported format '{format}' (use svg or dxf)"))?;
            let file = pf_core::export(&design.project()?, f).map_err(|e| e.to_string())?;
            match output.as_deref() {
                Some(p) if p.as_os_str() == "-" => std::io::stdout()
                    .write_all(&file.bytes)
                    .map_err(|e| e.to_string())?,
                Some(p) => write_file(p, &file.bytes)?,
                None => write_file(&PathBuf::from(&file.filename), &file.bytes)?,
            }
        }
    }
    Ok(())
}

fn write_file(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    eprintln!("wrote {} ({} bytes)", path.display(), bytes.len());
    Ok(())
}

fn pretty<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string_pretty(v).map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
