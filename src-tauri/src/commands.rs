//! Tauri IPC commands. Every project mutation goes through here (the frontend
//! never edits the model directly), and the same functions back the MCP server.

#[allow(unused_imports)]
use crate::assets::{self, AssetInfo};
use crate::diff::{self, DiffEntry};
use crate::exporter::{self, ExportReport};
use crate::generator::GenOptions;
use crate::model::*;
use crate::sdk::{BuildReport, LintReport, SdkInfo, SdkService};
use crate::settings::{RecentProject, Settings};
use crate::store::{SnapshotInfo, Store, StoreStatus};
use crate::validation::{self, GraphData, ValidationReport};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct AppState {
    pub store: Mutex<Store>,
    pub settings: Mutex<Settings>,
    pub sdk: SdkService,
    pub app: AppHandle,
    pub watcher: Mutex<Option<crate::watcher::ProjectWatcher>>,
    pub mcp: Mutex<Option<crate::mcp::McpHandle>>,
    pub last_export: Mutex<Option<ExportReport>>,
    pub build: Mutex<BuildState>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildState {
    pub running: bool,
    pub package: String,
    pub log: Vec<String>,
    pub ok: Option<bool>,
    pub artifact: Option<String>,
}

pub type Shared = Arc<AppState>;

impl AppState {
    pub fn new(app: AppHandle) -> Shared {
        let settings = Settings::load();
        Arc::new(AppState {
            store: Mutex::new(Store::default()),
            settings: Mutex::new(settings),
            sdk: SdkService::default(),
            app,
            watcher: Mutex::new(None),
            mcp: Mutex::new(None),
            last_export: Mutex::new(None),
            build: Mutex::new(BuildState::default()),
        })
    }

    pub fn root(&self) -> Option<PathBuf> {
        self.store.lock().ok().and_then(|s| s.root.clone())
    }

    pub fn sdk_info(&self) -> Option<SdkInfo> {
        let settings = self.settings.lock().ok()?;
        let root = self.root();
        self.sdk
            .detect(settings.sdk_path.as_deref(), root.as_deref(), false)
    }

    pub fn emit_project_changed(&self, reason: &str) {
        let payload = json!({ "reason": reason, "at": crate::model::now_iso() });
        let _ = self.app.emit("studio://project-changed", payload);
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StateEnvelope {
    pub project: Project,
    pub status: StoreStatus,
    pub issues: ValidationReport,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub settings: Settings,
    pub sdk: Option<SdkInfo>,
    pub project: Option<StateEnvelope>,
    pub recent: Vec<RecentProject>,
    pub version: String,
    pub mcp_running: bool,
    pub mcp_port: Option<u16>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MappedProblem {
    pub file: String,
    pub line: u32,
    pub message: String,
    pub severity: String,
    pub node_id: Option<String>,
    pub scene_id: Option<String>,
    pub scene_title: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LintPayload {
    pub report: LintReport,
    pub mapped: Vec<MappedProblem>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub ok: bool,
    pub message: String,
    pub pid: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportPayload {
    pub project: Option<Project>,
    pub report: crate::importer::ImportReport,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lock_store(state: &AppState) -> Result<std::sync::MutexGuard<'_, Store>, String> {
    state
        .store
        .lock()
        .map_err(|_| "Внутренняя ошибка: модель проекта заблокирована".to_string())
}

pub fn envelope_of(store: &Store) -> StateEnvelope {
    let project = store.project.clone();
    let assets = store
        .root
        .as_deref()
        .map(assets::scan)
        .unwrap_or_default();
    let issues = validation::validate(&project, &assets);
    StateEnvelope {
        project,
        status: store.status(),
        issues,
    }
}

fn envelope(state: &AppState) -> Result<StateEnvelope, String> {
    let store = lock_store(state)?;
    Ok(envelope_of(&store))
}

fn default_locale_text(project: &Project, value: &str) -> LocalizedText {
    let mut map = LocalizedText::new();
    for locale in &project.locales {
        map.insert(locale.clone(), value.to_string());
    }
    if map.is_empty() {
        map.insert(project.meta.default_locale.clone(), value.to_string());
    }
    map
}

/// Sensible starting parameters for each block kind.
pub fn default_params(project: &Project, kind: &str) -> Value {
    match kind {
        NodeKind::SAY => {
            let who = project
                .characters
                .iter()
                .find(|c| c.kind == CharacterKind::Character)
                .map(|c| c.id.clone());
            json!({
                "who": who,
                "text": default_locale_text(project, ""),
                "image": null,
                "voice": null,
                "cps": null
            })
        }
        NodeKind::NARRATE => json!({ "text": default_locale_text(project, "") }),
        NodeKind::COMMENT => json!({ "text": default_locale_text(project, ""), "internalOnly": false }),
        NodeKind::SCENE => json!({ "bg": "", "transition": "dissolve", "duration": null }),
        NodeKind::SHOW => json!({
            "who": project.characters.first().map(|c| c.id.clone()).unwrap_or_default(),
            "image": "",
            "asset": null,
            "position": "center",
            "customX": null,
            "transition": "dissolve",
            "flip": false
        }),
        NodeKind::HIDE => json!({ "who": project.characters.first().map(|c| c.id.clone()), "image": null, "transition": "dissolve" }),
        NodeKind::POSE => json!({
            "who": project.characters.first().map(|c| c.id.clone()).unwrap_or_default(),
            "image": "",
            "asset": null,
            "transition": "dissolve"
        }),
        NodeKind::ANIM => json!({ "who": project.characters.first().map(|c| c.id.clone()).unwrap_or_default(), "preset": "shake" }),
        NodeKind::MENU => json!({
            "prompt": default_locale_text(project, ""),
            "options": [
                { "id": new_id(), "text": default_locale_text(project, "Вариант 1"), "condition": null, "target": null, "setVars": [] },
                { "id": new_id(), "text": default_locale_text(project, "Вариант 2"), "condition": null, "target": null, "setVars": [] }
            ],
            "withCaption": true
        }),
        NodeKind::JUMP => json!({ "target": null, "transition": null }),
        NodeKind::RETURN => json!({ "toMainMenu": false }),
        NodeKind::IF => json!({
            "mode": "builder",
            "variable": project.variables.first().map(|v| v.name.clone()),
            "op": "gte",
            "value": 1,
            "expression": "",
            "thenTarget": null,
            "elseTarget": null,
            "hasElse": false
        }),
        NodeKind::PAUSE => json!({ "seconds": 0.0, "hard": false }),
        NodeKind::LABEL => json!({ "name": "" }),
        NodeKind::MUSIC => json!({ "action": "play", "file": null, "fadeout": 0.0, "loop": true, "channel": null }),
        NodeKind::SOUND => json!({ "file": "", "loop": false, "volume": null }),
        NodeKind::SET => json!({
            "variable": project.variables.first().map(|v| v.name.clone()).unwrap_or_default(),
            "op": "set",
            "value": 0
        }),
        NodeKind::CALL_SCREEN => json!({ "screen": "", "withNone": true }),
        NodeKind::HIDE_SCREEN => json!({ "screen": "" }),
        NodeKind::CODE => json!({ "code": "", "language": "renpy" }),
        NodeKind::POINTCLICK => json!({
            "bg": "",
            "prompt": default_locale_text(project, ""),
            "hotspots": [],
            "hoverSound": null,
            "clickSound": null
        }),
        _ => json!({}),
    }
}

/// Rebuilds validation + notifies the UI after an MCP-style mutation.
fn finish_mutation(state: &AppState, reason: &str) -> Result<StateEnvelope, String> {
    let env = envelope(state)?;
    state.emit_project_changed(reason);
    Ok(env)
}

fn regenerate(state: &AppState, force: bool) -> Result<ExportReport, String> {
    let (root, project, locale) = {
        let store = lock_store(state)?;
        let root = store.root.clone().ok_or("Проект не открыт")?;
        (
            root,
            store.project.clone(),
            store.project.meta.default_locale.clone(),
        )
    };
    let opts = GenOptions {
        locale: if locale.is_empty() { "ru".into() } else { locale },
        force,
    };
    crate::watcher::suppress_for(2500);
    let (mut report, _, _) = exporter::export(&project, &root, &opts)?;
    crate::sdk::write_project_settings(
        &root,
        state
            .settings
            .lock()
            .ok()
            .and_then(|s| s.sdk_path.clone())
            .as_deref(),
    )
    .ok();
    if !report.conflicts.is_empty() {
        report
            .warnings
            .push("Часть файлов не перезаписана: они изменены снаружи.".into());
    }
    if let Ok(mut last) = state.last_export.lock() {
        *last = Some(report.clone());
    }
    Ok(report)
}

fn open_root(state: &AppState, root: &Path) -> Result<StateEnvelope, String> {
    let mut store = Store::open(root)?;
    let env = envelope_of(&store);
    store.clear_history();
    {
        let mut guard = lock_store(state)?;
        *guard = store;
    }
    // Restart the file watcher for the new project.
    if let Ok(mut watcher) = state.watcher.lock() {
        *watcher = None;
        match crate::watcher::start(root.to_path_buf(), state.app.clone()) {
            Ok(w) => *watcher = Some(w),
            Err(err) => eprintln!("watcher: {err}"),
        }
    }
    {
        if let Ok(mut settings) = state.settings.lock() {
            settings.remember_project(&root.display().to_string(), &env.project.meta.title);
            let _ = settings.save();
        }
    }
    state.emit_project_changed("opened");
    Ok(env)
}

// ---------------------------------------------------------------------------
// Bootstrap & settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn app_bootstrap(state: State<'_, Shared>) -> Result<Bootstrap, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let sdk = state.sdk_info();
    let project = {
        let store = lock_store(&state)?;
        if store.is_open() {
            Some(envelope_of(&store))
        } else {
            None
        }
    };
    let (mcp_running, mcp_port) = state
        .mcp
        .lock()
        .map(|g| match g.as_ref() {
            Some(h) => (true, Some(h.port)),
            None => (false, None),
        })
        .unwrap_or((false, None));
    Ok(Bootstrap {
        recent: settings.recent_projects.clone(),
        settings,
        sdk,
        project,
        version: env!("CARGO_PKG_VERSION").to_string(),
        mcp_running,
        mcp_port,
    })
}

#[tauri::command]
pub fn settings_save(state: State<'_, Shared>, settings: Settings) -> Result<Settings, String> {
    let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
    let mut incoming = settings;
    // The webview never receives the key, so an empty field means "keep the
    // stored key" — only a non-empty value replaces it.
    if incoming.ai.api_key.trim().is_empty() {
        incoming.ai.api_key = guard.ai.api_key.clone();
        if incoming.ai.api_key_enc.is_none() {
            incoming.ai.api_key_enc = guard.ai.api_key_enc.clone();
            incoming.ai.api_key_plain = guard.ai.api_key_plain.clone();
        }
    }
    incoming.ai.has_key = !incoming.ai.api_key.is_empty();
    *guard = incoming;
    guard.save()?;
    state.sdk.invalidate();
    Ok(guard.clone())
}

/// Drops the stored provider key (the UI cannot clear it by sending an empty
/// field, because empty means "unchanged").
#[tauri::command]
pub fn ai_forget_key(state: State<'_, Shared>) -> Result<Settings, String> {
    let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
    guard.ai.api_key.clear();
    guard.ai.api_key_enc = None;
    guard.ai.api_key_plain = None;
    guard.ai.has_key = false;
    guard.save()?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn sdk_detect(state: State<'_, Shared>, force: Option<bool>) -> Option<SdkInfo> {
    let preferred = state
        .settings
        .lock()
        .ok()
        .and_then(|s| s.sdk_path.clone());
    let root = state.root();
    state
        .sdk
        .detect(preferred.as_deref(), root.as_deref(), force.unwrap_or(false))
}

#[tauri::command]
pub fn sdk_set_path(state: State<'_, Shared>, path: Option<String>) -> Option<SdkInfo> {
    if let Ok(mut settings) = state.settings.lock() {
        settings.sdk_path = path.clone();
        let _ = settings.save();
    }
    state.sdk.invalidate();
    let root = state.root();
    let info = state.sdk.detect(path.as_deref(), root.as_deref(), true);
    if let (Some(root), Some(info)) = (root, info.as_ref()) {
        let _ = crate::sdk::write_project_settings(&root, Some(&info.root));
    }
    info
}

// ---------------------------------------------------------------------------
// Project lifecycle
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn project_new(
    state: State<'_, Shared>,
    parent: String,
    title: String,
) -> Result<StateEnvelope, String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("Введите название новеллы".into());
    }
    let slug = slugify(&title);
    let parent_path = PathBuf::from(parent);
    let mut target = parent_path.join(&title);
    if target.exists() {
        let mut n = 2;
        loop {
            let candidate = parent_path.join(format!("{title} {n}"));
            if !candidate.exists() {
                target = candidate;
                break;
            }
            n += 1;
            if n > 200 {
                return Err("Не удалось подобрать имя папки проекта".into());
            }
        }
    }
    let sdk_root = state.sdk_info().map(|s| PathBuf::from(s.root));
    crate::templates::scaffold(&target, sdk_root.as_deref(), &title, &slug)?;
    let mut store = Store::create(&target, &title)?;
    store.project.meta.renpy_version = state
        .sdk_info()
        .map(|s| s.version)
        .unwrap_or_else(|| "8.x".into());
    let _ = store.save();
    let env = envelope_of(&store);
    {
        let mut guard = lock_store(&state)?;
        *guard = store;
    }
    // First export so `game/` is immediately runnable.
    let _ = regenerate(&state, true);
    open_root(&state, &target)?;
    state.emit_project_changed("created");
    Ok(env)
}

#[tauri::command]
pub fn project_open(state: State<'_, Shared>, path: String) -> Result<StateEnvelope, String> {
    let path = PathBuf::from(&path);
    let root = if path.join("renpy-studio").join("project.json").is_file() {
        path
    } else if path.is_file() {
        path.parent()
            .map(|p| p.to_path_buf())
            .ok_or("Не удалось определить папку проекта")?
    } else {
        path
    };
    if !root.join("renpy-studio").join("project.json").is_file() {
        return Err(format!(
            "В папке нет проекта RenPy Studio ({}).",
            root.join("renpy-studio/project.json").display()
        ));
    }
    open_root(&state, &root)
}

#[tauri::command]
pub fn project_create_in_existing(
    state: State<'_, Shared>,
    path: String,
    title: String,
) -> Result<StateEnvelope, String> {
    let root = PathBuf::from(path);
    let sdk_root = state.sdk_info().map(|s| PathBuf::from(s.root));
    let slug = slugify(&title);
    crate::templates::scaffold(&root, sdk_root.as_deref(), &title, &slug)?;
    let store = Store::create(&root, &title)?;
    {
        let mut guard = lock_store(&state)?;
        *guard = store;
    }
    let _ = regenerate(&state, true);
    open_root(&state, &root)
}

#[tauri::command]
pub fn project_state(state: State<'_, Shared>) -> Result<Option<StateEnvelope>, String> {
    let store = lock_store(&state)?;
    if store.is_open() {
        Ok(Some(envelope_of(&store)))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn project_close(state: State<'_, Shared>) -> Result<(), String> {
    if let Ok(mut watcher) = state.watcher.lock() {
        *watcher = None;
    }
    let mut store = lock_store(&state)?;
    *store = Store::default();
    Ok(())
}

#[tauri::command]
pub fn project_save(state: State<'_, Shared>) -> Result<StoreStatus, String> {
    let mut store = lock_store(&state)?;
    if !store.is_open() {
        return Err("Проект не открыт".into());
    }
    store.save()?;
    let status = store.status();
    drop(store);
    let _ = state
        .app
        .emit("studio://saved", json!({ "at": crate::model::now_iso() }));
    Ok(status)
}

#[tauri::command]
pub fn project_reload(state: State<'_, Shared>) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        if !store.is_open() {
            return Err("Проект не открыт".into());
        }
        store.reload()?;
    }
    state.emit_project_changed("reloaded");
    envelope(&state)
}

#[tauri::command]
pub fn project_export(
    state: State<'_, Shared>,
    force: Option<bool>,
) -> Result<ExportReport, String> {
    {
        let mut store = lock_store(&state)?;
        if !store.is_open() {
            return Err("Проект не открыт".into());
        }
        if store.dirty {
            store.save()?;
        }
    }
    let report = regenerate(&state, force.unwrap_or(false))?;
    let _ = state
        .app
        .emit("studio://exported", json!({ "written": report.written.len() }));
    Ok(report)
}

#[tauri::command]
pub fn project_validate(state: State<'_, Shared>) -> Result<ValidationReport, String> {
    let store = lock_store(&state)?;
    let assets = store.root.as_deref().map(assets::scan).unwrap_or_default();
    Ok(validation::validate(&store.project, &assets))
}

#[tauri::command]
pub fn project_graph(state: State<'_, Shared>) -> Result<GraphData, String> {
    let store = lock_store(&state)?;
    Ok(validation::graph_data(&store.project))
}

#[tauri::command]
pub fn assets_list(state: State<'_, Shared>) -> Result<Vec<AssetInfo>, String> {
    let store = lock_store(&state)?;
    Ok(store.root.as_deref().map(assets::scan).unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Undo / redo
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn undo(state: State<'_, Shared>) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        if !store.undo() {
            return Err("Нечего отменять".into());
        }
    }
    envelope(&state)
}

#[tauri::command]
pub fn redo(state: State<'_, Shared>) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        if !store.redo() {
            return Err("Нечего повторять".into());
        }
    }
    envelope(&state)
}

// ---------------------------------------------------------------------------
// Scenes
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn scene_create(
    state: State<'_, Shared>,
    title: String,
    index: Option<usize>,
) -> Result<StateEnvelope, String> {
    let title = title.trim().to_string();
    let title = if title.is_empty() {
        "Новая сцена".to_string()
    } else {
        title
    };
    {
        let mut store = lock_store(&state)?;
        if !store.is_open() {
            return Err("Проект не открыт".into());
        }
        store.checkpoint();
        let scene = Scene {
            id: new_id(),
            title: title.clone(),
            slug: slugify(&title),
            summary: String::new(),
            label_name: None,
            color: None,
            nodes: Vec::new(),
        };
        let pos = index.unwrap_or(store.project.scenes.len()).min(store.project.scenes.len());
        store.project.scenes.insert(pos, scene.clone());
        if store.project.meta.start_scene.is_none() {
            store.project.meta.start_scene = Some(scene.id.clone());
        }
    }
    finish_mutation(&state, "scene-created")
}

#[tauri::command]
pub fn scene_update(
    state: State<'_, Shared>,
    scene_id: String,
    title: Option<String>,
    summary: Option<String>,
    color: Option<String>,
    slug_value: Option<String>,
    label_name: Option<String>,
) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint_tagged(&format!("scene:{scene_id}"));
        let scene = store
            .project
            .scenes
            .iter_mut()
            .find(|s| s.id == scene_id)
            .ok_or("Сцена не найдена")?;
        if let Some(title) = title {
            scene.title = title;
            if scene.slug.trim().is_empty() {
                scene.slug = slugify(&scene.title);
            }
        }
        if let Some(summary) = summary {
            scene.summary = summary;
        }
        if let Some(color) = color {
            scene.color = if color.trim().is_empty() { None } else { Some(color) };
        }
        if let Some(slug_value) = slug_value {
            scene.slug = slugify(&slug_value);
        }
        if let Some(label_name) = label_name {
            let cleaned = label_name.trim().to_string();
            let has_word = cleaned.chars().any(|c| c.is_alphanumeric());
            scene.label_name = if has_word {
                Some(crate::model::slugify(&cleaned))
            } else {
                None
            };
        }
    }
    finish_mutation(&state, "scene-updated")
}

