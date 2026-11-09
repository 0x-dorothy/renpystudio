//! Application wiring: plugins, managed state, IPC handler, asset protocol.

use crate::commands::{self, AppState, Shared};
use crate::settings::Settings;
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
            let settings = Settings::load();

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

            if let Some(path) = settings.last_project.clone() {
                let path = std::path::PathBuf::from(path);
                if path.join("renpy-studio").join("project.json").is_file() {
                    match crate::store::Store::open(&path) {
                        Ok(store) => {
                            let env = crate::commands::envelope_of(&store);
                            let title = env.project.meta.title.clone();
                            if let Ok(mut guard) = state.store.lock() {
                                *guard = store;
                            }
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
            commands::project_build,
            commands::project_build_start,
            commands::project_build_status,
            commands::asset_import,
            commands::project_translate,
            commands::external_diff,
            commands::node_retarget,
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
            commands::project_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running RenPy Studio");
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
