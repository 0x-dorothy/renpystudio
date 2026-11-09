//! Best-effort `.rpy` importer.
//!
//! Policy (see TZ §8.1): anything we recognise becomes a visual block, everything
//! else is preserved verbatim in a "Код" block, so an imported project keeps
//! running exactly as before.

use crate::model::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub files: usize,
    pub scenes: usize,
    pub recognized: usize,
    pub code_blocks: usize,
    pub characters: usize,
    pub variables: usize,
    pub warnings: Vec<String>,
    pub log: Vec<String>,
    pub skipped_files: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ImportInput {
    pub path: String,
    pub content: String,
}

#[derive(Clone, Debug)]
struct Line {
    indent: usize,
    text: String,
}

fn make_lines(content: &str) -> Vec<Line> {
    content
        .trim_start_matches('\u{feff}')
        .replace('\u{feff}', "")
        .replace("\r\n", "\n")
        .replace('\t', "    ")
        .lines()
        .map(|raw| {
            let trimmed = raw.trim_end();
            let indent = trimmed.len() - trimmed.trim_start().len();
            Line {
                indent,
                text: trimmed.trim_start().to_string(),
            }
        })
        .collect()
}

fn is_blank(line: &Line) -> bool {
    line.text.is_empty() || line.text.starts_with('#')
}

/// Index just past the body of the statement starting at `start`.
/// A statement's body is every following line indented deeper than `base`
/// (blank and comment lines are skipped).
fn block_end(lines: &[Line], start: usize, base: usize) -> usize {
    let mut i = start + 1;
    while i < lines.len() {
        if is_blank(&lines[i]) {
            i += 1;
            continue;
        }
        if lines[i].indent <= base {
            break;
        }
        i += 1;
    }
    i
}

/// Returns the raw block text (dedented so the statement starts at column 0).
fn raw_block(lines: &[Line], start: usize, end: usize) -> String {
    let mut out = String::new();
    for line in &lines[start..end] {
        if line.text.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&" ".repeat(line.indent.saturating_sub(lines[start].indent)));
            out.push_str(&line.text);
            out.push('\n');
        }
    }
    out.trim_end().to_string()
}

/// Parses `"text"` at the start of `s`, returning the unescaped value and the rest.
fn parse_string(s: &str) -> Option<(String, String)> {
    let s = s.trim_start();
    if !s.starts_with('"') {
        return None;
    }
    let mut value = String::new();
    let mut escaped = false;
    for (idx, ch) in s.char_indices().skip(1) {
        if escaped {
            match ch {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                other => {
                    value.push('\\');
                    value.push(other);
                }
            }
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => {
                let rest = s[idx + 1..].to_string();
                return Some((value, rest));
            }
            other => value.push(other),
        }
    }
    None
}

fn unescape_rpy_text(s: &str) -> String {
    s.replace("[[", "[")
        .replace("{{", "{")
        .replace("]]", "]")
        .replace("}}", "}")
}

fn humanize_label(label: &str) -> String {
    let base = label.strip_prefix("scene_").unwrap_or(label);
    let spaced = base.replace(['_', '.'], " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => "Сцена".into(),
    }
}

fn label_slug(label: &str) -> String {
    let base = label.strip_prefix("scene_").unwrap_or(label);
    slugify(base)
}

struct Ctx {
    characters: Vec<Character>,
    variables: Vec<Variable>,
    char_vars: std::collections::HashMap<String, String>,
    init_chunks: Vec<String>,
    report: ImportReport,
}

impl Ctx {
    fn new() -> Self {
        Ctx {
            characters: Vec::new(),
            variables: Vec::new(),
            char_vars: Default::default(),
            init_chunks: Vec::new(),
            report: ImportReport::default(),
        }
    }

    fn ensure_character(&mut self, var: &str) -> String {
        if let Some(id) = self.char_vars.get(var) {
            return id.clone();
        }
        let ch = Character {
            id: new_id(),
            name: humanize_label(var),
            kind: CharacterKind::Character,
            color: None,
            avatar: None,
            sprite_prefix: Some(var.to_string()),
            voice_tag: None,
            prefix: None,
            suffix: None,
            notes: "Импортировано из .rpy".into(),
        };
        self.char_vars.insert(var.to_string(), ch.id.clone());
        self.characters.push(ch.clone());
        self.report.characters += 1;
        self.report
            .warnings
            .push(format!("Персонаж «{var}» восстановлен по имени переменной в реплике."));
        ch.id
    }

