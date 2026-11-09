//! Structural project diff used by the history timeline.

use crate::model::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiffEntry {
    /// scene-added | scene-removed | scene-renamed | blocks-added | blocks-removed |
    /// block-changed | characters | variables | meta
    pub kind: String,
    pub summary: String,
    pub scene_id: Option<String>,
    pub node_id: Option<String>,
    pub details: Vec<String>,
}

fn scene_map(project: &Project) -> HashMap<&str, &Scene> {
    project.scenes.iter().map(|s| (s.id.as_str(), s)).collect()
}

fn node_label(node: &Node) -> String {
    let base = crate::generator::kind_label(&node.kind);
    let text = match node.kind.as_str() {
        NodeKind::SAY => {
            let p: SayParams = node.typed();
            p.text.values().next().cloned().unwrap_or_default()
        }
        NodeKind::NARRATE => {
            let p: NarrateParams = node.typed();
            p.text.values().next().cloned().unwrap_or_default()
        }
        NodeKind::COMMENT => {
            let p: CommentParams = node.typed();
            p.text.values().next().cloned().unwrap_or_default()
        }
        NodeKind::MENU => {
            let p: MenuParams = node.typed();
            p.prompt.values().next().cloned().unwrap_or_default()
        }
        _ => String::new(),
    };
    let text = text.trim();
    if text.is_empty() {
        base.to_string()
    } else {
        format!("{base}: «{}»", crate::util::truncate(text, 48))
    }
}

/// A human-readable summary of what changed between two project states.
pub fn diff_projects(before: &Project, after: &Project) -> Vec<DiffEntry> {
    let mut out = Vec::new();
    if before.meta.title != after.meta.title {
        out.push(DiffEntry {
            kind: "meta".into(),
            summary: format!("Название: «{}» → «{}»", before.meta.title, after.meta.title),
            scene_id: None,
            node_id: None,
            details: vec![],
        });
    }

    let old_scenes = scene_map(before);
    let new_scenes = scene_map(after);

    for scene in &after.scenes {
        match old_scenes.get(scene.id.as_str()) {
            None => out.push(DiffEntry {
                kind: "scene-added".into(),
                summary: format!("Добавлена сцена «{}» ({} блоков)", scene.title, scene.nodes.len()),
                scene_id: Some(scene.id.clone()),
                node_id: None,
                details: scene.nodes.iter().map(node_label).collect(),
            }),
            Some(old) => {
                let mut details = Vec::new();
                if old.title != scene.title {
                    details.push(format!("Название: «{}» → «{}»", old.title, scene.title));
                }
                let old_nodes: HashMap<&str, &Node> =
                    old.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
                let new_ids: HashSet<&str> = scene.nodes.iter().map(|n| n.id.as_str()).collect();
                let mut added = 0;
                let mut changed = 0;
                for node in &scene.nodes {
                    match old_nodes.get(node.id.as_str()) {
                        None => {
                            added += 1;
                            details.push(format!("+ {}", node_label(node)));
                        }
                        Some(prev) => {
                            if serde_json::to_string(&prev.params).ok()
                                != serde_json::to_string(&node.params).ok()
                                || prev.kind != node.kind
                            {
                                changed += 1;
                                details.push(format!("~ {}", node_label(node)));
                            }
                        }
                    }
                }
                let removed = old
                    .nodes
                    .iter()
                    .filter(|n| !new_ids.contains(n.id.as_str()))
                    .count();
                for node in old.nodes.iter().filter(|n| !new_ids.contains(n.id.as_str())) {
                    details.push(format!("− {}", node_label(node)));
                }
                if !details.is_empty() {
                    let mut parts = Vec::new();
                    if added > 0 {
                        parts.push(format!("{added} добавлено"));
                    }
                    if removed > 0 {
                        parts.push(format!("{removed} удалено"));
                    }
                    if changed > 0 {
                        parts.push(format!("{changed} изменено"));
                    }
                    out.push(DiffEntry {
                        kind: if added + removed + changed > 0 {
                            "block-changed".into()
                        } else {
                            "scene-renamed".into()
                        },
                        summary: format!("Сцена «{}»: {}", scene.title, parts.join(", ")),
                        scene_id: Some(scene.id.clone()),
                        node_id: None,
                        details,
                    });
                }
            }
        }
    }
    for scene in &before.scenes {
        if !new_scenes.contains_key(scene.id.as_str()) {
            out.push(DiffEntry {
                kind: "scene-removed".into(),
                summary: format!("Удалена сцена «{}»", scene.title),
                scene_id: Some(scene.id.clone()),
                node_id: None,
                details: vec![],
            });
        }
    }

    // Characters
    let old_chars: HashMap<&str, &Character> =
        before.characters.iter().map(|c| (c.id.as_str(), c)).collect();
    let new_chars: HashMap<&str, &Character> =
        after.characters.iter().map(|c| (c.id.as_str(), c)).collect();
    for ch in &after.characters {
        match old_chars.get(ch.id.as_str()) {
            None => out.push(DiffEntry {
                kind: "characters".into(),
                summary: format!("Добавлен персонаж «{}»", ch.name),
                scene_id: None,
                node_id: None,
                details: vec![],
            }),
            Some(old) if old.name != ch.name || old.color != ch.color => out.push(DiffEntry {
                kind: "characters".into(),
                summary: format!("Изменён персонаж «{}»", ch.name),
                scene_id: None,
                node_id: None,
                details: vec![format!("«{}» → «{}»", old.name, ch.name)],
            }),
            _ => {}
        }
    }
    for ch in &before.characters {
        if !new_chars.contains_key(ch.id.as_str()) {
            out.push(DiffEntry {
                kind: "characters".into(),
                summary: format!("Удалён персонаж «{}»", ch.name),
                scene_id: None,
                node_id: None,
                details: vec![],
            });
        }
    }

    // Variables
    let old_vars: HashMap<&str, &Variable> = before.variables.iter().map(|v| (v.id.as_str(), v)).collect();
    let new_vars: HashMap<&str, &Variable> = after.variables.iter().map(|v| (v.id.as_str(), v)).collect();
    for var in &after.variables {
        match old_vars.get(var.id.as_str()) {
            None => out.push(DiffEntry {
                kind: "variables".into(),
                summary: format!("Добавлена переменная «{}»", var.name),
                scene_id: None,
                node_id: None,
                details: vec![],
            }),
            Some(old) if old.default != var.default || old.var_type != var.var_type => out.push(DiffEntry {
                kind: "variables".into(),
                summary: format!("Изменена переменная «{}»", var.name),
                scene_id: None,
                node_id: None,
                details: vec![format!("{} → {}", old.default, var.default)],
            }),
            _ => {}
        }
    }
    for var in &before.variables {
        if !new_vars.contains_key(var.id.as_str()) {
            out.push(DiffEntry {
                kind: "variables".into(),
                summary: format!("Удалена переменная «{}»", var.name),
                scene_id: None,
                node_id: None,
                details: vec![],
            });
        }
    }

    out
}
