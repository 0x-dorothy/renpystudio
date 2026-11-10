//! MCP server (Model Context Protocol) exposing the project to external agents.
//!
//! Transport: Streamable HTTP on 127.0.0.1 with JSON responses (no SSE stream).
//! Every mutation is validated with the same rules as the UI, snapshot-ed first,
//! and tagged with `source: "mcp"` so the studio can highlight it.

use crate::commands::{default_params, AppState, Shared};
use crate::generator::kind_label;
use crate::model::*;
use crate::validation;
use axum::body::Bytes;
use axum::extract::State as AxumState;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

pub struct McpHandle {
    pub port: u16,
    stop: Arc<AtomicBool>,
}

impl Drop for McpHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

struct Ctx {
    state: Shared,
    token: Option<String>,
}

const PROTOCOL_VERSION: &str = "2025-06-18";

pub fn start(state: Shared, port: u16, token: Option<String>) -> Result<McpHandle, String> {
    let addr = format!("127.0.0.1:{port}");
    let listener = std::net::TcpListener::bind(&addr)
        .map_err(|e| format!("Не удалось занять порт {port}: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let ctx = Ctx { state, token };

    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(err) => {
                eprintln!("MCP runtime: {err}");
                return;
            }
        };
        runtime.block_on(async move {
            let app = Router::new()
                .route("/mcp", post(handle_rpc).get(handle_get))
                .route("/health", get(|| async { Json(json!({"ok": true})) }))
                .with_state(Arc::new(ctx));
            let listener = match tokio::net::TcpListener::from_std(listener) {
                Ok(l) => l,
                Err(err) => {
                    eprintln!("MCP listener: {err}");
                    return;
                }
            };
            let shutdown_flag = stop_thread.clone();
            let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                loop {
                    if shutdown_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            });
            if let Err(err) = server.await {
                eprintln!("MCP server: {err}");
            }
        });
    });

    Ok(McpHandle { port, stop })
}

async fn handle_get() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(json!({
            "error": "This MCP server answers JSON-RPC over HTTP POST /mcp (Streamable HTTP)."
        })),
    )
        .into_response()
}