    fn push_init(&mut self, code: String) {
        if !code.trim().is_empty() {
            self.init_chunks.push(code);
        }
    }
}

fn text_map(value: &str, locales: &[String]) -> LocalizedText {
    let mut map = LocalizedText::new();
    let cleaned = unescape_rpy_text(value);
    for loc in locales {
        map.insert(loc.clone(), cleaned.clone());
    }
    map
}

fn parse_character_define(text: &str) -> Option<(String, Character)> {
    // define m = Character("Мира", color="#e96aa5", image="mira")
    let rest = text.strip_prefix("define ")?;
    let (var, rhs) = rest.split_once('=')?;
    let var = var.trim();
    let rhs = rhs.trim();
    let inner = rhs.strip_prefix("Character(")?.strip_suffix(')')?;
    if !var.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let mut name = String::new();
    let mut color = None;
    let mut image = None;
    let mut prefix = None;
    let mut suffix = None;
    let mut kind = CharacterKind::Character;
    let mut voice_tag = None;

    // Split args on commas that are not inside strings or parentheses.
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for ch in inner.chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            '(' | '[' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }

    for (idx, arg) in args.iter().enumerate() {
        if arg == "None" {
            if idx == 0 {
                kind = CharacterKind::Thought;
            }
            continue;
        }
        if let Some((value, _)) = parse_string(arg) {
            if idx == 0 && name.is_empty() {
                name = unescape_rpy_text(&value);
            }
            continue;
        }
        if let Some((key, value)) = arg.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if let Some((string, _)) = parse_string(value) {
                match key {
                    "color" | "who_color" => color = Some(string),
                    "image" => image = Some(string),
                    "what_prefix" => prefix = Some(string),
                    "what_suffix" => suffix = Some(string),
                    "voice_tag" => voice_tag = Some(string),
                    _ => {}
                }
            }
            if key == "kind" && value.contains("narrator") {
                kind = CharacterKind::Narrator;
            }
        }
    }

    if name.is_empty() && kind == CharacterKind::Character {
        name = humanize_label(var);
    }

    Some((
        var.to_string(),
        Character {
            id: new_id(),
            name,
            kind,
            color,
            avatar: None,
            sprite_prefix: image.or_else(|| Some(var.to_string())),
            voice_tag,
            prefix,
            suffix,
            notes: "Импортировано из .rpy".into(),
        },
    ))
}

fn parse_default(text: &str) -> Option<(String, Variable)> {
    let rest = text.strip_prefix("default ")?;
    let (name, value) = rest.split_once('=')?;
    let name = name.trim();
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let value = value.trim();
    let (var_type, default) = if value == "True" || value == "False" {
        (VarType::Bool, json!(value == "True"))
    } else if let Ok(number) = value.parse::<f64>() {
        (VarType::Number, json!(number))
    } else if let Some((string, _)) = parse_string(value) {
        (VarType::Text, json!(unescape_rpy_text(&string)))
    } else {
        return None;
    };
    Some((
        name.to_string(),
        Variable {
            id: new_id(),
            name: name.to_string(),
            var_type,
            default,
            description: "Импортировано из .rpy".into(),
        },
    ))
}

fn parse_position(at_clause: &str) -> Option<(Position, Option<f32>)> {
    let first = at_clause.split(',').next()?.trim();
    match first {
        "left" => Some((Position::Left, None)),
        "center" | "truecenter" => Some((Position::Center, None)),
        "right" => Some((Position::Right, None)),
        "" => Some((Position::Auto, None)),
        _ => None,
    }
}

