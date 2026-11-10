//! Project data model (`renpy-studio/project.json`, format version 1).
//!
//! Design notes:
//! * The studio model is the source of truth; `.rpy` files are generated artifacts.
//! * Every object has a stable id (UUIDv7 style string) — references never use names.
//! * Texts are stored per locale so multilingual novels are possible from day one.
//! * `Node.kind` is a string (not an enum) on purpose: unknown kinds coming from a
//!   newer format version survive a round-trip instead of being lost.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const FORMAT_VERSION: u32 = 1;

/// `{"ru": "Привет", "en": "Hello"}`
pub type LocalizedText = BTreeMap<String, String>;

pub fn default_locales() -> Vec<String> {
    vec!["ru".to_string(), "en".to_string()]
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    pub meta: Meta,
    #[serde(default = "default_locales")]
    pub locales: Vec<String>,
    #[serde(default)]
    pub characters: Vec<Character>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    /// Plugins copied into this project (files live in game/, state lives here).
    #[serde(default)]
    pub plugins: Vec<PluginRef>,
    /// Raw Ren'Py statements emitted verbatim into `game/zz_init.rpy`
    /// (imported `screen`/`transform`/`init python`, or hand-written glue).
    #[serde(default)]
    pub init_script: String,
    #[serde(default)]
    pub ui: UiConfig,
}

fn default_format_version() -> u32 {
    FORMAT_VERSION
}

