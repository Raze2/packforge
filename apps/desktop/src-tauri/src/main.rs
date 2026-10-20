// PackForge desktop: the web designer (apps/web) in a native window, with native file
// dialogs, menus, recent files, drag-and-drop, file associations and native exports.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod files;

use files::{
    display_name, export_to, read_project, with_extension, write_project, OpenedFile, Recent,
    PROJECT_EXT,
};
use pf_core::{Format, Project};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
#[cfg(target_os = "macos")]
use tauri::RunEvent;
use tauri::{AppHandle, DragDropEvent, Emitter, Manager, State, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[derive(Default)]
struct Doc {
    path: Option<PathBuf>,
    dirty: bool,
}

#[derive(Default)]
struct AppState {
    doc: Mutex<Doc>,
    /// File passed on the command line or via the OS before the UI was ready.
    pending: Mutex<Option<PathBuf>>,
    ui_ready: AtomicBool,
}

impl AppState {
    fn doc(&self) -> std::sync::MutexGuard<'_, Doc> {
        self.doc.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[derive(Serialize)]
struct SavedFile {
    path: String,
    name: String,
}

fn recent(app: &AppHandle) -> Option<Recent> {
    app.path().app_config_dir().ok().map(|d| Recent::new(&d))
}

fn remember(app: &AppHandle, state: &AppState, path: &std::path::Path) {
    {
        let mut doc = state.doc();
        doc.path = Some(path.to_path_buf());
        doc.dirty = false;
    }
    if let Some(r) = recent(app) {
        r.add(path);
    }
    if let Ok(menu) = build_menu(app) {
        let _ = app.set_menu(menu);
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Opens a project: from `path`, or from a file dialog when `path` is None.
#[tauri::command]
async fn open_project(
    app: AppHandle,
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<Option<OpenedFile>, String> {
    let path = match path {
        Some(p) => PathBuf::from(p),
        None => {
            let picked = app
                .dialog()
                .file()
                .add_filter("PackForge project", &[PROJECT_EXT, "json"])
                .blocking_pick_file();
            match picked.and_then(|f| f.into_path().ok()) {
                Some(p) => p,
                None => return Ok(None),
            }
        }
    };
    let opened = read_project(&path)?;
    remember(&app, &state, &path);
    Ok(Some(opened))
}

/// Saves project JSON to the current file, or asks for a location (first save / Save As).
#[tauri::command]
async fn save_project(
    app: AppHandle,
    state: State<'_, AppState>,
    contents: String,
    save_as: bool,
    suggested_name: String,
) -> Result<Option<SavedFile>, String> {
    let current = state.doc().path.clone();
    let path = match current.filter(|_| !save_as) {
        Some(p) => p,
        None => {
            let picked = app
                .dialog()
                .file()
                .set_file_name(format!("{suggested_name}.{PROJECT_EXT}"))
                .add_filter("PackForge project", &[PROJECT_EXT])
                .blocking_save_file();
            match picked.and_then(|f| f.into_path().ok()) {
                Some(p) => with_extension(p, PROJECT_EXT),
                None => return Ok(None),
            }
        }
    };
    write_project(&path, &contents)?;
    remember(&app, &state, &path);
    Ok(Some(SavedFile {
        path: path.display().to_string(),
        name: display_name(&path),
    }))
}

/// Exports with the native core after asking for a location. Returns the written path.
#[tauri::command]
async fn export_file(
    app: AppHandle,
    project: Project,
    format: String,
    suggested_name: String,
    options: Option<pf_core::ExportOptions>,
) -> Result<Option<String>, String> {
    let fmt = Format::parse(&format).ok_or_else(|| format!("unsupported format '{format}'"))?;
    let ext = fmt.extension();
    let picked = app
        .dialog()
        .file()
        .set_file_name(format!("{suggested_name}.{ext}"))
        .add_filter(ext.to_uppercase(), &[ext])
        .blocking_save_file();
    let Some(path) = picked.and_then(|f| f.into_path().ok()) else {
        return Ok(None);
    };
    let path = with_extension(path, ext);
    export_to(&path, &project, fmt, &options.unwrap_or_default())?;
    Ok(Some(path.display().to_string()))
}

/// Starts a new, unsaved document.
#[tauri::command]
fn new_document(state: State<'_, AppState>) {
    *state.doc() = Doc::default();
}

/// Updates the window title and the unsaved-changes flag used when closing.
#[tauri::command]
fn set_document(app: AppHandle, state: State<'_, AppState>, name: Option<String>, dirty: bool) {
    state.doc().dirty = dirty;
    let title = match name {
        Some(n) => format!("{}{n} — PackForge", if dirty { "• " } else { "" }),
        None => format!("{}Untitled — PackForge", if dirty { "• " } else { "" }),
    };
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_title(&title);
    }
}

/// Called once by the UI on start: returns a file opened from the OS / command line, if any.
#[tauri::command]
fn take_startup_file(app: AppHandle, state: State<'_, AppState>) -> Option<OpenedFile> {
    state.ui_ready.store(true, Ordering::SeqCst);
    let path = state.pending.lock().ok()?.take()?;
    match read_project(&path) {
        Ok(f) => {
            remember(&app, &state, &path);
            Some(f)
        }
        Err(e) => {
            let _ = app.emit("error", e);
            None
        }
    }
}

/// Opens a file that arrived from outside (drop, Finder, recent menu) in the running UI.
fn open_external(app: &AppHandle, path: PathBuf) {
    let state = app.state::<AppState>();
    if !state.ui_ready.load(Ordering::SeqCst) {
        if let Ok(mut p) = state.pending.lock() {
            *p = Some(path);
        }
        return;
    }
    match read_project(&path) {
        Ok(f) => {
            remember(app, &state, &path);
            let _ = app.emit("open-file", f);
        }
        Err(e) => {
            let _ = app.emit("error", e);
        }
    }
}

fn is_project_file(p: &std::path::Path) -> bool {
    p.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(PROJECT_EXT) || e.eq_ignore_ascii_case("json"))
}

// ── Menu ──────────────────────────────────────────────────────────────────────

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, label: &str, accel: Option<&str>| {
        let b = MenuItemBuilder::with_id(id, label);
        match accel {
            Some(a) => b.accelerator(a).build(app),
            None => b.build(app),
        }
    };

    let mut recent_menu = SubmenuBuilder::new(app, "Open Recent");
    let list = recent(app).map(|r| r.list()).unwrap_or_default();
    if list.is_empty() {
        recent_menu = recent_menu.item(
            &MenuItemBuilder::with_id("recent.none", "No recent files")
                .enabled(false)
                .build(app)?,
        );
    } else {
        for (i, p) in list.iter().enumerate() {
            let label = display_name(std::path::Path::new(p));
            recent_menu = recent_menu.item(&item(&format!("recent.{i}"), &label, None)?);
        }
    }

    let file = SubmenuBuilder::new(app, "File")
        .item(&item("file.new", "New", Some("CmdOrCtrl+N"))?)
        .item(&item("file.open", "Open…", Some("CmdOrCtrl+O"))?)
        .item(&recent_menu.build()?)
        .separator()
        .item(&item("file.save", "Save", Some("CmdOrCtrl+S"))?)
        .item(&item(
            "file.save-as",
            "Save As…",
            Some("CmdOrCtrl+Shift+S"),
        )?)
        .separator()
        .item(&item(
            "file.export-dxf",
            "Export DXF…",
            Some("CmdOrCtrl+E"),
        )?)
        .item(&item(
            "file.export-svg",
            "Export SVG…",
            Some("CmdOrCtrl+Shift+E"),
        )?);
    #[cfg(not(target_os = "macos"))]
    let file = file.separator().quit();

    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .item(&item("view.fit", "Fit to Window", Some("CmdOrCtrl+0"))?)
        .separator()
        .fullscreen()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .close_window()
        .build()?;

    let mut menu = MenuBuilder::new(app);
    #[cfg(target_os = "macos")]
    {
        let app_menu = SubmenuBuilder::new(app, "PackForge")
            .about(None)
            .separator()
            .services()
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .quit()
            .build()?;
        menu = menu.item(&app_menu);
    }
    menu = menu
        .item(&file.build()?)
        .item(&edit)
        .item(&view)
        .item(&window);
    #[cfg(not(target_os = "macos"))]
    {
        let help = SubmenuBuilder::new(app, "Help").about(None).build()?;
        menu = menu.item(&help);
    }
    menu.build()
}

fn on_menu(app: &AppHandle, id: &str) {
    if let Some(i) = id
        .strip_prefix("recent.")
        .and_then(|i| i.parse::<usize>().ok())
    {
        if let Some(p) = recent(app).and_then(|r| r.list().into_iter().nth(i)) {
            open_external(app, PathBuf::from(p));
        }
        return;
    }
    // Everything else is handled by the UI (it owns the document).
    let _ = app.emit("menu", id.to_string());
}

// ── App ───────────────────────────────────────────────────────────────────────

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            // Windows/Linux pass an opened file as an argument.
            if let Some(p) = std::env::args()
                .skip(1)
                .map(PathBuf::from)
                .find(|p| is_project_file(p) && p.exists())
            {
                *app.state::<AppState>()
                    .pending
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(p);
            }
            let menu = build_menu(app.handle())?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| on_menu(app, event.id().as_ref()));
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let app = window.app_handle().clone();
                if !app.state::<AppState>().doc().dirty {
                    return;
                }
                api.prevent_close();
                let win = window.clone();
                app.dialog()
                    .message("Your design has unsaved changes. Close without saving?")
                    .title("Unsaved changes")
                    .kind(MessageDialogKind::Warning)
                    .buttons(MessageDialogButtons::OkCancelCustom(
                        "Close Without Saving".into(),
                        "Cancel".into(),
                    ))
                    .show(move |discard| {
                        if discard {
                            win.app_handle().state::<AppState>().doc().dirty = false;
                            let _ = win.destroy();
                        }
                    });
            }
            tauri::WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) => {
                if let Some(p) = paths.iter().find(|p| is_project_file(p)) {
                    open_external(window.app_handle(), p.clone());
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            open_project,
            save_project,
            export_file,
            new_document,
            set_document,
            take_startup_file
        ])
        .build(tauri::generate_context!())
        .expect("error while building PackForge");

    app.run(|_app, _event| {
        // macOS delivers files opened from Finder (file association) as an event.
        #[cfg(target_os = "macos")]
        if let RunEvent::Opened { urls } = &_event {
            if let Some(p) = urls
                .iter()
                .filter_map(|u| u.to_file_path().ok())
                .find(|p| is_project_file(p))
            {
                open_external(_app, p);
            }
        }
    });
}
