use std::path::Path;

use anyhow::Result;
use gio::Settings;
use gio::prelude::*;
pub fn set_wallpaper(path: &Path) -> Result<()> {
    let file = gio::File::for_path(path);
    let uri = file.uri().to_string();
    let settings = Settings::new("org.gnome.desktop.background");
    settings.set_string("picture-uri", &uri)?;
    settings.set_string("picture-uri-dark", &uri)?;
    settings.set_string("picture-options", "zoom")?;
    Ok(())
}

pub fn gtk_theme_for_mode(dark: bool) -> (&'static str, &'static str) {
    if dark {
        ("prefer-dark", "adw-gtk3-dark")
    } else {
        ("prefer-light", "adw-gtk3")
    }
}

pub fn apply_gtk_interface(dark: bool) -> Result<()> {
    let (scheme, gtk_theme) = gtk_theme_for_mode(dark);
    let settings = Settings::new("org.gnome.desktop.interface");
    settings.set_string("color-scheme", scheme)?;
    // Toggle to force GTK3 reload
    settings.set_string("gtk-theme", "Adwaita")?;
    settings.set_string("gtk-theme", gtk_theme)?;
    Ok(())
}

/// True when both `adw-gtk3` theme names resolve to an installed theme
/// directory (spec §4.5). Warn-only: CSS files are still written.
pub fn adw_gtk3_installed() -> bool {
    ["adw-gtk3", "adw-gtk3-dark"]
        .iter()
        .all(|name| theme_name_installed(name))
}

fn theme_name_installed(name: &str) -> bool {
    if name.is_empty() || name.contains('/') || name.contains("..") {
        return false;
    }
    for base in [
        std::path::Path::new("/usr/share/themes"),
        &crate::paths::home().join(".themes"),
        &crate::paths::home().join(".local/share/themes"),
    ] {
        if base.join(name).is_dir() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_theme_table() {
        // Spec §4.5: never hardcode adw-gtk3-dark in the light path, and
        // never write a hex color into the fixed-enum accent-color key
        // (this table is the only gtk-theme/color-scheme source).
        assert_eq!(gtk_theme_for_mode(true), ("prefer-dark", "adw-gtk3-dark"));
        assert_eq!(gtk_theme_for_mode(false), ("prefer-light", "adw-gtk3"));
    }

    #[test]
    fn theme_lookup_rejects_bad_names() {
        assert!(!theme_name_installed(""));
        assert!(!theme_name_installed("a/b"));
        assert!(!theme_name_installed(".."));
        // Runs anywhere without panicking; result depends on the machine.
        let _ = adw_gtk3_installed();
    }
}
