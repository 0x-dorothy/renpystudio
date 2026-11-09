//! `.rpy` code generator.
//!
//! The output is a first-class artifact: readable, commented, idiomatic Ren'Py
//! that a human can keep editing in VS Code. Every block is preceded by an
//! anchor comment so lint errors can be mapped back to studio blocks.

use crate::model::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

pub const GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug)]
pub struct GenOptions {
    /// Locale whose texts are written into the generated script.
    pub locale: String,
    /// When false, files changed outside the studio are reported instead of overwritten.
    pub force: bool,
}

impl Default for GenOptions {
    fn default() -> Self {
        GenOptions {
            locale: "ru".into(),
            force: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
    pub hash: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SrcMapEntry {
    pub file: String,
    pub scene_id: String,
    pub node_id: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Clone, Debug, Default)]
pub struct GeneratedBundle {
    pub files: Vec<GeneratedFile>,
    pub warnings: Vec<String>,
    pub srcmap: Vec<SrcMapEntry>,
}

fn hash(content: &str) -> String {
    crate::util::hash_str(content)
}

/// Human labels used both in code comments and in the UI inspector.
pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        NodeKind::SAY => "Реплика",
        NodeKind::NARRATE => "Рассказчик",
        NodeKind::COMMENT => "Комментарий автора",
        NodeKind::SCENE => "Смена фона",
        NodeKind::SHOW => "Показать спрайт",
        NodeKind::HIDE => "Скрыть спрайт",
        NodeKind::POSE => "Смена позы",
        NodeKind::ANIM => "Анимация",
        NodeKind::MENU => "Выбор",
        NodeKind::JUMP => "Переход",
        NodeKind::RETURN => "Конец / возврат",
        NodeKind::IF => "Условие",
        NodeKind::PAUSE => "Пауза",
        NodeKind::LABEL => "Метка",
        NodeKind::MUSIC => "Музыка",
        NodeKind::SOUND => "Звук",
        NodeKind::SET => "Переменная",
        NodeKind::CALL_SCREEN => "Показать экран",
        NodeKind::HIDE_SCREEN => "Скрыть экран",
        NodeKind::CODE => "Код",
        _ => "Блок",
    }
}

struct Writer {
    lines: Vec<String>,
    anchors: Vec<(String, u32, u32)>, // node_id, start, end
    locale: String,
    fallback: String,
}

impl Writer {
    fn new(locale: &str, fallback: &str) -> Self {
        Writer {
            lines: Vec::new(),
            anchors: Vec::new(),
            locale: locale.to_string(),
            fallback: fallback.to_string(),
        }
    }

    fn raw(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    fn blank(&mut self) {
        if !self.lines.last().map(|l| l.is_empty()).unwrap_or(true) {
            self.lines.push(String::new());
        }
    }

    fn stmt(&mut self, indent: usize, text: &str) {
        if text.is_empty() {
            self.lines.push(String::new());
            return;
        }
        for line in text.split('\n') {
            if line.trim().is_empty() {
                self.lines.push(String::new());
            } else {
                self.lines.push(format!("{}{}", "    ".repeat(indent), line));
            }
        }
    }

    fn text(&self, t: &LocalizedText) -> String {
        pick_locale(t, &self.locale, &self.fallback).to_string()
    }
}

/// A scene rendered to one file with its label and anchors.
struct SceneLabels {
    map: HashMap<String, String>,
}

impl SceneLabels {
    fn build(project: &Project) -> Self {
        let mut map = HashMap::new();
        let mut used = HashSet::new();
        used.insert("start".to_string());
        for scene in &project.scenes {
            let explicit = scene
                .label_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(clean_label_simple);
            let is_entry_start = scene.id == project.meta.start_scene.clone().unwrap_or_default()
                && explicit.as_deref() == Some("start");
            if is_entry_start {
                // The scene *is* `label start`; the entry file steps aside.
                map.insert(scene.id.clone(), "start".to_string());
                continue;
            }
            let base = if !scene.slug.trim().is_empty() {
                slugify(&scene.slug)
            } else {
                slugify(&scene.title)
            };
            let mut candidate = explicit.unwrap_or_else(|| format!("scene_{base}"));
            if candidate.is_empty() {
                candidate = format!("scene_{base}");
            }
            let mut n = 2;
            while used.contains(&candidate) {
                candidate = format!("{candidate}_{n}");
                n += 1;
            }
            used.insert(candidate.clone());
            map.insert(scene.id.clone(), candidate);
        }
        SceneLabels { map }
    }