#[tauri::command]
pub fn scene_delete(state: State<'_, Shared>, scene_id: String) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        let before = store.project.scenes.len();
        store.project.scenes.retain(|s| s.id != scene_id);
        if store.project.scenes.len() == before {
            return Err("Сцена не найдена".into());
        }
        if store.project.meta.start_scene.as_deref() == Some(scene_id.as_str()) {
            store.project.meta.start_scene = store.project.scenes.first().map(|s| s.id.clone());
        }
        // Clear dangling references so the project stays valid.
        for scene in &mut store.project.scenes {
            for node in &mut scene.nodes {
                match node.kind.as_str() {
                    NodeKind::JUMP => {
                        let mut p: JumpParams = node.typed();
                        if p.target.as_deref() == Some(scene_id.as_str()) {
                            p.target = None;
                            node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                        }
                    }
                    NodeKind::MENU => {
                        let mut p: MenuParams = node.typed();
                        let mut changed = false;
                        for opt in &mut p.options {
                            if opt.target.as_deref() == Some(scene_id.as_str()) {
                                opt.target = None;
                                changed = true;
                            }
                        }
                        if changed {
                            node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                        }
                    }
                    NodeKind::IF => {
                        let mut p: IfParams = node.typed();
                        let mut changed = false;
                        if p.then_target.as_deref() == Some(scene_id.as_str()) {
                            p.then_target = None;
                            changed = true;
                        }
                        if p.else_target.as_deref() == Some(scene_id.as_str()) {
                            p.else_target = None;
                            changed = true;
                        }
                        if changed {
                            node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    finish_mutation(&state, "scene-deleted")
}

#[tauri::command]
pub fn scene_duplicate(state: State<'_, Shared>, scene_id: String) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        let index = store
            .project
            .scenes
            .iter()
            .position(|s| s.id == scene_id)
            .ok_or("Сцена не найдена")?;
        let mut copy = store.project.scenes[index].clone();
        copy.id = new_id();
        copy.label_name = None;
        copy.title = format!("{} (копия)", copy.title);
        copy.slug = slugify(&copy.title);
        for node in &mut copy.nodes {
            node.id = new_id();
        }
        store.project.scenes.insert(index + 1, copy);
    }
    finish_mutation(&state, "scene-duplicated")
}

#[tauri::command]
pub fn scene_reorder(
    state: State<'_, Shared>,
    scene_id: String,
    to_index: usize,
) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        let from = store
            .project
            .scenes
            .iter()
            .position(|s| s.id == scene_id)
            .ok_or("Сцена не найдена")?;
        if from != to_index {
            store.checkpoint();
            let scene = store.project.scenes.remove(from);
            let pos = to_index.min(store.project.scenes.len());
            store.project.scenes.insert(pos, scene);
        }
    }
    finish_mutation(&state, "scene-reordered")
}

#[tauri::command]
pub fn scene_set_start(state: State<'_, Shared>, scene_id: String) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        store.project.meta.start_scene = Some(scene_id);
    }
    finish_mutation(&state, "start-scene")
}

// ---------------------------------------------------------------------------
// Blocks
// ---------------------------------------------------------------------------

fn with_scene_mut<T>(
    store: &mut Store,
    scene_id: &str,
    f: impl FnOnce(&mut Scene) -> Result<T, String>,
) -> Result<T, String> {
    let scene = store
        .project
        .scenes
        .iter_mut()
        .find(|s| s.id == scene_id)
        .ok_or("Сцена не найдена")?;
    f(scene)
}

#[tauri::command]
pub fn node_add(
    state: State<'_, Shared>,
    scene_id: String,
    kind: String,
    index: Option<usize>,
    params: Option<Value>,
    source: Option<NodeSource>,
) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        if !NodeKind::ALL.contains(&kind.as_str()) {
            return Err(format!("Неизвестный тип блока: {kind}"));
        }
        let params = params.unwrap_or_else(|| default_params(&store.project, &kind));
        store.checkpoint();
        let node = Node {
            id: new_id(),
            kind: kind.clone(),
            params,
            source: source.unwrap_or(NodeSource::Studio),
            locked: false,
        };
        let node_id = node.id.clone();
        with_scene_mut(&mut store, &scene_id, |scene| {
            let pos = index.unwrap_or(scene.nodes.len()).min(scene.nodes.len());
            scene.nodes.insert(pos, node);
            Ok(())
        })?;
        let _ = node_id;
    }
    finish_mutation(&state, "node-added")
}

