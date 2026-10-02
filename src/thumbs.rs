use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use image::imageops::FilterType;

use crate::paths::thumb_cache_dir;

pub fn thumb_path_for(source: &Path) -> PathBuf {
    let mtime = fs::metadata(source)
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(0);
    let name = format!(
        "{}-{}.png",
        source.file_name().unwrap_or_default().to_string_lossy(),
        mtime
    );
    thumb_cache_dir().join(name)
}

pub fn ensure_thumbnail(source: &Path) -> Result<PathBuf> {
    fs::create_dir_all(thumb_cache_dir())?;
    let dest = thumb_path_for(source);
    if dest.is_file() {
        return Ok(dest);
    }
    let img = image::open(source)?;
    let thumb = img.resize(256, 256, FilterType::Triangle);
    thumb.save(&dest)?;
    Ok(dest)
}

pub fn list_wallpapers() -> Vec<PathBuf> {
    let dir = crate::paths::wallpapers_dir();
    let mut files = vec![];
    if !dir.is_dir() {
        return files;
    }
    for entry in fs::read_dir(dir).into_iter().flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
            files.push(path);
        }
    }
    files.sort();
    files
}
