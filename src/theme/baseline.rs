use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use gio::Settings;
use serde::{Deserialize, Serialize};

use super::atomic::atomic_write;
use crate::paths::{
    GTK_CSS_MARKER, applied_state_path, baseline_dir, gtk3_css_path, gtk4_css_path,
};

/// Snapshot of every gsettings key and file Atmosphere may modify,
/// captured once before the first successful Apply (spec §4.11).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub picture_uri: String,
    pub picture_uri_dark: String,
    pub picture_options: String,
    pub color_scheme: String,
    pub gtk_theme: String,
    pub icon_theme: String,
    pub user_theme_name: String,
}

pub fn baseline_exists() -> bool {
    crate::paths::baseline_toml_path().is_file()
}

fn background_settings() -> Result<Settings> {
    Settings::new("org.gnome.desktop.background").context("background gsettings")
}

fn interface_settings() -> Result<Settings> {
    Settings::new("org.gnome.desktop.interface").context("interface gsettings")
}

fn user_theme_schema_installed() -> bool {
    gio::SettingsSchemaSource::default()
        .map(|src| {
            src.lookup("org.gnome.shell.extensions.user-theme", false)
                .is_some()
        })
        .unwrap_or(false)
}

fn read_string(s: &Settings, key: &str) -> String {
    s.string(key).to_string()
}

/// Capture the baseline snapshot once. No-op when it already exists.
pub fn capture_baseline_once() -> Result<()> {
    if baseline_exists() {
        return Ok(());
    }
    let dir = baseline_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("create {}", dir.display()))?;

    let bg = background_settings()?;
    let iface = interface_settings()?;
    let user_theme_name = if user_theme_schema_installed() {
        Settings::new("org.gnome.shell.extensions.user-theme")
            .map(|s| read_string(&s, "name"))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let baseline = Baseline {
        picture_uri: read_string(&bg, "picture-uri"),
        picture_uri_dark: read_string(&bg, "picture-uri-dark"),
        picture_options: read_string(&bg, "picture-options"),
        color_scheme: read_string(&iface, "color-scheme"),
        gtk_theme: read_string(&iface, "gtk-theme"),
        icon_theme: read_string(&iface, "icon-theme"),
        user_theme_name,
    };
    atomic_write(
        &crate::paths::baseline_toml_path(),
        &toml::to_string_pretty(&baseline).context("serialize baseline")?,
    )?;

    snapshot_css(&gtk3_css_path(), &dir.join("gtk-3.0.css"))?;
    snapshot_css(&gtk4_css_path(), &dir.join("gtk-4.0.css"))?;
    atomic_write(
        &dir.join("README"),
        "Created before first Atmosphere Apply; used by Restore Ubuntu defaults.\n",
    )?;
    Ok(())
}

fn snapshot_css(live: &Path, snap: &Path) -> Result<()> {
    if live.is_file() {
        let data = fs::read(live)
            .with_context(|| format!("snapshot {}", live.display()))?;
        fs::write(snap, data)
            .with_context(|| format!("store {}", snap.display()))?;
    } else {
        atomic_write(snap, "absent\n")?;
    }
    Ok(())
}

pub fn load_baseline() -> Result<Option<Baseline>> {
    let path = crate::paths::baseline_toml_path();
    if !path.is_file() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    Ok(Some(toml::from_str(&data).context("parse baseline.toml")?))
}

/// Restore the desktop to the baseline snapshot (spec §3.5).
/// Falls back to documented Ubuntu 26 defaults when no baseline exists.
pub fn restore_baseline() -> Result<()> {
    match load_baseline()? {
        Some(b) => apply_baseline(&b),
        None => apply_fallback_defaults(),
    }?;
    // Owned CSS was already handled by apply_baseline; fallbacks handled below.
    // Clear applied state so no theme shows as "Applied".
    let applied = applied_state_path();
    if applied.is_file() {
        fs::remove_file(&applied).ok();
    }
    Ok(())
}

fn apply_baseline(b: &Baseline) -> Result<()> {
    let bg = background_settings()?;
    let iface = interface_settings()?;
    bg.set_string("picture-uri", &b.picture_uri)?;
    bg.set_string("picture-uri-dark", &b.picture_uri_dark)?;
    bg.set_string("picture-options", &b.picture_options)?;
    iface.set_string("color-scheme", &b.color_scheme)?;
    if valid_gtk_theme(&b.gtk_theme) {
        iface.set_string("gtk-theme", &b.gtk_theme)?;
    }
    if valid_icon_theme(&b.icon_theme) {
        iface.set_string("icon-theme", &b.icon_theme)?;
    }
    if user_theme_schema_installed() {
        if let Ok(s) = Settings::new("org.gnome.shell.extensions.user-theme") {
            s.set_string("name", &b.user_theme_name)?;
        }
    }
    restore_css(&gtk3_css_path(), &baseline_dir().join("gtk-3.0.css"))?;
    restore_css(&gtk4_css_path(), &baseline_dir().join("gtk-4.0.css"))?;
    Ok(())
}

fn restore_css(live: &Path, snap: &Path) -> Result<()> {
    if !snap.is_file() {
        return Ok(());
    }
    let data = fs::read(snap)?;
    if data == b"absent\n" || data.is_empty() {
        // User had no custom CSS: remove Atmosphere-owned file only.
        if live.is_file() {
            let existing = fs::read_to_string(live).unwrap_or_default();
            if existing.contains(GTK_CSS_MARKER) {
                fs::remove_file(live).ok();
            }
        }
        return Ok(());
    }
    // Restore the user's original CSS (never guess).
    if let Some(parent) = live.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(live, data)?;
    Ok(())
}

fn apply_fallback_defaults() -> Result<()> {
    let bg = background_settings()?;
    let iface = interface_settings()?;
    // Documented Ubuntu 26 fallbacks (spec §4.11); warn caller via error text
    // chain is not possible here, so the UI toast must mention approximation.
    bg.set_string("picture-options", "zoom")?;
    iface.set_string("color-scheme", "default")?;
    if valid_gtk_theme("Yaru") {
        iface.set_string("gtk-theme", "Yaru")?;
    }
    if user_theme_schema_installed() {
        if let Ok(s) = Settings::new("org.gnome.shell.extensions.user-theme") {
            s.set_string("name", "")?;
        }
    }
    for live in [gtk3_css_path(), gtk4_css_path()] {
        let saved = live.with_file_name("gtk.css.user-saved");
        if saved.is_file() {
            fs::copy(&saved, &live).ok();
        } else if live.is_file()
            && fs::read_to_string(&live)
                .unwrap_or_default()
                .contains(GTK_CSS_MARKER)
        {
            fs::remove_file(&live).ok();
        }
    }
    Ok(())
}

fn valid_gtk_theme(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    ["/usr/share/themes", "/usr/local/share/themes"].iter().any(|base| {
        Path::new(base).join(name).join("gtk-3.0").is_dir()
            || Path::new(base).join(name).join("gtk-4.0").is_dir()
    }) || crate::paths::home()
        .join(".themes")
        .join(name)
        .is_dir()
        || crate::paths::home()
            .join(".local/share/themes")
            .join(name)
            .is_dir()
}

fn valid_icon_theme(name: &str) -> bool {
    super::icons::icon_theme_installed(name)
}