async fn handle_rpc(
    AxumState(ctx): AxumState<Arc<Ctx>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Some(expected) = &ctx.token {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim_start_matches("Bearer ").trim().to_string())
            .or_else(|| {
                headers
                    .get("x-mcp-token")
                    .and_then(|v| v.to_str().ok())
                    .map(|v| v.to_string())
            });
        if provided.as_deref() != Some(expected.as_str()) {
            return (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid token"}))).into_response();
        }
    }
    let payload: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("Parse error: {err}") }
                })),
            )
                .into_response()
        }
    };

    // Batches are handled one by one (rare in practice).
    if let Some(items) = payload.as_array() {
        let mut responses = Vec::new();
        for item in items {
            if let Some(resp) = dispatch(&ctx, item).await {
                responses.push(resp);
            }
        }
        if responses.is_empty() {
            return StatusCode::ACCEPTED.into_response();
        }
        return Json(Value::Array(responses)).into_response();
    }

    match dispatch(&ctx, &payload).await {
        Some(resp) => Json(resp).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

async fn dispatch(ctx: &Arc<Ctx>, payload: &Value) -> Option<Value> {
    let id = payload.get("id").cloned();
    let method = payload.get("method").and_then(|m| m.as_str()).unwrap_or("");
    // Notifications have no id: acknowledge silently.
    let is_notification = id.is_none();
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "renpy-studio", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "RenPy Studio project server. Read the project, then edit scenes and dialogue blocks. Call request_snapshot before bulk edits."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => {
            let name = payload
                .pointer("/params/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = payload
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match call_tool(&ctx.state, name, &args) {
                Ok(value) => Ok(json!({
                    "content": [ { "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_default() } ],
                    "structuredContent": value
                })),
                Err(err) => Ok(json!({
                    "content": [ { "type": "text", "text": format!("Ошибка: {err}") } ],
                    "isError": true
                })),
            }
        }
        "resources/list" => Ok(json!({ "resources": [] })),
        "prompts/list" => Ok(json!({ "prompts": [] })),
        _ => Err((StatusCode::OK, json!({ "code": -32601, "message": format!("Method not found: {method}") }))),
    };

    if is_notification {
        return None;
    }
    let id = id.unwrap_or(Value::Null);
    Some(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err((_, error)) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    })
}

fn schema(props: Value, required: Vec<&str>) -> Value {
    json!({
        "type": "object",
        "properties": props,
        "required": required,
        "additionalProperties": false
    })
}

fn tools() -> Value {
    json!([
        {
            "name": "get_project_overview",
            "description": "Project metadata plus the list of scenes, characters, variables and validation summary.",
            "inputSchema": schema(json!({}), vec![])
        },
        {
            "name": "get_scene",
            "description": "Full scene contents as ordered blocks. Accepts sceneId or title.",
            "inputSchema": schema(json!({
                "sceneId": {"type": "string"},
                "title": {"type": "string"}
            }), vec![])
        },
        {
            "name": "create_scene",
            "description": "Create a scene. Returns the new scene id and label.",
            "inputSchema": schema(json!({
                "title": {"type": "string"},
                "afterSceneId": {"type": "string"},
                "summary": {"type": "string"}
            }), vec!["title"])
        },
        {
            "name": "update_scene",
            "description": "Rename a scene, change its summary, or replace all of its blocks.",
            "inputSchema": schema(json!({
                "sceneId": {"type": "string"},
                "title": {"type": "string"},
                "summary": {"type": "string"},
                "blocks": {"type": "array", "items": {"type": "object"}}
            }), vec!["sceneId"])
        },
        {
            "name": "create_say_block",
            "description": "Append (or insert) a dialogue line. `character` accepts a character id or name.",
            "inputSchema": schema(json!({
                "sceneId": {"type": "string"},
                "text": {"type": "string"},
                "character": {"type": "string"},
                "locale": {"type": "string"},
                "afterNodeId": {"type": "string"},
                "index": {"type": "integer"},
                "image": {"type": "string"}
            }), vec!["sceneId", "text"])
        },
        {
            "name": "create_menu_block",
            "description": "Add a choice menu. Each option may point to a scene id or title.",
            "inputSchema": schema(json!({
                "sceneId": {"type": "string"},
                "prompt": {"type": "string"},
                "options": {"type": "array", "items": {"type": "object", "properties": {
                    "text": {"type": "string"},
                    "target": {"type": "string"},
                    "condition": {"type": "string"}
                }, "required": ["text"]}}
            }), vec!["sceneId", "options"])
        },
        {
            "name": "add_block",
            "description": "Add any supported block by kind (scene, show, hide, jump, pause, set, music, sound, code, ...).",
            "inputSchema": schema(json!({
                "sceneId": {"type": "string"},
                "kind": {"type": "string"},
                "params": {"type": "object"},
                "index": {"type": "integer"}
            }), vec!["sceneId", "kind"])
        },
        {
            "name": "delete_block",
            "description": "Delete a block from a scene.",
            "inputSchema": schema(json!({
                "sceneId": {"type": "string"},
                "nodeId": {"type": "string"}
            }), vec!["sceneId", "nodeId"])
        },
        {
            "name": "list_characters",
            "description": "All characters with ids, names, colors and sprite prefixes.",
            "inputSchema": schema(json!({}), vec![])
        },
        {
            "name": "upsert_character",
            "description": "Create or update a character. Omit id to create.",
            "inputSchema": schema(json!({
                "id": {"type": "string"},
                "name": {"type": "string"},
                "color": {"type": "string"},
                "kind": {"type": "string", "enum": ["character", "thought", "narrator"]},
                "spritePrefix": {"type": "string"}
            }), vec![])
        },
        {
            "name": "list_variables",
            "description": "All story variables.",
            "inputSchema": schema(json!({}), vec![])
        },
        {
            "name": "upsert_variable",
            "description": "Create or update a variable (number/bool/text).",
            "inputSchema": schema(json!({
                "id": {"type": "string"},
                "name": {"type": "string"},
                "type": {"type": "string", "enum": ["number", "bool", "text"]},
                "default": {},
                "description": {"type": "string"}
            }), vec!["name"])
        },
        {
            "name": "search_text",
            "description": "Search dialogue and comments across all locales.",
            "inputSchema": schema(json!({
                "query": {"type": "string"},
                "limit": {"type": "integer"}
            }), vec!["query"])
        },
        {
            "name": "request_snapshot",
            "description": "Save a manual snapshot before bulk edits.",
            "inputSchema": schema(json!({
                "label": {"type": "string"}
            }), vec![])
        },
        {
            "name": "list_snapshots",
            "description": "List available history snapshots.",
            "inputSchema": schema(json!({}), vec![])
        },
        {
            "name": "rollback",
            "description": "Roll the project back to a snapshot file name from list_snapshots.",
            "inputSchema": schema(json!({
                "file": {"type": "string"}
            }), vec!["file"])
        },
        {
            "name": "validate_project",
            "description": "Run studio validation (dangling jumps, dead ends, missing assets).",
            "inputSchema": schema(json!({}), vec![])
        }
    ])
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

fn lock<'a>(state: &'a AppState) -> Result<std::sync::MutexGuard<'a, crate::store::Store>, String> {
    state
        .store
        .lock()
        .map_err(|_| "Модель проекта заблокирована".to_string())
}

fn require_open(store: &crate::store::Store) -> Result<(), String> {
    if store.is_open() {
        Ok(())
    } else {
        Err("Проект не открыт в RenPy Studio".into())
    }
}

fn find_scene<'a>(project: &'a Project, scene_id: Option<&str>, title: Option<&str>) -> Option<&'a Scene> {
    if let Some(id) = scene_id {
        if let Some(scene) = project.scene_by_id(id) {
            return Some(scene);
        }
    }
    if let Some(title) = title {
        return project
            .scenes
            .iter()
            .find(|s| s.title.eq_ignore_ascii_case(title) || s.slug == slugify(title));
    }
    None
}

