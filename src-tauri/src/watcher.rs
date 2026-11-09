//! Watches the project folder for changes made outside the studio (VS Code, git pull).

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

static SUPPRESS_UNTIL_MS: AtomicI64 = AtomicI64::new(0);

/// Ignores events caused by the studio's own export for the next `ms` milliseconds.
pub fn suppress_for(ms: u64) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    SUPPRESS_UNTIL_MS.store(now + ms as i64, Ordering::SeqCst);
}

fn suppressed() -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    now < SUPPRESS_UNTIL_MS.load(Ordering::SeqCst)
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExternalChange {
    pub rpy: Vec<String>,
    pub assets: Vec<String>,
    pub project_file: bool,
}

pub struct ProjectWatcher {
    _watcher: RecommendedWatcher,
    stop: Arc<Mutex<bool>>,
}

fn ignored(rel: &str) -> bool {
    rel.contains("renpy-studio/history")
        || rel.ends_with("renpy-studio/generated.json")
        || rel.ends_with("lint.txt")
        || rel.ends_with(".tmp")
        || rel.contains("/cache/")
        || rel.contains("/saves/")
        || rel.ends_with(".rpyc")
        || rel.ends_with("project.json.tmp")
}

fn classify(rel: &str) -> Option<&'static str> {
    if rel == "renpy-studio/project.json" {
        return Some("project");
    }
    if rel.starts_with("game/") && rel.ends_with(".rpy") {
        return Some("rpy");
    }
    let ext = rel.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "avif" | "svg" | "ogg" | "opus" | "mp3" | "wav" | "flac"
        | "ttf" | "otf" | "mp4" | "webm" => Some("asset"),
        _ => None,
    }
}

pub fn start(root: PathBuf, app: AppHandle) -> Result<ProjectWatcher, String> {
    let (tx, rx) = channel::<Event>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })
    .map_err(|e| format!("Наблюдатель файлов: {e}"))?;

    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| format!("Не удалось следить за папкой проекта: {e}"))?;

    let stop = Arc::new(Mutex::new(false));
    let stop_thread = stop.clone();
    let root_clone = root.clone();

    std::thread::spawn(move || {
        let mut pending = ExternalChange::default();
        let mut deadline: Option<Instant> = None;
        loop {
            if *stop_thread.lock().unwrap() {
                break;
            }
            let timeout = deadline
                .map(|d| d.saturating_duration_since(Instant::now()))
                .unwrap_or(Duration::from_millis(400));
            match rx.recv_timeout(timeout) {
                Ok(event) => {
                    if !matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
                    ) {
                        continue;
                    }
                    for path in event.paths {
                        let rel = path
                            .strip_prefix(&root_clone)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .replace('\\', "/");
                        if ignored(&rel) {
                            continue;
                        }
                        match classify(&rel) {
                            Some("rpy") => {
                                if !pending.rpy.contains(&rel) {
                                    pending.rpy.push(rel);
                                }
                            }
                            Some("asset") => {
                                if !pending.assets.contains(&rel) {
                                    pending.assets.push(rel);
                                }
                            }
                            Some("project") => pending.project_file = true,
                            _ => {}
                        }
                    }
                    if !pending.rpy.is_empty() || !pending.assets.is_empty() || pending.project_file {
                        deadline = Some(Instant::now() + Duration::from_millis(900));
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if deadline.is_some()
                        && pending.rpy.len() + pending.assets.len() > 0
                        || pending.project_file
                    {
                        if !suppressed() {
                            let _ = app.emit("studio://external-change", pending.clone());
                        }
                        pending = ExternalChange::default();
                        deadline = None;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(ProjectWatcher {
        _watcher: watcher,
        stop,
    })
}

impl Drop for ProjectWatcher {
    fn drop(&mut self) {
        if let Ok(mut flag) = self.stop.lock() {
            *flag = true;
        }
    }
}