fn split_show(rest: &str) -> (String, Vec<String>, Option<String>, Option<String>) {
    // tag attrs... [at X] [with Y] [behind ...] [zorder N] [as Z]
    let mut tag = String::new();
    let mut attrs: Vec<String> = Vec::new();
    let mut at = None;
    let mut with = None;
    let mut tokens = rest.split_whitespace().peekable();
    if let Some(first) = tokens.next() {
        tag = first.to_string();
    }
    let mut mode = "attrs";
    let mut buffer: Vec<String> = Vec::new();
    for token in tokens {
        match token {
            "at" => {
                if mode == "at" && !buffer.is_empty() {
                    at = Some(buffer.join(" "));
                }
                buffer.clear();
                mode = "at";
            }
            "with" => {
                if mode == "at" && !buffer.is_empty() {
                    at = Some(buffer.join(" "));
                }
                buffer.clear();
                mode = "with";
            }
            "behind" | "as" | "zorder" | "onlayer" => {
                mode = "unsupported";
                buffer.clear();
            }
            other => match mode {
                "attrs" => attrs.push(other.to_string()),
                "at" => buffer.push(other.to_string()),
                "with" => buffer.push(other.to_string()),
                _ => {}
            },
        }
    }
    if mode == "at" && !buffer.is_empty() {
        at = Some(buffer.join(" "));
    }
    if mode == "with" && !buffer.is_empty() {
        with = Some(buffer.join(" "));
    }
    if mode == "unsupported" {
        with = Some("__unsupported__".into());
    }
    (tag, attrs, at, with)
}

struct Parser<'a> {
    lines: &'a [Line],
    ctx: &'a mut Ctx,
    locales: Vec<String>,
}

impl<'a> Parser<'a> {
    fn code_node(&mut self, start: usize, end: usize) -> Node {
        let raw = raw_block(self.lines, start, end);
        self.ctx.report.code_blocks += 1;
        Node {
            id: new_id(),
            kind: NodeKind::CODE.into(),
            params: json!({ "code": raw, "language": "renpy" }),
            source: NodeSource::Import,
            locked: false,
        }
    }

    fn parse_body(&mut self, start: usize, end: usize, base: usize) -> Vec<Node> {
        let mut out: Vec<Node> = Vec::new();
        let mut i = start;
        while i < end {
            if is_blank(&self.lines[i]) {
                i += 1;
                continue;
            }
            if self.lines[i].indent < base {
                break;
            }
            let stmt_start = i;
            let block_end_idx = block_end(self.lines, i, base);
            let text = self.lines[i].text.clone();
            let node = self.parse_statement(&text, i, block_end_idx, base, &mut out);
            if let Some(node) = node {
                out.push(node);
            }
            i = if block_end_idx > stmt_start { block_end_idx } else { i + 1 };
        }
        out
    }