impl Default for Project {
    fn default() -> Self {
        Project {
            format_version: FORMAT_VERSION,
            meta: Meta::default(),
            locales: default_locales(),
            characters: Vec::new(),
            variables: Vec::new(),
            scenes: Vec::new(),
            plugins: Vec::new(),
            init_script: String::new(),
            ui: UiConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub renpy_version: String,
    /// Folder-name-friendly project slug, also used for build artifacts.
    #[serde(default)]
    pub game_slug: String,
    /// Scene shown first when the game starts (`label start` jumps here).
    #[serde(default)]
    pub start_scene: Option<String>,
    #[serde(default)]
    pub default_locale: String,
    #[serde(default = "now_iso")]
    pub created_at: String,
    #[serde(default = "now_iso")]
    pub updated_at: String,
}

impl Default for Meta {
    fn default() -> Self {
        Meta {
            title: "Новая новелла".into(),
            author: String::new(),
            description: String::new(),
            renpy_version: String::new(),
            game_slug: "novella".into(),
            start_scene: None,
            default_locale: "ru".into(),
            created_at: now_iso(),
            updated_at: now_iso(),
        }
    }
}

// ---------------------------------------------------------------------------
// Characters & variables
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CharacterKind {
    /// Normal speaking character: `mira "..."`.
    #[default]
    Character,
    /// Internal monologue: rendered in parentheses without a name box.
    Thought,
    /// Pure narrator (no name box, no define emitted).
    Narrator,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: CharacterKind,
    /// Folder with this character's sprites, relative to `game/images`.
    #[serde(default)]
    pub sprite_folder: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    /// Avatar / side image, path relative to `game/`.
    #[serde(default)]
    pub avatar: Option<String>,
    /// Ren'Py `image` tag used by `show <prefix> <pose>`; defaults to the slug of the name.
    #[serde(default)]
    pub sprite_prefix: Option<String>,
    #[serde(default)]
    pub voice_tag: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VarType {
    #[default]
    Number,
    Bool,
    Text,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub id: String,
    /// Python identifier, e.g. `trust`.
    pub name: String,
    #[serde(default)]
    pub var_type: VarType,
    #[serde(default)]
    pub default: Value,
    #[serde(default)]
    pub description: String,
}

impl Variable {
    /// Default value coerced to the declared type (keeps the generator valid).
    pub fn default_value(&self) -> Value {
        match self.var_type {
            VarType::Number => json!(self.default.as_f64().unwrap_or(0.0)),
            VarType::Bool => json!(self.default.as_bool().unwrap_or(false)),
            VarType::Text => json!(self.default.as_str().unwrap_or("")),
        }
    }
}

// ---------------------------------------------------------------------------
// Scenes & blocks
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: String,
    pub title: String,
    /// Label name; derived from the title when empty.
    #[serde(default)]
    pub slug: String,
    /// Explicit Ren'Py label (used for imported scenes so existing jumps keep working).
    #[serde(default)]
    pub label_name: Option<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub nodes: Vec<Node>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NodeSource {
    #[default]
    Studio,
    Mcp,
    Import,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    /// One of `NodeKind::ALL`. Stored as a string for forward compatibility.
    pub kind: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub source: NodeSource,
    /// Imported code is never rewritten by the studio.
    #[serde(default)]
    pub locked: bool,
}

impl Node {
    #[allow(dead_code)]
    pub fn new(kind: &str, params: Value) -> Self {
        Node {
            id: new_id(),
            kind: kind.to_string(),
            params,
            source: NodeSource::Studio,
            locked: false,
        }
    }

    pub fn typed<T: serde::de::DeserializeOwned + Default>(&self) -> T {
        serde_json::from_value(self.params.clone()).unwrap_or_default()
    }
}

pub struct NodeKind;

impl NodeKind {
    pub const SAY: &'static str = "say";
    pub const NARRATE: &'static str = "narrate";
    pub const COMMENT: &'static str = "comment";
    pub const SCENE: &'static str = "scene";
    pub const SHOW: &'static str = "show";
    pub const HIDE: &'static str = "hide";
    pub const POSE: &'static str = "pose";
    pub const ANIM: &'static str = "anim";
    pub const MENU: &'static str = "menu";
    pub const JUMP: &'static str = "jump";
    pub const RETURN: &'static str = "return";
    pub const IF: &'static str = "if";
    pub const PAUSE: &'static str = "pause";
    pub const LABEL: &'static str = "label";
    pub const MUSIC: &'static str = "music";
    pub const SOUND: &'static str = "sound";
    pub const SET: &'static str = "set";
    pub const CALL_SCREEN: &'static str = "show_screen";
    pub const HIDE_SCREEN: &'static str = "hide_screen";
    pub const CODE: &'static str = "code";
    pub const POINTCLICK: &'static str = "pointclick";

    pub const ALL: [&'static str; 21] = [
        Self::SAY,
        Self::NARRATE,
        Self::COMMENT,
        Self::SCENE,
        Self::SHOW,
        Self::HIDE,
        Self::POSE,
        Self::ANIM,
        Self::MENU,
        Self::JUMP,
        Self::RETURN,
        Self::IF,
        Self::PAUSE,
        Self::LABEL,
        Self::MUSIC,
        Self::SOUND,
        Self::SET,
        Self::CALL_SCREEN,
        Self::HIDE_SCREEN,
        Self::CODE,
        Self::POINTCLICK,
    ];

    /// Kinds that create an outgoing edge on the graph.
    #[allow(dead_code)]
    pub fn is_flow(kind: &str) -> bool {
        matches!(
            kind,
            Self::MENU | Self::JUMP | Self::IF | Self::RETURN | Self::POINTCLICK
        )
    }
}

// ---------------------------------------------------------------------------
// Block parameters (typed views over `Node.params`)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SayParams {
    /// Character id; `None` means narrator.
    pub who: Option<String>,
    pub text: LocalizedText,
    /// Sprite expression shown with the line, e.g. `happy`.
    pub image: Option<String>,
    pub voice: Option<String>,
    pub cps: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NarrateParams {
    pub text: LocalizedText,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CommentParams {
    pub text: LocalizedText,
    /// Show the note only in the studio, without emitting a comment line.
    pub internal_only: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SceneParams {
    /// Image path relative to `game/`, e.g. `images/bg/bg_room.png`.
    pub bg: String,
    /// Transition name: dissolve, fade, move, pixellate, wipeleft, ...
    pub transition: Option<String>,
    pub duration: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    /// No `at` clause at all (imported blocks keep their original placement).
    Auto,
    Left,
    #[default]
    Center,
    Right,
    FarLeft,
    FarRight,
    Custom,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ShowParams {
    /// Character id or image tag.
    pub who: String,
    /// Attribute shown in code: `show mira happy`.
    pub image: Option<String>,
    /// Actual asset file used by the image definition, path relative to `game/`.
    pub asset: Option<String>,
    pub position: Position,
    /// Raw ATL/xalign when `position == custom`.
    pub custom_x: Option<f32>,
    pub transition: Option<String>,
    pub flip: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HideParams {
    pub who: Option<String>,
    pub image: Option<String>,
    pub transition: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PoseParams {
    pub who: String,
    pub image: String,
    /// Asset file for this pose, path relative to `game/`.
    pub asset: Option<String>,
    pub transition: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AnimParams {
    pub who: String,
    /// `shake`, `bounce`, `flash`, `slide_left`, `slide_right`, `zoom_in`, `zoom_out`.
    pub preset: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MenuOption {
    pub id: String,
    pub text: LocalizedText,
    /// Ren'Py condition expression; empty = always visible.
    #[serde(default)]
    pub condition: Option<String>,
    /// Target scene id (preferred) or label name.
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub set_vars: Vec<VarOp>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct VarOp {
    pub variable: String,
    pub op: String, // set | add | sub | mul
    pub value: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MenuParams {
    pub prompt: LocalizedText,
    pub options: Vec<MenuOption>,
    /// Emit `menu:` with no caption when the prompt is empty.
    pub with_caption: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct JumpParams {
    /// Target scene id (preferred) or raw label.
    pub target: Option<String>,
    pub transition: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ReturnParams {
    /// `true` → restart to the main menu instead of returning to the caller.
    pub to_main_menu: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct IfParams {
    /// Expression source: `"builder"` (variable/op/value) or `"expression"`.
    pub mode: String,
    pub variable: Option<String>,
    pub op: Option<String>, // eq | ne | gt | gte | lt | lte | is_true | is_false
    pub value: Option<Value>,
    pub expression: Option<String>,
    /// Target scene id or label for the *then* branch.
    pub then_target: Option<String>,
    pub else_target: Option<String>,
    /// When false the `else` branch is simply skipped (no jump emitted).
    pub has_else: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PauseParams {
    /// 0 = wait for a click.
    pub seconds: f32,
    pub hard: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LabelParams {
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MusicParams {
    /// play | stop | fadeout | queue
    pub action: String,
    pub file: Option<String>,
    #[serde(default)]
    pub fadeout: f32,
    #[serde(default = "default_true")]
    pub r#loop: bool,
    #[serde(default)]
    pub channel: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SoundParams {
    pub file: String,
    #[serde(default)]
    pub r#loop: bool,
    pub volume: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SetParams {
    pub variable: String,
    /// set | add | sub | mul
    pub op: String,
    pub value: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CallScreenParams {
    pub screen: String,
    #[serde(default = "default_true")]
    pub with_none: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HideScreenParams {
    pub screen: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CodeParams {
    pub code: String,
    /// `renpy` (top-level statements) or `python` ($-block / python:).
    #[serde(default)]
    pub language: String,
}

// ---------------------------------------------------------------------------
// Point-and-click
// ---------------------------------------------------------------------------

/// A scene rendered as a background with clickable zones.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PointClickParams {
    /// Background image path (`images/...`), shown while the screen is open.
    pub bg: String,
    /// Optional line of text at the top of the screen.
    pub prompt: LocalizedText,
    pub hotspots: Vec<Hotspot>,
}

/// One clickable zone. All coordinates are normalized to the screen (0..1), so
/// the project renders correctly at any resolution.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Hotspot {
    pub id: String,
    /// Shown as a tooltip on hover and used as the graph edge label.
    pub label: LocalizedText,
    /// `rect` or `polygon`.
    pub shape: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Polygon vertices as normalized `[x, y]` pairs (empty for rectangles).
    pub points: Vec<Vec<f32>>,
    /// `jump` | `tooltip` (no action) | `return`.
    pub action: String,
    /// Scene id or raw label for `jump`.
    pub target: Option<String>,
}

impl Hotspot {
    /// Bounding box in normalized coordinates. Polygons derive it from their
    /// points so a hand-edited project can never desynchronise the two.
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        if self.shape == "polygon" && self.points.len() >= 3 {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for point in &self.points {
                if point.len() < 2 {
                    continue;
                }
                min_x = min_x.min(point[0]);
                min_y = min_y.min(point[1]);
                max_x = max_x.max(point[0]);
                max_y = max_y.max(point[1]);
            }
            if min_x <= max_x && min_y <= max_y {
                let x = min_x.clamp(0.0, 1.0);
                let y = min_y.clamp(0.0, 1.0);
                let w = (max_x - min_x).clamp(0.0, 1.0 - x);
                let h = (max_y - min_y).clamp(0.0, 1.0 - y);
                return (x, y, w, h);
            }
        }
        let x = self.x.clamp(0.0, 1.0);
        let y = self.y.clamp(0.0, 1.0);
        let w = self.w.clamp(0.0, 1.0 - x);
        let h = self.h.clamp(0.0, 1.0 - y);
        (x, y, w, h)
    }

    /// Polygon vertices relative to the bounding box (0..1), as Ren'Py needs
    /// them for the `focus_mask` callable.
    pub fn local_points(&self) -> Vec<(f32, f32)> {
        let (bx, by, bw, bh) = self.bounds();
        let sx = if bw > 0.0 { bw } else { 1.0 };
        let sy = if bh > 0.0 { bh } else { 1.0 };
        self.points
            .iter()
            .filter(|point| point.len() >= 2)
            .map(|point| ((point[0] - bx) / sx, (point[1] - by) / sy))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// UI (menus / screens builder)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MainMenuConfig {
    pub template: String,
    pub background: Option<String>,
    pub logo: Option<String>,
    pub title: LocalizedText,
    pub tagline: LocalizedText,
    pub music: Option<String>,
    pub accent: String,
    pub text_color: String,
    pub show_start: bool,
    pub show_load: bool,
    pub show_preferences: bool,
    pub show_about: bool,
    pub show_quit: bool,
    pub button_align: String,
}

impl Default for MainMenuConfig {
    fn default() -> Self {
        MainMenuConfig {
            template: "aurora".into(),
            background: None,
            logo: None,
            title: LocalizedText::new(),
            tagline: LocalizedText::new(),
            music: None,
            accent: "#8b6cf2".into(),
            text_color: "#f5f3ff".into(),
            show_start: true,
            show_load: true,
            show_preferences: true,
            show_about: true,
            show_quit: true,
            button_align: "center".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct UiConfig {
    pub main_menu: MainMenuConfig,
    /// Extra screens built in the visual builder.
    pub screens: Vec<ScreenDef>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenDef {
    pub id: String,
    pub name: String,
    pub template: String,
    pub title: LocalizedText,
    pub elements: Vec<ScreenElement>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenElement {
    pub id: String,
    /// text | button | image | vbox | hbox | frame | spacer
    pub kind: String,
    pub text: LocalizedText,
    pub image: Option<String>,
    pub action: Option<String>,
    pub x: f32,
    pub y: f32,
    pub style: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// UUIDv7 — time-ordered ids keep project.json diffs readable.
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Cyrillic-aware transliteration + slugification for labels and identifiers.
pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        let mapped: Option<&str> = match lower {
            'а' => Some("a"), 'б' => Some("b"), 'в' => Some("v"), 'г' => Some("g"),
            'д' => Some("d"), 'е' => Some("e"), 'ё' => Some("e"), 'ж' => Some("zh"),
            'з' => Some("z"), 'и' => Some("i"), 'й' => Some("y"), 'к' => Some("k"),
            'л' => Some("l"), 'м' => Some("m"), 'н' => Some("n"), 'о' => Some("o"),
            'п' => Some("p"), 'р' => Some("r"), 'с' => Some("s"), 'т' => Some("t"),
            'у' => Some("u"), 'ф' => Some("f"), 'х' => Some("h"), 'ц' => Some("ts"),
            'ч' => Some("ch"), 'ш' => Some("sh"), 'щ' => Some("sch"), 'ъ' => None,
            'ы' => Some("y"), 'ь' => None, 'э' => Some("e"), 'ю' => Some("yu"),
            'я' => Some("ya"),
            _ => None,
        };
        if let Some(m) = mapped {
            out.push_str(m);
        } else if lower.is_ascii_alphanumeric() {
            out.push(lower);
        } else if lower == '_' || lower == '-' || lower.is_whitespace() {
            if !out.ends_with('_') {
                out.push('_');
            }
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "item".to_string()
    } else if trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        format!("x_{trimmed}")
    } else {
        trimmed
    }
}

/// Escapes a Rust string for a Ren'Py double-quoted string literal.
pub fn rpy_string(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

/// Dialogue literal: escapes interpolation and text tags unless the author asked
/// for them explicitly (a stray `[` would otherwise crash the game).
pub fn rpy_text(s: &str, allow_tags: bool) -> String {
    let mut body = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    if !allow_tags {
        body = body.replace('[', "[[").replace('{', "{{");
    }
    format!("\"{body}\"")
}

/// Picks a locale text with a sensible fallback chain.
pub fn pick_locale<'a>(text: &'a LocalizedText, locale: &'a str, fallback: &'a str) -> &'a str {
    text.get(locale)
        .map(|s| s.as_str())
        .or_else(|| text.get(fallback).map(|s| s.as_str()))
        .or_else(|| text.values().next().map(|s| s.as_str()))
        .unwrap_or("")
}

impl Project {
    pub fn scene_by_id(&self, id: &str) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.id == id)
    }

    pub fn character_by_id(&self, id: &str) -> Option<&Character> {
        self.characters.iter().find(|c| c.id == id)
    }

    pub fn variable_by_name(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }

    pub fn touch(&mut self) {
        self.meta.updated_at = now_iso();
    }
}

/// A plugin copied into the project. Files live under `game/`; this is the state.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginRef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub enabled_at: String,
}