#[tauri::command]
pub fn node_update(
    state: State<'_, Shared>,
    scene_id: String,
    node_id: String,
    params: Value,
) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint_tagged(&format!("node:{node_id}"));
        with_scene_mut(&mut store, &scene_id, |scene| {
            let node = scene
                .nodes
                .iter_mut()
                .find(|n| n.id == node_id)
                .ok_or("Блок не найден")?;
            if node.locked {
                return Err("Импортированный блок защищён от правки".into());
            }
            node.params = params;
            Ok(())
        })?;
    }
    finish_mutation(&state, "node-updated")
}

#[tauri::command]
pub fn node_delete(
    state: State<'_, Shared>,
    scene_id: String,
    node_id: String,
) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        with_scene_mut(&mut store, &scene_id, |scene| {
            let before = scene.nodes.len();
            scene.nodes.retain(|n| n.id != node_id);
            if scene.nodes.len() == before {
                return Err("Блок не найден".into());
            }
            Ok(())
        })?;
    }
    finish_mutation(&state, "node-deleted")
}

#[tauri::command]
pub fn node_move(
    state: State<'_, Shared>,
    scene_id: String,
    node_id: String,
    to_index: usize,
) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        with_scene_mut(&mut store, &scene_id, |scene| {
            let from = scene
                .nodes
                .iter()
                .position(|n| n.id == node_id)
                .ok_or("Блок не найден")?;
            if from == to_index {
                return Ok(());
            }
            let node = scene.nodes.remove(from);
            let pos = to_index.min(scene.nodes.len());
            scene.nodes.insert(pos, node);
            Ok(())
        })?;
    }
    finish_mutation(&state, "node-moved")
}