    fn parse_statement(
        &mut self,
        text: &str,
        start: usize,
        end: usize,
        base: usize,
        previous: &mut Vec<Node>,
    ) -> Option<Node> {
        let trimmed = text.trim_end();
        // --- control flow -------------------------------------------------
        if trimmed == "menu:" || trimmed.starts_with("menu ") {
            if let Some(node) = self.parse_menu(start, end, base) {
                return Some(node);
            }
            return Some(self.code_node(start, end));
        }
        if trimmed.starts_with("if ") && trimmed.ends_with(':') {
            if let Some(node) = self.parse_if(start, end, base) {
                return Some(node);
            }
            return Some(self.code_node(start, end));
        }
        if trimmed == "python:" || trimmed.starts_with("python ") {
            let raw = raw_block(self.lines, start, end);
            let body = raw.lines().skip(1).map(|l| l.strip_prefix("    ").unwrap_or(l)).collect::<Vec<_>>().join("\n");
            self.ctx.report.code_blocks += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::CODE.into(),
                params: json!({ "code": body, "language": "python" }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if let Some(rest) = trimmed.strip_prefix('$') {
            if let Some(node) = self.parse_python_line(rest.trim()) {
                self.ctx.report.recognized += 1;
                return Some(node);
            }
            return Some(self.code_node(start, end));
        }

        // --- visuals ------------------------------------------------------
        if let Some(rest) = trimmed.strip_prefix("scene ") {
            let (image, with) = strip_with(rest);
            if image.trim().is_empty() || image.contains(' ') && image.split_whitespace().count() > 3 {
                return Some(self.code_node(start, end));
            }
            self.ctx.report.recognized += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::SCENE.into(),
                params: json!({
                    "bg": image.trim().to_lowercase(),
                    "transition": with.unwrap_or_else(|| "dissolve".into()),
                    "duration": null
                }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if let Some(rest) = trimmed.strip_prefix("show screen ") {
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            self.ctx.report.recognized += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::CALL_SCREEN.into(),
                params: json!({ "screen": name, "withNone": true }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if let Some(rest) = trimmed.strip_prefix("hide screen ") {
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            self.ctx.report.recognized += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::HIDE_SCREEN.into(),
                params: json!({ "screen": name }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if let Some(rest) = trimmed.strip_prefix("show ") {
            let (tag, attrs, at, with) = split_show(rest);
            if with.as_deref() == Some("__unsupported__") {
                return Some(self.code_node(start, end));
            }
            let (position, custom) = match at.as_deref() {
                Some(clause) => match parse_position(clause) {
                    Some(p) => p,
                    None => return Some(self.code_node(start, end)),
                },
                None => (Position::Auto, None),
            };
            let who_id = self
                .ctx
                .char_vars
                .get(&tag)
                .cloned()
                .unwrap_or_else(|| tag.clone());
            self.ctx.report.recognized += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::SHOW.into(),
                params: json!({
                    "who": who_id,
                    "image": attrs.join(" "),
                    "asset": null,
                    "position": position,
                    "customX": custom,
                    "transition": with.unwrap_or_default(),
                    "flip": false
                }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if let Some(rest) = trimmed.strip_prefix("hide ") {
            let (tag, attrs, at, _) = split_show(rest);
            if at.is_some() {
                return Some(self.code_node(start, end));
            }
            let who_id = self
                .ctx
                .char_vars
                .get(&tag)
                .cloned()
                .unwrap_or_else(|| tag.clone());
            self.ctx.report.recognized += 1;
            let image = attrs.join(" ");
            return Some(Node {
                id: new_id(),
                kind: NodeKind::HIDE.into(),
                params: json!({
                    "who": if who_id.is_empty() { None } else { Some(who_id) },
                    "image": if image.is_empty() { None } else { Some(image) },
                    "transition": "dissolve"
                }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if let Some(rest) = trimmed.strip_prefix("play ") {
            if let Some(node) = self.parse_play(rest) {
                self.ctx.report.recognized += 1;
                return Some(node);
            }
            return Some(self.code_node(start, end));
        }
        if let Some(rest) = trimmed.strip_prefix("stop ") {
            let mut parts = rest.split_whitespace();
            let channel = parts.next().unwrap_or("music");
            let mut fadeout = 0.0f32;
            let tokens: Vec<&str> = parts.collect();
            for (idx, token) in tokens.iter().enumerate() {
                if *token == "fadeout" {
                    if let Some(value) = tokens.get(idx + 1).and_then(|v| v.parse::<f32>().ok()) {
                        fadeout = value;
                    }
                }
            }
            self.ctx.report.recognized += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::MUSIC.into(),
                params: json!({
                    "action": "stop",
                    "file": null,
                    "fadeout": fadeout,
                    "loop": true,
                    "channel": if channel == "music" { serde_json::Value::Null } else { json!(channel) }
                }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if trimmed == "pause" || trimmed.starts_with("pause ") {
            let tokens: Vec<&str> = trimmed.split_whitespace().skip(1).collect();
            let hard = tokens.iter().any(|t| *t == "hard");
            let seconds = tokens
                .iter()
                .find_map(|t| t.parse::<f32>().ok())
                .unwrap_or(0.0);
            self.ctx.report.recognized += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::PAUSE.into(),
                params: json!({ "seconds": seconds, "hard": hard }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if let Some(rest) = trimmed.strip_prefix("jump ") {
            let target = rest.split_whitespace().next().unwrap_or("").to_string();
            if target.is_empty() {
                return Some(self.code_node(start, end));
            }
            self.ctx.report.recognized += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::JUMP.into(),
                params: json!({ "target": target, "transition": null }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if trimmed == "return" {
            self.ctx.report.recognized += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::RETURN.into(),
                params: json!({ "toMainMenu": false }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if let Some(rest) = trimmed.strip_prefix("voice ") {
            if let Some((voice, _)) = parse_string(rest) {
                if let Some(last) = previous.last_mut() {
                    if last.kind == NodeKind::SAY {
                        let mut params: SayParams = last.typed();
                        params.voice = Some(voice);
                        last.params = serde_json::to_value(params).unwrap_or(Value::Null);
                        return None;
                    }
                }
                return Some(self.code_node(start, end));
            }
        }
        if let Some(rest) = trimmed.strip_prefix("with ") {
            if let Some(last) = previous.last_mut() {
                let transition = rest.trim().to_string();
                match last.kind.as_str() {
                    NodeKind::SCENE => {
                        let mut p: SceneParams = last.typed();
                        p.transition = Some(transition);
                        last.params = serde_json::to_value(p).unwrap_or(Value::Null);
                        return None;
                    }
                    NodeKind::SHOW => {
                        let mut p: ShowParams = last.typed();
                        p.transition = Some(transition);
                        last.params = serde_json::to_value(p).unwrap_or(Value::Null);
                        return None;
                    }
                    _ => {}
                }
            }
            return Some(self.code_node(start, end));
        }

        // --- dialogue -----------------------------------------------------
        if let Some((text, _)) = parse_string(trimmed) {
            self.ctx.report.recognized += 1;
            return Some(Node {
                id: new_id(),
                kind: NodeKind::NARRATE.into(),
                params: json!({ "text": text_map(&text, &self.locales) }),
                source: NodeSource::Import,
                locked: false,
            });
        }
        if let Some((var, rest)) = split_identifier(trimmed) {
            if let Some((text, tail)) = parse_string(rest) {
                if tail.trim().is_empty() {
                    let who = if var == "narrator" {
                        None
                    } else {
                        Some(self.ctx.ensure_character(var))
                    };
                    self.ctx.report.recognized += 1;
                    return Some(Node {
                        id: new_id(),
                        kind: NodeKind::SAY.into(),
                        params: json!({
                            "who": who,
                            "text": text_map(&text, &self.locales),
                            "image": null,
                            "voice": null,
                            "cps": null
                        }),
                        source: NodeSource::Import,
                        locked: false,
                    });
                }
            }
        }

        // Anything else is preserved untouched.
        Some(self.code_node(start, end))
    }

    fn parse_menu(&mut self, start: usize, end: usize, base: usize) -> Option<Node> {
        let mut prompt = LocalizedText::new();
        let mut options: Vec<MenuOption> = Vec::new();
        let mut i = start + 1;
        while i < end {
            if is_blank(&self.lines[i]) {
                i += 1;
                continue;
            }
            if self.lines[i].indent <= base {
                break;
            }
            let option_indent = self.lines[i].indent;
            let text = self.lines[i].text.clone();
            // A bare string right under `menu:` is the caption.
            if let Some((caption, rest)) = parse_string(&text) {
                if rest.trim().is_empty() && options.is_empty() && prompt.is_empty() {
                    prompt = text_map(&caption, &self.locales);
                    i += 1;
                    continue;
                }
            }
            let Some(colon) = text.rfind(':') else {
                return None;
            };
            let head = text[..colon].trim();
            let Some((label_text, rest)) = parse_string(head) else {
                return None;
            };
            let condition = rest
                .trim()
                .strip_prefix("if ")
                .map(|c| c.trim().to_string());
            let body_start = i + 1;
            let body_end = block_end(self.lines, i, option_indent);
            let body = &self.lines[body_start..body_end];
            let mut set_vars: Vec<VarOp> = Vec::new();
            let mut target: Option<String> = None;
            let mut ok = true;
            for line in body {
                if is_blank(line) {
                    continue;
                }
                if let Some(rest) = line.text.strip_prefix('$') {
                    match self.parse_assignment(rest.trim()) {
                        Some(op) => set_vars.push(op),
                        None => {
                            ok = false;
                            break;
                        }
                    }
                    continue;
                }
                if let Some(rest) = line.text.strip_prefix("jump ") {
                    if target.is_some() {
                        ok = false;
                        break;
                    }
                    target = rest.split_whitespace().next().map(|s| s.to_string());
                    continue;
                }
                ok = false;
                break;
            }
            if !ok {
                return None;
            }
            options.push(MenuOption {
                id: new_id(),
                text: text_map(&label_text, &self.locales),
                condition,
                target,
                set_vars,
            });
            i = body_end;
        }
        if options.is_empty() {
            return None;
        }
        self.ctx.report.recognized += 1;
        Some(Node {
            id: new_id(),
            kind: NodeKind::MENU.into(),
            params: serde_json::to_value(MenuParams {
                prompt: prompt.clone(),
                options,
                with_caption: !prompt.is_empty(),
            })
            .unwrap_or(Value::Null),
            source: NodeSource::Import,
            locked: false,
        })
    }

    fn parse_if(&mut self, start: usize, end: usize, base: usize) -> Option<Node> {
        let head = self.lines[start].text.trim_end();
        let condition = head.strip_prefix("if ")?.strip_suffix(':')?.trim().to_string();
        let if_end = block_end(self.lines, start, base);
        let then_body: Vec<&Line> = self.lines[start + 1..if_end]
            .iter()
            .filter(|l| !is_blank(l))
            .collect();
        if then_body.len() != 1 {
            return None;
        }
        let Some(then_target) = then_body[0].text.strip_prefix("jump ") else {
            return None;
        };
        let then_target = then_target.split_whitespace().next()?.to_string();

        let mut has_else = false;
        let mut else_target = None;
        let mut i = if_end;
        while i < end {
            if is_blank(&self.lines[i]) {
                i += 1;
                continue;
            }
            if self.lines[i].indent != base {
                break;
            }
            let text = self.lines[i].text.trim_end();
            if text.starts_with("elif ") {
                return None;
            }
            if text == "else:" {
                let else_end = block_end(self.lines, i, base);
                let body: Vec<&Line> = self.lines[i + 1..else_end]
                    .iter()
                    .filter(|l| !is_blank(l))
                    .collect();
                if body.len() != 1 {
                    return None;
                }
                let target = body[0].text.strip_prefix("jump ")?.split_whitespace().next()?.to_string();
                has_else = true;
                else_target = Some(target);
                i = else_end;
                continue;
            }
            break;
        }
        self.ctx.report.recognized += 1;
        Some(Node {
            id: new_id(),
            kind: NodeKind::IF.into(),
            params: serde_json::to_value(IfParams {
                mode: "expression".into(),
                variable: None,
                op: None,
                value: None,
                expression: Some(condition),
                then_target: Some(then_target),
                else_target,
                has_else,
            })
            .unwrap_or(Value::Null),
            source: NodeSource::Import,
            locked: false,
        })
    }

    fn parse_play(&mut self, rest: &str) -> Option<Node> {
        let mut parts = rest.split_whitespace();
        let channel = parts.next()?;
        let mut file: Option<String> = None;
        let mut fadeout = 0.0f32;
        let mut volume: Option<f32> = None;
        let mut tokens: Vec<String> = parts.map(|s| s.to_string()).collect();
        if let Some((first, rest_after)) = parse_string(&tokens.join(" ")) {
            file = Some(first);
            // Keep tokens after the string for the flag scan.
            tokens = rest_after
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
        }
        for (idx, token) in tokens.iter().enumerate() {
            if token == "fadeout" {
                fadeout = tokens.get(idx + 1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            }
            if token == "volume" {
                volume = tokens.get(idx + 1).and_then(|v| v.parse().ok());
            }
        }
        let file = file?;
        match channel {
            "music" => Some(Node {
                id: new_id(),
                kind: NodeKind::MUSIC.into(),
                params: json!({
                    "action": "play",
                    "file": file,
                    "fadeout": fadeout,
                    "loop": !tokens.iter().any(|t| t == "noloop"),
                    "channel": null
                }),
                source: NodeSource::Import,
                locked: false,
            }),
            "sound" | "audio" => Some(Node {
                id: new_id(),
                kind: NodeKind::SOUND.into(),
                params: json!({ "file": file, "loop": tokens.iter().any(|t| t == "loop"), "volume": volume }),
                source: NodeSource::Import,
                locked: false,
            }),
            _ => None,
        }
    }

    fn parse_python_line(&mut self, code: &str) -> Option<Node> {
        let op = self.parse_assignment(code)?;
        Some(Node {
            id: new_id(),
            kind: NodeKind::SET.into(),
            params: serde_json::to_value(SetParams {
                variable: op.variable,
                op: op.op,
                value: op.value,
            })
            .unwrap_or(Value::Null),
            source: NodeSource::Import,
            locked: false,
        })
    }

    fn parse_assignment(&mut self, code: &str) -> Option<VarOp> {
        let (op_symbol, op) = if code.contains("+=") {
            ("+=", "add")
        } else if code.contains("-=") {
            ("-=", "sub")
        } else if code.contains("*=") {
            ("*=", "mul")
        } else {
            ("=", "set")
        };
        let (lhs, rhs) = code.split_once(op_symbol)?;
        let name = lhs.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        if name == "renpy" || name.starts_with("renpy.") || name.starts_with("persistent.") {
            return None;
        }
        let rhs = rhs.trim();
        let value = if rhs == "True" {
            json!(true)
        } else if rhs == "False" {
            json!(false)
        } else if let Ok(n) = rhs.parse::<f64>() {
            json!(n)
        } else if let Some((s, rest)) = parse_string(rhs) {
            if !rest.trim().is_empty() {
                return None;
            }
            json!(unescape_rpy_text(&s))
        } else {
            return None;
        };
        Some(VarOp {
            variable: name.to_string(),
            op: op.to_string(),
            value,
        })
    }
}

fn strip_with(rest: &str) -> (String, Option<String>) {
    match rest.split_once(" with ") {
        Some((image, with)) => (image.trim().to_string(), Some(with.trim().to_string())),
        None => (rest.trim().to_string(), None),
    }
}

fn split_identifier(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    let (head, rest) = s.split_at(end);
    if head.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
        Some((head, rest.trim_start()))
    } else {
        None
    }
}

/// Parses a set of `.rpy` files into a fresh studio project.
pub fn import(inputs: &[ImportInput], title: &str, folder_slug: &str) -> (Project, ImportReport) {
    let mut ctx = Ctx::new();
    let locales = default_locales();

    // Pass 1: characters, variables, top-level code.
    let mut label_blocks: Vec<(String, usize, usize, usize)> = Vec::new(); // name, file_idx, start, end
    let mut files_lines: Vec<Vec<Line>> = Vec::new();

    for input in inputs {
        let lines = make_lines(&input.content);
        ctx.report.files += 1;
        let mut i = 0;
        while i < lines.len() {
            if is_blank(&lines[i]) || lines[i].indent != 0 {
                i += 1;
                continue;
            }
            let text = lines[i].text.clone();
            if let Some(rest) = text.strip_prefix("label ") {
                if let Some(name) = rest.trim_end().strip_suffix(':') {
                    let end = block_end(&lines, i, 0);
                    label_blocks.push((name.trim().to_string(), files_lines.len(), i, end));
                    i = end;
                    continue;
                }
            }
            let end = block_end(&lines, i, 0);
            if let Some((var, character)) = parse_character_define(&text) {
                ctx.char_vars.insert(var, character.id.clone());
                ctx.characters.push(character);
                ctx.report.characters += 1;
                i = end;
                continue;
            }
            if let Some((name, variable)) = parse_default(&text) {
                if !ctx.variables.iter().any(|v| v.name == name) {
                    ctx.variables.push(variable);
                    ctx.report.variables += 1;
                }
                i = end;
                continue;
            }
            if text.starts_with("init ") || text.starts_with("init:") {
                ctx.push_init(raw_block(&lines, i, end));
                i = end;
                continue;
            }
            // Everything else at file level is kept verbatim for `zz_init.rpy`.
            ctx.push_init(raw_block(&lines, i, end));
            i = end;
        }
        files_lines.push(lines);
    }

    // Pass 2: labels → scenes.
    let mut scenes: Vec<Scene> = Vec::new();
    let mut label_to_scene: std::collections::HashMap<String, String> = Default::default();
    let mut original_labels: std::collections::HashMap<String, String> = Default::default();
    for (name, file_idx, start, end) in &label_blocks {
        let lines = &files_lines[*file_idx];
        let body_start = *start + 1;
        let body_indent = lines
            .iter()
            .skip(body_start)
            .find(|line| !is_blank(line))
            .map(|line| line.indent)
            .unwrap_or(4);
        let mut parser = Parser {
            lines,
            ctx: &mut ctx,
            locales: locales.clone(),
        };
        let nodes = parser.parse_body(body_start, *end, body_indent);
        let scene_id = new_id();
        let slug = label_slug(name);
        let scene = Scene {
            id: scene_id.clone(),
            title: humanize_label(name),
            slug: if slug.is_empty() { "scene".into() } else { slug },
            label_name: Some(name.clone()),
            summary: format!("Импортировано из label {name}"),
            color: None,
            nodes,
        };
        original_labels.insert(name.clone(), scene_id.clone());
        label_to_scene.insert(name.clone(), scene_id.clone());
        scenes.push(scene);
        ctx.report.scenes += 1;
    }

    // Pass 3: rewrite raw label targets to scene ids so jumps survive renaming.
    for scene in &mut scenes {
        for node in &mut scene.nodes {
            match node.kind.as_str() {
                NodeKind::JUMP => {
                    let mut p: JumpParams = node.typed();
                    if let Some(target) = p.target.clone() {
                        if let Some(id) = label_to_scene.get(&target) {
                            p.target = Some(id.clone());
                            node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                        }
                    }
                }
                NodeKind::MENU => {
                    let mut p: MenuParams = node.typed();
                    for opt in &mut p.options {
                        if let Some(target) = opt.target.clone() {
                            if let Some(id) = label_to_scene.get(&target) {
                                opt.target = Some(id.clone());
                            }
                        }
                    }
                    node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                }
                NodeKind::IF => {
                    let mut p: IfParams = node.typed();
                    let mut changed = false;
                    if let Some(target) = p.then_target.clone() {
                        if let Some(id) = label_to_scene.get(&target) {
                            p.then_target = Some(id.clone());
                            changed = true;
                        }
                    }
                    if let Some(target) = p.else_target.clone() {
                        if let Some(id) = label_to_scene.get(&target) {
                            p.else_target = Some(id.clone());
                            changed = true;
                        }
                    }
                    if changed {
                        node.params = serde_json::to_value(p).unwrap_or(Value::Null);
                    }
                }
                _ => {}
            }
        }
    }

    let start_scene = label_to_scene
        .get("start")
        .cloned()
        .or_else(|| scenes.first().map(|s| s.id.clone()));

    let mut project = Project::default();
    project.meta.title = title.to_string();
    project.meta.game_slug = folder_slug.to_string();
    project.meta.start_scene = start_scene;
    project.meta.description = "Импортировано в RenPy Studio".into();
    project.locales = locales;
    project.characters = ctx.characters.clone();
    project.variables = ctx.variables.clone();
    project.scenes = scenes;
    project.init_script = ctx.init_chunks.join("\n\n");
    // Imported projects keep their own screens.rpy: do not override the main menu.
    project.ui.main_menu.template = "classic".into();

    let mut report = ctx.report.clone();
    if report.scenes == 0 {
        report.warnings.push(
            "В выбранных файлах не найдено ни одного label — импортировать нечего.".into(),
        );
    }
    report.log.push(format!("Файлов: {}", report.files));
    report.log.push(format!("Сцен: {}", report.scenes));
    report.log.push(format!("Блоков распознано: {}", report.recognized));
    report.log.push(format!("Блоков кода: {}", report.code_blocks));
    report.log
        .push(format!("Персонажей: {}", report.characters));
    report.log.push(format!("Переменных: {}", report.variables));
    if !original_labels.is_empty() {
        report.log.push(format!("Меток: {}", original_labels.len()));
    }
    (project, report)
}

/// Reads `.rpy` files from a folder (skipping the generated/standard ones).
pub fn collect_sources(root: &std::path::Path) -> Result<(Vec<ImportInput>, Vec<String>), String> {
    const SKIP: [&str; 6] = [
        "gui.rpy",
        "screens.rpy",
        "options.rpy",
        "guisupport.rpy",
        "testcases.rpy",
        "zz_init.rpy",
    ];
    let game = root.join("game");
    let dir = if game.is_dir() { game } else { root.to_path_buf() };
    let mut inputs = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(&dir)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rpy") {
            continue;
        }
        // Translations and caches are not story sources; they stay untouched.
        if path
            .components()
            .any(|c| matches!(c.as_os_str().to_str(), Some("tl") | Some("cache") | Some("saves")))
        {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if SKIP.contains(&name) {
            if !skipped.iter().any(|s| s == name) {
                skipped.push(name.to_string());
            }
            continue;
        }
        if path.components().any(|c| c.as_os_str() == "renpy-studio") {
            continue;
        }
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        inputs.push(ImportInput {
            path: crate::assets::relative(root, path),
            content,
        });
    }
    Ok((inputs, skipped))
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Unused;
