//! Plugin catalog: scans `plugins/` next to the app, inspects `.rpy`, folders and
//! zip packages, and copies a plugin's game files into the project.

use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PluginFile {
    pub source: String,
    pub target: String,
    pub size: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub source: String,
    pub size: u64,
    pub files: Vec<PluginFile>,
    pub snippet: Option<String>,
    pub docs: Option<String>,
    pub enabled: bool,
}

struct CatalogEntry {
    matcher: &'static str,
    name: &'static str,
    description: &'static str,
    snippet: Option<&'static str>,
}

const CATALOG: &[CatalogEntry] = &[
    CatalogEntry { matcher: "strat", name: "STRAT — переход-тряска", description: "Случайная тряска при переходе, как телепорт. Параметры: strat(t, x, y).", snippet: Some("with strat(0.6, 12, 8)") },
    CatalogEntry { matcher: "flip3d", name: "FLIP3D — 3D-переходы", description: "Четыре перехода-вращения куба, можно подключить звук поворотов.", snippet: Some("with flip3d") },
    CatalogEntry { matcher: "bub", name: "Bub — баблы у спрайтов", description: "Реплики в пузырях, привязанных к спрайту. Настраиваются цвет, отступы и фон.", snippet: Some("$ big = Bub(_(\"Здоровяк\"))\nshow big ok\nbig \"Уясе, какой здоровенный!\"") },
    CatalogEntry { matcher: "beep", name: "BEEP — псевдо-озвучка", description: "Писк/бормотание при выводе текста для всех персонажей, кроме рассказчика.", snippet: None },
    CatalogEntry { matcher: "auto_bubbles", name: "Bub — баблы (пример)", description: "Демо-проект баблов у спрайтов (bub.rpy + примеры).", snippet: Some("$ big = Bub(_(\"Здоровяк\"))\nshow big ok\nbig \"Уясе, какой здоровенный!\"") },
    CatalogEntry { matcher: "bw_shader", name: "BW Shader — ч/б манга", description: "Чёрно-белый шейдер с дизерингом для спрайта или слоя master.", snippet: None },
    CatalogEntry { matcher: "flashlight", name: "FLASHLIGHT — фонарик", description: "Шейдер фонарика с засветкой; дополнительных картинок не нужно.", snippet: None },
    CatalogEntry { matcher: "myframe", name: "MYFRAME + PIXELIZE", description: "Расширение Frame с обводкой и пиксельный режим.", snippet: None },
    CatalogEntry { matcher: "pixelize", name: "PIXELIZE — пиксельный режим", description: "Пикселизация изображения и текста.", snippet: None },
    CatalogEntry { matcher: "autoparallax", name: "AUTOPARALLAX — параллакс", description: "Параллакс в любой новелле: фоны дальше, спрайты ближе. Без кода.", snippet: None },
    CatalogEntry { matcher: "panorama", name: "Panorama — панорама за курсором", description: "Широкий фон двигается вслед за курсором.", snippet: None },
    CatalogEntry { matcher: "steps", name: "Steps — шаги и движущийся фон", description: "Спрайты шагают по экрану, многослойный фон движется мимо.", snippet: None },
    CatalogEntry { matcher: "stereo", name: "Stereo — звук за спрайтами", description: "Баланс каналов следует за положением спрайтов (работает с 7DOTS).", snippet: None },
    CatalogEntry { matcher: "oldtv", name: "OldTV — старый телевизор", description: "Режим старого телевизора для флэшбэков, много настроек.", snippet: None },
    CatalogEntry { matcher: "shaders2", name: "Simple Shaders 2 — космос", description: "Шейдеры полёта среди звёзд, галактики и волн.", snippet: None },
    CatalogEntry { matcher: "texture_pixel", name: "Texture Pixel", description: "Замена пикселей картинками или символами, регулировка размера и яркости.", snippet: None },
    CatalogEntry { matcher: "simple_inventory", name: "Simple Inventory", description: "Простейший инвентарь: сбор предметов во время чтения новеллы.", snippet: None },
    CatalogEntry { matcher: "quiz_timer", name: "Quiz Timer — викторина с таймером", description: "Викторина с таймером, запускается одной командой, стили настраиваются.", snippet: None },
    CatalogEntry { matcher: "quiz", name: "Quiz — викторина со свайпами", description: "Мини-викторина ДА/НЕТ кнопками или свайпами.", snippet: None },
    CatalogEntry { matcher: "tasks_tabs", name: "Tasks Tabs — список задач", description: "Улучшенный список задач с вкладками и иконками.", snippet: None },
    CatalogEntry { matcher: "tasks", name: "Tasks — список задач", description: "Задачи по группам, иконки, подсказки; работает с 7DOTS.", snippet: None },
    CatalogEntry { matcher: "info", name: "Info — статы и меню", description: "Окно со статами и меню выборов, вызываемое из экранов.", snippet: None },
    CatalogEntry { matcher: "cho", name: "Cho — кастомное меню", description: "Меню с положением, трансформами и неактивными кнопками.", snippet: None },
    CatalogEntry { matcher: "myinput", name: "MyInput — поле ввода", description: "Кастомное поле ввода myinput()/MyInput(), даже из экранов.", snippet: Some("$ myinput()") },
    CatalogEntry { matcher: "inputs", name: "Inputs — несколько полей", description: "Несколько полей ввода на одном экране со стилями.", snippet: None },
    CatalogEntry { matcher: "custom_textbox", name: "Custom Textbox", description: "Кастомные текстбоксы с предустановленными положениями.", snippet: None },
    CatalogEntry { matcher: "textaround", name: "TextAround — круговое меню", description: "Кнопки выборов по кругу, вращение, тряска, размытие.", snippet: None },
    CatalogEntry { matcher: "emochoice", name: "EmoChoice — эмоциональные выборы", description: "Варианты ответа с эмоциональной окраской и реакцией собеседника.", snippet: None },
    CatalogEntry { matcher: "rsp", name: "Rock-Scissors-Paper", description: "Щит-меч-скролл с настраиваемой логикой и анимацией.", snippet: None },
    CatalogEntry { matcher: "duckhunt", name: "Duck Hunt — тир", description: "Мини-игра «охота на уток» с помидорами; попадание по прозрачности пикселя.", snippet: None },
    CatalogEntry { matcher: "resolutions", name: "Resolutions — смена разрешения", description: "Смена разрешения из quick_menu или списка.", snippet: None },
    CatalogEntry { matcher: "show_new_track", name: "Show New Track", description: "Виджет с названием трека при каждом запуске музыки.", snippet: None },
    CatalogEntry { matcher: "sms", name: "SMS — чаты", description: "Обычные и групповые SMS-чаты, счётчики, эмодзи, картинки, скины.", snippet: None },
    CatalogEntry { matcher: "yet_another_phone", name: "Yet Another Phone", description: "Телефон-мессенджер с чатами и уведомлениями.", snippet: None },
    CatalogEntry { matcher: "phone_messenger", name: "Phone Messenger", description: "Полноэкранный мессенджер для игры.", snippet: None },
    CatalogEntry { matcher: "phone_number_input", name: "Phone Number Input", description: "Набор номера и имитация вызова; работает с 7DOTS.", snippet: None },
    CatalogEntry { matcher: "sideimage", name: "SIW — окно SideImage", description: "Окно бокового изображения с удобным вырезанием и увеличением.", snippet: None },
    CatalogEntry { matcher: "ava", name: "Face Editor — редактор головы", description: "Фоторобот-редактор персонажа для SideImage.", snippet: None },
    CatalogEntry { matcher: "particle", name: "Immersive Particle VFX", description: "Система частиц для Ren'Py: искры, снег, дым и другие эффекты.", snippet: None },
    CatalogEntry { matcher: "kinetic", name: "Kinetic Text Tags", description: "Анимированные текстовые теги: текст «оживает» прямо в диалоге.", snippet: None },
    CatalogEntry { matcher: "highlight", name: "Auto Highlight", description: "Автоматическая подсветка выделенного текста/элементов интерфейса.", snippet: None },
    CatalogEntry { matcher: "wave", name: "Wave Rendering", description: "Волновой рендеринг изображений и текста.", snippet: None },
    CatalogEntry { matcher: "chess", name: "Ren'Py Chess", description: "Шахматы на Ren'Py: доска, фигуры, ходы.", snippet: None },
    CatalogEntry { matcher: "dse", name: "DSE — Dating Sim Engine", description: "Движок симулятора свиданий: статы, календарь, события.", snippet: None },
    CatalogEntry { matcher: "syrup", name: "Syrup Lite RPG", description: "Лёгкий RPG-фреймворк: бои, статы, инвентарь.", snippet: None },
    CatalogEntry { matcher: "sheepstorm", name: "Sheepstorm Battle Framework", description: "Пошаговый боевой фреймворк с юнитами и навыками.", snippet: None },
    CatalogEntry { matcher: "7dots", name: "7DOTS", description: "Набор утилит, на котором работают многие плагины (SMS, tasks, stereo и др.).", snippet: None },
    CatalogEntry { matcher: "transitions", name: "Transitions", description: "Сборник дополнительных переходов для `with`.", snippet: None },
    CatalogEntry { matcher: "waste", name: "Waste Sorting", description: "Мини-игра сортировки мусора с баками и картинками.", snippet: None },
    CatalogEntry { matcher: "points", name: "Points & Events", description: "Игровая валюта, шкалы отношений и менеджер событий.", snippet: None },
    CatalogEntry { matcher: "face_editor", name: "Face Editor", description: "Редактор «говорящей головы» протагониста.", snippet: None },
];

