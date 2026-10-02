use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

/// Atomically install `content` at `path`: write to a temp file in the same
/// directory, then `rename(2)` into place so readers never see a half-written
/// file. Creates parent directories (`mkdir -p` equivalent).
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create directory {}", parent.display()))?;
    }
    let tmp = path.with_extension("atmosphere.tmp");
    {
        let mut f = fs::File::create(&tmp)
            .with_context(|| format!("write temp file {}", tmp.display()))?;
        f.write_all(content.as_bytes())
            .with_context(|| format!("write temp file {}", tmp.display()))?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, path)
        .with_context(|| format!("install {}", path.display()))?;
    Ok(())
}

/// Copy `path` to `path + ".atmosphere.bak"` when it exists (pre-apply backup).
/// No-op when the file is absent.
pub fn backup_next_to(path: &Path) -> Result<()> {
    if path.is_file() {
        let bak = backup_path_for(path);
        fs::copy(path, &bak)
            .with_context(|| format!("back up {}", path.display()))?;
    }
    Ok(())
}

/// Restore `path` from its `.atmosphere.bak` sibling when a backup exists.
pub fn restore_from_backup(path: &Path) -> Result<()> {
    let bak = backup_path_for(path);
    if bak.is_file() {
        fs::copy(&bak, path)
            .with_context(|| format!("restore {}", path.display()))?;
    }
    Ok(())
}

fn backup_path_for(path: &Path) -> std::path::PathBuf {
    let name = path
        .file_name()
        .map(|n| format!("{}.atmosphere.bak", n.to_string_lossy()))
        .unwrap_or_else(|| "file.atmosphere.bak".to_string());
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_atomic_write() {
        let dir = std::env::temp_dir().join("atmosphere-atomic-test");
        let _ = fs::remove_dir_all(&dir);
        let target = dir.join("sub").join("gtk.css");
        atomic_write(&target, "hello").expect("write");
        assert_eq!(fs::read_to_string(&target).expect("read"), "hello");
        backup_next_to(&target).expect("backup");
        atomic_write(&target, "changed").expect("rewrite");
        restore_from_backup(&target).expect("restore");
        assert_eq!(fs::read_to_string(&target).expect("read2"), "hello");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_backup_is_noop() {
        let target =
            std::env::temp_dir().join("atmosphere-missing-noop-test-gtk.css");
        let _ = fs::remove_file(&target);
        backup_next_to(&target).expect("noop ok");
        restore_from_backup(&target).expect("noop ok");
    }
}
