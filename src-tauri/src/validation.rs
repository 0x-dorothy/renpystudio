//! Project validation: the same rules guard the UI, the importer and the MCP server.

use crate::assets::AssetInfo;
use crate::generator::scene_labels;
use crate::model::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// `error` blocks running the project, `warning` is a likely mistake, `info` is a hint.
    pub severity: String,
    pub code: String,
    pub message: String,
    pub scene_id: Option<String>,
    pub node_id: Option<String>,
}

impl Issue {
    fn error(code: &str, message: impl Into<String>) -> Self {
        Issue {
            severity: "error".into(),
            code: code.into(),
            message: message.into(),
            scene_id: None,
            node_id: None,
        }
    }
    fn warning(code: &str, message: impl Into<String>) -> Self {
        Issue {
            severity: "warning".into(),
            code: code.into(),
            message: message.into(),
            scene_id: None,
            node_id: None,
        }
    }
    fn info(code: &str, message: impl Into<String>) -> Self {
        Issue {
            severity: "info".into(),
            code: code.into(),
            message: message.into(),
            scene_id: None,
            node_id: None,
        }
    }
    fn at(mut self, scene_id: &str, node_id: Option<&str>) -> Self {
        self.scene_id = Some(scene_id.to_string());
        self.node_id = node_id.map(|s| s.to_string());
        self
    }
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub issues: Vec<Issue>,
    pub errors: usize,
    pub warnings: usize,
    pub unreachable: Vec<String>,
    pub dead_ends: Vec<String>,
}

impl ValidationReport {
    #[allow(dead_code)]
    pub fn ok(&self) -> bool {
        self.errors == 0
    }
}

/// Collects every label a `jump`/`call` could legally target.
fn known_labels(project: &Project) -> (HashSet<String>, HashMap<String, String>) {
    let labels = scene_labels(project);
    let mut known: HashSet<String> = labels.values().cloned().collect();
    known.insert("start".into());
    for scene in &project.scenes {
        for node in &scene.nodes {
            if node.kind == NodeKind::LABEL {
                let p: LabelParams = node.typed();
                let name = slugify(&p.name);
                if !name.is_empty() {
                    known.insert(name);
                }
            }
        }
    }
    (known, labels)
}