#[tauri::command]
pub fn node_duplicate(
    state: State<'_, Shared>,
    scene_id: String,
    node_id: String,
) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        with_scene_mut(&mut store, &scene_id, |scene| {
            let index = scene
                .nodes
                .iter()
                .position(|n| n.id == node_id)
                .ok_or("Блок не найден")?;
            let mut copy = scene.nodes[index].clone();
            copy.id = new_id();
            copy.source = NodeSource::Studio;
            scene.nodes.insert(index + 1, copy);
            Ok(())
        })?;
    }
    finish_mutation(&state, "node-duplicated")
}

// ---------------------------------------------------------------------------
// Characters & variables
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn character_upsert(
    state: State<'_, Shared>,
    character: Character,
) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint_tagged(&format!("character:{}", character.id));
        let mut character = character;
        if character.id.trim().is_empty() {
            character.id = new_id();
        }
        match store
            .project
            .characters
            .iter_mut()
            .find(|c| c.id == character.id)
        {
            Some(existing) => *existing = character,
            None => store.project.characters.push(character),
        }
    }
    finish_mutation(&state, "character-saved")
}

#[tauri::command]
pub fn character_delete(state: State<'_, Shared>, id: String) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        store.project.characters.retain(|c| c.id != id);
        for scene in &mut store.project.scenes {
            for node in &mut scene.nodes {
                if node.kind == NodeKind::SAY {
                    let mut p: SayParams = node.typed();
                    if p.who.as_deref() == Some(id.as_str()) {
                        p.who = None;
                        node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                    }
                }
            }
        }
    }
    finish_mutation(&state, "character-deleted")
}

#[tauri::command]
pub fn variable_upsert(state: State<'_, Shared>, variable: Variable) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint_tagged(&format!("variable:{}", variable.id));
        let mut variable = variable;
        if variable.id.trim().is_empty() {
            variable.id = new_id();
        }
        variable.name = slugify(&variable.name);
        match store
            .project
            .variables
            .iter_mut()
            .find(|v| v.id == variable.id)
        {
            Some(existing) => *existing = variable,
            None => store.project.variables.push(variable),
        }
    }
    finish_mutation(&state, "variable-saved")
}