fn normalize_id(raw: &str) -> String {
    let mut value = raw.to_lowercase();
    for prefix in ["test_", "renpy-", "renpy_"] {
        value = value.trim_start_matches(prefix).to_string();
    }
    for suffix in ["-pc", "_pc", "-master", "_master"] {
        value = value.trim_end_matches(suffix).to_string();
    }
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn catalog_for(id: &str, file_name: &str) -> Option<&'static CatalogEntry> {
    let haystack = format!("{id} {file_name}").to_lowercase();
    CATALOG
        .iter()
        .filter(|entry| haystack.contains(entry.matcher))
        .max_by_key(|entry| entry.matcher.len())
}

/// Ren'Py core files of a demo game wrapped around the plugin. They must never
/// replace the host project's screens, options or story entry point.
const SCAFFOLD_FILES: [&str; 6] = [
    "script.rpy",
    "options.rpy",
    "gui.rpy",
    "screens.rpy",
    "common.rpy",
    "splashscreen.rpy",
];

fn is_scaffold(name: &str) -> bool {
    SCAFFOLD_FILES
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn clean_target(path: &str, plugin_id: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_start_matches("./");
    let trimmed = trimmed.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    if lower.contains("/cache/")
        || lower.starts_with("cache/")
        || lower.contains("/saves/")
        || lower.starts_with("saves/")
        || lower.ends_with(".rpyc")
        || lower.ends_with(".pyc")
        || lower.contains(".git/")
        || lower.ends_with(".exe")
    {
        return None;
    }

    // Work on path components instead of byte offsets: case-independent lookups
    // must not slice a string built from a different (lowercased) copy.
    let components: Vec<&str> = trimmed.split('/').filter(|part| !part.is_empty()).collect();
    if components.is_empty() || components.contains(&"..") {
        return None;
    }
    // Demo translations and core screens are skipped even outside `game/`,
    // where they used to slip through as loose scripts.
    if components.iter().any(|part| part.eq_ignore_ascii_case("tl")) {
        return None;
    }
    let name = *components.last()?;
    if is_scaffold(name) {
        return None;
    }

    let relative = if let Some(position) = components
        .iter()
        .position(|part| part.eq_ignore_ascii_case("game"))
    {
        components[position + 1..].join("/")
    } else if components
        .first()
        .map(|part| {
            matches!(
                part.to_ascii_lowercase().as_str(),
                "images" | "audio" | "fonts" | "gui" | "video"
            )
        })
        .unwrap_or(false)
    {
        components.join("/")
    } else if lower.ends_with(".rpy") {
        format!("plugins/{plugin_id}/{name}")
    } else {
        return None;
    };

    if relative.is_empty()
        || relative.split('/').any(|part| part == "..")
        || relative.starts_with('/')
    {
        return None;
    }
    Some(relative)
}

fn first_comments(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines().take(40) {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let comment = trimmed.trim_start_matches('#').trim();
            if !comment.is_empty() && !comment.starts_with('=') {
                lines.push(comment.to_string());
            }
        } else if !trimmed.is_empty() {
            break;
        }
        if lines.len() >= 3 {
            break;
        }
    }
    lines.join(" ")
}

