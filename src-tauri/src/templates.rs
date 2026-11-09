//! New-project scaffolding: copies Ren'Py's default GUI project from the SDK so
//! the generated game has real screens (say window, menus, saves) from minute one.

use std::fs;
use std::path::Path;

/// Files the studio owns: they are copied from the SDK template once and never
/// regenerated, so the user can restyle them by hand if they want.
const TEMPLATE_FILES: [&str; 4] = ["gui.rpy", "screens.rpy", "options.rpy", "guisupport.rpy"];

fn copy_dir_filtered(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "cache" || name.ends_with(".rpyc") || name.ends_with(".pyc") {
            continue;
        }
        let target = dst.join(&name);
        if path.is_dir() {
            copy_dir_filtered(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| format!("Копирование {name}: {e}"))?;
        }
    }
    Ok(())
}

/// Rewrites the few `options.rpy` values that must match the project.
fn patch_options(path: &Path, title: &str, slug: &str) -> Result<(), String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut out = String::new();
    for line in raw.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("define config.name =") {
            out.push_str(&format!(
                "define config.name = _({})\n",
                crate::model::rpy_string(title)
            ));
        } else if trimmed.starts_with("define build.name =") {
            out.push_str(&format!("define build.name = \"{slug}\"\n"));
        } else if trimmed.starts_with("define config.save_directory =") {
            out.push_str(&format!("define config.save_directory = \"{slug}-studio\"\n"));
        } else if trimmed.starts_with("define config.version =") {
            out.push_str("define config.version = \"1.0\"\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    crate::store::write_atomic(path, out.as_bytes())
}

/// Creates the standard Ren'Py folder layout and the SDK GUI template inside it.
pub fn scaffold(root: &Path, sdk_root: Option<&Path>, title: &str, slug: &str) -> Result<(), String> {
    for sub in ["game/images", "game/audio", "game/fonts", "game/gui", "renpy-studio/history"] {
        fs::create_dir_all(root.join(sub)).map_err(|e| format!("Не удалось создать {sub}: {e}"))?;
    }

    let Some(sdk_root) = sdk_root else {
        return Err(
            "Не найден Ren'Py SDK: без него нельзя создать полноценные экраны игры. Укажите путь к SDK в настройках."
                .into(),
        );
    };
    let template = sdk_root.join("gui").join("game");
    if !template.join("screens.rpy").is_file() {
        return Err(format!(
            "В SDK не найден шаблон проекта ({}). Проверьте целостность Ren'Py SDK.",
            template.display()
        ));
    }

    for file in TEMPLATE_FILES {
        let src = template.join(file);
        if src.is_file() {
            fs::copy(&src, root.join("game").join(file))
                .map_err(|e| format!("Копирование {file}: {e}"))?;
        }
    }
    copy_dir_filtered(&template.join("gui"), &root.join("game").join("gui"))?;
    // Empty placeholders so the library shows the standard folders.
    for sub in ["images", "audio", "fonts"] {
        fs::create_dir_all(root.join("game").join(sub)).map_err(|e| e.to_string())?;
    }
    let options = root.join("game").join("options.rpy");
    if options.is_file() {
        patch_options(&options, title, slug)?;
    }
    Ok(())
}