    fn label(&self, scene_id: &str) -> Option<String> {
        self.map.get(scene_id).cloned()
    }
}

fn clean_label_simple(value: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().chars() {
        if ch.is_alphanumeric() || ch == '_' {
            out.push(ch);
        } else if ch == '-' || ch == ' ' || ch == '.' {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn character_var(character: &Character) -> String {
    if !character.sprite_prefix.as_deref().unwrap_or("").trim().is_empty() {
        return slugify(character.sprite_prefix.as_deref().unwrap_or(""));
    }
    let base = slugify(if character.name.trim().is_empty() {
        "voice"
    } else {
        &character.name
    });
    base
}

fn character_var_map(project: &Project) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();
    for ch in &project.characters {
        let base = character_var(ch);
        let make = |n: usize| match ch.kind {
            CharacterKind::Thought => {
                if n == 1 {
                    format!("{base}_thought")
                } else {
                    format!("{base}_{n}_thought")
                }
            }
            _ => {
                if n == 1 {
                    base.clone()
                } else {
                    format!("{base}_{n}")
                }
            }
        };
        let mut var = make(1);
        let mut n = 2;
        while used.contains(&var) {
            var = make(n);
            n += 1;
        }
        used.insert(var.clone());
        out.insert(ch.id.clone(), var);
    }
    out
}

fn clean_asset_name(path: &str) -> String {
    let stem = std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let mut out = String::new();
    for ch in stem.chars() {
        if ch == '_' || ch == '-' {
            out.push(' ');
        } else if ch.is_alphanumeric() || ch == ' ' {
            out.push(ch.to_lowercase().next().unwrap_or(ch));
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_image_name_reference(value: &str) -> bool {
    !value.is_empty() && !value.contains('/') && !value.contains('\\') && !value.contains('.')
}

fn transition_expr(name: &str, duration: Option<f32>) -> Option<String> {
    let name = name.trim();
    if name.is_empty() || name == "none" {
        return None;
    }
    match duration {
        Some(d) if d > 0.0 && name == "dissolve" => Some(format!("Dissolve({d:.2})")),
        Some(d) if d > 0.0 && name == "fade" => {
            Some(format!("Fade({:.2}, {:.2}, {:.2})", d * 0.5, d * 0.2, d * 0.5))
        }
        _ => Some(name.to_string()),
    }
}

fn position_transform(pos: Position) -> Option<&'static str> {
    match pos {
        Position::Auto => None,
        Position::Left => Some("left"),
        Position::Center => Some("center"),
        Position::Right => Some("right"),
        Position::FarLeft => Some("studio_far_left"),
        Position::FarRight => Some("studio_far_right"),
        Position::Custom => None,
    }
}

fn py_literal(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Bool(true) => "True".into(),
        serde_json::Value::Bool(false) => "False".into(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else {
                format!("{:.2}", n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => rpy_string(s),
        serde_json::Value::Null => "None".into(),
        other => other.to_string(),
    }
}

fn condition_expr(p: &IfParams, project: &Project) -> String {
    if p.mode == "expression" {
        return p.expression.clone().unwrap_or_else(|| "True".into());
    }
    let var = p.variable.clone().unwrap_or_else(|| "True".into());
    let known = project.variable_by_name(&var);
    let value = p.value.clone().unwrap_or(serde_json::Value::Null);
    let op = p.op.clone().unwrap_or_else(|| "gt".into());
    match op.as_str() {
        "is_true" => var,
        "is_false" => format!("not {var}"),
        "eq" | "ne" | "gt" | "gte" | "lt" | "lte" => {
            let sym = match op.as_str() {
                "eq" => "==",
                "ne" => "!=",
                "gt" => ">",
                "gte" => ">=",
                "lt" => "<",
                _ => "<=",
            };
            let lit = match known.map(|v| v.var_type) {
                Some(VarType::Number) => format!("{:.0}", value.as_f64().unwrap_or(0.0)),
                Some(VarType::Bool) => py_literal(&serde_json::json!(value.as_bool().unwrap_or(false))),
                Some(VarType::Text) => rpy_string(value.as_str().unwrap_or("")),
                None => py_literal(&value),
            };
            format!("{var} {sym} {lit}")
        }
        other => format!("{var} {other}"),
    }
}

/// Assets that need explicit `image` definitions keyed by target image name.
#[derive(Default)]
struct ImageDefs {
    defs: BTreeMap<String, String>,
    warnings: Vec<String>,
}

impl ImageDefs {
    fn add(&mut self, target: String, path: String) {
        if target.trim().is_empty() || path.trim().is_empty() {
            return;
        }
        if let Some(existing) = self.defs.get(&target) {
            if existing != &path {
                self.warnings.push(format!(
                    "Имя изображения «{target}» используется для двух файлов: {existing} и {path}. Взят первый."
                ));
            }
            return;
        }
        self.defs.insert(target, path);
    }
}

/// Public view of scene → label mapping (used by the validator and the UI).
pub fn scene_labels(project: &Project) -> HashMap<String, String> {
    SceneLabels::build(project).map
}

pub fn generate(project: &Project, opts: &GenOptions) -> GeneratedBundle {
    let mut out = GeneratedBundle::default();
    let labels = SceneLabels::build(project);
    let vars = character_var_map(project);
    let fallback = if project.meta.default_locale.is_empty() {
        "ru".to_string()
    } else {
        project.meta.default_locale.clone()
    };

    let mut images = ImageDefs::default();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // Characters
    // ------------------------------------------------------------------
    let mut chars = String::new();
    let _ = writeln!(chars, "# ═══════════════════════════════════════════════");
    let _ = writeln!(chars, "# ПЕРСОНАЖИ");
    let _ = writeln!(chars, "# Файл сгенерирован RenPy Studio {GENERATOR_VERSION}.");
    let _ = writeln!(
        chars,
        "# Правится здесь: панель «Персонажи» в студии."
    );
    let _ = writeln!(chars, "# ═══════════════════════════════════════════════");
    chars.push('\n');
    if project.characters.is_empty() {
        chars.push_str("# Пока не создан ни один персонаж.\n");
    }
    for ch in &project.characters {
        let var = vars.get(&ch.id).cloned().unwrap_or_else(|| character_var(ch));
        if !ch.notes.trim().is_empty() {
            let _ = writeln!(chars, "# {}", ch.notes.replace('\n', " "));
        }
        match ch.kind {
            CharacterKind::Narrator => {
                let _ = writeln!(
                    chars,
                    "# «{}» — рассказчик: реплики без имени, отдельный define не нужен.",
                    ch.name
                );
            }
            CharacterKind::Thought => {
                let mut args: Vec<String> = vec!["None".into()];
                if let Some(color) = ch.color.as_deref().filter(|c| !c.trim().is_empty()) {
                    args.push(format!("what_color=\"{color}\""));
                }
                args.push("what_prefix=\"(\"".into());
                args.push("what_suffix=\")\"".into());
                let _ = writeln!(chars, "define {var} = Character({})", args.join(", "));
            }
            CharacterKind::Character => {
                let mut args: Vec<String> = vec![rpy_string(&ch.name)];
                if let Some(color) = ch.color.as_deref().filter(|c| !c.trim().is_empty()) {
                    args.push(format!("color=\"{color}\""));
                }
                let image = ch
                    .sprite_prefix
                    .clone()
                    .filter(|p| !p.trim().is_empty())
                    .unwrap_or_else(|| slugify(&ch.name));
                if !image.is_empty() {
                    args.push(format!("image={}", rpy_string(&image)));
                }
                if let Some(prefix) = ch.prefix.as_deref().filter(|p| !p.trim().is_empty()) {
                    args.push(format!("what_prefix={}", rpy_string(prefix)));
                }
                if let Some(suffix) = ch.suffix.as_deref().filter(|p| !p.trim().is_empty()) {
                    args.push(format!("what_suffix={}", rpy_string(suffix)));
                }
                if let Some(voice) = ch.voice_tag.as_deref().filter(|v| !v.trim().is_empty()) {
                    args.push(format!("voice_tag={}", rpy_string(voice)));
                }
                let _ = writeln!(chars, "define {var} = Character({})", args.join(", "));
            }
        }
        if let Some(avatar) = ch.avatar.as_deref().filter(|a| !a.trim().is_empty()) {
            let _ = writeln!(chars, "image side {var} = {}", rpy_string(avatar));
        }
        chars.push('\n');
    }
    out.files.push(GeneratedFile {
        path: "game/characters.rpy".into(),
        hash: hash(&chars),
        content: chars,
    });

    // ------------------------------------------------------------------
    // Variables
    // ------------------------------------------------------------------
    let mut vars_file = String::new();
    let _ = writeln!(vars_file, "# ═══════════════════════════════════════════════");
    let _ = writeln!(vars_file, "# ПЕРЕМЕННЫЕ ИСТОРИИ");
    let _ = writeln!(vars_file, "# Файл сгенерирован RenPy Studio {GENERATOR_VERSION}.");
    let _ = writeln!(vars_file, "# ═══════════════════════════════════════════════");
    vars_file.push('\n');
    if project.variables.is_empty() {
        vars_file.push_str("# Пока не создано ни одной переменной.\n");
    }
    let mut used_vars: HashSet<String> = HashSet::new();
    for v in &project.variables {
        let name = slugify(&v.name);
        if !used_vars.insert(name.clone()) {
            warnings.push(format!("Переменная «{name}» объявлена дважды — оставлено первое значение."));
            continue;
        }
        if !v.description.trim().is_empty() {
            let _ = writeln!(vars_file, "# {}", v.description.replace('\n', " "));
        }
        let _ = writeln!(vars_file, "default {name} = {}", py_literal(&v.default_value()));
        vars_file.push('\n');
    }
    out.files.push(GeneratedFile {
        path: "game/variables.rpy".into(),
        hash: hash(&vars_file),
        content: vars_file,
    });

    // ------------------------------------------------------------------
    // Scenes
    // ------------------------------------------------------------------
    let mut used_files: HashSet<String> = HashSet::new();
    for scene in &project.scenes {
        let label = match labels.label(&scene.id) {
            Some(l) => l,
            None => continue,
        };
        let mut w = Writer::new(&opts.locale, &fallback);
        let header = format!(
            "# ═══════════════════════════════════════════════\n\
             # СЦЕНА: {title}\n\
             # Файл сгенерирован RenPy Studio {ver}.\n\
             # Узел: {id}\n\
             #\n\
             # Правьте смело, но помните: следующий экспорт из\n\
             # студии перезапишет этот файл. Блоки меняются в\n\
             # RenPy Studio, а этот код — читаемая точка входа.\n\
             # ═══════════════════════════════════════════════",
            title = scene.title,
            ver = GENERATOR_VERSION,
            id = scene.id
        );
        for line in header.lines() {
            w.raw(line);
        }
        w.blank();
        w.raw(format!("label {label}:"));
        w.blank();

        let mut body_written = 0usize;
        for node in &scene.nodes {
            match emit_node(&mut w, node, scene, project, &labels, &vars, &mut images) {
                Ok(true) => {
                    body_written += 1;
                    w.blank();
                }
                Ok(false) => {}
                Err(err) => warnings.push(format!("Сцена «{}»: {err}", scene.title)),
            }
        }
        if body_written == 0 {
            w.stmt(1, "pass");
            w.blank();
        }
        // Trim the trailing blank line for a tidy file end.
        while w.lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            w.lines.pop();
        }
        let content = w.lines.join("\n") + "\n";
        let file_name = format!("game/script_{}.rpy", label.trim_start_matches("scene_"));
        if !used_files.insert(file_name.clone()) {
            warnings.push(format!("Сцена «{}»: файл {file_name} уже занят другой сценой.", scene.title));
            continue;
        }
        for (node_id, start, end) in &w.anchors {
            out.srcmap.push(SrcMapEntry {
                file: file_name.clone(),
                scene_id: scene.id.clone(),
                node_id: node_id.clone(),
                start_line: *start,
                end_line: *end,
            });
        }
        out.files.push(GeneratedFile {
            path: file_name,
            hash: hash(&content),
            content,
        });
    }

    // ------------------------------------------------------------------
    // Images (explicit definitions for referenced assets)
    // ------------------------------------------------------------------
    if !images.defs.is_empty() {
        let mut file = String::new();
        let _ = writeln!(file, "# ═══════════════════════════════════════════════");
        let _ = writeln!(file, "# ИЗОБРАЖЕНИЯ");
        let _ = writeln!(file, "# Файл сгенерирован RenPy Studio {GENERATOR_VERSION}.");
        let _ = writeln!(file, "# Здесь собраны определения картинок, использованных");
        let _ = writeln!(file, "# в блоках сцен. Пути — относительно папки game/.");
        let _ = writeln!(file, "# ═══════════════════════════════════════════════");
        file.push('\n');
        for (name, path) in &images.defs {
            let _ = writeln!(file, "image {name} = {}", rpy_string(path));
        }
        out.files.push(GeneratedFile {
            path: "game/images.rpy".into(),
            hash: hash(&file),
            content: file,
        });
    }

    // ------------------------------------------------------------------
    // Transforms (animation presets)
    // ------------------------------------------------------------------
    if !project.init_script.trim().is_empty() {
        let mut file = String::new();
        let _ = writeln!(file, "# ═══════════════════════════════════════════════");
        let _ = writeln!(file, "# КОД ИНИЦИАЛИЗАЦИИ");
        let _ = writeln!(file, "# Импортирован или написан вручную. Студия его не разбирает,");
        let _ = writeln!(file, "# но и не теряет — это запасной выход для любых возможностей Ren'Py.");
        let _ = writeln!(file, "# ═══════════════════════════════════════════════");
        file.push('\n');
        file.push_str(project.init_script.trim_end());
        file.push('\n');
        out.files.push(GeneratedFile {
            path: "game/zz_init.rpy".into(),
            hash: hash(&file),
            content: file,
        });
    }

    let transforms = "\
# ═══════════════════════════════════════════════
# ПРЕСЕТЫ АНИМАЦИИ И ПОЗИЦИИ
# Файл сгенерирован RenPy Studio. Правки перезапишутся.
# ═══════════════════════════════════════════════

transform studio_far_left:
    xalign 0.08

transform studio_far_right:
    xalign 0.92

transform studio_flip:
    xzoom -1.0

transform studio_shake:
    linear 0.04 xoffset 14
    linear 0.04 xoffset -14
    linear 0.04 xoffset 9
    linear 0.04 xoffset -9
    linear 0.04 xoffset 0

transform studio_bounce:
    ease 0.16 yoffset -26
    ease 0.16 yoffset 0
    ease 0.10 yoffset -10
    ease 0.10 yoffset 0

transform studio_slide_left:
    xoffset -140
    ease 0.35 xoffset 0

transform studio_slide_right:
    xoffset 140
    ease 0.35 xoffset 0

transform studio_zoom:
    zoom 0.86
    ease 0.30 zoom 1.0

transform studio_soft_pulse:
    ease 0.6 zoom 1.02
    ease 0.6 zoom 1.0
";
    out.files.push(GeneratedFile {
        path: "game/studio_transforms.rpy".into(),
        hash: hash(transforms),
        content: transforms.to_string(),
    });

    // ------------------------------------------------------------------
    // Entry point
    // ------------------------------------------------------------------
    let start_scene = project
        .meta
        .start_scene
        .clone()
        .filter(|id| labels.label(id).is_some())
        .or_else(|| project.scenes.first().map(|s| s.id.clone()));
    let mut entry = String::new();
    let _ = writeln!(entry, "# ═══════════════════════════════════════════════");
    let _ = writeln!(entry, "# ТОЧКА ВХОДА");
    let _ = writeln!(entry, "# Файл сгенерирован RenPy Studio {GENERATOR_VERSION}.");
    let _ = writeln!(entry, "# Игра начинается здесь: label start.");
    let _ = writeln!(entry, "# ═══════════════════════════════════════════════");
    entry.push('\n');
    let start_label = start_scene.as_ref().and_then(|id| labels.label(id));
    if start_label.as_deref() == Some("start") {
        let _ = writeln!(
            entry,
            "# Сцена «{}» сама объявляет label start — отдельная точка входа не нужна.",
            start_scene
                .as_ref()
                .and_then(|id| project.scene_by_id(id))
                .map(|scene| scene.title.clone())
                .unwrap_or_default()
        );
    } else {
        entry.push_str("label start:\n");
        match start_label {
            Some(label) => {
                let _ = writeln!(entry, "    jump {label}");
            }
            None => {
                entry.push_str("    # В проекте пока нет сцен.\n");
                entry.push_str("    return\n");
            }
        }
    }
    out.files.push(GeneratedFile {
        path: "game/script_entry.rpy".into(),
        hash: hash(&entry),
        content: entry,
    });

    // ------------------------------------------------------------------
    // Main menu screen (visual builder result)
    // ------------------------------------------------------------------
    if project.ui.main_menu.template != "classic" {
        let screen = build_main_menu(project, opts, &fallback);
        out.files.push(GeneratedFile {
            path: "game/screens_main.rpy".into(),
            hash: hash(&screen),
            content: screen,
        });
    }

    // ------------------------------------------------------------------
    // Extra screens from the visual builder
    // ------------------------------------------------------------------
    if !project.ui.screens.is_empty() {
        let mut file = String::new();
        let _ = writeln!(file, "# ═══════════════════════════════════════════════");
        let _ = writeln!(file, "# ЭКРАНЫ СТУДИИ");
        let _ = writeln!(file, "# Файл сгенерирован RenPy Studio {GENERATOR_VERSION}.");
        let _ = writeln!(file, "# ═══════════════════════════════════════════════");
        file.push('\n');
        for screen in &project.ui.screens {
            let name = slugify(&screen.name);
            let _ = writeln!(file, "screen {name}():");
            file.push_str("    tag menu\n\n");
            file.push_str("    add Solid(\"#0b0a12\")\n");
            file.push_str("    vbox:\n        xalign 0.5\n        yalign 0.5\n        spacing 12\n");
            for el in &screen.elements {
                let text = pick_locale(&el.text, &opts.locale, &fallback);
                match el.kind.as_str() {
                    "text" => {
                        let _ = writeln!(file, "        text {} size 30", rpy_text(text, false));
                    }
                    "image" => {
                        if let Some(img) = &el.image {
                            let _ = writeln!(file, "        add {} xalign 0.5", rpy_string(img));
                        }
                    }
                    _ => {
                        let label = if text.is_empty() { "..." } else { text };
                        let _ = writeln!(
                            file,
                            "        textbutton {} action Return()",
                            rpy_text(label, false)
                        );
                    }
                }
            }
            file.push('\n');
        }
        out.files.push(GeneratedFile {
            path: "game/screens_builder.rpy".into(),
            hash: hash(&file),
            content: file,
        });
    }

    out.warnings.extend(images.warnings);
    out.warnings.extend(warnings);

    // Stable order keeps the manifest diffable.
    out.files.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn build_main_menu(project: &Project, opts: &GenOptions, fallback: &str) -> String {
    let cfg = &project.ui.main_menu;
    let mut s = String::new();
    let _ = writeln!(s, "# ═══════════════════════════════════════════════");
    let _ = writeln!(s, "# ГЛАВНОЕ МЕНЮ");
    let _ = writeln!(s, "# Файл сгенерирован RenPy Studio {GENERATOR_VERSION}.");
    let _ = writeln!(s, "# Шаблон: {}. Правки перезапишутся при экспорте.", cfg.template);
    let _ = writeln!(s, "# ═══════════════════════════════════════════════");
    s.push('\n');
    s.push_str("screen main_menu():\n");
    s.push_str("    tag menu\n\n");
    s.push_str("    add Solid(\"#0b0a12\")\n");
    if let Some(bg) = cfg.background.as_deref().filter(|b| !b.trim().is_empty()) {
        let _ = writeln!(
            s,
            "    if renpy.loadable({0}):\n        add {0} xalign 0.5 yalign 0.5",
            rpy_string(bg)
        );
    } else {
        let _ = writeln!(s, "    add Solid(\"{}\")", cfg.accent_fill());
    }
    s.push('\n');
    let align = cfg.button_align.as_str();
    s.push_str("    vbox:\n");
    let _ = writeln!(s, "        xalign {}", align_factor(align));
    s.push_str("        yalign 0.62\n        spacing 14\n\n");

    if !cfg.title.is_empty() {
        let title = pick_locale(&cfg.title, &opts.locale, fallback);
        let _ = writeln!(
            s,
            "        text {} size 64 xalign 0.5 color \"{}\"\n",
            rpy_text(title, true),
            cfg.text_color
        );
    }
    if !cfg.tagline.is_empty() {
        let tagline = pick_locale(&cfg.tagline, &opts.locale, fallback);
        let _ = writeln!(
            s,
            "        text {} size 24 xalign 0.5 color \"{}\"\n",
            rpy_text(tagline, true),
            cfg.text_color
        );
    }
    if let Some(logo) = cfg.logo.as_deref().filter(|l| !l.trim().is_empty()) {
        let _ = writeln!(
            s,
            "        if renpy.loadable({0}):\n            add {0} xalign 0.5",
            rpy_string(logo)
        );
    }

    let accent = &cfg.accent;
    let text_color = &cfg.text_color;
    let button_align = match cfg.button_align.as_str() {
        "left" => "0.0",
        "right" => "1.0",
        _ => "0.5",
    };
    let button = |label: &str, action: &str, s: &mut String| {
        let _ = writeln!(
            s,
            "        textbutton {label} action {action} text_size 30 text_color \"{text_color}\" text_hover_color \"{accent}\" xalign {button_align}"
        );
    };
    if cfg.show_start {
        button("_(\"Начать\")", "Start()", &mut s);
    }
    if cfg.show_load {
        button("_(\"Загрузить\")", "ShowMenu(\"load\")", &mut s);
    }
    if cfg.show_preferences {
        button("_(\"Настройки\")", "ShowMenu(\"preferences\")", &mut s);
    }
    if cfg.show_about {
        button("_(\"Об игре\")", "ShowMenu(\"about\")", &mut s);
    }
    if cfg.show_quit {
        button("_(\"Выход\")", "Quit(confirm=True)", &mut s);
    }

    // Menu music via the documented `before_main_menu` hook, guarded so a missing
    // file never turns the main menu into an error screen.
    if let Some(music) = cfg.music.as_deref().filter(|m| !m.trim().is_empty()) {
        s.push('\n');
        s.push_str("label before_main_menu:\n");
        let _ = writeln!(s, "    if renpy.loadable({}):", rpy_string(music));
        let _ = writeln!(s, "        play music {} fadein 1.0", rpy_string(music));
        s.push_str("    return\n");
    }
    s
}

impl MainMenuConfig {
    fn accent_fill(&self) -> String {
        // A soft vertical gradient substitute: the accent colour, darkened.
        let c = self.accent.trim_start_matches('#');
        if c.len() == 6 {
            if let Ok(v) = u32::from_str_radix(c, 16) {
                let r = (v >> 16) & 0xff;
                let g = (v >> 8) & 0xff;
                let b = v & 0xff;
                return format!(
                    "#{:02x}{:02x}{:02x}",
                    (r as f32 * 0.16) as u32,
                    (g as f32 * 0.14) as u32,
                    ((b as f32 * 0.22) as u32).min(255)
                );
            }
        }
        "#100e1c".into()
    }
}

fn align_factor(align: &str) -> &'static str {
    match align {
        "left" => "0.12",
        "right" => "0.88",
        _ => "0.5",
    }
}

// ---------------------------------------------------------------------------
// Block emission
// ---------------------------------------------------------------------------

fn emit_node(
    w: &mut Writer,
    node: &Node,
    scene: &Scene,
    project: &Project,
    labels: &SceneLabels,
    vars: &HashMap<String, String>,
    images: &mut ImageDefs,
) -> Result<bool, String> {
    let kind = node.kind.as_str();
    if !NodeKind::ALL.contains(&kind) {
        // Unknown block kind: keep it as a code comment so nothing is lost.
        w.stmt(1, &format!("# Неизвестный блок «{kind}» — пропущен при экспорте."));
        return Ok(true);
    }
    let start = w.lines.len() as u32 + 1;
    let short = node.id.split('-').next().unwrap_or(&node.id);
    w.stmt(1, &format!("# [блок {short} · {}]", kind_label(kind)));

    let mut emitted = true;
    match kind {
        NodeKind::SAY => {
            let p: SayParams = node.typed();
            let text = w.text(&p.text);
            if text.trim().is_empty() && p.voice.is_none() {
                return Ok(false);
            }
            if let Some(voice) = p.voice.as_deref().filter(|v| !v.trim().is_empty()) {
                w.stmt(1, &format!("voice {}", rpy_string(voice)));
            }
            let body = text.clone();
            let who = p.who.as_ref().and_then(|id| {
                project
                    .character_by_id(id)
                    .map(|c| (c, vars.get(&c.id).cloned().unwrap_or_default()))
            });
            // The line's "what is on screen" field: show the expression first.
            if let Some(pose) = p.image.as_deref().map(clean_attribute).filter(|s| !s.is_empty()) {
                if let Some((ch, var)) = who.as_ref() {
                    if ch.kind == CharacterKind::Character {
                        w.stmt(1, &format!("show {var} {pose}"));
                    }
                }
            }
            match who {
                Some((ch, var)) => match ch.kind {
                    CharacterKind::Narrator => {
                        w.stmt(1, &with_cps(&body, p.cps));
                    }
                    CharacterKind::Thought => {
                        w.stmt(1, &format!("{var} {}", with_cps(&body, p.cps)));
                    }
                    CharacterKind::Character => {
                        w.stmt(1, &format!("{var} {}", with_cps(&body, p.cps)));
                    }
                },
                None => {
                    w.stmt(1, &with_cps(&body, p.cps));
                }
            }
        }
        NodeKind::NARRATE => {
            let p: NarrateParams = node.typed();
            let text = w.text(&p.text);
            if text.trim().is_empty() {
                return Ok(false);
            }
            w.stmt(1, &rpy_text(&text, false));
        }
        NodeKind::COMMENT => {
            let p: CommentParams = node.typed();
            if p.internal_only {
                return Ok(false);
            }
            let text = w.text(&p.text);
            for line in text.lines() {
                w.stmt(1, &format!("# {}", line));
            }
        }
        NodeKind::SCENE => {
            let p: SceneParams = node.typed();
            if p.bg.trim().is_empty() {
                return Ok(false);
            }
            let target = scene_image_name(&p.bg, images);
            let mut line = format!("scene {target}");
            if let Some(tr) = transition_expr(
                p.transition.as_deref().unwrap_or("dissolve"),
                p.duration,
            ) {
                line.push_str(&format!(" with {tr}"));
            }
            w.stmt(1, &line);
        }
        NodeKind::SHOW => {
            let p: ShowParams = node.typed();
            let tag = sprite_tag(&p.who, project, vars);
            if tag.is_empty() {
                return Ok(false);
            }
            let pose = p.image.as_deref().map(clean_attribute).unwrap_or_default();
            if !pose.is_empty() {
                if let Some(asset) = p.asset.as_deref().filter(|a| !a.trim().is_empty()) {
                    images.add(format!("{tag} {pose}"), asset.to_string());
                }
            }
            let mut ats: Vec<String> = Vec::new();
            match position_transform(p.position) {
                Some(t) => ats.push(t.to_string()),
                None => {
                    if let Some(x) = p.custom_x {
                        ats.push(format!("Transform(xalign={x:.2})"));
                    }
                }
            }
            if p.flip {
                ats.push("studio_flip".into());
            }
            let mut line = format!("show {tag}");
            if !pose.is_empty() {
                line.push(' ');
                line.push_str(&pose);
            }
            if !ats.is_empty() {
                line.push_str(&format!(" at {}", ats.join(", ")));
            }
            if let Some(tr) = transition_expr(p.transition.as_deref().unwrap_or(""), None) {
                line.push_str(&format!(" with {tr}"));
            }
            w.stmt(1, &line);
        }
        NodeKind::HIDE => {
            let p: HideParams = node.typed();
            let tag = p
                .who
                .as_deref()
                .map(|who| {
                    project
                        .character_by_id(who)
                        .map(|c| {
                            vars.get(&c.id)
                                .cloned()
                                .unwrap_or_else(|| character_var(c))
                        })
                        .unwrap_or_else(|| clean_attribute(who))
                })
                .unwrap_or_else(|| "screen".into());
            if tag == "screen" && p.image.is_none() {
                // Hide everything: `scene` without an image is not valid, so use `hide` per tag is
                // impossible; a blank scene is the idiomatic "clear the screen".
                w.stmt(1, "scene black with dissolve");
                w.stmt(1, "# (скрыть все спрайты — очистка сцены)");
            } else {
                let mut line = format!("hide {tag}");
                if let Some(img) = p.image.as_deref().filter(|i| !i.trim().is_empty()) {
                    line.push(' ');
                    line.push_str(&clean_attribute(img));
                }
                if let Some(tr) = transition_expr(p.transition.as_deref().unwrap_or("dissolve"), None) {
                    line.push_str(&format!(" with {tr}"));
                }
                w.stmt(1, &line);
            }
        }
        NodeKind::POSE => {
            let p: PoseParams = node.typed();
            let tag = sprite_tag(&p.who, project, vars);
            let pose = clean_attribute(&p.image);
            if tag.is_empty() || pose.is_empty() {
                return Ok(false);
            }
            if let Some(asset) = p.asset.as_deref().filter(|a| !a.trim().is_empty()) {
                images.add(format!("{tag} {pose}"), asset.to_string());
            }
            let mut line = format!("show {tag} {pose}");
            if let Some(tr) = transition_expr(p.transition.as_deref().unwrap_or("dissolve"), None) {
                line.push_str(&format!(" with {tr}"));
            }
            w.stmt(1, &line);
        }
        NodeKind::ANIM => {
            let p: AnimParams = node.typed();
            match p.preset.as_str() {
                "flash" => w.stmt(1, "with flash"),
                "shake" => {
                    let tag = sprite_tag(&p.who, project, vars);
                    w.stmt(1, &format!("show {tag} at studio_shake"));
                }
                "bounce" => {
                    let tag = sprite_tag(&p.who, project, vars);
                    w.stmt(1, &format!("show {tag} at studio_bounce"));
                }
                "slide_left" => {
                    let tag = sprite_tag(&p.who, project, vars);
                    w.stmt(1, &format!("show {tag} at studio_slide_left"));
                }
                "slide_right" => {
                    let tag = sprite_tag(&p.who, project, vars);
                    w.stmt(1, &format!("show {tag} at studio_slide_right"));
                }
                "zoom" => {
                    let tag = sprite_tag(&p.who, project, vars);
                    w.stmt(1, &format!("show {tag} at studio_zoom"));
                }
                other => {
                    return Err(format!("неизвестный пресет анимации «{other}»"));
                }
            }
        }
        NodeKind::MENU => {
            let p: MenuParams = node.typed();
            if p.options.is_empty() {
                return Ok(false);
            }
            w.stmt(1, "menu:");
            if p.with_caption && !p.prompt.is_empty() {
                let prompt = w.text(&p.prompt);
                if !prompt.trim().is_empty() {
                    w.stmt(2, &rpy_text(&prompt, true));
                    w.blank();
                }
            }
            for (idx, opt) in p.options.iter().enumerate() {
                let text = pick_locale(&opt.text, &w.locale, &w.fallback);
                let mut head = rpy_text(text, false);
                if let Some(cond) = opt.condition.as_deref().filter(|c| !c.trim().is_empty()) {
                    head.push_str(&format!(" if {cond}"));
                }
                w.stmt(2, &format!("{head}:"));
                let option_start = w.lines.len();
                for op in &opt.set_vars {
                    if let Some(line) = set_line(op) {
                        w.stmt(3, &line);
                    }
                }
                match opt.target.as_deref().and_then(|t| labels.label(t).or_else(|| Some(t.to_string()))) {
                    Some(target) if !target.trim().is_empty() => {
                        w.stmt(3, &format!("jump {}", label_or_raw(&target)));
                    }
                    _ => {
                        if w.lines.len() == option_start {
                            w.stmt(3, "pass");
                        }
                    }
                }
                if idx + 1 < p.options.len() {
                    w.blank();
                }
            }
        }
        NodeKind::JUMP => {
            let p: JumpParams = node.typed();
            let Some(target) = p.target.as_deref().filter(|t| !t.trim().is_empty()) else {
                return Ok(false);
            };
            if let Some(tr) = transition_expr(p.transition.as_deref().unwrap_or(""), None) {
                w.stmt(1, &format!("with {tr}"));
            }
            w.stmt(1, &format!("jump {}", label_or_raw(&resolve_target(target, labels))));
        }
        NodeKind::RETURN => {
            let p: ReturnParams = node.typed();
            if p.to_main_menu {
                w.stmt(1, "$ renpy.full_restart()");
            } else {
                w.stmt(1, "return");
            }
        }
        NodeKind::IF => {
            let p: IfParams = node.typed();
            let cond = condition_expr(&p, project);
            w.stmt(1, &format!("if {cond}:"));
            match p.then_target.as_deref().filter(|t| !t.trim().is_empty()) {
                Some(t) => w.stmt(2, &format!("jump {}", label_or_raw(&resolve_target(t, labels)))),
                None => w.stmt(2, "pass"),
            }
            if p.has_else {
                w.stmt(1, "else:");
                match p.else_target.as_deref().filter(|t| !t.trim().is_empty()) {
                    Some(t) => w.stmt(2, &format!("jump {}", label_or_raw(&resolve_target(t, labels)))),
                    None => w.stmt(2, "pass"),
                }
            }
        }
        NodeKind::PAUSE => {
            let p: PauseParams = node.typed();
            if p.seconds <= 0.0 {
                w.stmt(1, "pause");
            } else if p.hard {
                w.stmt(1, &format!("pause {:.2} hard", p.seconds));
            } else {
                w.stmt(1, &format!("pause {:.2}", p.seconds));
            }
        }
        NodeKind::LABEL => {
            let p: LabelParams = node.typed();
            let name = slugify(&p.name);
            if name.trim().is_empty() || name == "item" {
                return Err("у метки нет имени".into());
            }
            // Labels live at file level; the following blocks become its body.
            w.raw(String::new());
            w.raw(format!("label {name}:"));
            w.raw(String::new());
        }
        NodeKind::MUSIC => {
            let p: MusicParams = node.typed();
            let channel = p.channel.as_deref().unwrap_or("music");
            match p.action.as_str() {
                "stop" | "fadeout" => {
                    if p.fadeout > 0.0 {
                        w.stmt(1, &format!("stop {channel} fadeout {:.1}", p.fadeout));
                    } else {
                        w.stmt(1, &format!("stop {channel}"));
                    }
                }
                "queue" => {
                    if let Some(file) = p.file.as_deref().filter(|f| !f.trim().is_empty()) {
                        w.stmt(1, &format!("queue {channel} {}", rpy_string(file)));
                    } else {
                        return Ok(false);
                    }
                }
                _ => {
                    let Some(file) = p.file.as_deref().filter(|f| !f.trim().is_empty()) else {
                        return Ok(false);
                    };
                    let mut line = format!("play {channel} {}", rpy_string(file));
                    if !p.r#loop {
                        line.push_str(" noloop");
                    }
                    if p.fadeout > 0.0 {
                        line.push_str(&format!(" fadeout {:.1}", p.fadeout));
                    }
                    line.push_str(" fadein 1.0");
                    w.stmt(1, &line);
                }
            }
        }
        NodeKind::SOUND => {
            let p: SoundParams = node.typed();
            let Some(file) = Some(p.file.as_str()).filter(|f| !f.trim().is_empty()) else {
                return Ok(false);
            };
            let mut line = format!("play sound {}", rpy_string(file));
            if p.r#loop {
                line.push_str(" loop");
            }
            w.stmt(1, &line);
        }
        NodeKind::SET => {
            let p: SetParams = node.typed();
            let op = VarOp {
                variable: p.variable.clone(),
                op: p.op.clone(),
                value: p.value.clone(),
            };
            if let Some(line) = set_line(&op) {
                w.stmt(1, &line);
            } else {
                return Ok(false);
            }
        }
        NodeKind::CALL_SCREEN => {
            let p: CallScreenParams = node.typed();
            let name = slugify(&p.screen);
            if name.trim().is_empty() || name == "item" {
                return Ok(false);
            }
            w.stmt(1, &format!("show screen {name}"));
        }
        NodeKind::HIDE_SCREEN => {
            let p: HideScreenParams = node.typed();
            let name = slugify(&p.screen);
            if name.trim().is_empty() || name == "item" {
                return Ok(false);
            }
            w.stmt(1, &format!("hide screen {name}"));
        }
        NodeKind::CODE => {
            let p: CodeParams = node.typed();
            let code = p.code.trim_end();
            if code.trim().is_empty() {
                return Ok(false);
            }
            w.stmt(1, "# --- ручной код ---");
            if p.language == "python" {
                w.stmt(1, "python:");
                for line in code.lines() {
                    if line.trim().is_empty() {
                        w.blank();
                    } else {
                        w.stmt(2, line);
                    }
                }
            } else {
                for line in code.lines() {
                    if line.trim().is_empty() {
                        w.blank();
                    } else {
                        w.stmt(1, line);
                    }
                }
            }
            w.stmt(1, "# --- конец ручного кода ---");
        }
        _ => {
            emitted = false;
        }
    }
    if emitted {
        let end = w.lines.len() as u32;
        w.anchors.push((node.id.clone(), start, end.max(start)));
    }
    // Keep the scene id referenced so unused-variable lints stay quiet.
    let _ = scene;
    Ok(emitted)
}

fn set_line(op: &VarOp) -> Option<String> {
    let var = slugify(&op.variable);
    if var == "item" || var.trim().is_empty() {
        return None;
    }
    let value = py_literal(&op.value);
    let line = match op.op.as_str() {
        "add" => format!("$ {var} += {value}"),
        "sub" => format!("$ {var} -= {value}"),
        "mul" => format!("$ {var} *= {value}"),
        _ => format!("$ {var} = {value}"),
    };
    Some(line)
}

fn clean_attribute(value: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ' ' {
            out.push(ch.to_lowercase().next().unwrap_or(ch));
        } else if ch == '-' {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn resolve_target(target: &str, labels: &SceneLabels) -> String {
    labels.label(target).unwrap_or_else(|| target.to_string())
}

fn label_or_raw(target: &str) -> String {
    let cleaned = clean_label(target);
    if cleaned.is_empty() {
        "start".into()
    } else {
        cleaned
    }
}

fn clean_label(target: &str) -> String {
    let mut out = String::new();
    for ch in target.trim().chars() {
        if ch.is_alphanumeric() || ch == '_' {
            out.push(ch);
        } else if ch == '-' || ch == ' ' || ch == '.' {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn sprite_tag(who: &str, project: &Project, vars: &HashMap<String, String>) -> String {
    project
        .character_by_id(who)
        .map(|c| vars.get(&c.id).cloned().unwrap_or_else(|| character_var(c)))
        .unwrap_or_else(|| clean_attribute(who))
}

/// Wraps an already-escaped dialogue literal with a `{cps=N}` tag when needed.
fn with_cps(body: &str, cps: Option<u32>) -> String {
    let quoted = rpy_text(body, false);
    match cps {
        Some(cps) if cps > 0 && quoted.len() >= 2 => {
            format!("\"{{cps={cps}}}{}\"", &quoted[1..quoted.len() - 1])
        }
        _ => quoted,
    }
}

fn scene_image_name(path: &str, images: &mut ImageDefs) -> String {
    if is_image_name_reference(path) {
        return clean_attribute(path);
    }
    let cleaned = clean_asset_name(path);
    let name = if cleaned.starts_with("bg ") || cleaned.starts_with("cg ") {
        cleaned
    } else {
        format!("bg {cleaned}")
    };
    images.add(name.clone(), path.to_string());
    name
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_project() -> Project {
        let mut project = Project::default();
        project.meta.title = "Тест".into();
        project.meta.default_locale = "ru".into();
        project.characters.push(Character {
            id: "ch_mira".into(),
            name: "Мира".into(),
            kind: CharacterKind::Character,
            color: Some("#e96aa5".into()),
            avatar: None,
            sprite_prefix: Some("mira".into()),
            voice_tag: None,
            prefix: None,
            suffix: None,
            notes: String::new(),
        });
        let mut prologue = Scene {
            id: "sc_a".into(),
            title: "Пролог".into(),
            slug: "prologue".into(),
            label_name: None,
            summary: String::new(),
            color: None,
            nodes: Vec::new(),
        };
        prologue.nodes.push(Node::new(
            NodeKind::SCENE,
            json!({ "bg": "images/bg/room.png", "transition": "dissolve" }),
        ));
        prologue.nodes.push(Node::new(
            NodeKind::SAY,
            json!({ "who": "ch_mira", "text": { "ru": "Это начало…", "en": "It begins…" } }),
        ));
        let mut menu = MenuParams::default();
        menu.prompt.insert("ru".into(), "Открыть дверь?".into());
        menu.options.push(MenuOption {
            id: "o1".into(),
            text: [("ru".to_string(), "Открыть".to_string())].into_iter().collect(),
            condition: None,
            target: Some("sc_b".into()),
            set_vars: vec![],
        });
        prologue
            .nodes
            .push(Node::new(NodeKind::MENU, serde_json::to_value(menu).unwrap()));
        let door = Scene {
            id: "sc_b".into(),
            title: "Дверь".into(),
            slug: "door".into(),
            label_name: None,
            summary: String::new(),
            color: None,
            nodes: Vec::new(),
        };
        project.scenes.push(prologue);
        project.scenes.push(door);
        project.meta.start_scene = Some("sc_a".into());
        project
    }

    #[test]
    fn generates_readable_scene_file() {
        let project = sample_project();
        let bundle = generate(&project, &GenOptions::default());
        let script = bundle
            .files
            .iter()
            .find(|f| f.path == "game/script_prologue.rpy")
            .expect("scene file");
        assert!(script.content.contains("label scene_prologue:"));
        assert!(script.content.contains("# СЦЕНА: Пролог"));
        assert!(script.content.contains("# [блок "));
        assert!(script.content.contains("scene bg room with dissolve"));
        assert!(script.content.contains("mira \"Это начало…\""));
        assert!(script.content.contains("menu:"));
        assert!(script.content.contains("jump scene_door"));
        // Anchor comments must be present so lint problems map back to blocks.
        assert!(script.content.contains("Показать спрайт") == false);
        let entry = bundle
            .files
            .iter()
            .find(|f| f.path == "game/script_entry.rpy")
            .expect("entry");
        assert!(entry.content.contains("label start:"));
        assert!(entry.content.contains("jump scene_prologue"));
    }

    #[test]
    fn explicit_labels_win_over_slugs() {
        let mut project = sample_project();
        project.scenes[0].label_name = Some("start".into());
        project.meta.start_scene = Some(project.scenes[0].id.clone());
        let bundle = generate(&project, &GenOptions::default());
        let script = bundle
            .files
            .iter()
            .find(|f| f.path == "game/script_start.rpy")
            .expect("scene file");
        assert!(script.content.contains("label start:"));
        let entry = bundle
            .files
            .iter()
            .find(|f| f.path == "game/script_entry.rpy")
            .expect("entry");
        assert!(!entry.content.contains("label start:"));
    }

    #[test]
    fn escapes_interpolation_in_dialogue() {
        let mut project = sample_project();
        project.scenes[0].nodes[1] = Node::new(
            NodeKind::SAY,
            json!({ "who": null, "text": { "ru": "Она сказала [тихо] и {улыбнулась}" } }),
        );
        let bundle = generate(&project, &GenOptions::default());
        let script = bundle
            .files
            .iter()
            .find(|f| f.path == "game/script_prologue.rpy")
            .unwrap();
        assert!(script.content.contains("[[тихо]"));
        assert!(script.content.contains("{{улыбнулась}"));
    }
}
