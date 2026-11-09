//! Writes generated `.rpy` files into `game/` and keeps a manifest so external
//! edits are detected instead of silently overwritten.

use crate::generator::{self, GenOptions, GeneratedBundle, SrcMapEntry};
use crate::model::Project;
use crate::store::write_atomic;
use crate::util::hash_str;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const MANIFEST_REL: &str = "renpy-studio/generated.json";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub hash: String,
    #[serde(default)]
    pub scene_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format_version: u32,
    pub generated_at: String,
    pub locale: String,
    pub generator: String,
    pub files: BTreeMap<String, FileEntry>,
    #[serde(default)]
    pub srcmap: Vec<SrcMapEntry>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub path: String,
    pub reason: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub generated: Vec<String>,
    pub written: Vec<String>,
    pub skipped: Vec<String>,
    pub removed: Vec<String>,
    pub warnings: Vec<String>,
    pub conflicts: Vec<Conflict>,
    pub manifest_path: String,
    pub total_blocks: usize,
}

pub fn load_manifest(root: &Path) -> Manifest {
    let path = root.join(MANIFEST_REL);
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Manifest::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Generates and writes the project into `game/`.
///
/// `force == false` protects files edited outside the studio: they are reported
/// as conflicts and left untouched.
pub fn export(
    project: &Project,
    root: &Path,
    opts: &GenOptions,
) -> Result<(ExportReport, GeneratedBundle, Manifest), String> {
    let bundle = generator::generate(project, opts);
    let previous = load_manifest(root);
    let mut report = ExportReport {
        manifest_path: MANIFEST_REL.to_string(),
        total_blocks: project.scenes.iter().map(|s| s.nodes.len()).sum(),
        ..Default::default()
    };
    report.warnings.extend(bundle.warnings.clone());

    let mut next_files: BTreeMap<String, FileEntry> = BTreeMap::new();
    let mut scene_of_file: BTreeMap<String, String> = BTreeMap::new();
    for entry in &bundle.srcmap {
        scene_of_file.insert(entry.file.clone(), entry.scene_id.clone());
    }

    for file in &bundle.files {
        report.generated.push(file.path.clone());
        let abs = root.join(&file.path);
        let disk = std::fs::read_to_string(&abs).ok();
        if let Some(existing) = &disk {
            let existing_hash = hash_str(existing);
            let tracked = previous.files.get(&file.path).map(|f| f.hash.as_str());
            let unchanged_externally = tracked == Some(existing_hash.as_str());
            if !unchanged_externally && !opts.force && existing != &file.content {
                let reason = if tracked.is_none() {
                    "Файл создан вне студии и не tracked в манифесте".to_string()
                } else {
                    "Файл изменён снаружи после последнего экспорта".to_string()
                };
                report.conflicts.push(Conflict {
                    path: file.path.clone(),
                    reason,
                });
                report.skipped.push(file.path.clone());
                if let Some(prev) = previous.files.get(&file.path) {
                    next_files.insert(file.path.clone(), prev.clone());
                }
                continue;
            }
            if existing == &file.content {
                next_files.insert(
                    file.path.clone(),
                    FileEntry {
                        hash: file.hash.clone(),
                        scene_id: scene_of_file.get(&file.path).cloned(),
                    },
                );
                continue;
            }
        }
        let bytes = file.content.as_bytes();
        write_atomic(&abs, bytes)?;
        report.written.push(file.path.clone());
        next_files.insert(
            file.path.clone(),
            FileEntry {
                hash: file.hash.clone(),
                scene_id: scene_of_file.get(&file.path).cloned(),
            },
        );
    }

    // Files we generated before, but which no longer exist in the project.
    for path in previous.files.keys() {
        if next_files.contains_key(path) {
            continue;
        }
        let abs = root.join(path);
        let Ok(existing) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let existing_hash = hash_str(&existing);
        let tracked = previous.files.get(path).map(|f| f.hash.clone()).unwrap_or_default();
        if !opts.force && tracked != existing_hash {
            report.conflicts.push(Conflict {
                path: path.clone(),
                reason: "Сцена удалена, но файл правился снаружи".into(),
            });
            continue;
        }
        if std::fs::remove_file(&abs).is_ok() {
            report.removed.push(path.clone());
        }
    }

    let manifest = Manifest {
        format_version: 1,
        generated_at: crate::model::now_iso(),
        locale: opts.locale.clone(),
        generator: generator::GENERATOR_VERSION.to_string(),
        files: next_files,
        srcmap: bundle.srcmap.clone(),
    };
    let mut json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    json.push('\n');
    write_atomic(&root.join(MANIFEST_REL), json.as_bytes())?;

    Ok((report, bundle, manifest))
}

/// Maps a lint problem location back to a studio block through the manifest.
pub fn lookup_node(manifest: &Manifest, file: &str, line: u32) -> Option<String> {
    let normalized = file.replace('\\', "/");
    let file_key = if normalized.starts_with("game/") {
        normalized
    } else {
        format!("game/{}", normalized.trim_start_matches("./"))
    };
    manifest
        .srcmap
        .iter()
        .filter(|e| e.file == file_key)
        .filter(|e| line >= e.start_line && line <= e.end_line)
        .min_by_key(|e| e.end_line.saturating_sub(e.start_line))
        .map(|e| e.node_id.clone())
}