fn scene_json(project: &Project, scene: &Scene) -> Value {
    let labels = crate::generator::scene_labels(project);
    json!({
        "id": scene.id,
        "title": scene.title,
        "slug": scene.slug,
        "label": labels.get(&scene.id),
        "summary": scene.summary,
        "blocks": scene.nodes.iter().map(|n| json!({
            "id": n.id,
            "kind": n.kind,
            "kindLabel": kind_label(&n.kind),
            "params": n.params,
            "source": n.source,
        })).collect::<Vec<_>>()
    })
}

fn resolve_target(project: &Project, target: &str) -> Option<String> {
    if project.scene_by_id(target).is_some() {
        return Some(target.to_string());
    }
    project
        .scenes
        .iter()
        .find(|s| s.title.eq_ignore_ascii_case(target) || s.slug == slugify(target))
        .map(|s| s.id.clone())
}

/// Applies a mutation safely: snapshot, mutate, validate, revert on errors.
fn guarded<F>(state: &AppState, tool: &str, f: F) -> Result<Value, String>
where
    F: FnOnce(&mut Project) -> Result<Value, String>,
{
    let mut store = lock(state)?;
    require_open(&store)?;
    let _ = store.snapshot("auto", &format!("MCP: {tool}"));
    store.checkpoint();
    let result = f(&mut store.project);
    let result = match result {
        Ok(v) => v,
        Err(err) => {
            store.undo();
            return Err(err);
        }
    };
    let assets = store.root.as_deref().map(crate::assets::scan).unwrap_or_default();
    let report = validation::validate(&store.project, &assets);
    if report.errors > 0 {
        let first = report
            .issues
            .iter()
            .find(|i| i.severity == "error")
            .map(|i| i.message.clone())
            .unwrap_or_else(|| "невалидный проект".into());
        store.undo();
        return Err(format!("Изменение отклонено валидатором: {first}"));
    }
    store.save()?;
    drop(store);
    let _ = state.app.emit(
        "studio://project-changed",
        json!({ "reason": "mcp", "tool": tool }),
    );
    Ok(result)
}

