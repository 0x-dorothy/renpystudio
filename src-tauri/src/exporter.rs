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
            // Keep the entry: dropping it would orphan the file forever, since
            // the next export only looks at paths from the previous manifest.
            if let Some(entry) = previous.files.get(path) {
                next_files.insert(path.clone(), entry.clone());
            }
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
    let file_key = normalize_report_path(file);
    manifest
        .srcmap
        .iter()
        .filter(|e| e.file == file_key)
        .filter(|e| line >= e.start_line && line <= e.end_line)
        .min_by_key(|e| e.end_line.saturating_sub(e.start_line))
        .map(|e| e.node_id.clone())
}

/// `script_a.rpy`, `./game/script_a.rpy` and `C:/proj/game/script_a.rpy` all map
/// to `game/script_a.rpy`; a naive `format!("game/{}")` used to double the
/// prefix for paths that already contain it.
fn normalize_report_path(file: &str) -> String {
    let normalized = file.replace('\\', "/");
    let trimmed = normalized.trim_start_matches("./").trim_start_matches('/');
    if trimmed.starts_with("game/") {
        return trimmed.to_string();
    }
    if let Some(position) = trimmed.find("/game/") {
        return trimmed[position + 1..].to_string();
    }
    format!("game/{trimmed}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Character, CharacterKind, Node, NodeKind, Scene};
    use crate::util::hash_str;
    use serde_json::json;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const PROLOGUE_FILE: &str = "game/script_prologue.rpy";
    const DOOR_FILE: &str = "game/script_door.rpy";

    /// Temporary project root: unique per test, removed on drop (also on panic).
    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new(tag: &str) -> Self {
            static SEQ: AtomicUsize = AtomicUsize::new(0);
            let seq = SEQ.fetch_add(1, Ordering::SeqCst);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "renpy-studio-exporter-{tag}-{}-{seq}-{nanos}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("временная папка");
            TempRoot { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn read(root: &Path, rel: &str) -> String {
        std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("файл {rel} не читается: {e}"))
    }

    fn write(root: &Path, rel: &str, content: &str) {
        std::fs::write(root.join(rel), content)
            .unwrap_or_else(|e| panic!("файл {rel} не пишется: {e}"));
    }

    fn has(list: &[String], rel: &str) -> bool {
        list.iter().any(|item| item == rel)
    }

    fn node(id: &str, kind: &str, params: serde_json::Value) -> Node {
        let mut node = Node::new(kind, params);
        node.id = id.to_string();
        node
    }

    fn scene(id: &str, title: &str, slug: &str, nodes: Vec<Node>) -> Scene {
        Scene {
            id: id.into(),
            title: title.into(),
            slug: slug.into(),
            label_name: None,
            summary: String::new(),
            color: None,
            nodes,
        }
    }

    fn sample_project() -> Project {
        let mut project = Project::default();
        project.meta.title = "Тест экспорта".into();
        project.meta.default_locale = "ru".into();
        project.characters.push(Character {
            id: "ch_mira".into(),
            name: "Мира".into(),
            kind: CharacterKind::Character,
            sprite_folder: None,
            color: Some("#e96aa5".into()),
            avatar: None,
            sprite_prefix: Some("mira".into()),
            voice_tag: None,
            prefix: None,
            suffix: None,
            notes: String::new(),
        });
        project.scenes.push(scene(
            "sc_a",
            "Пролог",
            "prologue",
            vec![
                node(
                    "nd_bg",
                    NodeKind::SCENE,
                    json!({ "bg": "images/bg/room.png", "transition": "dissolve" }),
                ),
                node(
                    "nd_say",
                    NodeKind::SAY,
                    json!({ "who": "ch_mira", "text": { "ru": "Это начало." } }),
                ),
            ],
        ));
        project.scenes.push(scene(
            "sc_b",
            "Дверь",
            "door",
            vec![node(
                "nd_narrate",
                NodeKind::NARRATE,
                json!({ "text": { "ru": "Дверь скрипит." } }),
            )],
        ));
        project.meta.start_scene = Some("sc_a".into());
        project
    }

    fn export_default(project: &Project, root: &Path) -> (ExportReport, GeneratedBundle, Manifest) {
        export(project, root, &GenOptions::default()).expect("экспорт")
    }

    fn export_force(project: &Project, root: &Path) -> (ExportReport, GeneratedBundle, Manifest) {
        let opts = GenOptions {
            force: true,
            ..GenOptions::default()
        };
        export(project, root, &opts).expect("экспорт с force")
    }

    #[test]
    fn first_export_writes_scene_files_and_manifest() {
        let root = TempRoot::new("first");
        let project = sample_project();
        let (report, _, manifest) = export_default(&project, root.path());

        for rel in &report.generated {
            assert!(
                root.path().join(rel).is_file(),
                "сгенерированный файл {rel} не появился в game/"
            );
        }
        for rel in [PROLOGUE_FILE, DOOR_FILE, "game/characters.rpy", "game/script_entry.rpy"] {
            assert!(has(&report.generated, rel), "файл {rel} не сгенерирован");
            assert!(has(&report.written, rel), "файл {rel} не записан");
        }
        assert_eq!(report.written.len(), report.generated.len());
        assert!(report.conflicts.is_empty());
        assert!(report.skipped.is_empty());
        assert_eq!(report.total_blocks, 3);
        assert_eq!(report.manifest_path, MANIFEST_REL);

        let manifest_file = root.path().join(MANIFEST_REL);
        assert!(manifest_file.is_file(), "манифест не создан");
        let raw = std::fs::read_to_string(&manifest_file).expect("чтение манифеста");
        assert!(raw.contains("\"srcmap\""), "в манифесте нет srcmap");
        assert!(raw.contains("\"generatedAt\""));

        assert_eq!(manifest.format_version, 1);
        assert_eq!(manifest.locale, "ru");
        assert!(!manifest.srcmap.is_empty(), "srcmap пуст");
        assert_eq!(manifest.files.len(), report.generated.len());

        let reloaded = load_manifest(root.path());
        assert_eq!(reloaded.files.len(), manifest.files.len());
        assert_eq!(reloaded.srcmap.len(), manifest.srcmap.len());
        assert!(reloaded.srcmap.iter().any(|e| e.node_id == "nd_say"));
    }

    #[test]
    fn second_export_without_changes_writes_nothing() {
        let root = TempRoot::new("noop");
        let project = sample_project();
        let (first, _, before) = export_default(&project, root.path());
        let (second, _, after) = export_default(&project, root.path());

        assert!(second.written.is_empty(), "перезаписано: {:?}", second.written);
        assert!(second.skipped.is_empty(), "пропущено: {:?}", second.skipped);
        assert!(second.conflicts.is_empty(), "конфликты: {:?}", second.conflicts);
        assert!(second.removed.is_empty(), "удалено: {:?}", second.removed);
        assert_eq!(second.generated, first.generated);
        assert_eq!(after.files.len(), before.files.len());
        for (path, entry) in &before.files {
            let now = after.files.get(path).expect("запись манифеста потеряна");
            assert_eq!(now.hash, entry.hash, "хеш {path} изменился без правок");
        }
        assert_eq!(after.srcmap.len(), before.srcmap.len());
    }

    #[test]
    fn external_edit_is_conflict_and_kept_on_disk() {
        let root = TempRoot::new("conflict");
        let project = sample_project();
        let (_, _, _) = export_default(&project, root.path());
        let original = read(root.path(), PROLOGUE_FILE);
        let edited = format!("{original}\n# правка снаружи\n");
        write(root.path(), PROLOGUE_FILE, &edited);

        let (report, _, manifest) = export_default(&project, root.path());

        let conflict = report
            .conflicts
            .iter()
            .find(|c| c.path == PROLOGUE_FILE)
            .unwrap_or_else(|| panic!("нет конфликта по {PROLOGUE_FILE}: {:?}", report.conflicts));
        assert!(
            conflict.reason.contains("изменён снаружи"),
            "неожиданная причина: {}",
            conflict.reason
        );
        assert!(has(&report.skipped, PROLOGUE_FILE));
        assert!(!has(&report.written, PROLOGUE_FILE), "файл перезаписан без force");
        assert_eq!(read(root.path(), PROLOGUE_FILE), edited, "правка потеряна");
        // Манифест продолжает хранить хеш последней студийной версии.
        assert_eq!(
            manifest.files.get(PROLOGUE_FILE).map(|e| e.hash.clone()),
            Some(hash_str(&original))
        );
        // Остальные файлы не перезаписаны и не удалены.
        assert!(report.written.is_empty(), "лишняя запись: {:?}", report.written);
        assert!(report.removed.is_empty());
        assert!(root.path().join(DOOR_FILE).is_file());
    }

    #[test]
    fn force_overwrites_external_edit_without_conflict() {
        let root = TempRoot::new("force");
        let project = sample_project();
        let (_, _, _) = export_default(&project, root.path());
        let original = read(root.path(), PROLOGUE_FILE);
        write(
            root.path(),
            PROLOGUE_FILE,
            &format!("{original}\n# правка снаружи\n"),
        );

        let (report, bundle, manifest) = export(
            &project,
            root.path(),
            &GenOptions {
                locale: "ru".into(),
                force: true,
            },
        )
        .expect("экспорт с force");

        assert!(report.conflicts.is_empty(), "конфликты: {:?}", report.conflicts);
        assert!(has(&report.written, PROLOGUE_FILE));
        let expected = bundle
            .files
            .iter()
            .find(|f| f.path == PROLOGUE_FILE)
            .expect("файл в бандле");
        let on_disk = read(root.path(), PROLOGUE_FILE);
        assert_eq!(on_disk, expected.content);
        assert!(!on_disk.contains("правка снаружи"));
        assert_eq!(
            manifest.files.get(PROLOGUE_FILE).map(|e| e.hash.clone()),
            Some(expected.hash.clone())
        );
    }

    #[test]
    fn removed_scene_file_is_deleted() {
        let root = TempRoot::new("removed");
        let mut project = sample_project();
        let (_, _, _) = export_default(&project, root.path());
        assert!(root.path().join(DOOR_FILE).is_file());

        project.scenes.retain(|s| s.id != "sc_b");
        let (report, _, manifest) = export_default(&project, root.path());

        assert!(has(&report.removed, DOOR_FILE), "файл не удалён: {:?}", report.removed);
        assert!(!root.path().join(DOOR_FILE).exists(), "файл остался на диске");
        assert!(!manifest.files.contains_key(DOOR_FILE));
        assert!(report.conflicts.is_empty(), "конфликты: {:?}", report.conflicts);
        assert!(report.written.is_empty(), "лишняя запись: {:?}", report.written);
        assert!(root.path().join(PROLOGUE_FILE).is_file());
        assert_eq!(report.total_blocks, 2);
    }

    #[test]
    fn externally_edited_orphan_is_reported_not_deleted() {
        let root = TempRoot::new("orphan");
        let mut project = sample_project();
        let (_, _, _) = export_default(&project, root.path());
        let original = read(root.path(), DOOR_FILE);
        let edited = format!("{original}\n# правка снаружи\n");
        write(root.path(), DOOR_FILE, &edited);

        project.scenes.retain(|s| s.id != "sc_b");
        let (report, _, manifest) = export_default(&project, root.path());

        let conflict = report
            .conflicts
            .iter()
            .find(|c| c.path == DOOR_FILE)
            .unwrap_or_else(|| panic!("нет конфликта по {DOOR_FILE}: {:?}", report.conflicts));
        assert!(
            conflict.reason.contains("Сцена удалена"),
            "неожиданная причина: {}",
            conflict.reason
        );
        assert!(!has(&report.removed, DOOR_FILE), "правленый файл удалён");
        assert_eq!(read(root.path(), DOOR_FILE), edited, "правка потеряна");
        // Запись остаётся в манифесте: иначе файл «осиротел» бы навсегда,
        // потому что поиск устаревших файлов идёт только по previous.files.
        assert!(
            manifest.files.contains_key(DOOR_FILE),
            "конфликтный файл выпал из манифеста"
        );

        // Следующий force-экспорт всё-таки наводит порядок.
        let (forced, _, after) = export_force(&project, root.path());
        assert!(has(&forced.removed, DOOR_FILE), "force не удалил файл: {:?}", forced.removed);
        assert!(!root.path().join(DOOR_FILE).exists(), "файл остался на диске");
        assert!(!after.files.contains_key(DOOR_FILE));
    }

    #[test]
    fn lookup_node_maps_lines_through_srcmap() {
        let root = TempRoot::new("lookup");
        let project = sample_project();
        let (_, _, manifest) = export_default(&project, root.path());

        let entry = manifest
            .srcmap
            .iter()
            .find(|e| e.node_id == "nd_say")
            .expect("нет srcmap для реплики");
        assert_eq!(entry.file, PROLOGUE_FILE);
        assert!(entry.start_line >= 1);
        assert!(entry.end_line >= entry.start_line);

        for line in [entry.start_line, entry.end_line] {
            assert_eq!(
                lookup_node(&manifest, PROLOGUE_FILE, line).as_deref(),
                Some("nd_say"),
                "строка {line} не смаппилась"
            );
        }
        // Путь без префикса game/ и с обратными слэшами нормализуется.
        assert_eq!(
            lookup_node(&manifest, "script_prologue.rpy", entry.start_line).as_deref(),
            Some("nd_say")
        );
        assert_eq!(
            lookup_node(&manifest, "game\\script_prologue.rpy", entry.start_line).as_deref(),
            Some("nd_say")
        );
        // Префикс не удваивается: ./game/... и абсолютный путь тоже находятся.
        assert_eq!(
            lookup_node(&manifest, "./game/script_prologue.rpy", entry.start_line).as_deref(),
            Some("nd_say")
        );
        assert_eq!(
            lookup_node(
                &manifest,
                "C:/projects/demo/game/script_prologue.rpy",
                entry.start_line
            )
            .as_deref(),
            Some("nd_say")
        );
        // Вне диапазона и в чужом файле — ничего.
        assert_eq!(lookup_node(&manifest, PROLOGUE_FILE, 0), None);
        assert_eq!(lookup_node(&manifest, PROLOGUE_FILE, 100_000), None);
        assert_eq!(lookup_node(&manifest, "game/unknown.rpy", entry.start_line), None);

        // Якорь указывает ровно на комментарий блока.
        let text = read(root.path(), PROLOGUE_FILE);
        let line = text
            .lines()
            .nth(entry.start_line as usize - 1)
            .expect("строка существует");
        assert!(line.contains("# [блок"), "якорь ведёт на строку: {line}");
    }
}