#[tauri::command]
pub fn variable_delete(state: State<'_, Shared>, id: String) -> Result<StateEnvelope, String> {
    let mut store = lock_store(&state)?;
    store.checkpoint();
    store.project.variables.retain(|v| v.id != id);
    drop(store);
    finish_mutation(&state, "variable-deleted")
}

#[tauri::command]
pub fn meta_update(state: State<'_, Shared>, meta: Meta) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        let created = store.project.meta.created_at.clone();
        store.project.meta = meta;
        store.project.meta.created_at = created;
        store.project.meta.updated_at = crate::model::now_iso();
    }
    finish_mutation(&state, "meta-updated")
}

#[tauri::command]
pub fn ui_update(state: State<'_, Shared>, ui: UiConfig) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        store.project.ui = ui;
    }
    finish_mutation(&state, "ui-updated")
}

// ---------------------------------------------------------------------------
// SDK: lint / run / build
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn project_lint(state: State<'_, Shared>) -> Result<LintPayload, String> {
    {
        let mut store = lock_store(&state)?;
        if !store.is_open() {
            return Err("Проект не открыт".into());
        }
        if store.dirty {
            store.save()?;
        }
    }
    let _ = regenerate(&state, false)?;
    let sdk = state
        .sdk_info()
        .ok_or("Ren'Py SDK не найден. Укажите путь в настройках.")?;
    let root = state.root().ok_or("Проект не открыт")?;
    let report = state.sdk.lint(&sdk, &root);
    let manifest = exporter::load_manifest(&root);
    let mut mapped = Vec::new();
    for problem in &report.problems {
        let node_id = exporter::lookup_node(&manifest, &problem.file, problem.line);
        let scene_id = node_id.as_ref().and_then(|node| {
            manifest
                .srcmap
                .iter()
                .find(|e| &e.node_id == node)
                .map(|e| e.scene_id.clone())
        });
        let scene_title = scene_id.as_ref().and_then(|sid| {
            lock_store(&state)
                .ok()
                .and_then(|store| store.project.scene_by_id(sid).map(|s| s.title.clone()))
        });
        mapped.push(MappedProblem {
            file: problem.file.clone(),
            line: problem.line,
            message: problem.message.clone(),
            severity: problem.severity.clone(),
            node_id,
            scene_id,
            scene_title,
        });
    }
    Ok(LintPayload { report, mapped })
}

#[tauri::command]
pub fn project_run(state: State<'_, Shared>) -> Result<RunResult, String> {
    let sdk = state
        .sdk_info()
        .ok_or("Ren'Py SDK не найден. Укажите путь к SDK в настройках.")?;
    let root = state.root().ok_or("Проект не открыт")?;
    {
        let mut store = lock_store(&state)?;
        if store.dirty {
            store.save()?;
        }
        let _ = store.snapshot("pre-run", "Перед запуском игры");
    }
    let report = regenerate(&state, false)?;
    if !report.conflicts.is_empty() {
        return Ok(RunResult {
            ok: false,
            message: format!(
                "Запуск отменён: {} файлов изменены вне студии. Откройте экспорт и решите конфликт.",
                report.conflicts.len()
            ),
            pid: None,
        });
    }
    match state.sdk.run_game(&sdk, &root) {
        Ok(pid) => Ok(RunResult {
            ok: true,
            message: "Игра запущена".into(),
            pid: Some(pid),
        }),
        Err(e) => Ok(RunResult {
            ok: false,
            message: e,
            pid: None,
        }),
    }
}

#[tauri::command]
pub fn project_stop(state: State<'_, Shared>) -> Result<bool, String> {
    Ok(state.sdk.stop_game())
}

#[tauri::command]
pub fn project_build(
    state: State<'_, Shared>,
    package: String,
    destination: Option<String>,
) -> Result<BuildReport, String> {
    let sdk = state
        .sdk_info()
        .ok_or("Ren'Py SDK не найден. Укажите путь к SDK в настройках.")?;
    let root = state.root().ok_or("Проект не открыт")?;
    {
        let mut store = lock_store(&state)?;
        if store.dirty {
            store.save()?;
        }
        let _ = store.snapshot("pre-build", "Перед сборкой релиза");
    }
    let report = regenerate(&state, false)?;
    if !report.conflicts.is_empty() {
        return Err("Сборка отменена: есть конфликты с файлами, изменёнными вне студии.".into());
    }
    let dest = destination
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("dist"));
    Ok(state.sdk.build(&sdk, &root, &package, &dest))
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn snapshot_create(
    state: State<'_, Shared>,
    label: String,
) -> Result<SnapshotInfo, String> {
    let mut store = lock_store(&state)?;
    if store.dirty {
        store.save()?;
    }
    store.snapshot("manual", &label)
}

#[tauri::command]
pub fn snapshot_list(state: State<'_, Shared>) -> Result<Vec<SnapshotInfo>, String> {
    let store = lock_store(&state)?;
    store.list_snapshots()
}

#[tauri::command]
pub fn snapshot_restore(state: State<'_, Shared>, file: String) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.restore_snapshot(&file)?;
    }
    let _ = regenerate(&state, false);
    state.emit_project_changed("restored");
    envelope(&state)
}

