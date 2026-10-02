use std::fs;
use std::path::Path;

use anyhow::Result;

use super::atomic::atomic_write;
use super::model::Palette;
use super::validate::checked_hex;
use crate::paths::{GTK_CSS_MARKER, gtk3_css_path, gtk4_css_path};

pub fn render_gtk_css(palette: &Palette) -> Result<String> {
    Ok(format!(
        "{marker}\n\
@define-color accent_bg_color {accent};\n\
@define-color accent_fg_color {on_accent};\n\
@define-color window_bg_color {background};\n\
@define-color window_fg_color {foreground};\n\
@define-color view_bg_color {surface};\n\
@define-color view_fg_color {foreground};\n",
        marker = GTK_CSS_MARKER,
        accent = checked_hex(&palette.accent, "accent")?,
        on_accent = checked_hex(&palette.on_accent, "on accent")?,
        background = checked_hex(&palette.background, "background")?,
        foreground = checked_hex(&palette.foreground, "foreground")?,
        surface = checked_hex(&palette.surface, "surface")?,
    ))
}

pub fn write_gtk_css_files(palette: &Palette) -> Result<()> {
    let body = render_gtk_css(palette)?;

    write_owned_css(&gtk4_css_path(), &body)?;
    write_owned_css(&gtk3_css_path(), &body)?;
    Ok(())
}

fn write_owned_css(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.is_file() {
        let existing = fs::read_to_string(path)?;
        if !existing.contains(GTK_CSS_MARKER) {
            let backup = path.with_file_name("gtk.css.user-saved");
            if !backup.is_file() {
                fs::copy(path, &backup)?;
            }
        }
    }
    atomic_write(path, content)
}
