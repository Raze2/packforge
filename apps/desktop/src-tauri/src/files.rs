//! File operations behind the desktop commands. Kept free of Tauri types so they are unit-tested.

use pf_core::{Format, Project};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROJECT_EXT: &str = "pfproj";
const MAX_RECENT: usize = 8;

/// A project file read from disk.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OpenedFile {
    pub path: String,
    pub name: String,
    pub contents: String,
}

pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Appends `ext` unless the path already ends with it (case-insensitive).
pub fn with_extension(path: PathBuf, ext: &str) -> PathBuf {
    let has = path
        .extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(ext));
    if has {
        path
    } else {
        let mut s = path.into_os_string();
        s.push(".");
        s.push(ext);
        PathBuf::from(s)
    }
}

/// Reads and validates a project file (must be a JSON project).
pub fn read_project(path: &Path) -> Result<OpenedFile, String> {
    let contents =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str::<Project>(&contents)
        .map_err(|e| format!("{} is not a PackForge project: {e}", display_name(path)))?;
    Ok(OpenedFile {
        path: path.display().to_string(),
        name: display_name(path),
        contents,
    })
}

/// Writes project JSON after checking it parses; writes atomically via a temporary file.
pub fn write_project(path: &Path, contents: &str) -> Result<(), String> {
    serde_json::from_str::<Project>(contents).map_err(|e| format!("invalid project: {e}"))?;
    write_atomic(path, contents.as_bytes())
}

/// Renders and writes an export with the native core.
pub fn export_to(
    path: &Path,
    project: &Project,
    format: Format,
    options: &pf_core::ExportOptions,
) -> Result<(), String> {
    let file = pf_core::export_with(project, format, options).map_err(|e| e.to_string())?;
    write_atomic(path, &file.bytes)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("pf-tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("cannot write {}: {e}", path.display())
    })
}

/// Recently used project files, newest first, persisted as JSON.
pub struct Recent {
    file: PathBuf,
}

impl Recent {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            file: config_dir.join("recent.json"),
        }
    }

    pub fn list(&self) -> Vec<String> {
        fs::read_to_string(&self.file)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|p| Path::new(p).exists())
            .take(MAX_RECENT)
            .collect()
    }

    pub fn add(&self, path: &Path) -> Vec<String> {
        let p = path.display().to_string();
        let mut list: Vec<String> = self.list().into_iter().filter(|x| *x != p).collect();
        list.insert(0, p);
        list.truncate(MAX_RECENT);
        if let Some(dir) = self.file.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(
            &self.file,
            serde_json::to_string_pretty(&list).unwrap_or_default(),
        );
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT: &str = r#"{"component_id":"fefco-0201","params":{"L":300,"W":200,"H":150},"board":"C-flute","units":"mm"}"#;

    #[test]
    fn extension_is_added_once() {
        assert_eq!(
            with_extension("box".into(), "pfproj"),
            PathBuf::from("box.pfproj")
        );
        assert_eq!(
            with_extension("box.PFPROJ".into(), "pfproj"),
            PathBuf::from("box.PFPROJ")
        );
        assert_eq!(
            with_extension("a.b".into(), "dxf"),
            PathBuf::from("a.b.dxf")
        );
    }

    #[test]
    fn project_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("box.pfproj");
        write_project(&path, PROJECT).expect("write");
        let opened = read_project(&path).expect("read");
        assert_eq!(opened.contents, PROJECT);
        assert_eq!(opened.name, "box.pfproj");
        assert!(!dir.path().join("box.pf-tmp").exists());
    }

    #[test]
    fn invalid_projects_are_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("x.pfproj");
        assert!(write_project(&path, "{\"nope\":1}").is_err());
        assert!(!path.exists());
        fs::write(&path, "not json").expect("write");
        assert!(read_project(&path)
            .unwrap_err()
            .contains("not a PackForge project"));
    }

    #[test]
    fn native_export_writes_dxf() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("box.dxf");
        let project: Project = serde_json::from_str(PROJECT).expect("project");
        export_to(&path, &project, Format::Dxf, &Default::default()).expect("export");
        assert!(fs::read_to_string(&path).expect("read").ends_with("EOF\n"));
        let pdf = dir.path().join("box.pdf");
        let opts = pf_core::ExportOptions {
            page: Some("a4".into()),
            ..Default::default()
        };
        export_to(&pdf, &project, Format::Pdf, &opts).expect("pdf");
        assert!(fs::read(&pdf).expect("read").starts_with(b"%PDF-1.4"));
    }

    #[test]
    fn recent_list_is_deduplicated_newest_first_and_capped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let recent = Recent::new(dir.path());
        let files: Vec<PathBuf> = (0..10)
            .map(|i| {
                let p = dir.path().join(format!("{i}.pfproj"));
                fs::write(&p, PROJECT).expect("write");
                p
            })
            .collect();
        for f in &files {
            recent.add(f);
        }
        recent.add(&files[3]);
        let list = recent.list();
        assert_eq!(list.len(), MAX_RECENT);
        assert_eq!(list[0], files[3].display().to_string());
        assert_eq!(
            list.iter()
                .filter(|p| **p == files[3].display().to_string())
                .count(),
            1
        );
        fs::remove_file(&files[9]).expect("rm");
        assert!(!recent.list().contains(&files[9].display().to_string()));
    }
}