fn call_tool(state: &AppState, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "get_project_overview" => {
            let store = lock(state)?;
            require_open(&store)?;
            let project = &store.project;
            let assets = store.root.as_deref().map(crate::assets::scan).unwrap_or_default();
            let report = validation::validate(project, &assets);
            let labels = crate::generator::scene_labels(project);
            Ok(json!({
                "title": project.meta.title,
                "author": project.meta.author,
                "description": project.meta.description,
                "startSceneId": project.meta.start_scene,
                "locales": project.locales,
                "defaultLocale": project.meta.default_locale,
                "projectPath": store.root.as_ref().map(|p| p.display().to_string()),
                "scenes": project.scenes.iter().map(|s| json!({
                    "id": s.id, "title": s.title, "label": labels.get(&s.id),
                    "blocks": s.nodes.len(), "summary": s.summary
                })).collect::<Vec<_>>(),
                "characters": project.characters.iter().map(|c| json!({
                    "id": c.id, "name": c.name, "kind": c.kind, "color": c.color, "spritePrefix": c.sprite_prefix
                })).collect::<Vec<_>>(),
                "variables": project.variables.iter().map(|v| json!({
                    "id": v.id, "name": v.name, "type": v.var_type, "default": v.default
                })).collect::<Vec<_>>(),
                "validation": { "errors": report.errors, "warnings": report.warnings }
            }))
        }
        "get_scene" => {
            let store = lock(state)?;
            require_open(&store)?;
            let scene = find_scene(
                &store.project,
                args.get("sceneId").and_then(|v| v.as_str()),
                args.get("title").and_then(|v| v.as_str()),
            )
            .ok_or("Сцена не найдена")?;
            Ok(scene_json(&store.project, scene))
        }
        "create_scene" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or("Нужен title")?
                .to_string();
            let after = args.get("afterSceneId").and_then(|v| v.as_str()).map(String::from);
            let summary = args
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            guarded(state, "create_scene", |project| {
                let scene = Scene {
                    id: new_id(),
                    title: title.clone(),
                    slug: slugify(&title),
                    summary: summary.clone(),
                    label_name: None,
                    color: None,
                    nodes: Vec::new(),
                };
                let index = after
                    .as_deref()
                    .and_then(|id| project.scenes.iter().position(|s| s.id == id))
                    .map(|i| i + 1)
                    .unwrap_or(project.scenes.len());
                let id = scene.id.clone();
                project.scenes.insert(index, scene);
                if project.meta.start_scene.is_none() {
                    project.meta.start_scene = Some(id.clone());
                }
                let labels = crate::generator::scene_labels(project);
                Ok(json!({ "sceneId": id, "title": title, "label": labels.get(&id) }))
            })
        }
        "update_scene" => {
            let scene_id = args
                .get("sceneId")
                .and_then(|v| v.as_str())
                .ok_or("Нужен sceneId")?
                .to_string();
            guarded(state, "update_scene", |project| {
                if let Some(blocks) = args.get("blocks").and_then(|v| v.as_array()) {
                    let mut nodes = Vec::new();
                    for block in blocks {
                        let kind = block
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .unwrap_or(NodeKind::CODE);
                        let params = block
                            .get("params")
                            .cloned()
                            .unwrap_or_else(|| default_params(project, kind));
                        nodes.push(Node {
                            id: block
                                .get("id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(new_id),
                            kind: kind.to_string(),
                            params,
                            source: NodeSource::Mcp,
                            locked: false,
                        });
                    }
                    let scene = project
                        .scenes
                        .iter_mut()
                        .find(|s| s.id == scene_id)
                        .ok_or("Сцена не найдена")?;
                    scene.nodes = nodes;
                }
                let scene = project
                    .scenes
                    .iter_mut()
                    .find(|s| s.id == scene_id)
                    .ok_or("Сцена не найдена")?;
                if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
                    scene.title = title.to_string();
                    scene.slug = slugify(title);
                }
                if let Some(summary) = args.get("summary").and_then(|v| v.as_str()) {
                    scene.summary = summary.to_string();
                }
                Ok(json!({ "sceneId": scene_id }))
            })
        }
        "create_say_block" => {
            let scene_id = args
                .get("sceneId")
                .and_then(|v| v.as_str())
                .ok_or("Нужен sceneId")?
                .to_string();
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("Нужен text")?
                .to_string();
            let character = args.get("character").and_then(|v| v.as_str()).map(String::from);
            let locale = args.get("locale").and_then(|v| v.as_str()).map(String::from);
            let after = args.get("afterNodeId").and_then(|v| v.as_str()).map(String::from);
            let index = args.get("index").and_then(|v| v.as_u64()).map(|v| v as usize);
            let image = args.get("image").and_then(|v| v.as_str()).map(String::from);
            guarded(state, "create_say_block", |project| {
                let locale = locale
                    .clone()
                    .unwrap_or_else(|| project.meta.default_locale.clone());
                let mut localized = LocalizedText::new();
                for loc in &project.locales {
                    localized.insert(loc.clone(), text.clone());
                }
                if localized.is_empty() {
                    localized.insert(locale.clone(), text.clone());
                }
                let who = character.as_deref().and_then(|c| {
                    project
                        .characters
                        .iter()
                        .find(|ch| {
                            ch.id == c
                                || ch.name.eq_ignore_ascii_case(c)
                                || ch.sprite_prefix.as_deref() == Some(c)
                        })
                        .map(|ch| ch.id.clone())
                });
                let node = Node {
                    id: new_id(),
                    kind: NodeKind::SAY.into(),
                    params: json!({
                        "who": who,
                        "text": localized,
                        "image": image,
                        "voice": null,
                        "cps": null
                    }),
                    source: NodeSource::Mcp,
                    locked: false,
                };
                let node_id = node.id.clone();
                let scene = project
                    .scenes
                    .iter_mut()
                    .find(|s| s.id == scene_id)
                    .ok_or("Сцена не найдена")?;
                let pos = index
                    .or_else(|| {
                        after.as_deref().and_then(|id| {
                            scene.nodes.iter().position(|n| n.id == id).map(|i| i + 1)
                        })
                    })
                    .unwrap_or(scene.nodes.len())
                    .min(scene.nodes.len());
                scene.nodes.insert(pos, node);
                Ok(json!({ "nodeId": node_id, "index": pos }))
            })
        }
        "create_menu_block" => {
            let scene_id = args
                .get("sceneId")
                .and_then(|v| v.as_str())
                .ok_or("Нужен sceneId")?
                .to_string();
            let options = args
                .get("options")
                .and_then(|v| v.as_array())
                .cloned()
                .ok_or("Нужен массив options")?;
            let prompt = args
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            guarded(state, "create_menu_block", |project| {
                let mut prompt_map = LocalizedText::new();
                for loc in &project.locales {
                    prompt_map.insert(loc.clone(), prompt.clone());
                }
                let mut menu_options = Vec::new();
                for option in &options {
                    let text = option
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let mut localized = LocalizedText::new();
                    for loc in &project.locales {
                        localized.insert(loc.clone(), text.clone());
                    }
                    let target = option
                        .get("target")
                        .and_then(|v| v.as_str())
                        .and_then(|t| resolve_target(project, t));
                    menu_options.push(MenuOption {
                        id: new_id(),
                        text: localized,
                        condition: option
                            .get("condition")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        target,
                        set_vars: Vec::new(),
                    });
                }
                let node = Node {
                    id: new_id(),
                    kind: NodeKind::MENU.into(),
                    params: serde_json::to_value(MenuParams {
                        prompt: prompt_map,
                        options: menu_options,
                        with_caption: !prompt.is_empty(),
                    })
                    .unwrap_or(Value::Null),
                    source: NodeSource::Mcp,
                    locked: false,
                };
                let node_id = node.id.clone();
                let scene = project
                    .scenes
                    .iter_mut()
                    .find(|s| s.id == scene_id)
                    .ok_or("Сцена не найдена")?;
                scene.nodes.push(node);
                Ok(json!({ "nodeId": node_id }))
            })
        }
        "add_block" => {
            let scene_id = args
                .get("sceneId")
                .and_then(|v| v.as_str())
                .ok_or("Нужен sceneId")?
                .to_string();
            let kind = args
                .get("kind")
                .and_then(|v| v.as_str())
                .ok_or("Нужен kind")?
                .to_string();
            if !NodeKind::ALL.contains(&kind.as_str()) {
                return Err(format!("Неизвестный тип блока: {kind}"));
            }
            let explicit = args.get("params").cloned();
            let index = args.get("index").and_then(|v| v.as_u64()).map(|v| v as usize);
            guarded(state, "add_block", |project| {
                let params = explicit.unwrap_or_else(|| default_params(project, &kind));
                let node = Node {
                    id: new_id(),
                    kind: kind.clone(),
                    params,
                    source: NodeSource::Mcp,
                    locked: false,
                };
                let node_id = node.id.clone();
                let scene = project
                    .scenes
                    .iter_mut()
                    .find(|s| s.id == scene_id)
                    .ok_or("Сцена не найдена")?;
                let pos = index.unwrap_or(scene.nodes.len()).min(scene.nodes.len());
                scene.nodes.insert(pos, node);
                Ok(json!({ "nodeId": node_id, "index": pos }))
            })
        }
        "delete_block" => {
            let scene_id = args
                .get("sceneId")
                .and_then(|v| v.as_str())
                .ok_or("Нужен sceneId")?
                .to_string();
            let node_id = args
                .get("nodeId")
                .and_then(|v| v.as_str())
                .ok_or("Нужен nodeId")?
                .to_string();
            guarded(state, "delete_block", |project| {
                let scene = project
                    .scenes
                    .iter_mut()
                    .find(|s| s.id == scene_id)
                    .ok_or("Сцена не найдена")?;
                let before = scene.nodes.len();
                scene.nodes.retain(|n| n.id != node_id);
                if before == scene.nodes.len() {
                    return Err("Блок не найден".into());
                }
                Ok(json!({ "deleted": node_id }))
            })
        }
        "list_characters" => {
            let store = lock(state)?;
            require_open(&store)?;
            Ok(json!(store.project.characters))
        }
        "upsert_character" => guarded(state, "upsert_character", |project| {
            let id = args.get("id").and_then(|v| v.as_str()).map(String::from);
            let mut character = match id.as_deref().and_then(|id| {
                project.characters.iter().find(|c| c.id == id).cloned()
            }) {
                Some(existing) => existing,
                None => Character {
                    id: new_id(),
                    name: String::new(),
                    kind: CharacterKind::Character,
                    sprite_folder: None,
                    color: None,
                    avatar: None,
                    sprite_prefix: None,
                    voice_tag: None,
                    prefix: None,
                    suffix: None,
                    notes: String::new(),
                },
            };
            if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
                character.name = name.to_string();
            }
            if let Some(color) = args.get("color").and_then(|v| v.as_str()) {
                character.color = Some(color.to_string());
            }
            if let Some(prefix) = args.get("spritePrefix").and_then(|v| v.as_str()) {
                character.sprite_prefix = Some(slugify(prefix));
            }
            if let Some(kind) = args.get("kind").and_then(|v| v.as_str()) {
                character.kind = match kind {
                    "narrator" => CharacterKind::Narrator,
                    "thought" => CharacterKind::Thought,
                    _ => CharacterKind::Character,
                };
            }
            if character.name.trim().is_empty() && character.kind == CharacterKind::Character {
                return Err("У персонажа должно быть имя".into());
            }
            let out = serde_json::to_value(&character).unwrap_or(Value::Null);
            match project.characters.iter_mut().find(|c| c.id == character.id) {
                Some(existing) => *existing = character,
                None => project.characters.push(character),
            }
            Ok(out)
        }),
        "list_variables" => {
            let store = lock(state)?;
            require_open(&store)?;
            Ok(json!(store.project.variables))
        }
        "upsert_variable" => guarded(state, "upsert_variable", |project| {
            let id = args.get("id").and_then(|v| v.as_str()).map(String::from);
            let mut variable = match id.as_deref().and_then(|id| {
                project.variables.iter().find(|v| v.id == id).cloned()
            }) {
                Some(existing) => existing,
                None => Variable {
                    id: new_id(),
                    name: String::new(),
                    var_type: VarType::Number,
                    default: json!(0),
                    description: String::new(),
                },
            };
            if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
                variable.name = slugify(name);
            }
            if let Some(kind) = args.get("type").and_then(|v| v.as_str()) {
                variable.var_type = match kind {
                    "bool" => VarType::Bool,
                    "text" => VarType::Text,
                    _ => VarType::Number,
                };
            }
            if let Some(default) = args.get("default") {
                variable.default = default.clone();
            }
            if let Some(description) = args.get("description").and_then(|v| v.as_str()) {
                variable.description = description.to_string();
            }
            if variable.name.trim().is_empty() {
                return Err("У переменной должно быть имя".into());
            }
            let out = json!({
                "id": variable.id, "name": variable.name, "type": variable.var_type,
                "default": variable.default_value()
            });
            match project.variables.iter_mut().find(|v| v.id == variable.id) {
                Some(existing) => *existing = variable,
                None => project.variables.push(variable),
            }
            Ok(out)
        }),
        "search_text" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or("Нужен query")?
                .to_lowercase();
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
            let store = lock(state)?;
            require_open(&store)?;
            let mut hits = Vec::new();
            for scene in &store.project.scenes {
                for node in &scene.nodes {
                    let texts: Vec<String> = match node.kind.as_str() {
                        NodeKind::SAY => node.typed::<SayParams>().text.values().cloned().collect(),
                        NodeKind::NARRATE => {
                            node.typed::<NarrateParams>().text.values().cloned().collect()
                        }
                        NodeKind::COMMENT => {
                            node.typed::<CommentParams>().text.values().cloned().collect()
                        }
                        NodeKind::MENU => node.typed::<MenuParams>().prompt.values().cloned().collect(),
                        _ => vec![],
                    };
                    if texts.iter().any(|t| t.to_lowercase().contains(&query)) {
                        hits.push(json!({
                            "sceneId": scene.id,
                            "sceneTitle": scene.title,
                            "nodeId": node.id,
                            "kind": node.kind,
                            "text": texts.first().cloned().unwrap_or_default()
                        }));
                        if hits.len() >= limit {
                            break;
                        }
                    }
                }
            }
            Ok(json!({ "hits": hits, "count": hits.len() }))
        }
        "request_snapshot" => {
            let label = args
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("Снимок от ИИ-агента")
                .to_string();
            let mut store = lock(state)?;
            require_open(&store)?;
            if store.dirty {
                store.save()?;
            }
            let info = store.snapshot("manual", &label)?;
            Ok(serde_json::to_value(info).unwrap_or(Value::Null))
        }
        "list_snapshots" => {
            let store = lock(state)?;
            Ok(json!(store.list_snapshots()?))
        }
        "rollback" => {
            let file = args
                .get("file")
                .and_then(|v| v.as_str())
                .ok_or("Нужен file")?
                .to_string();
            let mut store = lock(state)?;
            require_open(&store)?;
            store.restore_snapshot(&file)?;
            drop(store);
            let _ = state
                .app
                .emit("studio://project-changed", json!({ "reason": "mcp-rollback" }));
            Ok(json!({ "restored": file }))
        }
        "validate_project" => {
            let store = lock(state)?;
            require_open(&store)?;
            let assets = store.root.as_deref().map(crate::assets::scan).unwrap_or_default();
            Ok(serde_json::to_value(validation::validate(&store.project, &assets)).unwrap_or(Value::Null))
        }
        other => Err(format!("Неизвестный инструмент: {other}")),
    }
}