#[tauri::command]
pub fn snapshot_diff(state: State<'_, Shared>, file: String) -> Result<Vec<DiffEntry>, String> {
    let store = lock_store(&state)?;
    let raw = store.snapshot_bytes(&file)?;
    let before: Project = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("Снимок повреждён: {e}"))?;
    Ok(diff::diff_projects(&before, &store.project))
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn open_in_explorer(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    let arg = if target.is_dir() {
        target.display().to_string()
    } else {
        format!("/select,{}", target.display())
    };
    std::process::Command::new("explorer")
        .arg(arg)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn default_block_params(state: State<'_, Shared>, kind: String) -> Result<Value, String> {
    let store = lock_store(&state)?;
    Ok(default_params(&store.project, &kind))
}

#[tauri::command]
pub fn project_path(state: State<'_, Shared>) -> Result<Option<String>, String> {
    Ok(state.root().map(|p| p.display().to_string()))
}

// ---------------------------------------------------------------------------
// Import (.rpy → blocks)
// ---------------------------------------------------------------------------

fn import_root(path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.join("game").is_dir() {
        path
    } else if path.is_file() {
        path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
    } else {
        path
    }
}

pub(crate) fn backup_sources(root: &Path, inputs: &[crate::importer::ImportInput]) -> Result<(), String> {
    let backup = root.join("renpy-studio").join("imported-source");
    for input in inputs {
        let source = root.join(&input.path);
        if !source.is_file() {
            continue;
        }
        let target = backup.join(&input.path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if target.exists() {
            let _ = std::fs::remove_file(&target);
        }
        std::fs::rename(&source, &target)
            .or_else(|_| {
                std::fs::copy(&source, &target)
                    .and_then(|_| std::fs::remove_file(&source))
                    .map(|_| ())
            })
            .map_err(|e| format!("Не удалось сохранить копию {}: {e}", input.path))?;
    }
    Ok(())
}

#[tauri::command]
pub fn import_scan(path: String) -> Result<crate::importer::ImportReport, String> {
    let root = import_root(&path);
    let (inputs, skipped) = crate::importer::collect_sources(&root)?;
    let title = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Импорт".into());
    let (_project, mut report) = crate::importer::import(&inputs, &title, &slugify(&title));
    report.skipped_files = skipped;
    Ok(report)
}

#[tauri::command]
pub fn import_apply(
    state: State<'_, Shared>,
    path: String,
    mode: String,
) -> Result<ImportPayload, String> {
    let root = import_root(&path);
    let (inputs, skipped) = crate::importer::collect_sources(&root)?;
    if inputs.is_empty() {
        return Err("В папке не найдено .rpy-файлов со сценарием".into());
    }
    let title = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Импорт".into());
    let (mut imported, mut report) = crate::importer::import(&inputs, &title, &slugify(&title));
    report.skipped_files = skipped;
    if imported.scenes.is_empty() {
        return Err("Не удалось найти ни одной сцены (label) в выбранных файлах".into());
    }

    // Original .rpy files must not coexist with generated ones: duplicate labels
    // would break the game. Keep them under renpy-studio/imported-source.
    backup_sources(&root, &inputs)?;

    let mut store = lock_store(&state)?;
    let mut direct_env = None;
    // Importing into a folder that has no studio project yet: the folder becomes
    // the project root (Scenario B of the spec: "Open project → import").
    if !store.is_open() {
        store.root = Some(root.clone());
        store.project.meta.title = title.clone();
        store.project.meta.game_slug = slugify(&title);
    }
    if mode == "merge" {
        if !store.is_open() {
            return Err("Сначала откройте проект RenPy Studio".into());
        }
        let mut existing_scene_ids: std::collections::HashSet<String> =
            store.project.scenes.iter().map(|s| s.id.clone()).collect();
        for scene in &mut imported.scenes {
            if !existing_scene_ids.insert(scene.id.clone()) {
                scene.id = new_id();
            }
            for node in &mut scene.nodes {
                node.id = new_id();
            }
        }
        let existing_var_names: std::collections::HashSet<String> =
            store.project.variables.iter().map(|v| v.name.clone()).collect();
        for variable in imported.variables {
            if !existing_var_names.contains(&variable.name) {
                store.project.variables.push(variable);
            }
        }
        let existing_chars: std::collections::HashSet<String> = store
            .project
            .characters
            .iter()
            .map(|c| c.name.to_lowercase())
            .collect();
        for character in imported.characters {
            if !existing_chars.contains(&character.name.to_lowercase()) {
                store.project.characters.push(character);
            }
        }
        store.project.scenes.extend(imported.scenes);
        if !imported.init_script.trim().is_empty() {
            store.project.init_script.push_str("\n\n");
            store.project.init_script.push_str(&imported.init_script);
        }
    } else {
        let root = store.root.clone().ok_or("Проект не открыт")?;
        imported.meta.game_slug = store.project.meta.game_slug.clone();
        if imported.meta.start_scene.is_none() {
            imported.meta.start_scene = imported.scenes.first().map(|s| s.id.clone());
        }
        store.project = imported.clone();
        store.root = Some(root);
        store.clear_history();
        direct_env = Some(crate::commands::envelope_of(&store));
    }
    store.checkpoint();
    store.save()?;
    let root_for_watch = store.root.clone();
    let title_for_settings = store.project.meta.title.clone();
    drop(store);
    if let Some(root) = root_for_watch {
        if let Ok(mut watcher) = state.watcher.lock() {
            *watcher = None;
            if let Ok(started) = crate::watcher::start(root.clone(), state.app.clone()) {
                *watcher = Some(started);
            }
        }
        if let Ok(mut settings) = state.settings.lock() {
            settings.remember_project(&root.display().to_string(), &title_for_settings);
            let _ = settings.save();
        }
    }
    let _ = regenerate(&state, true);
    let _ = state
        .app
        .emit("studio://project-changed", json!({ "reason": "imported" }));

    let project = match direct_env {
        Some(env) => Some(env.project),
        None => Some(envelope(&state)?.project),
    };
    Ok(ImportPayload { project, report })
}

// ---------------------------------------------------------------------------
// MCP server control
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn mcp_start(state: State<'_, Shared>, port: Option<u16>) -> Result<Value, String> {
    let mut guard = state.mcp.lock().map_err(|e| e.to_string())?;
    if let Some(handle) = guard.as_ref() {
        return Ok(json!({ "running": true, "port": handle.port }));
    }
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let chosen = port.unwrap_or(settings.mcp_port);
    let handle = crate::mcp::start(state.inner().clone(), chosen, None)?;
    settings.mcp_port = chosen;
    settings.mcp_enabled = true;
    if let Ok(mut s) = state.settings.lock() {
        *s = settings.clone();
        let _ = s.save();
    }
    *guard = Some(handle);
    Ok(json!({ "running": true, "port": chosen }))
}

#[tauri::command]
pub fn mcp_stop(state: State<'_, Shared>) -> Result<Value, String> {
    let mut guard = state.mcp.lock().map_err(|e| e.to_string())?;
    *guard = None;
    if let Ok(mut settings) = state.settings.lock() {
        settings.mcp_enabled = false;
        let _ = settings.save();
    }
    Ok(json!({ "running": false }))
}

#[tauri::command]
pub fn mcp_status(state: State<'_, Shared>) -> Result<Value, String> {
    let guard = state.mcp.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(handle) => Ok(json!({
            "running": true,
            "port": handle.port,
            "url": format!("http://127.0.0.1:{}/mcp", handle.port)
        })),
        None => Ok(json!({ "running": false })),
    }
}

// ---------------------------------------------------------------------------
// Assets: drag & drop import
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportReport {
    pub imported: Vec<String>,
    pub skipped: Vec<String>,
    pub assets: Vec<AssetInfo>,
}

#[tauri::command]
pub fn asset_import(
    state: State<'_, Shared>,
    paths: Vec<String>,
) -> Result<AssetImportReport, String> {
    let root = state.root().ok_or("Проект не открыт")?;
    if paths.is_empty() {
        return Err("Нет файлов для импорта".into());
    }
    let sources: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let (imported, skipped) = crate::assets::import_files(&root, &sources);
    if !imported.is_empty() {
        crate::watcher::suppress_for(1500);
    }
    let assets = crate::assets::scan(&root);
    let _ = state.app.emit(
        "studio://assets-changed",
        json!({ "imported": imported.len() }),
    );
    Ok(AssetImportReport {
        imported,
        skipped,
        assets,
    })
}

// ---------------------------------------------------------------------------
// Localization: SDK translation files
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranslateReport {
    pub ok: bool,
    pub locale: String,
    pub log: String,
    pub files: usize,
}

#[tauri::command]
pub fn project_translate(
    state: State<'_, Shared>,
    locale: String,
) -> Result<TranslateReport, String> {
    let locale = slugify(&locale);
    if locale.is_empty() {
        return Err("Некорректный код языка".into());
    }
    let sdk = state
        .sdk_info()
        .ok_or("Ren'Py SDK не найден. Укажите путь к SDK в настройках.")?;
    let root = state.root().ok_or("Проект не открыт")?;
    {
        let store = lock_store(&state)?;
        if !store.project.locales.iter().any(|l| l == &locale) {
            return Err(format!(
                "Язык «{locale}» не объявлен в проекте. Добавьте его в настройках проекта."
            ));
        }
    }
    let mut cmd = Command::new(&sdk.exe);
    cmd.arg(&root)
        .arg("translate")
        .arg(&locale)
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd
        .output()
        .map_err(|e| format!("Не удалось запустить перевод: {e}"))?;
    let mut log = String::from_utf8_lossy(&output.stdout).to_string();
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    let dir = root.join("game").join("tl").join(&locale);
    let files = walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("rpy"))
        .count();
    crate::watcher::suppress_for(3000);
    Ok(TranslateReport {
        ok: output.status.success(),
        locale,
        log,
        files,
    })
}

