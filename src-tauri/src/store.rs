//! On-disk project I/O: atomic saves, undo/redo, snapshot history.

use crate::model::{now_iso, Project, Scene, FORMAT_VERSION};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const STUDIO_DIR: &str = "renpy-studio";
pub const PROJECT_FILE: &str = "project.json";
pub const MARKER_FILE: &str = "renpy-studio.rpyproj";
pub const HISTORY_DIR: &str = "history";
pub const INDEX_FILE: &str = "index.json";
pub const MAX_AUTO_SNAPSHOTS: usize = 50;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMarker {
    pub format_version: u32,
    pub title: String,
    pub created_at: String,
    pub studio: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub id: String,
    pub file: String,
    pub created_at: String,
    /// `auto` | `manual` | `pre-run` | `pre-build` | `pre-import` | `pre-rollback`
    pub kind: String,
    pub label: String,
    pub revision: u64,
    pub scenes: usize,
    pub blocks: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoreStatus {
    pub root: Option<String>,
    pub title: String,
    pub revision: u64,
    pub dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub last_saved: Option<String>,
    pub scene_count: usize,
    pub block_count: usize,
}

pub struct Store {
    pub root: Option<PathBuf>,
    pub project: Project,
    pub revision: u64,
    pub dirty: bool,
    pub last_saved: Option<String>,
    undo: Vec<Project>,
    redo: Vec<Project>,
    undo_limit: usize,
}

impl Default for Store {
    fn default() -> Self {
        Store {
            root: None,
            project: Project::default(),
            revision: 0,
            dirty: false,
            last_saved: None,
            undo: Vec::new(),
            redo: Vec::new(),
            undo_limit: 200,
        }
    }
}

impl Store {
    pub fn status(&self) -> StoreStatus {
        StoreStatus {
            root: self.root.as_ref().map(|p| p.display().to_string()),
            title: self.project.meta.title.clone(),
            revision: self.revision,
            dirty: self.dirty,
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            last_saved: self.last_saved.clone(),
            scene_count: self.project.scenes.len(),
            block_count: self.project.scenes.iter().map(|s| s.nodes.len()).sum(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.root.is_some()
    }

    /// Records the current state for undo before a mutation.
    pub fn checkpoint(&mut self) {
        self.undo.push(self.project.clone());
        if self.undo.len() > self.undo_limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.revision += 1;
        self.dirty = true;
        self.project.touch();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo.pop() {
            let current = std::mem::replace(&mut self.project, prev);
            self.redo.push(current);
            self.revision += 1;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            let current = std::mem::replace(&mut self.project, next);
            self.undo.push(current);
            self.revision += 1;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    // -----------------------------------------------------------------------
    // Paths
    // -----------------------------------------------------------------------

    pub fn studio_dir(&self) -> Option<PathBuf> {
        self.root.as_ref().map(|r| r.join(STUDIO_DIR))
    }

    pub fn project_path(&self) -> Option<PathBuf> {
        self.studio_dir().map(|d| d.join(PROJECT_FILE))
    }

    fn history_dir(&self) -> Option<PathBuf> {
        self.studio_dir().map(|d| d.join(HISTORY_DIR))
    }

    // -----------------------------------------------------------------------
    // Open / create / save
    // -----------------------------------------------------------------------

    /// Opens an existing project folder (contains `renpy-studio/project.json`).
    pub fn open(root: &Path) -> Result<Self, String> {
        let project_path = root.join(STUDIO_DIR).join(PROJECT_FILE);
        let raw = fs::read_to_string(&project_path)
            .map_err(|e| format!("Не удалось прочитать {}: {e}", project_path.display()))?;
        let mut project: Project = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("project.json повреждён: {e}"))?;
        migrate(&mut project)?;

        let mut store = Store::default();
        store.root = Some(root.to_path_buf());
        store.project = project;
        store.revision = 1;
        store.dirty = false;
        store.last_saved = Some(now_iso());
        store.ensure_dirs()?;
        Ok(store)
    }

    /// Creates a fresh Ren'Py project skeleton in `root`.
    pub fn create(root: &Path, title: &str) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|e| format!("Не удалось создать папку: {e}"))?;
        for sub in ["game/images", "game/audio", "game/fonts", "game/gui"] {
            fs::create_dir_all(root.join(sub)).map_err(|e| format!("Не удалось создать {sub}: {e}"))?;
        }

        let mut project = Project::default();
        project.meta.title = title.to_string();
        project.meta.game_slug = crate::model::slugify(title);
        project.meta.start_scene = None;

        // A brand-new novel already has one scene so the editor is never empty.
        let scene = Scene {
            id: crate::model::new_id(),
            title: "Пролог".into(),
            slug: "prologue".into(),
            summary: String::new(),
            label_name: None,
            color: None,
            nodes: vec![],
        };
        project.meta.start_scene = Some(scene.id.clone());
        project.scenes.push(scene);

        let mut store = Store::default();
        store.root = Some(root.to_path_buf());
        store.project = project;
        store.revision = 1;
        store.dirty = true;
        store.ensure_dirs()?;
        store.write_marker()?;
        store.save()?;
        Ok(store)
    }

    fn ensure_dirs(&self) -> Result<(), String> {
        if let Some(dir) = self.studio_dir() {
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            fs::create_dir_all(dir.join(HISTORY_DIR)).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn write_marker(&self) -> Result<(), String> {
        let Some(root) = &self.root else { return Ok(()) };
        let marker = ProjectMarker {
            format_version: FORMAT_VERSION,
            title: self.project.meta.title.clone(),
            created_at: self.project.meta.created_at.clone(),
            studio: env!("CARGO_PKG_VERSION").to_string(),
        };
        let json = serde_json::to_string_pretty(&marker).map_err(|e| e.to_string())?;
        write_atomic(&root.join(MARKER_FILE), json.as_bytes())
    }

    pub fn save(&mut self) -> Result<(), String> {
        let Some(path) = self.project_path() else {
            return Err("Проект не открыт".into());
        };
        if let Some(dir) = self.studio_dir() {
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        }
        let mut project = self.project.clone();
        project.format_version = FORMAT_VERSION;
        project.meta.updated_at = now_iso();
        let mut json = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
        json.push('\n');
        write_atomic(&path, json.as_bytes())?;
        self.last_saved = Some(now_iso());
        self.dirty = false;
        if self.root.is_some() {
            let _ = self.write_marker();
        }
        Ok(())
    }

    /// Reads project.json again from disk (e.g. after `git pull`).
    pub fn reload(&mut self) -> Result<(), String> {
        let Some(root) = self.root.clone() else {
            return Err("Проект не открыт".into());
        };
        let fresh = Store::open(&root)?;
        let old_project = fresh.project;
        self.checkpoint();
        self.project = old_project;
        self.dirty = false;
        self.last_saved = Some(now_iso());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // History / snapshots
    // -----------------------------------------------------------------------

    pub fn snapshot(&mut self, kind: &str, label: &str) -> Result<SnapshotInfo, String> {
        let Some(dir) = self.history_dir() else {
            return Err("Проект не открыт".into());
        };
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let created = now_iso();
        let safe_ts = created.replace(':', "-");
        let id = format!("{}_{}", safe_ts, crate::model::new_id().split('-').next().unwrap_or("snap"));
        let file = format!("{id}.json");
        let mut json = serde_json::to_string_pretty(&self.project).map_err(|e| e.to_string())?;
        json.push('\n');
        write_atomic(&dir.join(&file), json.as_bytes())?;

        let info = SnapshotInfo {
            id,
            file,
            created_at: created,
            kind: kind.to_string(),
            label: label.to_string(),
            revision: self.revision,
            scenes: self.project.scenes.len(),
            blocks: self.project.scenes.iter().map(|s| s.nodes.len()).sum(),
        };
        let mut index = self.load_index();
        index.push(info.clone());
        self.save_index(&index)?;
        self.prune_snapshots()?;
        Ok(info)
    }

    pub fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>, String> {
        let Some(dir) = self.history_dir() else { return Ok(vec![]) };
        if !dir.exists() {
            return Ok(vec![]);
        }
        let index = self.load_index();
        let mut out: Vec<SnapshotInfo> = index
            .iter()
            .filter(|info| dir.join(&info.file).is_file())
            .cloned()
            .collect();
        let known: std::collections::HashSet<String> = out.iter().map(|i| i.file.clone()).collect();
        // Snapshots created by older builds have no index entry: recover them.
        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if file_name == INDEX_FILE || known.contains(&file_name) {
                continue;
            }
            let Ok(raw) = fs::read_to_string(&path) else { continue };
            let Ok(project) = serde_json::from_str::<Project>(raw.trim_start_matches('\u{feff}')) else { continue };
            let meta = snapshot_meta_from_path(&path);
            out.push(SnapshotInfo {
                id: meta.0,
                file: file_name,
                created_at: meta.1,
                kind: meta.2,
                label: meta.3,
                revision: 0,
                scenes: project.scenes.len(),
                blocks: project.scenes.iter().map(|s| s.nodes.len()).sum(),
            });
        }
        // Newest first.
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| b.file.cmp(&a.file)));
        Ok(out)
    }

    pub fn restore_snapshot(&mut self, file: &str) -> Result<(), String> {
        let Some(dir) = self.history_dir() else {
            return Err("Проект не открыт".into());
        };
        let path = dir.join(file);
        if !path.exists() {
            return Err(format!("Снимок не найден: {file}"));
        }
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut project: Project = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
            .map_err(|e| e.to_string())?;
        migrate(&mut project)?;
        // Nothing may be lost: keep the current state in history before rolling back.
        let _ = self.snapshot("pre-rollback", "Перед откатом");
        self.checkpoint();
        self.project = project;
        self.save()?;
        Ok(())
    }

    pub fn snapshot_bytes(&self, file: &str) -> Result<String, String> {
        let Some(dir) = self.history_dir() else {
            return Err("Проект не открыт".into());
        };
        fs::read_to_string(dir.join(file)).map_err(|e| e.to_string())
    }

    fn prune_snapshots(&self) -> Result<(), String> {
        let Some(dir) = self.history_dir() else { return Ok(()) };
        let mut index = self.load_index();
        index.retain(|info| dir.join(&info.file).is_file());
        let mut autos: Vec<SnapshotInfo> = index
            .iter()
            .filter(|info| info.kind == "auto")
            .cloned()
            .collect();
        autos.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        if autos.len() > MAX_AUTO_SNAPSHOTS {
            let excess = autos.len() - MAX_AUTO_SNAPSHOTS;
            for info in autos.into_iter().take(excess) {
                let _ = fs::remove_file(dir.join(&info.file));
                index.retain(|entry| entry.file != info.file);
            }
        }
        self.save_index(&index)?;
        Ok(())
    }

    fn index_path(&self) -> Option<PathBuf> {
        self.history_dir().map(|dir| dir.join(INDEX_FILE))
    }

    fn load_index(&self) -> Vec<SnapshotInfo> {
        let Some(path) = self.index_path() else { return vec![] };
        let Ok(raw) = fs::read_to_string(path) else { return vec![] };
        serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
    }

    fn save_index(&self, entries: &[SnapshotInfo]) -> Result<(), String> {
        let Some(path) = self.index_path() else { return Ok(()) };
        let mut json = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
        json.push('\n');
        write_atomic(&path, json.as_bytes())
    }
}

fn snapshot_meta_from_path(path: &Path) -> (String, String, String, String) {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let mut parts = stem.splitn(2, '_');
    let ts = parts.next().unwrap_or("").to_string();
    let rest = parts.next().unwrap_or("");
    let kind = rest.split('_').next().unwrap_or("auto").to_string();
    // 2026-11-09T21-15-00Z -> readable local-ish string
    let created = ts.replace('T', " ").replace('Z', "");
    (stem.clone(), created, kind, String::new())
}

/// Atomic write: temp file in the same directory, then rename over the target.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("dat")
    ));
    fs::write(&tmp, bytes).map_err(|e| format!("Запись {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Переименование {}: {e}", path.display())
    })
}

/// Format migrations. v1 is the first version; future versions hook in here.
fn migrate(project: &mut Project) -> Result<(), String> {
    if project.format_version == 0 {
        project.format_version = 1;
    }
    if project.format_version > FORMAT_VERSION {
        return Err(format!(
            "Проект создан более новой версией студии (формат {}). Обновите RenPy Studio.",
            project.format_version
        ));
    }
    project.format_version = FORMAT_VERSION;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_index_keeps_labels_and_files() {
        let dir = std::env::temp_dir().join(format!(
            "renpy-studio-test-{}",
            crate::model::new_id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let mut store = Store::create(&dir, "Test project").unwrap();
        let info = store.snapshot("manual", "Перед правкой").unwrap();
        assert!(dir.join(STUDIO_DIR).join(HISTORY_DIR).join(&info.file).is_file());

        // A fresh store (simulating a restart) must recover the label from the index.
        let reopened = Store::open(&dir).unwrap();
        let list = reopened.list_snapshots().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].label, "Перед правкой");
        assert_eq!(list[0].kind, "manual");
        let _ = fs::remove_dir_all(&dir);
    }
}
