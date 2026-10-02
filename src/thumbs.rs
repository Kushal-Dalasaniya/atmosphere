use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::paths::thumb_cache_dir;

pub fn thumb_path_for(source: &Path) -> PathBuf {
    let mtime = fs::metadata(source)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        })
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
    // Aspect-preserving: longest side 256px. Call from a worker thread
    // (`gio::spawn_blocking`); never on the GTK main loop.
    // A decode failure here (including webp gdk-pixbuf cannot handle)
    // propagates so callers can list the file in the "could not read" note.
    let img = image::open(source)?;
    let thumb = img.thumbnail(256, 256);
    thumb.save(&dest)?;
    Ok(dest)
}

/// Render the theme `preview.png` for a saved theme directory.
/// Longer side 512px so cards stay crisp on HiDPI.
pub fn write_preview(source: &Path, theme_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(theme_dir)?;
    let dest = theme_dir.join("preview.png");
    let img = image::open(source)?;
    img.thumbnail(512, 512).save(&dest)?;
    Ok(dest)
}

pub fn list_wallpapers() -> Vec<PathBuf> {
    let dir = crate::paths::wallpapers_dir();
    let mut files = vec![];
    if !dir.is_dir() {
        return files;
    }
    for entry in fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn write_png(path: &Path) {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::new(32, 24));
        img.save(path).expect("write test png");
    }

    #[test]
    fn cache_name_tracks_file() {
        let dir = std::env::temp_dir().join("atmosphere-thumb-name-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let src = dir.join("wall.jpg");
        write_png(&src);
        let named = thumb_path_for(&src);
        // Name embeds the source file name and its mtime key…
        assert!(named.to_string_lossy().contains("wall.jpg-"));
        assert!(!named.to_string_lossy().ends_with("-0.png"));
        // …while a missing file falls back to the zero key.
        let missing = thumb_path_for(&dir.join("ghost.jpg"));
        assert!(missing.to_string_lossy().ends_with("-0.png"));
        // Same file + same mtime → same cache path (stable key).
        assert_eq!(thumb_path_for(&src), named);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn broken_files_error() {
        let dir = std::env::temp_dir().join("atmosphere-thumb-broken-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let bad = dir.join("broken.jpg");
        fs::write(&bad, b"not an image").unwrap();
        assert!(ensure_thumbnail(&bad).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_is_written() {
        let dir = std::env::temp_dir().join("atmosphere-preview-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let src = dir.join("wall.png");
        write_png(&src);
        let theme_dir = dir.join("theme");
        let dest = write_preview(&src, &theme_dir).unwrap();
        assert_eq!(dest, theme_dir.join("preview.png"));
        assert!(dest.is_file());
        let _ = fs::remove_dir_all(&dir);
    }
}
