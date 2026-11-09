//! Asset library: scans the standard Ren'Py folders without copying anything.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    /// Path relative to the project root, always with forward slashes.
    pub path: String,
    pub name: String,
    /// image | audio | font | video | other
    pub kind: String,
    pub size: u64,
    pub extension: String,
}

pub fn classify(ext: &str) -> &'static str {
    match ext {
        "png" | "jpg" | "jpeg" | "webp" | "avif" | "svg" | "gif" | "bmp" => "image",
        "ogg" | "opus" | "mp3" | "wav" | "flac" | "m4a" => "audio",
        "ttf" | "otf" | "woff" | "woff2" => "font",
        "webm" | "mp4" | "avi" | "mkv" => "video",
        _ => "other",
    }
}

pub fn kind_for_path(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    classify(&ext)
}

/// Folder inside `game/` that owns this asset kind.
pub fn target_folder(kind: &str) -> Option<&'static str> {
    match kind {
        "image" => Some("images"),
        "audio" => Some("audio"),
        "font" => Some("fonts"),
        "video" => Some("video"),
        _ => None,
    }
}

/// Copies external files into the project without ever overwriting existing assets.
pub fn import_files(root: &Path, sources: &[PathBuf]) -> (Vec<String>, Vec<String>) {
    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    for source in sources {
        if !source.is_file() {
            skipped.push(format!("{}: не файл", source.display()));
            continue;
        }
        let kind = kind_for_path(source);
        let Some(folder) = target_folder(kind) else {
            skipped.push(format!("{}: неподдерживаемый тип", source.display()));
            continue;
        };
        let dir = root.join("game").join(folder);
        if let Err(err) = std::fs::create_dir_all(&dir) {
            skipped.push(format!("{}: {err}", source.display()));
            continue;
        }
        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "asset".into());
        let ext = source
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let clean: String = stem
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let clean = clean.trim_matches('_').to_string();
        let base = if clean.is_empty() { "asset".to_string() } else { clean };
        let mut candidate = if ext.is_empty() {
            dir.join(&base)
        } else {
            dir.join(format!("{base}.{ext}"))
        };
        let mut n = 2;
        while candidate.exists() {
            candidate = if ext.is_empty() {
                dir.join(format!("{base}_{n}"))
            } else {
                dir.join(format!("{base}_{n}.{ext}"))
            };
            n += 1;
        }
        match std::fs::copy(source, &candidate) {
            Ok(_) => imported.push(relative(root, &candidate)),
            Err(err) => skipped.push(format!("{}: {err}", source.display())),
        }
    }
    (imported, skipped)
}
pub fn scan(root: &Path) -> Vec<AssetInfo> {
    let mut out = Vec::new();
    for folder in ["images", "audio", "fonts", "video", "gui"] {
        let dir = root.join("game").join(folder);
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .max_depth(6)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let kind = classify(&ext);
            if kind == "other" {
                continue;
            }
            // The default GUI bundles hundreds of images; keep the library focused.
            let rel = relative(root, path);
            if rel.starts_with("game/gui/") || rel.starts_with("game/gui7/") {
                continue;
            }
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| rel.clone());
            out.push(AssetInfo {
                path: rel,
                name,
                kind: kind.to_string(),
                size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                extension: ext,
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Resolves a project-relative asset path, refusing to escape the project root.
pub fn resolve_within(root: &Path, relative: &str) -> Option<PathBuf> {
    let cleaned = relative.replace('\\', "/");
    let trimmed = cleaned.trim_start_matches('/');
    if trimmed.split('/').any(|seg| seg == "..") {
        return None;
    }
    let path = root.join(trimmed);
    let canon_root = root.canonicalize().ok()?;
    let canon = path.canonicalize().ok()?;
    if canon.starts_with(&canon_root) {
        Some(canon)
    } else {
        None
    }
}

pub fn mime_for(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "ogg" | "opus" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_copies_files_and_never_overwrites() {
        let base = std::env::temp_dir().join(format!("renpy-assets-{}", crate::model::new_id()));
        let project = base.join("project");
        let source = base.join("source");
        std::fs::create_dir_all(project.join("game")).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        let first = source.join("Mira Happy.png");
        std::fs::write(&first, b"one").unwrap();
        let second = source.join("mira_happy.png");
        std::fs::write(&second, b"two").unwrap();

        let (imported, skipped) = import_files(&project, &[first, second]);
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(imported.len(), 2);
        assert!(imported[0].starts_with("game/images/"));
        assert_ne!(imported[0], imported[1], "second copy must get a suffix");
        let bytes = std::fs::read(project.join(&imported[1])).unwrap();
        assert_eq!(bytes, b"two");
        let _ = std::fs::remove_dir_all(&base);
    }
}