/// Validates structure, references and assets.
pub fn validate(project: &Project, assets: &[AssetInfo]) -> ValidationReport {
    let mut report = ValidationReport::default();
    let (known, labels) = known_labels(project);
    let asset_paths: HashSet<&str> = assets.iter().map(|a| a.path.as_str()).collect();

    // --- scenes ---------------------------------------------------------
    if project.scenes.is_empty() {
        report.issues.push(Issue::warning(
            "no-scenes",
            "В проекте нет ни одной сцены — игра начнётся с пустого экрана.",
        ));
    }
    if let Some(start) = project.meta.start_scene.as_deref() {
        if project.scene_by_id(start).is_none() {
            report.issues.push(Issue::warning(
                "bad-start",
                "Стартовая сцена не найдена. Будет использована первая сцена проекта.",
            ));
        }
    }

    let mut label_owner: HashMap<String, String> = HashMap::new();
    for scene in &project.scenes {
        if let Some(label) = labels.get(&scene.id) {
            if let Some(_other) = label_owner.get(label) {
                report.issues.push(
                    Issue::warning(
                        "label-conflict",
                        format!("Две сцены претендуют на метку {label}. В коде вторая получит суффикс."),
                    )
                    .at(&scene.id, None),
                );
            } else {
                label_owner.insert(label.clone(), scene.id.clone());
            }
        }
    }

    // --- variables & characters ----------------------------------------
    let mut var_names: HashSet<String> = HashSet::new();
    for var in &project.variables {
        let name = slugify(&var.name);
        if !var_names.insert(name.clone()) {
            report.issues.push(Issue::error(
                "duplicate-variable",
                format!("Переменная «{name}» объявлена дважды."),
            ));
        }
        if name == "item" || name.is_empty() {
            report.issues.push(Issue::error(
                "bad-variable-name",
                format!("У переменной «{}» некорректное имя.", var.name),
            ));
        }
    }
    let mut char_vars: HashSet<String> = HashSet::new();
    for ch in &project.characters {
        let base = if let Some(prefix) = ch.sprite_prefix.as_deref().filter(|p| !p.trim().is_empty()) {
            slugify(prefix)
        } else {
            slugify(&ch.name)
        };
        let var = match ch.kind {
            CharacterKind::Thought => format!("{base}_thought"),
            _ => base.clone(),
        };
        if ch.kind != CharacterKind::Narrator && !char_vars.insert(var.clone()) {
            report.issues.push(Issue::error(
                "duplicate-character",
                format!("Два персонажа используют одно имя в коде: {var}."),
            ));
        }
        if ch.kind == CharacterKind::Character && ch.name.trim().is_empty() {
            report.issues.push(Issue::warning(
                "empty-character-name",
                "У персонажа не заполнено имя — в игре будет пустая плашка.",
            ));
        }
    }

    // --- blocks ---------------------------------------------------------
    for scene in &project.scenes {
        let mut outgoing = 0usize;
        let mut ends_game = false;
        for node in &scene.nodes {
            match node.kind.as_str() {
                NodeKind::JUMP => {
                    let p: JumpParams = node.typed();
                    match p.target.as_deref().filter(|t| !t.trim().is_empty()) {
                        Some(target) => {
                            outgoing += 1;
                            if !known.contains(target) && project.scene_by_id(target).is_none() {
                                report.issues.push(
                                    Issue::error(
                                        "dangling-jump",
                                        format!(
                                            "Переход ведёт в никуда: «{target}». Выберите сцену из списка."
                                        ),
                                    )
                                    .at(&scene.id, Some(&node.id)),
                                );
                            }
                        }
                        None => report.issues.push(
                            Issue::warning("empty-jump", "Блок перехода не указывает цель.")
                                .at(&scene.id, Some(&node.id)),
                        ),
                    }
                }
                NodeKind::MENU => {
                    let p: MenuParams = node.typed();
                    if p.options.is_empty() {
                        report.issues.push(
                            Issue::warning("empty-menu", "Выбор без вариантов — игрок увидит пустое меню.")
                                .at(&scene.id, Some(&node.id)),
                        );
                    }
                    for opt in &p.options {
                        if let Some(target) = opt.target.as_deref().filter(|t| !t.trim().is_empty()) {
                            outgoing += 1;
                            if !known.contains(target) && project.scene_by_id(target).is_none() {
                                report.issues.push(
                                    Issue::error(
                                        "dangling-menu",
                                        format!("Вариант выбора ведёт в никуда: «{target}»."),
                                    )
                                    .at(&scene.id, Some(&node.id)),
                                );
                            }
                        }
                        if opt.text.values().all(|t| t.trim().is_empty()) {
                            report.issues.push(
                                Issue::warning(
                                    "empty-menu-text",
                                    "У варианта выбора пустой текст — игрок не поймёт, что нажимает.",
                                )
                                .at(&scene.id, Some(&node.id)),
                            );
                        }
                        for op in &opt.set_vars {
                            if !var_names.contains(&slugify(&op.variable)) {
                                report.issues.push(
                                    Issue::warning(
                                        "unknown-variable",
                                        format!("Вариант меняет неизвестную переменную «{}».", op.variable),
                                    )
                                    .at(&scene.id, Some(&node.id)),
                                );
                            }
                        }
                    }
                }
                NodeKind::IF => {
                    let p: IfParams = node.typed();
                    for target in [p.then_target.as_deref(), p.else_target.as_deref()]
                        .into_iter()
                        .flatten()
                        .filter(|t| !t.trim().is_empty())
                    {
                        outgoing += 1;
                        if !known.contains(target) && project.scene_by_id(target).is_none() {
                            report.issues.push(
                                Issue::error("dangling-if", format!("Условие ведёт в никуда: «{target}»."))
                                    .at(&scene.id, Some(&node.id)),
                            );
                        }
                    }
                    if p.mode == "builder" {
                        if let Some(var) = p.variable.as_deref() {
                            if !var_names.contains(&slugify(var)) {
                                report.issues.push(
                                    Issue::warning(
                                        "unknown-variable",
                                        format!("Условие использует неизвестную переменную «{var}»."),
                                    )
                                    .at(&scene.id, Some(&node.id)),
                                );
                            }
                        }
                    }
                }
                NodeKind::RETURN => {
                    ends_game = true;
                }
                NodeKind::SAY => {
                    let p: SayParams = node.typed();
                    if p.text.values().all(|t| t.trim().is_empty()) && p.voice.is_none() {
                        report.issues.push(
                            Issue::warning("empty-say", "Реплика без текста — в игре будет пустое окно.")
                                .at(&scene.id, Some(&node.id)),
                        );
                    }
                    if let Some(who) = p.who.as_deref() {
                        if project.character_by_id(who).is_none() {
                            report.issues.push(
                                Issue::error("unknown-character", "Реплика ссылается на удалённого персонажа.")
                                    .at(&scene.id, Some(&node.id)),
                            );
                        }
                    }
                }
                NodeKind::SCENE => {
                    let p: SceneParams = node.typed();
                    if !p.bg.trim().is_empty() && !p.bg.contains('/') && !p.bg.contains('.') {
                        // Bare image names are legitimate, nothing to check.
                    } else if !p.bg.trim().is_empty() && !asset_paths.contains(p.bg.as_str()) {
                        report.issues.push(
                            Issue::warning(
                                "missing-bg",
                                format!("Фон не найден в папке assets: {}", p.bg),
                            )
                            .at(&scene.id, Some(&node.id)),
                        );
                    }
                }
                NodeKind::MUSIC => {
                    let p: MusicParams = node.typed();
                    if let Some(file) = p.file.as_deref().filter(|f| !f.trim().is_empty()) {
                        if !asset_paths.contains(file) {
                            report.issues.push(
                                Issue::warning("missing-music", format!("Музыка не найдена: {file}"))
                                    .at(&scene.id, Some(&node.id)),
                            );
                        }
                    }
                }
                NodeKind::SOUND => {
                    let p: SoundParams = node.typed();
                    if !p.file.trim().is_empty() && !asset_paths.contains(p.file.as_str()) {
                        report.issues.push(
                            Issue::warning("missing-sound", format!("Звук не найден: {}", p.file))
                                .at(&scene.id, Some(&node.id)),
                        );
                    }
                }
                NodeKind::SHOW | NodeKind::POSE => {
                    let asset = serde_json::from_value::<ShowParams>(node.params.clone())
                        .ok()
                        .and_then(|p| p.asset)
                        .or_else(|| {
                            serde_json::from_value::<PoseParams>(node.params.clone())
                                .ok()
                                .and_then(|p| p.asset)
                        });
                    if let Some(asset) = asset.filter(|a| !a.trim().is_empty()) {
                        if !asset_paths.contains(asset.as_str()) {
                            report.issues.push(
                                Issue::warning("missing-sprite", format!("Спрайт не найден: {asset}"))
                                    .at(&scene.id, Some(&node.id)),
                            );
                        }
                    }
                }
                NodeKind::CODE => {
                    let p: CodeParams = node.typed();
                    if p.code.trim().is_empty() {
                        report.issues
                            .push(Issue::info("empty-code", "Пустой блок кода.").at(&scene.id, Some(&node.id)));
                    }
                }
                _ => {}
            }
        }
        if outgoing == 0 && !ends_game && !scene.nodes.is_empty() {
            report.dead_ends.push(scene.id.clone());
            report.issues.push(
                Issue::warning(
                    "dead-end",
                    format!(
                        "Сцена «{}» никуда не ведёт и не заканчивает историю — игрок застрянет.",
                        scene.title
                    ),
                )
                .at(&scene.id, None),
            );
        }
    }

    // --- reachability ---------------------------------------------------
    let start = project
        .meta
        .start_scene
        .clone()
        .filter(|id| project.scene_by_id(id).is_some())
        .or_else(|| project.scenes.first().map(|s| s.id.clone()));
    if let Some(start) = start {
        let mut seen: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(start.clone());
        seen.insert(start);
        while let Some(id) = queue.pop_front() {
            let Some(scene) = project.scene_by_id(&id) else { continue };
            for node in &scene.nodes {
                let targets: Vec<String> = match node.kind.as_str() {
                    NodeKind::JUMP => {
                        let p: JumpParams = node.typed();
                        p.target.into_iter().collect()
                    }
                    NodeKind::MENU => {
                        let p: MenuParams = node.typed();
                        p.options.into_iter().filter_map(|o| o.target).collect()
                    }
                    NodeKind::IF => {
                        let p: IfParams = node.typed();
                        [p.then_target, p.else_target].into_iter().flatten().collect()
                    }
                    _ => vec![],
                };
                for target in targets {
                    // Targets may be raw labels (jump inside a scene) — follow only scene ids.
                    if project.scene_by_id(&target).is_some() && seen.insert(target.clone()) {
                        queue.push_back(target);
                    }
                }
            }
        }
        for scene in &project.scenes {
            if !seen.contains(&scene.id) {
                report.unreachable.push(scene.id.clone());
                report.issues.push(
                    Issue::warning(
                        "unreachable",
                        format!("Сцена «{}» недостижима из начала истории.", scene.title),
                    )
                    .at(&scene.id, None),
                );
            }
        }
    }

    report.errors = report.issues.iter().filter(|i| i.severity == "error").count();
    report.warnings = report.issues.iter().filter(|i| i.severity == "warning").count();
    report
}

