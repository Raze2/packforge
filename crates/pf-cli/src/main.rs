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
    /// Fold a design in 3D and print the panel corners as JSON (progress 0 = flat, 1 = erected).
    Fold {
        #[command(flatten)]
        design: DesignArgs,
        #[arg(long, default_value_t = 1.0)]
        progress: f64,
    },
    /// Export a design to a file.
    Export {
        #[command(flatten)]
        design: DesignArgs,
        /// Output format: svg | dxf | pdf | cf2 | hpgl | glb
        #[arg(short, long)]
        format: String,
        /// Output file ("-" for stdout). Default: <component>.<ext>
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[command(flatten)]
        production: ProductionArgs,
    },
    /// How many blanks fit on a sheet.
    Nest {
        #[command(flatten)]
        design: DesignArgs,
        /// Sheet size in mm, e.g. 2400x1600
        #[arg(long)]
        sheet: String,
        #[arg(long, default_value_t = 8.0)]
        gap: f64,
        #[arg(long, default_value_t = 10.0)]
        margin: f64,
    },
}

#[derive(Args)]
struct ProductionArgs {
    /// Rotate counter-clockwise: 0, 90, 180, 270
    #[arg(long, default_value_t = 0)]
    rotate: i32,
    /// Mirror left/right
    #[arg(long)]
    mirror: bool,
    /// PDF page: actual (1:1), a4, a3, letter
    #[arg(long)]
    page: Option<String>,
    /// PDF watermark text
    #[arg(long)]
    watermark: Option<String>,
    /// Output profile: standard, cut-crease
    #[arg(long)]
    profile: Option<String>,
    /// Nest on a sheet, e.g. 2400x1600 (mm)
    #[arg(long)]
    sheet: Option<String>,
    /// Gap between nested blanks (mm)
    #[arg(long, default_value_t = 8.0)]
    gap: f64,
    /// Sheet margin (mm)
    #[arg(long, default_value_t = 10.0)]
    margin: f64,
}

fn parse_sheet(s: &str, gap: f64, margin: f64) -> Result<pf_core::Nesting, String> {
    let (w, h) = s
        .to_ascii_lowercase()
        .split_once('x')
        .and_then(|(w, h)| Some((w.trim().parse::<f64>().ok()?, h.trim().parse::<f64>().ok()?)))
        .ok_or_else(|| format!("sheet must look like 2400x1600, got '{s}'"))?;
    Ok(pf_core::Nesting {
        sheet_width: w,
        sheet_height: h,
        gap,
        margin,
        allow_rotation: true,
        max_count: None,
    })
}

impl ProductionArgs {
    fn options(&self) -> Result<pf_core::ExportOptions, String> {
        Ok(pf_core::ExportOptions {
            rotate: self.rotate,
            mirror: self.mirror,
            page: self.page.clone(),
            watermark: self.watermark.clone(),
            profile: self.profile.clone(),
            custom_profile: None,
            nesting: self
                .sheet
                .as_deref()
                .map(|s| parse_sheet(s, self.gap, self.margin))
                .transpose()?,
        })
    }
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
        Command::Fold { design, progress } => {
            let f = pf_core::fold(&design.project()?, progress).map_err(|e| e.to_string())?;
            println!("{}", pretty(&f)?);
        }
        Command::Nest {
            design,
            sheet,
            gap,
            margin,
        } => {
            let n = pf_core::nest_preview(&design.project()?, &parse_sheet(&sheet, gap, margin)?)
                .map_err(|e| e.to_string())?;
            println!(
                "{} blanks ({} × {}{}) of {:.0} × {:.0} mm, {:.0}% of the sheet used",
                n.count,
                n.cols,
                n.rows,
                if n.rotated { ", turned 90°" } else { "" },
                n.blank_width,
                n.blank_height,
                n.utilisation * 100.0
            );
        }
        Command::Export {
            design,
            format,
            output,
            production,
        } => {
            let f = Format::parse(&format).ok_or_else(|| {
                format!("unsupported format '{format}' (use svg, dxf, pdf, cf2, hpgl or glb)")
            })?;
            let file = pf_core::export_with(&design.project()?, f, &production.options()?)
                .map_err(|e| e.to_string())?;
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
