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

fn classify(ext: &str) -> &'static str {
    match ext {
        "png" | "jpg" | "jpeg" | "webp" | "avif" | "svg" | "gif" | "bmp" => "image",
        "ogg" | "opus" | "mp3" | "wav" | "flac" | "m4a" => "audio",
        "ttf" | "otf" | "woff" | "woff2" => "font",
        "webm" | "mp4" | "avi" | "mkv" => "video",
        _ => "other",
    }
}

/// Scans `game/images`, `game/audio`, `game/fonts`, `game/video` recursively.
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