fn read_zip_text(archive: &mut zip::ZipArchive<fs::File>, name: &str) -> Option<String> {
    let mut file = archive.by_name(name).ok()?;
    let mut text = String::new();
    file.read_to_string(&mut text).ok()?;
    Some(text)
}

fn zip_plugin(id: &str, path: &Path, meta: &fs::Metadata) -> Option<PluginInfo> {
    let file = fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut files = Vec::new();
    let mut description = String::new();
    let mut snippet = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).ok()?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let lower = name.to_lowercase();
        if description.is_empty()
            && (lower.ends_with("readme.txt") || lower.ends_with("readme.md") || lower.ends_with("readme.rpy"))
        {
            drop(entry);
            if let Some(text) = read_zip_text(&mut archive, &name) {
                description = first_comments(&text);
            }
            continue;
        }
        if let Some(target) = clean_target(&name, id) {
            files.push(PluginFile {
                source: name,
                target,
                size: entry.size(),
            });
        }
    }
    let catalog = catalog_for(id, path.file_name()?.to_string_lossy().as_ref());
    if let Some(entry) = catalog {
        if description.is_empty() {
            description = entry.description.to_string();
        }
        snippet = entry.snippet.map(|value| value.to_string());
    }
    let size = files.iter().map(|file| file.size).sum::<u64>();
    Some(PluginInfo {
        id: id.to_string(),
        name: catalog
            .map(|entry| entry.name.to_string())
            .unwrap_or_else(|| id.replace('_', " ")),
        description,
        kind: "zip".into(),
        source: path.display().to_string(),
        size: if size == 0 { meta.len() } else { size },
        files,
        snippet,
        docs: None,
        enabled: false,
    })
}

