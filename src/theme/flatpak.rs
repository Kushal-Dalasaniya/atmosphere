use std::fs;
use std::path::PathBuf;

use anyhow::Result;

fn override_flag_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".local/share/atmosphere/flatpak-overrides.done")
}

pub fn ensure_flatpak_overrides_once() -> Result<()> {
    if override_flag_file().is_file() {
        return Ok(());
    }
    if std::process::Command::new("flatpak")
        .arg("--version")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        return Ok(());
    }
    let _ = std::process::Command::new("flatpak")
        .args([
            "override",
            "--user",
            "--filesystem=xdg-config/gtk-4.0:ro",
        ])
        .status();
    let _ = std::process::Command::new("flatpak")
        .args([
            "override",
            "--user",
            "--filesystem=xdg-config/gtk-3.0:ro",
        ])
        .status();
    if let Some(parent) = override_flag_file().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(override_flag_file(), "ok")?;
    Ok(())
}