/// Edge list for the graph tab.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: String,
    pub from_scene: String,
    pub to_scene: Option<String>,
    /// scene | choice | condition | jump
    pub kind: String,
    pub label: String,
    pub node_id: String,
    pub condition: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GraphData {
    pub edges: Vec<GraphEdge>,
    pub dead_ends: Vec<String>,
    pub unreachable: Vec<String>,
    pub node_counts: HashMap<String, usize>,
}

pub fn graph_data(project: &Project) -> GraphData {
    let report = validate(project, &[]);
    let mut edges = Vec::new();
    let mut node_counts = HashMap::new();
    for scene in &project.scenes {
        node_counts.insert(scene.id.clone(), scene.nodes.len());
        for node in &scene.nodes {
            match node.kind.as_str() {
                NodeKind::JUMP => {
                    let p: JumpParams = node.typed();
                    if let Some(target) = p.target.filter(|t| !t.trim().is_empty()) {
                        edges.push(GraphEdge {
                            id: format!("{}:jump", node.id),
                            from_scene: scene.id.clone(),
                            to_scene: Some(target.clone()),
                            kind: "jump".into(),
                            label: String::new(),
                            node_id: node.id.clone(),
                            condition: None,
                        });
                    }
                }
                NodeKind::MENU => {
                    let p: MenuParams = node.typed();
                    for opt in p.options {
                        let text = opt
                            .text
                            .get(&project.meta.default_locale)
                            .cloned()
                            .or_else(|| opt.text.values().next().cloned())
                            .unwrap_or_default();
                        edges.push(GraphEdge {
                            id: format!("{}:{}", node.id, opt.id),
                            from_scene: scene.id.clone(),
                            to_scene: opt.target.clone(),
                            kind: "choice".into(),
                            label: text,
                            node_id: node.id.clone(),
                            condition: opt.condition.clone(),
                        });
                    }
                }
                NodeKind::IF => {
                    let p: IfParams = node.typed();
                    let cond = p
                        .expression
                        .clone()
                        .or_else(|| p.variable.clone())
                        .unwrap_or_else(|| "условие".into());
                    if let Some(t) = p.then_target {
                        edges.push(GraphEdge {
                            id: format!("{}:then", node.id),
                            from_scene: scene.id.clone(),
                            to_scene: Some(t),
                            kind: "condition".into(),
                            label: format!("{cond} → да"),
                            node_id: node.id.clone(),
                            condition: Some(cond.clone()),
                        });
                    }
                    if let Some(t) = p.else_target {
                        edges.push(GraphEdge {
                            id: format!("{}:else", node.id),
                            from_scene: scene.id.clone(),
                            to_scene: Some(t),
                            kind: "condition".into(),
                            label: format!("{cond} → нет"),
                            node_id: node.id.clone(),
                            condition: Some(cond),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    GraphData {
        edges,
        dead_ends: report.dead_ends,
        unreachable: report.unreachable,
        node_counts,
    }
}