fn folder_plugin(id: &str, path: &Path) -> Option<PluginInfo> {
    let mut files = Vec::new();
    let mut description = String::new();
    for entry in walkdir::WalkDir::new(path).max_depth(6).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(path)
            .ok()?
            .to_string_lossy()
            .replace('\\', "/");
        let lower = relative.to_lowercase();
        if description.is_empty() && (lower.ends_with("readme.txt") || lower.ends_with("readme.md")) {
            if let Ok(text) = fs::read_to_string(entry.path()) {
                description = first_comments(&text);
            }
        }
        if lower.ends_with(".rpy") && description.is_empty() {
            if let Ok(text) = fs::read_to_string(entry.path()) {
                description = first_comments(&text);
            }
        }
        if let Some(target) = clean_target(&relative, id) {
            files.push(PluginFile {
                source: entry.path().display().to_string(),
                target,
                size: entry.metadata().map(|m| m.len()).unwrap_or(0),
            });
        }
    }
    let catalog = catalog_for(id, path.file_name()?.to_string_lossy().as_ref());
    if let Some(entry) = catalog {
        if description.is_empty() {
            description = entry.description.to_string();
        }
    }
    let size = files.iter().map(|file| file.size).sum();
    Some(PluginInfo {
        id: id.to_string(),
        name: catalog
            .map(|entry| entry.name.to_string())
            .unwrap_or_else(|| id.replace('_', " ")),
        description,
        kind: "folder".into(),
        source: path.display().to_string(),
        size,
        files,
        snippet: catalog.and_then(|entry| entry.snippet.map(|value| value.to_string())),
        docs: None,
        enabled: false,
    })
}

fn file_plugin(id: &str, path: &Path, meta: &fs::Metadata) -> Option<PluginInfo> {
    let text = fs::read_to_string(path).unwrap_or_default();
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let catalog = catalog_for(id, &file_name);
    let description = if text.is_empty() {
        catalog.map(|entry| entry.description.to_string()).unwrap_or_default()
    } else {
        first_comments(&text)
    };
    let target = format!("plugins/{id}/{file_name}");
    Some(PluginInfo {
        id: id.to_string(),
        name: catalog
            .map(|entry| entry.name.to_string())
            .unwrap_or_else(|| id.replace('_', " ")),
        description,
        kind: "file".into(),
        source: path.display().to_string(),
        size: meta.len(),
        files: vec![PluginFile {
            source: path.display().to_string(),
            target,
            size: meta.len(),
        }],
        snippet: catalog.and_then(|entry| entry.snippet.map(|value| value.to_string())),
        docs: None,
        enabled: false,
    })
}