// ---------------------------------------------------------------------------
// Build with a live log
// ---------------------------------------------------------------------------

fn push_build_line(state: &Shared, line: String) {
    let line = line.trim_end().to_string();
    if line.is_empty() {
        return;
    }
    if let Ok(mut build) = state.build.lock() {
        build.log.push(line.clone());
        if build.log.len() > 4000 {
            build.log.remove(0);
        }
    }
    let _ = state
        .app
        .emit("studio://build-log", json!({ "line": line }));
}

#[tauri::command]
pub fn project_build_start(
    state: State<'_, Shared>,
    package: String,
    destination: Option<String>,
) -> Result<(), String> {
    {
        let build = state.build.lock().map_err(|e| e.to_string())?;
        if build.running {
            return Err("Сборка уже идёт".into());
        }
    }
    let sdk = state
        .sdk_info()
        .ok_or("Ren'Py SDK не найден. Укажите путь к SDK в настройках.")?;
    let root = state.root().ok_or("Проект не открыт")?;
    {
        let mut store = lock_store(&state)?;
        if store.dirty {
            store.save()?;
        }
        let _ = store.snapshot("pre-build", "Перед сборкой релиза");
    }
    let report = regenerate(&state, false)?;
    if !report.conflicts.is_empty() {
        return Err("Сборка отменена: есть конфликты с файлами, изменёнными вне студии.".into());
    }
    let dest = destination
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("dist"));
    let _ = std::fs::create_dir_all(&dest);
    {
        let mut build = state.build.lock().map_err(|e| e.to_string())?;
        *build = BuildState {
            running: true,
            package: package.clone(),
            log: vec![format!("Сборка пакета «{package}» → {}", dest.display())],
            ok: None,
            artifact: None,
        };
    }
    let shared = state.inner().clone();
    std::thread::spawn(move || {
        let mut cmd = Command::new(&sdk.exe);
        cmd.arg("launcher")
            .arg("distribute")
            .arg(&root)
            .arg("--package")
            .arg(&package)
            .arg("--destination")
            .arg(&dest)
            .current_dir(&sdk.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(err) => {
                push_build_line(&shared, format!("Не удалось запустить сборку: {err}"));
                if let Ok(mut build) = shared.build.lock() {
                    build.running = false;
                    build.ok = Some(false);
                }
                let _ = shared
                    .app
                    .emit("studio://build-done", json!({ "ok": false }));
                return;
            }
        };
        let mut readers = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            let shared = shared.clone();
            readers.push(std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    push_build_line(&shared, line);
                }
            }));
        }
        if let Some(stderr) = child.stderr.take() {
            let shared = shared.clone();
            readers.push(std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    push_build_line(&shared, line);
                }
            }));
        }
        let status = child.wait();
        for reader in readers {
            let _ = reader.join();
        }
        let success = status.map(|s| s.success()).unwrap_or(false);
        let artifact = newest_entry(&dest);
        let ok = success && artifact.is_some();
        if success && artifact.is_none() {
            push_build_line(
                &shared,
                "Пакеты не выбраны или SDK ничего не создал — проверьте журнал выше.".into(),
            );
        }
        if let Ok(mut build) = shared.build.lock() {
            build.running = false;
            build.ok = Some(ok);
            build.artifact = artifact.clone();
        }
        let _ = shared
            .app
            .emit("studio://build-done", json!({ "ok": ok, "artifact": artifact }));
    });
    Ok(())
}

fn newest_entry(dir: &Path) -> Option<String> {
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if newest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
                    newest = Some((modified, entry.path()));
                }
            }
        }
    }
    newest.map(|(_, path)| path.display().to_string())
}

#[tauri::command]
pub fn project_build_status(state: State<'_, Shared>) -> Result<BuildState, String> {
    Ok(state.build.lock().map_err(|e| e.to_string())?.clone())
}

// ---------------------------------------------------------------------------
// Diff of files changed outside the studio
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    /// modified | added | removed
    pub status: String,
    pub lines: Vec<String>,
}

