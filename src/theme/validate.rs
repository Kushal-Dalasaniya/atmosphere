use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Returns true only for `#RRGGBB` (leading `#`, exactly 6 hex digits).
/// Rejects anything that could break out of a CSS value context
/// (`{`, `}`, `url(`, `javascript:`, newlines, quotes, …) by construction.
pub fn is_valid_hex(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return false;
    }
    bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
}

/// Validate a palette color for interpolation into generated CSS/TOML.
pub fn checked_hex(value: &str, role: &str) -> Result<String> {
    if is_valid_hex(value) {
        Ok(value.to_string())
    } else {
        anyhow::bail!("Invalid {} color {:?}: expected #RRGGBB", role, value)
    }
}

/// Canonicalize `path` and require it to be a readable regular file under
/// `$HOME`. Rejects missing files, directories, `..` escapes outside home,
/// and non-regular files. Returns the canonical path.
pub fn checked_wallpaper_path(path: &Path) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(path)
        .with_context(|| format!("Could not read the wallpaper: {}", path.display()))?;
    let home = crate::paths::home();
    let home_canon = std::fs::canonicalize(&home).unwrap_or(home);
    if !canonical.starts_with(&home_canon) {
        anyhow::bail!("Could not read the wallpaper: outside home directory");
    }
    let meta = std::fs::symlink_metadata(&canonical)
        .with_context(|| format!("Could not read the wallpaper: {}", canonical.display()))?;
    if !meta.is_file() {
        anyhow::bail!("Could not read the wallpaper: not a regular file");
    }
    Ok(canonical)
}

/// Decide whether `candidate` (a symlink or regular file being imported by
/// Add-wallpaper) may be copied into the wallpaper library: regular files
/// only, and a symlink must resolve inside the source file's own directory.
pub fn importable_regular_file(candidate: &Path) -> Result<PathBuf> {
    let meta = std::fs::symlink_metadata(candidate)
        .with_context(|| format!("Could not read file: {}", candidate.display()))?;
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(candidate)
            .with_context(|| format!("Could not read link: {}", candidate.display()))?;
        let resolved = if target.is_absolute() {
            target
        } else {
            candidate
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(target)
        };
        let canonical = std::fs::canonicalize(&resolved)
            .with_context(|| format!("Could not resolve link: {}", candidate.display()))?;
        let parent = candidate
            .parent()
            .map(|p| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()))
            .unwrap_or_default();
        if !canonical.starts_with(&parent) {
            anyhow::bail!("Refusing to import a link that escapes its directory");
        }
        let tmeta = std::fs::symlink_metadata(&canonical)
            .with_context(|| format!("Could not read link target: {}", canonical.display()))?;
        if !tmeta.is_file() {
            anyhow::bail!("Only regular image files can be imported");
        }
        return Ok(canonical);
    }
    if !meta.is_file() {
        anyhow::bail!("Only regular image files can be imported");
    }
    Ok(candidate.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_rrggbb() {
        assert!(is_valid_hex("#1b1b1f"));
        assert!(is_valid_hex("#C5C0FF"));
    }

    #[test]
    fn rejects_injection_shapes() {
        for bad in [
            "#000\"; url(javascript:1)",
            "#fff;}",
            "{background}",
            "javascript:1",
            "#12345",
            "#1234567",
            "123456",
            "#zzzzzz",
            "#12\n34",
            "",
        ] {
            assert!(!is_valid_hex(bad), "should reject {bad:?}");
        }
    }

    #[test]
    fn rejects_traversal_outside_home() {
        assert!(checked_wallpaper_path(Path::new("/etc/hostname")).is_err());
        assert!(checked_wallpaper_path(Path::new("/nonexistent-xyz.jpg")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape_on_import() {
        use std::os::unix::fs::symlink;
        let dir = std::env::temp_dir().join("atmosphere-symlink-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        let outside = dir.join("outside.jpg");
        std::fs::write(&outside, b"fake").unwrap();
        let inside = dir.join("lib").join("inside.jpg");
        std::fs::write(&inside, b"fake").unwrap();

        // Link inside the library pointing outside its directory: refused.
        let escape = dir.join("lib").join("escape.jpg");
        symlink(&outside, &escape).unwrap();
        assert!(importable_regular_file(&escape).is_err());

        // Link resolving inside its own directory: accepted.
        let local = dir.join("lib").join("local.jpg");
        symlink(&inside, &local).unwrap();
        assert!(importable_regular_file(&local).is_ok());

        // Plain regular file: accepted.
        assert!(importable_regular_file(&inside).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