pub fn scan(root: &Path) -> Vec<PluginInfo> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
            continue;
        };
        let id = normalize_id(&stem);
        if id.is_empty() {
            continue;
        }
        let lower = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let plugin = if meta.is_dir() {
            if lower == "node_modules" || lower.starts_with('.') {
                continue;
            }
            folder_plugin(&id, &path)
        } else if lower.ends_with(".zip") {
            zip_plugin(&id, &path, &meta)
        } else if lower.ends_with(".rpy") || lower.ends_with(".rpy.txt") {
            file_plugin(&id, &path, &meta)
        } else {
            None
        };
        if let Some(plugin) = plugin {
            if !plugin.files.is_empty() {
                out.push(plugin);
            }
        }
    }
    // The same plugin often arrives as a loose .rpy and as a demo zip.
    let mut by_id: std::collections::HashMap<String, PluginInfo> = std::collections::HashMap::new();
    for plugin in out {
        match by_id.get(&plugin.id) {
            Some(existing) if existing.files.len() >= plugin.files.len() => {}
            _ => {
                by_id.insert(plugin.id.clone(), plugin);
            }
        }
    }
    let mut out: Vec<PluginInfo> = by_id.into_values().collect();
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

/// Copies a plugin's files into the project and returns the relative paths.
pub fn install(root: &Path, plugin: &PluginInfo) -> Result<Vec<String>, String> {
    let mut copied = Vec::new();
    match plugin.kind.as_str() {
        "zip" => {
            let file = fs::File::open(&plugin.source).map_err(|e| e.to_string())?;
            let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            for plan in &plugin.files {
                let mut entry = archive
                    .by_name(&plan.source)
                    .map_err(|e| format!("{}: {e}", plan.source))?;
                let target = safe_join(root, &plan.target)?;
                if target.exists() {
                    // Never overwrite what the author already has: plugin zips
                    // ship a demo's gui art and core screens as well as the
                    // plugin itself.
                    continue;
                }
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut out = fs::File::create(&target).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                copied.push(format!("game/{}", plan.target));
            }
        }
        "folder" => {
            for plan in &plugin.files {
                let source = PathBuf::from(&plan.source);
                let target = safe_join(root, &plan.target)?;
                if target.exists() {
                    continue;
                }
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                fs::copy(&source, &target).map_err(|e| {
                    format!("{} → {}: {e}", source.display(), target.display())
                })?;
                copied.push(format!("game/{}", plan.target));
            }
        }
        _ => {
            let plan = plugin.files.first().ok_or("У плагина нет файлов")?;
            let target = safe_join(root, &plan.target)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::copy(&plan.source, &target).map_err(|e| e.to_string())?;
            copied.push(format!("game/{}", plan.target));
        }
    }
    Ok(copied)
}

pub fn uninstall(root: &Path, files: &[String]) -> Vec<String> {
    let mut removed = Vec::new();
    for relative in files {
        let trimmed = relative.trim_start_matches("game/");
        let Ok(path) = safe_join(root, trimmed) else { continue };
        if path.is_file() && fs::remove_file(&path).is_ok() {
            removed.push(relative.clone());
            // Ren'Py keeps loading orphaned .rpyc files, so a deleted plugin
            // script must take its compiled twin with it.
            if path.extension().and_then(|ext| ext.to_str()) == Some("rpy") {
                let compiled = path.with_extension("rpyc");
                if compiled.is_file() {
                    let _ = fs::remove_file(&compiled);
                }
            }
        }
    }
    removed
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut path = root.join("game");
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => path.push(part),
            _ => return Err("Недопустимый путь плагина".into()),
        }
    }
    Ok(path)
}

/// Where plugins live: `plugins/` next to the executable or in the workspace.
pub fn plugins_root() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().skip(1).take(6) {
            let candidate = ancestor.join("plugins");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("plugins");
        if candidate.is_dir() {
            return candidate;
        }
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RenPyStudio")
        .join("plugins")
}

