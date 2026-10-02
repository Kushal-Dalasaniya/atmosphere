use std::fs;

use anyhow::Result;

use crate::paths::flatpak_done_path;

const OVERRIDES: &[&str] = &[
    "--filesystem=xdg-config/gtk-4.0:ro",
    "--filesystem=xdg-config/gtk-3.0:ro",
];

fn flatpak_present() -> bool {
    std::process::Command::new("flatpak")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Query the real override state (fixed args only) instead of trusting the
/// flag file alone — the user may have reset overrides externally.
fn overrides_present() -> bool {
    let out = std::process::Command::new("flatpak")
        .args(["override", "--user", "--show"])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            OVERRIDES
                .iter()
                .all(|want| text.contains(&want["--filesystem=".len()..]))
        }
        _ => false,
    }
}

fn mark_done() -> Result<()> {
    let flag = flatpak_done_path();
    if let Some(parent) = flag.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(flag, "ok")?;
    Ok(())
}

pub fn ensure_flatpak_overrides_once() -> Result<()> {
    if !flatpak_present() {
        return Ok(());
    }
    if overrides_present() {
        mark_done()?;
        return Ok(());
    }
    for fs in OVERRIDES {
        let _ = std::process::Command::new("flatpak")
            .args(["override", "--user", fs])
            .status();
    }
    mark_done()?;
    Ok(())
}
