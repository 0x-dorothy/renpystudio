//! Ready-made screens for the visual builder.
//!
//! A template owns the Ren'Py logic (slots, persistent unlocks, preferences,
//! music room, ...), the author only tunes a few parameters. Generated lines are
//! appended to `game/screens_builder.rpy` next to hand-built screens.

use crate::model::ScreenDef;
use serde::Deserialize;
use serde_json::Value;

pub struct Ctx<'a> {
    pub width: u32,
    pub locales: &'a [String],
    /// Primary locale of the project: default template texts follow it.
    pub locale: String,
    pub accent: String,
    pub text: String,
    pub muted: String,
    pub background: String,
}

impl Ctx<'_> {
    fn px(&self, base: f32) -> i32 {
        ((base * self.width.max(320) as f32) / 1280.0).round() as i32
    }

    /// Built-in template text in the project's language (ru defaults stay Russian).
    fn tr(&self, ru: &str, en: &str) -> String {
        if self.locale.trim().to_lowercase().starts_with("ru") {
            ru.to_string()
        } else {
            en.to_string()
        }
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn quoted(value: &str) -> String {
    crate::model::rpy_string(value.trim())
}

struct Lines {
    out: Vec<String>,
}

impl Lines {
    fn new() -> Self {
        Lines { out: Vec::new() }
    }

    fn p(&mut self, indent: usize, text: impl Into<String>) {
        self.out.push(format!("{}{}", "    ".repeat(indent), text.into()));
    }

    fn blank(&mut self) {
        if !self.out.last().map(|line| line.trim().is_empty()).unwrap_or(true) {
            self.out.push(String::new());
        }
    }
}

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SaveLoadParams {
    mode: String,
    title: String,
    columns: u32,
    rows: u32,
    thumbnails: bool,
    delete_key: bool,
}

impl SaveLoadParams {
    fn sane(mut self) -> Self {
        if self.columns == 0 {
            self.columns = 3;
        }
        if self.rows == 0 {
            self.rows = 2;
        }
        if self.mode != "load" {
            self.mode = "save".into();
        }
        self
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PreferencesParams {
    title: String,
    show_language: bool,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
struct ImageItem {
    image: String,
    title: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct GalleryParams {
    title: String,
    columns: u32,
    items: Vec<ImageItem>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
struct MediaItem {
    file: String,
    title: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct MusicParams {
    title: String,
    tracks: Vec<MediaItem>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
struct ChapterItem {
    title: String,
    target: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ChaptersParams {
    title: String,
    chapters: Vec<ChapterItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct NotificationParams {
    seconds: f32,
    position: String,
}

impl Default for NotificationParams {
    fn default() -> Self {
        NotificationParams {
            seconds: 3.0,
            position: "top-right".into(),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct HudParams {
    title: String,
    variable: String,
    label: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ConfirmParams {
    yes: String,
    no: String,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
struct StatItem {
    label: String,
    variable: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct StatsParams {
    title: String,
    stats: Vec<StatItem>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
struct JournalEntry {
    title: String,
    text: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct JournalParams {
    title: String,
    entries: Vec<JournalEntry>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct InventoryParams {
    title: String,
    columns: u32,
    items: Vec<ImageItem>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
struct PhoneMessage {
    who: String,
    text: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PhoneParams {
    title: String,
    messages: Vec<PhoneMessage>,
}

fn params<T: serde::de::DeserializeOwned + Default>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn render(screen: &ScreenDef, ctx: &Ctx) -> Option<Vec<String>> {
    let name = crate::model::slugify(&screen.name);
    if name == "item" {
        return None;
    }
    let mut lines = match screen.template.as_str() {
        "save_load" => render_save_load(&name, params(&screen.params), ctx),
        "preferences" => render_preferences(&name, params(&screen.params), ctx),
        "gallery" => render_gallery(&name, params(&screen.params), ctx),
        "music_room" => render_music(&name, params(&screen.params), ctx),
        "chapter_select" => render_chapters(&name, params(&screen.params), ctx),
        "notification" => render_notification(&name, params(&screen.params), ctx),
        "hud" => render_hud(&name, params(&screen.params), ctx),
        "confirm" => render_confirm(params(&screen.params), ctx),
        "stats" => render_stats(&name, params(&screen.params), ctx),
        "journal" => render_journal(&name, params(&screen.params), ctx),
        "inventory" => render_inventory(&name, params(&screen.params), ctx),
        "phone" => render_phone(&name, params(&screen.params), ctx),
        _ => return None,
    };
    lines.insert(
        0,
        format!("# ── Шаблон «{}» · {name} ─────────────────────────", screen.template),
    );
    Some(lines)
}

fn header(lines: &mut Lines, name: &str, title: &str, ctx: &Ctx, menu: bool) {
    lines.p(0, format!("screen {name}():"));
    if menu {
        lines.p(1, "tag menu");
        lines.p(1, "modal True");
    }
    lines.blank();
    lines.p(1, format!("add Solid({})", quoted(&ctx.background)));
    if !title.trim().is_empty() {
        lines.p(
            1,
            format!(
                "text {} xalign 0.5 ypos 0.035 size {} color {}",
                quoted(title),
                ctx.px(46.0),
                quoted(&ctx.text)
            ),
        );
    }
    lines.blank();
}

fn back_button(lines: &mut Lines, ctx: &Ctx, action: &str) {
    let label = ctx.tr("Назад", "Back");
    lines.p(
        1,
        format!(
            "textbutton {} action {action} xalign 0.5 ypos 0.93 text_size {}",
            quoted(&label),
            ctx.px(26.0)
        ),
    );
}

// ---------------------------------------------------------------------------
// Save / load
// ---------------------------------------------------------------------------

fn render_save_load(name: &str, raw: SaveLoadParams, ctx: &Ctx) -> Vec<String> {
    let mut p = raw.sane();
    if p.title.trim().is_empty() {
        p.title = if p.mode == "load" {
            ctx.tr("Загрузка", "Load")
        } else {
            ctx.tr("Сохранение", "Save")
        };
    }
    let mut lines = Lines::new();
    header(&mut lines, name, &p.title, ctx, true);
    let slot_w = ctx.px(340.0);
    let slot_h = ctx.px(216.0);
    let thumb_h = (slot_h - ctx.px(74.0)).max(40);
    let count = p.columns * p.rows;
    lines.p(1, format!("grid {} {}:", p.columns, p.rows));
    lines.p(2, "xalign 0.5");
    lines.p(2, format!("ypos {}", 0.13));
    lines.p(2, format!("spacing {}", ctx.px(18.0)));
    lines.blank();
    lines.p(2, format!("for _slot in range(1, {} + 1):", count));
    lines.p(3, "button:");
    if p.mode == "load" {
        lines.p(4, "action FileLoad(_slot)");
    } else {
        lines.p(4, "action FileSave(_slot, confirm=True)");
    }
    lines.p(4, format!("xysize ({slot_w}, {slot_h})"));
    lines.p(4, "background Solid(\"#15131f\")");
    lines.p(4, "hover_background Solid(\"#221f33\")");
    if p.delete_key {
        lines.p(4, "key \"save_delete\" action FileDelete(_slot)");
    }
    lines.blank();
    lines.p(4, "vbox:");
    lines.p(5, "spacing 6");
    if p.thumbnails {
        lines.p(
            5,
            format!(
                "add FileScreenshot(_slot) xysize ({}, {thumb_h}) xalign 0.5",
                slot_w - ctx.px(16.0)
            ),
        );
    } else {
        lines.p(
            5,
            format!(
                "add Solid(\"#0f0d18\") xysize ({}, {}) xalign 0.5",
                slot_w - ctx.px(16.0),
                thumb_h
            ),
        );
    }
    lines.p(
        5,
        format!(
            "text FileTime(_slot, format=\"%d.%m.%Y %H:%M\", empty=\"—\") xalign 0.5 size {} color {}",
            ctx.px(18.0),
            quoted(&ctx.muted)
        ),
    );
    lines.p(
        5,
        format!(
            "text FileSaveName(_slot) xalign 0.5 size {} color {}",
            ctx.px(16.0),
            quoted(&ctx.muted)
        ),
    );
    lines.blank();
    lines.p(1, "hbox:");
    lines.p(2, "xalign 0.5");
    lines.p(2, "ypos 0.88");
    lines.p(2, format!("spacing {}", ctx.px(14.0)));
    lines.p(2, format!("textbutton \"←\" action FilePagePrevious() text_size {}", ctx.px(24.0)));
    lines.p(2, format!("text FilePageName() size {} color {}", ctx.px(22.0), quoted(&ctx.text)));
    lines.p(2, format!("textbutton \"→\" action FilePageNext() text_size {}", ctx.px(24.0)));
    lines.blank();
    back_button(&mut lines, ctx, "Return()");
    lines.p(1, "# Hint");
    lines.p(
        1,
        format!(
            "text {} xalign 0.5 ypos 0.965 size {} color {}",
            quoted(&if p.mode == "load" {
                ctx.tr("Delete — удалить слот, клик — загрузить.", "Delete — delete a slot, click — load.")
            } else {
                ctx.tr("Delete — удалить слот, клик — перезаписать.", "Delete — delete a slot, click — overwrite.")
            }),
            ctx.px(16.0),
            quoted(&ctx.muted)
        ),
    );
    lines.out
}

// ---------------------------------------------------------------------------
// Preferences
// ---------------------------------------------------------------------------

fn render_preferences(name: &str, raw: PreferencesParams, ctx: &Ctx) -> Vec<String> {
    let title = if raw.title.trim().is_empty() {
        ctx.tr("Настройки", "Preferences")
    } else {
        raw.title
    };
    let mut lines = Lines::new();
    header(&mut lines, name, &title, ctx, true);

    lines.p(1, "hbox:");
    lines.p(2, "xalign 0.5");
    lines.p(2, "ypos 0.14");
    lines.p(2, format!("spacing {}", ctx.px(56.0)));
    lines.blank();

    // Text group.
    lines.p(2, "vbox:");
    lines.p(3, format!("spacing {}", ctx.px(10.0)));
    lines.p(3, format!("text {} size {} color {}", quoted(&ctx.tr("Текст", "Text")), ctx.px(26.0), quoted(&ctx.accent)));
    lines.p(3, format!("text {} size {} color {}", quoted(&ctx.tr("Скорость текста", "Text Speed")), ctx.px(18.0), quoted(&ctx.muted)));
    lines.p(3, format!("bar value Preference(\"text speed\") xsize {}", ctx.px(360.0)));
    lines.p(3, format!("text {} size {} color {}", quoted(&ctx.tr("Автопрокрутка", "Auto-Advance")), ctx.px(18.0), quoted(&ctx.muted)));
    lines.p(3, format!("bar value Preference(\"auto-forward time\") xsize {}", ctx.px(360.0)));
    lines.p(3, format!("textbutton {} action Preference(\"skip\", \"toggle\") text_size {}", quoted(&ctx.tr("Пропуск непрочитанного", "Skip Unseen")), ctx.px(20.0)));
    lines.p(3, format!("textbutton {} action Preference(\"after choices\", \"toggle\") text_size {}", quoted(&ctx.tr("Пропуск после выборов", "Skip After Choices")), ctx.px(20.0)));
    lines.blank();

    // Volume group.
    lines.p(2, "vbox:");
    lines.p(3, format!("spacing {}", ctx.px(10.0)));
    lines.p(3, format!("text {} size {} color {}", quoted(&ctx.tr("Звук", "Audio")), ctx.px(26.0), quoted(&ctx.accent)));
    lines.p(3, "if config.has_music:");
    lines.p(4, format!("text {} size {} color {}", quoted(&ctx.tr("Музыка", "Music")), ctx.px(18.0), quoted(&ctx.muted)));
    lines.p(4, format!("bar value Preference(\"music volume\") xsize {}", ctx.px(360.0)));
    lines.p(3, "if config.has_sound:");
    lines.p(4, format!("text {} size {} color {}", quoted(&ctx.tr("Эффекты", "Sound Effects")), ctx.px(18.0), quoted(&ctx.muted)));
    lines.p(4, format!("bar value Preference(\"sound volume\") xsize {}", ctx.px(360.0)));
    lines.p(3, "if config.has_voice:");
    lines.p(4, format!("text {} size {} color {}", quoted(&ctx.tr("Озвучка", "Voice")), ctx.px(18.0), quoted(&ctx.muted)));
    lines.p(4, format!("bar value Preference(\"voice volume\") xsize {}", ctx.px(360.0)));
    lines.p(3, format!("textbutton {} action Preference(\"all mute\", \"toggle\") text_size {}", quoted(&ctx.tr("Выключить всё", "Mute All")), ctx.px(20.0)));
    lines.blank();

    // Display.
    lines.p(2, "vbox:");
    lines.p(3, format!("spacing {}", ctx.px(10.0)));
    lines.p(3, format!("text {} size {} color {}", quoted(&ctx.tr("Экран", "Display")), ctx.px(26.0), quoted(&ctx.accent)));
    lines.p(3, format!("textbutton {} action Preference(\"display\", \"window\") text_size {}", quoted(&ctx.tr("В окне", "Window")), ctx.px(20.0)));
    lines.p(3, format!("textbutton {} action Preference(\"display\", \"fullscreen\") text_size {}", quoted(&ctx.tr("Полный экран", "Fullscreen")), ctx.px(20.0)));
    if raw.show_language && ctx.locales.len() > 1 {
        lines.p(3, format!("text {} size {} color {}", quoted(&ctx.tr("Язык", "Language")), ctx.px(22.0), quoted(&ctx.accent)));
        lines.p(3, "hbox:");
        lines.p(4, format!("spacing {}", ctx.px(10.0)));
        for locale in ctx.locales {
            lines.p(
                4,
                format!(
                    "textbutton {} action Language({}) text_size {}",
                    quoted(&locale.to_uppercase()),
                    quoted(locale),
                    ctx.px(20.0)
                ),
            );
        }
    }
    lines.blank();
    back_button(&mut lines, ctx, "Return()");
    lines.out
}

// ---------------------------------------------------------------------------
// CG gallery
// ---------------------------------------------------------------------------

fn render_gallery(name: &str, raw: GalleryParams, ctx: &Ctx) -> Vec<String> {
    let title = if raw.title.trim().is_empty() {
        ctx.tr("Галерея", "Gallery")
    } else {
        raw.title
    };
    let columns = raw.columns.clamp(1, 6);
    let items: Vec<ImageItem> = raw
        .items
        .into_iter()
        .filter(|item| non_empty(&item.image).is_some())
        .collect();
    let mut lines = Lines::new();

    lines.p(0, "init python:");
    lines.p(1, "studio_gallery_items = [");
    for item in &items {
        let caption = if item.title.trim().is_empty() {
            item.image.clone()
        } else {
            item.title.clone()
        };
        lines.p(2, format!("({}, {}),", quoted(&item.image), quoted(&caption)));
    }
    lines.p(1, "]");
    lines.p(1, "if persistent.studio_gallery_unlocked is None:");
    lines.p(2, "persistent.studio_gallery_unlocked = set()");
    lines.blank();
    lines.p(1, "def studio_unlock(image):");
    lines.p(2, "persistent.studio_gallery_unlocked.add(image)");
    lines.blank();

    header(&mut lines, name, &title, ctx, true);
    if items.is_empty() {
        lines.p(
            1,
            format!(
                "text {} xalign 0.5 yalign 0.5 size {} color {}",
                quoted(&ctx.tr("Добавьте картинки в настройках шаблона.", "Add images in the template settings.")),
                ctx.px(22.0),
                quoted(&ctx.muted)
            ),
        );
    } else {
        let rows = (items.len() as u32).div_ceil(columns);
        lines.p(1, format!("grid {columns} {rows}:"));
        lines.p(2, "xalign 0.5");
        lines.p(2, "ypos 0.13");
        lines.p(2, format!("spacing {}", ctx.px(14.0)));
        lines.blank();
        lines.p(2, "for _image, _title in studio_gallery_items:");
        lines.p(3, "if _image in persistent.studio_gallery_unlocked:");
        lines.p(4, "button:");
        lines.p(5, format!("action Show(\"{name}_view\", path=_image, title=_title)"));
        lines.p(5, format!("xysize ({}, {})", ctx.px(300.0), ctx.px(180.0)));
        lines.p(5, format!("add _image xysize ({}, {})", ctx.px(300.0), ctx.px(180.0)));
        lines.p(3, "else:");
        lines.p(4, "frame:");
        lines.p(5, format!("xysize ({}, {})", ctx.px(300.0), ctx.px(180.0)));
        lines.p(5, "background Solid(\"#15131f\")");
        lines.p(5, format!("text \"?\" xalign 0.5 yalign 0.5 size {} color \"#5a5a6e\"", ctx.px(44.0)));
    }
    lines.blank();
    back_button(&mut lines, ctx, "Return()");
    lines.blank();

    // Fullscreen viewer.
    lines.p(0, format!("screen {name}_view(path, title):"));
    lines.p(1, "modal True");
    lines.p(1, "zorder 200");
    lines.p(1, "add Solid(\"#000000\")");
    lines.p(1, "add path xalign 0.5 yalign 0.5");
    lines.p(
        1,
        format!(
            "text title xalign 0.5 ypos 0.02 size {} color {}",
            ctx.px(26.0),
            quoted(&ctx.text)
        ),
    );
    lines.p(
        1,
        format!(
            "textbutton {} action Hide(\"{name}_view\") xalign 0.5 ypos 0.94 text_size {}",
            quoted(&ctx.tr("Закрыть", "Close")),
            ctx.px(24.0)
        ),
    );
    lines.out
}

// ---------------------------------------------------------------------------
// Music room
// ---------------------------------------------------------------------------

fn render_music(name: &str, raw: MusicParams, ctx: &Ctx) -> Vec<String> {
    let title = if raw.title.trim().is_empty() {
        ctx.tr("Музыка", "Music")
    } else {
        raw.title
    };
    let tracks: Vec<MediaItem> = raw
        .tracks
        .into_iter()
        .filter(|track| non_empty(&track.file).is_some())
        .collect();
    let mut lines = Lines::new();
    lines.p(0, "init python:");
    lines.p(1, "studio_music_tracks = [");
    for track in &tracks {
        let caption = if track.title.trim().is_empty() {
            track.file.clone()
        } else {
            track.title.clone()
        };
        lines.p(2, format!("({}, {}),", quoted(&track.file), quoted(&caption)));
    }
    lines.p(1, "]");
    lines.blank();

    header(&mut lines, name, &title, ctx, true);
    if tracks.is_empty() {
        lines.p(
            1,
            format!(
                "text {} xalign 0.5 yalign 0.5 size {} color {}",
                quoted(&ctx.tr("Добавьте треки в настройках шаблона.", "Add tracks in the template settings.")),
                ctx.px(22.0),
                quoted(&ctx.muted)
            ),
        );
    } else {
        lines.p(1, "vbox:");
        lines.p(2, "xalign 0.5");
        lines.p(2, "ypos 0.14");
        lines.p(2, format!("spacing {}", ctx.px(8.0)));
        lines.blank();
        lines.p(2, "for _file, _title in studio_music_tracks:");
        lines.p(3, "button:");
        lines.p(4, "action Play(\"music\", _file)");
        lines.p(4, format!("xysize ({}, {})", ctx.px(560.0), ctx.px(54.0)));
        lines.p(4, "background Solid(\"#15131f\")");
        lines.p(4, "hover_background Solid(\"#221f33\")");
        lines.p(4, "text _title size ".to_string() + &ctx.px(22.0).to_string() + " xalign 0.5 yalign 0.5 color " + &quoted(&ctx.text));
    }
    lines.blank();
    lines.p(1, "hbox:");
    lines.p(2, "xalign 0.5");
    lines.p(2, "ypos 0.88");
    lines.p(2, format!("spacing {}", ctx.px(18.0)));
    lines.p(2, format!("textbutton {} action Stop(\"music\", fadeout=0.5) text_size {}", quoted(&ctx.tr("Стоп", "Stop")), ctx.px(24.0)));
    lines.p(2, format!("textbutton {} action Return() text_size {}", quoted(&ctx.tr("Назад", "Back")), ctx.px(24.0)));
    lines.out
}

// ---------------------------------------------------------------------------
// Chapter select
// ---------------------------------------------------------------------------

fn render_chapters(name: &str, raw: ChaptersParams, ctx: &Ctx) -> Vec<String> {
    let title = if raw.title.trim().is_empty() {
        ctx.tr("Главы", "Chapters")
    } else {
        raw.title
    };
    let chapters: Vec<ChapterItem> = raw
        .chapters
        .into_iter()
        .filter(|chapter| non_empty(&chapter.target).is_some())
        .collect();
    let mut lines = Lines::new();
    header(&mut lines, name, &title, ctx, true);
    if chapters.is_empty() {
        lines.p(
            1,
            format!(
                "text {} xalign 0.5 yalign 0.5 size {} color {}",
                quoted(&ctx.tr("Добавьте главы в настройках шаблона.", "Add chapters in the template settings.")),
                ctx.px(22.0),
                quoted(&ctx.muted)
            ),
        );
    } else {
        lines.p(1, "vbox:");
        lines.p(2, "xalign 0.5");
        lines.p(2, "ypos 0.16");
        lines.p(2, format!("spacing {}", ctx.px(12.0)));
        for chapter in &chapters {
            let caption = if chapter.title.trim().is_empty() {
                chapter.target.clone()
            } else {
                chapter.title.clone()
            };
            lines.p(2, format!("textbutton {} action Start({}) xysize ({}, {}) text_size {}",
                quoted(&caption),
                quoted(&chapter.target),
                ctx.px(520.0),
                ctx.px(58.0),
                ctx.px(24.0)));
        }
    }
    lines.blank();
    back_button(&mut lines, ctx, "Return()");
    lines.out
}

// ---------------------------------------------------------------------------
// Notification
// ---------------------------------------------------------------------------

fn render_notification(name: &str, raw: NotificationParams, ctx: &Ctx) -> Vec<String> {
    let seconds = if raw.seconds > 0.0 { raw.seconds } else { 3.0 };
    let (xalign, yalign) = match raw.position.as_str() {
        "top-left" => (0.02, 0.03),
        "bottom-right" => (0.98, 0.95),
        "bottom-left" => (0.02, 0.95),
        _ => (0.98, 0.03),
    };
    let mut lines = Lines::new();
    lines.p(0, format!("screen {name}(message=\"\"):"));
    lines.p(1, "zorder 200");
    lines.p(1, "frame:");
    lines.p(2, format!("xalign {xalign}"));
    lines.p(2, format!("yalign {yalign}"));
    lines.p(2, "background Solid(\"#0b0a12e6\")");
    lines.p(2, format!("padding ({}, {})", ctx.px(18.0), ctx.px(10.0)));
    lines.p(2, format!("text message size {} color {}", ctx.px(22.0), quoted(&ctx.text)));
    lines.p(1, format!("timer {seconds:.1} action Hide(\"{name}\")"));
    lines.blank();
    lines.p(0, "init python:");
    lines.p(1, format!("def studio_notify_{name}(message):"));
    lines.p(2, format!("renpy.show_screen(\"{name}\", message=message)"));
    lines.out
}

// ---------------------------------------------------------------------------
// HUD
// ---------------------------------------------------------------------------

fn render_hud(name: &str, raw: HudParams, ctx: &Ctx) -> Vec<String> {
    let title = if raw.title.trim().is_empty() {
        ctx.tr("Глава 1", "Chapter 1")
    } else {
        raw.title
    };
    let mut lines = Lines::new();
    lines.p(0, format!("screen {name}():"));
    lines.p(1, "zorder 50");
    lines.p(1, "frame:");
    lines.p(2, "xalign 0.02");
    lines.p(2, "yalign 0.02");
    lines.p(2, "background Solid(\"#0b0a12aa\")");
    lines.p(2, format!("padding ({}, {})", ctx.px(14.0), ctx.px(8.0)));
    lines.p(2, "vbox:");
    lines.p(3, "spacing 2");
    lines.p(3, format!("text {} size {} color {}", quoted(&title), ctx.px(22.0), quoted(&ctx.text)));
    if let Some(variable) = non_empty(&raw.variable) {
        let label = if raw.label.trim().is_empty() {
            variable.to_string()
        } else {
            raw.label.clone()
        };
        lines.p(3, format!("text {} size {} color {}", quoted(&format!("{label}: [{variable}]")), ctx.px(18.0), quoted(&ctx.muted)));
    }
    lines.out
}

// ---------------------------------------------------------------------------
// Confirm (overrides the built-in dialog)
// ---------------------------------------------------------------------------

fn render_confirm(raw: ConfirmParams, ctx: &Ctx) -> Vec<String> {
    let yes = if raw.yes.trim().is_empty() { ctx.tr("Да", "Yes") } else { raw.yes };
    let no = if raw.no.trim().is_empty() { ctx.tr("Нет", "No") } else { raw.no };
    let mut lines = Lines::new();
    lines.p(0, "screen confirm(message, yes_action, no_action):");
    lines.p(1, "modal True");
    lines.p(1, "zorder 200");
    lines.p(1, "add Solid(\"#000000b0\")");
    lines.p(1, "frame:");
    lines.p(2, "xalign 0.5");
    lines.p(2, "yalign 0.5");
    lines.p(2, "background Solid(\"#12101e\")");
    lines.p(2, format!("padding ({}, {})", ctx.px(36.0), ctx.px(28.0)));
    lines.p(2, "vbox:");
    lines.p(3, format!("spacing {}", ctx.px(22.0)));
    lines.p(3, format!("text message xalign 0.5 size {} color {}", ctx.px(26.0), quoted(&ctx.text)));
    lines.p(3, "hbox:");
    lines.p(4, "xalign 0.5");
    lines.p(4, format!("spacing {}", ctx.px(18.0)));
    lines.p(4, format!("textbutton {} action yes_action text_size {}", quoted(&yes), ctx.px(24.0)));
    lines.p(4, format!("textbutton {} action no_action text_size {}", quoted(&no), ctx.px(24.0)));
    lines.p(1, "key \"game_menu\" action no_action");
    lines.out
}

// ---------------------------------------------------------------------------
// Stats, journal, inventory, phone
// ---------------------------------------------------------------------------

fn render_stats(name: &str, raw: StatsParams, ctx: &Ctx) -> Vec<String> {
    let title = if raw.title.trim().is_empty() {
        ctx.tr("Статистика", "Stats").to_string()
    } else {
        raw.title
    };
    let stats: Vec<StatItem> = raw
        .stats
        .into_iter()
        .filter(|stat| non_empty(&stat.variable).is_some())
        .collect();
    let mut lines = Lines::new();
    header(&mut lines, name, &title, ctx, true);
    if stats.is_empty() {
        lines.p(
            1,
            format!(
                "text {} xalign 0.5 yalign 0.5 size {} color {}",
                quoted(&ctx.tr("Добавьте показатели в настройках шаблона.", "Add stats in the template settings.")),
                ctx.px(22.0),
                quoted(&ctx.muted)
            ),
        );
    } else {
        lines.p(1, "vbox:");
        lines.p(2, "xalign 0.5");
        lines.p(2, "ypos 0.18");
        lines.p(2, format!("spacing {}", ctx.px(12.0)));
        for stat in &stats {
            let label = if stat.label.trim().is_empty() {
                stat.variable.clone()
            } else {
                stat.label.clone()
            };
            lines.p(
                2,
                format!(
                    "text {} size {} color {}",
                    quoted(&format!("{label}: [{}]", stat.variable)),
                    ctx.px(24.0),
                    quoted(&ctx.text)
                ),
            );
        }
    }
    lines.blank();
    back_button(&mut lines, ctx, "Return()");
    lines.out
}

fn render_journal(name: &str, raw: JournalParams, ctx: &Ctx) -> Vec<String> {
    let title = if raw.title.trim().is_empty() {
        ctx.tr("Дневник", "Journal").to_string()
    } else {
        raw.title
    };
    let mut lines = Lines::new();
    header(&mut lines, name, &title, ctx, true);
    lines.p(1, "viewport:");
    lines.p(2, "xalign 0.5");
    lines.p(2, "ypos 0.13");
    lines.p(2, format!("xsize {}", ctx.px(900.0)));
    lines.p(2, format!("ysize {}", ctx.px(560.0)));
    lines.p(2, "mousewheel True");
    lines.p(2, "vbox:");
    lines.p(3, format!("spacing {}", ctx.px(14.0)));
    if raw.entries.is_empty() {
        lines.p(
            3,
            format!(
                "text {} size {} color {}",
                quoted(&ctx.tr("Пока пусто.", "Nothing here yet.")),
                ctx.px(22.0),
                quoted(&ctx.muted)
            ),
        );
    } else {
        for entry in &raw.entries {
            lines.p(3, "frame:");
            lines.p(4, "background Solid(\"#15131f\")");
            lines.p(4, format!("padding ({}, {})", ctx.px(18.0), ctx.px(12.0)));
            lines.p(4, "vbox:");
            lines.p(5, "spacing 6");
            if !entry.title.trim().is_empty() {
                lines.p(5, format!("text {} size {} color {}", quoted(&entry.title), ctx.px(24.0), quoted(&ctx.accent)));
            }
            lines.p(5, format!("text {} size {} color {}", quoted(&entry.text), ctx.px(20.0), quoted(&ctx.text)));
        }
    }
    lines.blank();
    back_button(&mut lines, ctx, "Return()");
    lines.out
}

fn render_inventory(name: &str, raw: InventoryParams, ctx: &Ctx) -> Vec<String> {
    let title = if raw.title.trim().is_empty() {
        ctx.tr("Инвентарь", "Inventory").to_string()
    } else {
        raw.title
    };
    let columns = raw.columns.clamp(1, 8);
    let items: Vec<ImageItem> = raw
        .items
        .into_iter()
        .filter(|item| non_empty(&item.image).is_some())
        .collect();
    let mut lines = Lines::new();
    lines.p(0, "init python:");
    lines.p(1, "studio_inventory_items = [");
    for item in &items {
        let caption = if item.title.trim().is_empty() {
            item.image.clone()
        } else {
            item.title.clone()
        };
        lines.p(2, format!("({}, {}),", quoted(&item.image), quoted(&caption)));
    }
    lines.p(1, "]");
    lines.blank();
    header(&mut lines, name, &title, ctx, true);
    if items.is_empty() {
        lines.p(
            1,
            format!(
                "text {} xalign 0.5 yalign 0.5 size {} color {}",
                quoted(&ctx.tr("Добавьте предметы в настройках шаблона.", "Add items in the template settings.")),
                ctx.px(22.0),
                quoted(&ctx.muted)
            ),
        );
    } else {
        let rows = (items.len() as u32).div_ceil(columns);
        lines.p(1, format!("grid {columns} {rows}:"));
        lines.p(2, "xalign 0.5");
        lines.p(2, "ypos 0.14");
        lines.p(2, format!("spacing {}", ctx.px(14.0)));
        lines.p(2, "for _image, _title in studio_inventory_items:");
        lines.p(3, "frame:");
        lines.p(4, format!("xysize ({}, {})", ctx.px(150.0), ctx.px(170.0)));
        lines.p(4, "background Solid(\"#15131f\")");
        lines.p(4, "vbox:");
        lines.p(5, "spacing 8");
        lines.p(5, format!("add _image xysize ({}, {}) xalign 0.5", ctx.px(104.0), ctx.px(104.0)));
        lines.p(5, format!("text _title xalign 0.5 size {} color {}", ctx.px(18.0), quoted(&ctx.text)));
    }
    lines.blank();
    back_button(&mut lines, ctx, "Return()");
    lines.out
}

fn render_phone(name: &str, raw: PhoneParams, ctx: &Ctx) -> Vec<String> {
    let title = if raw.title.trim().is_empty() {
        ctx.tr("Сообщения", "Messages").to_string()
    } else {
        raw.title
    };
    let mut lines = Lines::new();
    header(&mut lines, name, &title, ctx, true);
    lines.p(1, "viewport:");
    lines.p(2, "xalign 0.5");
    lines.p(2, "ypos 0.13");
    lines.p(2, format!("xsize {}", ctx.px(760.0)));
    lines.p(2, format!("ysize {}", ctx.px(560.0)));
    lines.p(2, "mousewheel True");
    lines.p(2, "vbox:");
    lines.p(3, format!("spacing {}", ctx.px(10.0)));
    if raw.messages.is_empty() {
        lines.p(
            3,
            format!(
                "text {} size {} color {}",
                quoted(&ctx.tr("Пока пусто.", "Nothing here yet.")),
                ctx.px(22.0),
                quoted(&ctx.muted)
            ),
        );
    } else {
        for message in &raw.messages {
            let mine = message.who.eq_ignore_ascii_case("me") || message.who.eq_ignore_ascii_case("я");
            lines.p(3, "frame:");
            lines.p(4, format!("xalign {}", if mine { 1.0 } else { 0.0 }));
            lines.p(4, "xmaximum " .to_string() + &ctx.px(520.0).to_string());
            lines.p(4, "background Solid(\"#1d2438\")");
            lines.p(4, format!("padding ({}, {})", ctx.px(16.0), ctx.px(10.0)));
            lines.p(4, "vbox:");
            lines.p(5, "spacing 4");
            if !message.who.trim().is_empty() && !mine {
                lines.p(5, format!("text {} size {} color {}", quoted(&message.who), ctx.px(16.0), quoted(&ctx.accent)));
            }
            lines.p(5, format!("text {} size {} color {}", quoted(&message.text), ctx.px(20.0), quoted(&ctx.text)));
        }
    }
    lines.blank();
    back_button(&mut lines, ctx, "Return()");
    lines.out
}