#[tauri::command]
pub fn external_diff(
    state: State<'_, Shared>,
    files: Vec<String>,
) -> Result<Vec<FileDiff>, String> {
    let (root, project) = {
        let store = lock_store(&state)?;
        (
            store.root.clone().ok_or("Проект не открыт")?,
            store.project.clone(),
        )
    };
    let locale = if project.meta.default_locale.is_empty() {
        "ru".to_string()
    } else {
        project.meta.default_locale.clone()
    };
    let bundle = crate::generator::generate(
        &project,
        &GenOptions {
            locale,
            force: true,
        },
    );
    let generated: std::collections::HashMap<String, String> = bundle
        .files
        .into_iter()
        .map(|file| (file.path, file.content))
        .collect();
    let mut out = Vec::new();
    for path in files {
        let abs = root.join(&path);
        let disk = std::fs::read_to_string(&abs);
        match (generated.get(&path), disk) {
            (Some(expected), Ok(actual)) => {
                if expected == &actual {
                    continue;
                }
                let diff = similar::TextDiff::from_lines(expected, &actual);
                let mut lines = Vec::new();
                for group in diff.grouped_ops(2) {
                    if lines.len() >= 400 {
                        break;
                    }
                    if !lines.is_empty() {
                        lines.push("…".to_string());
                    }
                    for op in group {
                        for change in diff.iter_changes(&op) {
                            if lines.len() >= 400 {
                                break;
                            }
                            let sign = match change.tag() {
                                similar::ChangeTag::Delete => "-",
                                similar::ChangeTag::Insert => "+",
                                similar::ChangeTag::Equal => " ",
                            };
                            lines.push(format!("{sign}{}", change.value().trim_end()));
                        }
                    }
                }
                out.push(FileDiff {
                    path,
                    status: "modified".into(),
                    lines,
                });
            }
            (Some(_), Err(_)) => out.push(FileDiff {
                path,
                status: "removed".into(),
                lines: Vec::new(),
            }),
            (None, Ok(actual)) => out.push(FileDiff {
                path,
                status: "added".into(),
                lines: actual
                    .lines()
                    .take(300)
                    .map(|line| format!("+{line}"))
                    .collect(),
            }),
            (None, Err(_)) => {}
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Graph: retarget an edge
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn node_retarget(
    state: State<'_, Shared>,
    scene_id: String,
    node_id: String,
    target: Option<String>,
    option_id: Option<String>,
    branch: Option<String>,
) -> Result<StateEnvelope, String> {
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        with_scene_mut(&mut store, &scene_id, |scene| {
            let node = scene
                .nodes
                .iter_mut()
                .find(|n| n.id == node_id)
                .ok_or("Блок не найден")?;
            match node.kind.as_str() {
                NodeKind::JUMP => {
                    let mut p: JumpParams = node.typed();
                    p.target = target;
                    node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                }
                NodeKind::MENU => {
                    let option_id = option_id.ok_or("Не указан вариант выбора")?;
                    let mut p: MenuParams = node.typed();
                    let option = p
                        .options
                        .iter_mut()
                        .find(|option| option.id == option_id)
                        .ok_or("Вариант не найден")?;
                    option.target = target;
                    node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                }
                NodeKind::IF => {
                    let mut p: IfParams = node.typed();
                    match branch.as_deref() {
                        Some("else") => p.else_target = target,
                        _ => p.then_target = target,
                    }
                    node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                }
                NodeKind::POINTCLICK => {
                    let hotspot_id = option_id.ok_or("Не указана зона клика")?;
                    let mut p: PointClickParams = node.typed();
                    let hotspot = p
                        .hotspots
                        .iter_mut()
                        .find(|hotspot| hotspot.id == hotspot_id)
                        .ok_or("Зона не найдена")?;
                    hotspot.target = target.clone();
                    if target.is_some() {
                        hotspot.action = "jump".into();
                    }
                    node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                }
                _ => return Err("У этого блока нет перехода".into()),
            }
            Ok(())
        })?;
    }
    finish_mutation(&state, "edge-retargeted")
}

// ---------------------------------------------------------------------------
// Plugins
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PluginsPayload {
    pub root: String,
    pub plugins: Vec<crate::plugins::PluginInfo>,
}

#[tauri::command]
pub fn plugins_list(state: State<'_, Shared>) -> Result<PluginsPayload, String> {
    let root = crate::plugins::plugins_root();
    let mut plugins = crate::plugins::scan(&root);
    let enabled: std::collections::HashSet<String> = lock_store(&state)?
        .project
        .plugins
        .iter()
        .map(|plugin| plugin.id.clone())
        .collect();
    for plugin in &mut plugins {
        plugin.enabled = enabled.contains(&plugin.id);
    }
    Ok(PluginsPayload {
        root: root.display().to_string(),
        plugins,
    })
}

#[tauri::command]
pub fn plugin_enable(state: State<'_, Shared>, id: String) -> Result<StateEnvelope, String> {
    let root = state.root().ok_or("Проект не открыт")?;
    let plugins_root = crate::plugins::plugins_root();
    let plugins = crate::plugins::scan(&plugins_root);
    let plugin = plugins
        .into_iter()
        .find(|plugin| plugin.id == id)
        .ok_or("Плагин не найден в папке plugins")?;
    let files = crate::plugins::install(&root, &plugin)?;
    {
        let mut store = lock_store(&state)?;
        store.checkpoint();
        store.project.plugins.retain(|entry| entry.id != plugin.id);
        store.project.plugins.push(crate::model::PluginRef {
            id: plugin.id.clone(),
            name: plugin.name.clone(),
            kind: plugin.kind.clone(),
            files: files.clone(),
            enabled_at: crate::model::now_iso(),
        });
        store.save()?;
    }
    let _ = crate::sdk::write_project_settings(&root, None).ok();
    state.emit_project_changed("plugin-enabled");
    envelope(&state)
}

#[tauri::command]
pub fn plugin_disable(state: State<'_, Shared>, id: String) -> Result<StateEnvelope, String> {
    let root = state.root().ok_or("Проект не открыт")?;
    {
        let mut store = lock_store(&state)?;
        let files = store
            .project
            .plugins
            .iter()
            .find(|plugin| plugin.id == id)
            .map(|plugin| plugin.files.clone())
            .ok_or("Плагин не включён в проект")?;
        // Shared dependencies (7DOTS.rpy and friends) may be shipped by several
        // plugins: keep anything another enabled plugin still declares.
        let others: std::collections::HashSet<String> = store
            .project
            .plugins
            .iter()
            .filter(|plugin| plugin.id != id)
            .map(|plugin| plugin.id.clone())
            .collect();
        let keep: std::collections::HashSet<String> = if others.is_empty() {
            Default::default()
        } else {
            crate::plugins::scan(&crate::plugins::plugins_root())
                .into_iter()
                .filter(|plugin| others.contains(&plugin.id))
                .flat_map(|plugin| plugin.files.into_iter().map(|file| file.target))
                .collect()
        };
        let removable: Vec<String> = files
            .into_iter()
            .filter(|file| !keep.contains(file.trim_start_matches("game/")))
            .collect();
        crate::plugins::uninstall(&root, &removable);
        store.checkpoint();
        store.project.plugins.retain(|plugin| plugin.id != id);
        store.save()?;
    }
    state.emit_project_changed("plugin-disabled");
    envelope(&state)
}

#[tauri::command]
pub fn plugin_docs(id: String) -> Result<Option<String>, String> {
    let root = crate::plugins::plugins_root();
    let plugins = crate::plugins::scan(&root);
    let plugin = plugins
        .into_iter()
        .find(|plugin| plugin.id == id)
        .ok_or("Плагин не найден")?;
    Ok(crate::plugins::plugin_docs(&plugin))
}

#[tauri::command]
pub fn plugins_root() -> Result<String, String> {
    Ok(crate::plugins::plugins_root().display().to_string())
}

#[tauri::command]
pub fn ensure_sprite_folder(
    state: State<'_, Shared>,
    relative: String,
) -> Result<String, String> {
    let root = state.root().ok_or("Проект не открыт")?;
    let cleaned = relative.trim().replace('\\', "/");
    let mut path = root.join("game").join("images");
    for part in cleaned.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            continue;
        }
        path = path.join(part);
    }
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

// ---------------------------------------------------------------------------
// AI assistant (OpenAI-compatible provider)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn ai_models(state: State<'_, Shared>) -> Result<Vec<crate::ai::AiModelInfo>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Настройки недоступны".to_string())?
        .ai
        .clone();
    tauri::async_runtime::spawn_blocking(move || crate::ai::list_models(&settings))
        .await
        .map_err(|err| format!("Сбой запроса: {err}"))?
}

#[tauri::command]
pub async fn ai_chat(
    state: State<'_, Shared>,
    messages: Vec<crate::ai::ChatMessage>,
    scene_id: Option<String>,
) -> Result<crate::ai::AiReply, String> {
    let (settings, system) = {
        let store = lock_store(&state)?;
        let system = crate::ai::system_prompt(&store.project, scene_id.as_deref());
        drop(store);
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Настройки недоступны".to_string())?
            .ai
            .clone();
        (settings, system)
    };
    if messages.is_empty() {
        return Err("Пустой запрос".into());
    }
    tauri::async_runtime::spawn_blocking(move || crate::ai::chat(&settings, &system, &messages))
        .await
        .map_err(|err| format!("Сбой запроса: {err}"))?
}
