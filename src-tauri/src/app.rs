//! Application wiring: plugins, managed state, IPC handler, asset protocol.

use crate::commands::{self, AppState, Shared};
use crate::settings::Settings;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::http::{header, Response, StatusCode};
use tauri::{Emitter, Manager};

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("studio-asset", |ctx, request| {
            let app = ctx.app_handle();
            let state = app.state::<Shared>();
            let root = state.root();
            let path = request.uri().path().trim_start_matches('/');
            let Some(root) = root else {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Vec::new())
                    .unwrap();
            };
            let decoded = percent_decode(path);
            match crate::assets::resolve_within(&root, &decoded) {
                Some(file) => match std::fs::read(&file) {
                    Ok(bytes) => {
                        let ext = file
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, crate::assets::mime_for(&ext))
                            .header(header::CACHE_CONTROL, "no-cache")
                            .body(bytes)
                            .unwrap()
                    }
                    Err(_) => Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Vec::new())
                        .unwrap(),
                },
                None => Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Vec::new())
                    .unwrap(),
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let state: Shared = AppState::new(handle.clone());
            app.manage(state.clone());

            // Autosave loop: writes project.json every N seconds while dirty.
            let autosave_state = state.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(5));
                let mut last_save = std::time::Instant::now();
                loop {
                    std::thread::sleep(Duration::from_secs(2));
                    let seconds = autosave_state
                        .settings
                        .lock()
                        .map(|s| s.autosave_seconds.max(5))
                        .unwrap_or(30) as u64;
                    if last_save.elapsed() < Duration::from_secs(seconds) {
                        continue;
                    }
                    let saved = {
                        let mut store = match autosave_state.store.lock() {
                            Ok(s) => s,
                            Err(_) => break,
                        };
                        if store.is_open() && store.dirty {
                            match store.save() {
                                Ok(()) => true,
                                Err(err) => {
                                    eprintln!("autosave: {err}");
                                    false
                                }
                            }
                        } else {
                            false
                        }
                    };
                    if saved {
                        let _ = autosave_state.app.emit(
                            "studio://saved",
                            serde_json::json!({ "at": crate::model::now_iso(), "auto": true }),
                        );
                        last_save = std::time::Instant::now();
                    }
                }
            });

            // Re-open the last project so the studio starts where the author left.
            let mut settings = Settings::load();
            // A path from the command line (`.rpyproj` association, shortcut with
            // a project) wins over the last opened project.
            let cli_project = project_from_args();

            if settings.mcp_enabled {
                match crate::mcp::start(state.clone(), settings.mcp_port, None) {
                    Ok(mcp_handle) => {
                        if let Ok(mut guard) = state.mcp.lock() {
                            *guard = Some(mcp_handle);
                        }
                        eprintln!("[studio] MCP on 127.0.0.1:{}", settings.mcp_port);
                    }
                    Err(err) => eprintln!("[studio] MCP failed: {err}"),
                }
            }

            let initial = cli_project.or_else(|| settings.last_project.clone().map(PathBuf::from));
            if let Some(path) = initial {
                if path.join("renpy-studio").join("project.json").is_file() {
                    match crate::store::Store::open(&path) {
                        Ok(store) => {
                            let env = crate::commands::envelope_of(&store);
                            let title = env.project.meta.title.clone();
                            if let Ok(mut guard) = state.store.lock() {
                                *guard = store;
                            }
                            settings.remember_project(&path.display().to_string(), &title);
                            let _ = settings.save();
                            let _ = handle.emit(
                                "studio://project-opened",
                                serde_json::json!({ "title": title }),
                            );
                        }
                        Err(err) => eprintln!("[studio] open failed: {err}"),
                    }
                } else {
                    eprintln!("[studio] project.json missing under {}", path.display());
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::app_bootstrap,
            commands::settings_save,
            commands::sdk_detect,
            commands::sdk_set_path,
            commands::project_new,
            commands::project_open,
            commands::project_create_in_existing,
            commands::project_state,
            commands::project_close,
            commands::project_save,
            commands::project_reload,
            commands::project_export,
            commands::project_validate,
            commands::project_graph,
            commands::assets_list,
            commands::undo,
            commands::redo,
            commands::scene_create,
            commands::scene_update,
            commands::scene_delete,
            commands::scene_duplicate,
            commands::scene_reorder,
            commands::scene_set_start,
            commands::node_add,
            commands::node_update,
            commands::node_delete,
            commands::node_move,
            commands::node_duplicate,
            commands::character_upsert,
            commands::character_delete,
            commands::variable_upsert,
            commands::variable_delete,
            commands::meta_update,
            commands::ui_update,
            commands::project_lint,
            commands::project_run,
            commands::project_stop,
            commands::project_preview_screen,
            commands::project_preview_command,
            commands::project_build,
            commands::project_build_start,
            commands::project_build_status,
            commands::asset_import,
            commands::project_translate,
            commands::external_diff,
            commands::node_retarget,
            commands::plugins_list,
            commands::plugin_enable,
            commands::plugin_disable,
            commands::plugin_docs,
            commands::plugins_root,
            commands::ensure_sprite_folder,
            commands::snapshot_create,
            commands::snapshot_list,
            commands::snapshot_restore,
            commands::snapshot_diff,
            commands::import_scan,
            commands::import_apply,
            commands::mcp_start,
            commands::mcp_stop,
            commands::mcp_status,
            commands::open_in_explorer,
            commands::default_block_params,
            commands::project_path,
            commands::ai_models,
            commands::ai_chat,
            commands::ai_forget_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running RenPy Studio");
}

/// Project folder passed on the command line: an `.rpyproj` marker file, a
/// project directory, or any file inside one. Flags are ignored.
fn project_from_args() -> Option<PathBuf> {
    std::env::args_os()
        .skip(1)
        .filter_map(|arg| resolve_project_arg(Path::new(&arg)))
        .next()
}

fn resolve_project_arg(path: &Path) -> Option<PathBuf> {
    let raw = path.to_string_lossy();
    if raw.trim().is_empty() || raw.starts_with('-') {
        return None;
    }
    let root = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    let project = root.join("renpy-studio").join("project.json");
    project.is_file().then_some(root)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(value);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::resolve_project_arg;
    use std::fs;
    use std::path::PathBuf;

    fn temp_project(name: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "renpy-studio-args-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("renpy-studio")).unwrap();
        fs::write(root.join("renpy-studio").join("project.json"), b"{}").unwrap();
        let marker = root.join("renpy-studio.rpyproj");
        fs::write(&marker, b"{}").unwrap();
        (root, marker)
    }

    #[test]
    fn project_arg_accepts_folder_marker_and_flags() {
        let (root, marker) = temp_project("cli");
        assert_eq!(resolve_project_arg(&root), Some(root.clone()));
        assert_eq!(resolve_project_arg(&marker), Some(root.clone()));
        assert_eq!(resolve_project_arg(&PathBuf::from("--no-splash")), None);
        assert_eq!(
            resolve_project_arg(&root.join("missing")),
            None,
            "a folder without project.json must be ignored"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
