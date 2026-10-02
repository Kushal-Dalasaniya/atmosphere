use std::path::Path;

use anyhow::{Context, Result};
use gio::Settings;
pub fn set_wallpaper(path: &Path) -> Result<()> {
    let file = gio::File::for_path(path);
    let uri = file.uri().to_string();
    let settings =
        Settings::new("org.gnome.desktop.background").context("background gsettings")?;
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
    let settings =
        Settings::new("org.gnome.desktop.interface").context("interface gsettings")?;
    settings.set_string("color-scheme", scheme)?;
    // Toggle to force GTK3 reload
    settings.set_string("gtk-theme", "Adwaita")?;
    settings.set_string("gtk-theme", gtk_theme)?;
    Ok(())
}