/// Copies a README-like file next to the plugin so the UI can show instructions.
pub fn plugin_docs(plugin: &PluginInfo) -> Option<String> {
    match plugin.kind.as_str() {
        "folder" => {
            let dir = Path::new(&plugin.source);
            for name in ["README.txt", "README.md", "readme.txt", "readme.md"] {
                let path = dir.join(name);
                if let Ok(text) = fs::read_to_string(&path) {
                    return Some(text);
                }
            }
            None
        }
        "zip" => {
            let file = fs::File::open(&plugin.source).ok()?;
            let mut archive = zip::ZipArchive::new(file).ok()?;
            for name in ["README.txt", "README.md", "readme.txt", "readme.md"] {
                for index in 0..archive.len() {
                    let entry = archive.by_index(index).ok()?;
                    if entry.name().to_lowercase().ends_with(name) {
                        let entry_name = entry.name().to_string();
                        drop(entry);
                        return read_zip_text(&mut archive, &entry_name);
                    }
                }
            }
            None
        }
        _ => fs::read_to_string(&plugin.source).ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// Self-cleaning scratch directory: a failing test must not leave temp files.
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> TempDir {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "renpy-studio-plugins-{tag}-{}-{seq}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TempDir { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    /// Builds a plugin archive in the scratch dir, entry order preserved.
    fn build_zip(path: &Path, entries: &[(&str, &str)]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut writer = ZipWriter::new(fs::File::create(path).unwrap());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, body) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(body.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn clean_target_strips_game_prefix() {
        assert_eq!(
            clean_target("game/images/bg/x.png", "demo").as_deref(),
            Some("images/bg/x.png")
        );
        // A demo archive wraps the plugin into its own game/ folder.
        assert_eq!(
            clean_target("Demo/game/quiz.rpy", "demo").as_deref(),
            Some("quiz.rpy")
        );
        assert_eq!(
            clean_target("./game/quiz.rpy", "demo").as_deref(),
            Some("quiz.rpy")
        );
        // Windows separators are normalised before matching.
        assert_eq!(
            clean_target("game\\gui\\main_menu.png", "demo").as_deref(),
            Some("gui/main_menu.png")
        );
        assert_eq!(
            clean_target("audio/theme.ogg", "demo").as_deref(),
            Some("audio/theme.ogg")
        );
    }

    #[test]
    fn clean_target_puts_loose_scripts_under_plugins_id() {
        assert_eq!(
            clean_target("thing.rpy", "my_plug").as_deref(),
            Some("plugins/my_plug/thing.rpy")
        );
        assert_eq!(clean_target("readme.md", "my_plug"), None);
    }

    #[test]
    fn clean_target_drops_demo_extras_outside_game_dir() {
        // A bare `tl/` tree and the demo's core screens used to slip through as
        // loose scripts under plugins/<id>/ and could break the host project.
        assert_eq!(clean_target("tl/ru/x.rpy", "my_plug"), None);
        assert_eq!(clean_target("sub/tl/x.rpy", "my_plug"), None);
        assert_eq!(clean_target("script.rpy", "my_plug"), None);
        assert_eq!(clean_target("Screens.rpy", "my_plug"), None);
    }

    #[test]
    fn clean_target_rejects_escapes_and_binaries() {
        for bad in [
            "",
            "../evil",
            "/etc/passwd",
            "game/../evil.rpy",
            "game/plugins/x.rpyc",
            "game/plugins/x.pyc",
            "game/cache/thumb.png",
            "game/Saves/slot1.save",
            "game/.git/config",
            "bin/tool.exe",
            "C:/Program Files/evil.exe",
        ] {
            assert_eq!(clean_target(bad, "demo"), None, "expected {bad:?} to be dropped");
        }
        // A traversal is rejected outright: nothing outside game/ is ever written.
        assert_eq!(clean_target("../evil.rpy", "demo"), None);
    }

    #[test]
    fn clean_target_drops_demo_translations_and_core_screens() {
        for bad in [
            "game/tl/ru/x.rpy",
            "Demo/game/tl/english/x.rpy",
            "game/script.rpy",
            "game/screens.rpy",
            "game/options.rpy",
            "game/gui.rpy",
            "game/common.rpy",
        ] {
            assert_eq!(clean_target(bad, "demo"), None, "expected {bad:?} to be dropped");
        }
    }

    #[test]
    fn normalize_id_builds_stable_ids() {
        assert_eq!(normalize_id("Test_My-Plug-in_pc"), "my_plug_in");
        assert_eq!(normalize_id("RenPy-Flip3D-master"), "flip3d");
        assert_eq!(normalize_id("renpy_quiz"), "quiz");
        assert_eq!(normalize_id("  "), "");
    }

    #[test]
    fn scan_reports_rpy_file_and_zip_package() {
        let temp = TempDir::new("scan");
        let catalog = temp.path().join("plugins");
        write_file(
            &catalog.join("alpha.rpy"),
            "# Alpha plugin\n# adds a flavour screen\n\ninit python:\n    pass\n",
        );
        build_zip(&catalog.join("beta_pack.zip"), &[("game/beta.rpy", "# Beta\n")]);
        write_file(&catalog.join("notes.txt"), "not a plugin");

        let found = scan(&catalog);
        assert_eq!(found.len(), 2, "only .rpy and .zip entries become plugins");
        let alpha = found.iter().find(|p| p.id == "alpha").expect("alpha");
        assert_eq!(alpha.kind, "file");
        assert_eq!(alpha.description, "Alpha plugin adds a flavour screen");
        assert_eq!(alpha.files.len(), 1);
        assert_eq!(alpha.files[0].target, "plugins/alpha/alpha.rpy");
        let beta = found.iter().find(|p| p.id == "beta_pack").expect("beta_pack");
        assert_eq!(beta.kind, "zip");
        assert_eq!(beta.files.len(), 1);
        assert_eq!(beta.files[0].target, "beta.rpy");
    }

    #[test]
    fn install_zip_copies_files_and_keeps_existing_project_assets() {
        let temp = TempDir::new("install-zip");
        let project = temp.path().join("project");
        let catalog = temp.path().join("plugins");
        // The author already owns this image; the demo zip ships one as well.
        write_file(&project.join("game/gui/button/x.png"), "PROJECT-OWN");
        build_zip(
            &catalog.join("myplug.zip"),
            &[
                ("Demo/game/gui/button/x.png", "PLUGIN-ART"),
                ("Demo/game/plugin.rpy", "init python:\n    pass\n"),
            ],
        );

        let found = scan(&catalog);
        assert_eq!(found.len(), 1);
        let plugin = &found[0];
        assert_eq!(plugin.id, "myplug");
        assert_eq!(plugin.kind, "zip");

        let copied = install(&project, plugin).unwrap();
        assert_eq!(copied, vec!["game/plugin.rpy".to_string()]);
        assert_eq!(
            fs::read_to_string(project.join("game/plugin.rpy")).unwrap(),
            "init python:\n    pass\n"
        );
        assert_eq!(
            fs::read_to_string(project.join("game/gui/button/x.png")).unwrap(),
            "PROJECT-OWN",
            "install must never overwrite an existing project file"
        );
    }

    #[test]
    fn install_folder_copies_only_missing_files() {
        let temp = TempDir::new("install-folder");
        let project = temp.path().join("project");
        let catalog = temp.path().join("plugins");
        write_file(&catalog.join("demo/game/plugin.rpy"), "# script\n");
        write_file(&catalog.join("demo/game/gui/theme.png"), "PLUGIN-ART");
        write_file(&project.join("game/gui/theme.png"), "PROJECT-OWN");

        let found = scan(&catalog);
        assert_eq!(found.len(), 1);
        let plugin = &found[0];
        assert_eq!(plugin.kind, "folder");

        let copied = install(&project, plugin).unwrap();
        assert_eq!(copied, vec!["game/plugin.rpy".to_string()]);
        assert_eq!(
            fs::read_to_string(project.join("game/gui/theme.png")).unwrap(),
            "PROJECT-OWN"
        );
    }

    #[test]
    fn uninstall_removes_installed_scripts_and_their_compiled_pairs() {
        let temp = TempDir::new("uninstall");
        let project = temp.path().join("project");
        write_file(&project.join("game/plugins/demo/demo.rpy"), "init python:\n    pass\n");
        write_file(&project.join("game/plugins/demo/demo.rpyc"), "compiled");
        write_file(&project.join("game/plugins/demo/art.png"), "art");
        write_file(&project.join("game/plugins/demo/keep.rpy"), "keep");
        write_file(&project.join("game/plugins/demo/keep.rpyc"), "keep-compiled");
        write_file(&project.join("escape.txt"), "outside");

        let removed = uninstall(
            &project,
            &[
                "game/plugins/demo/demo.rpy".to_string(),
                "game/plugins/demo/art.png".to_string(),
                "game/plugins/demo/missing.rpy".to_string(),
                "game/../escape.txt".to_string(),
            ],
        );

        assert_eq!(
            removed,
            vec![
                "game/plugins/demo/demo.rpy".to_string(),
                "game/plugins/demo/art.png".to_string(),
            ]
        );
        assert!(!project.join("game/plugins/demo/demo.rpy").exists());
        // Ren'Py would keep loading the orphaned compiled script.
        assert!(!project.join("game/plugins/demo/demo.rpyc").exists());
        // Nothing that was not installed may disappear.
        assert!(project.join("game/plugins/demo/keep.rpy").is_file());
        assert!(project.join("game/plugins/demo/keep.rpyc").is_file());
        assert!(project.join("escape.txt").is_file());
    }
}


