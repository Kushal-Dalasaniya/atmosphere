use anyhow::{Context, Result};

use super::gtk_css;
use super::model::{ColorMode, Theme};
use super::shell;
use super::shell_check;
use super::wallpaper;
use crate::theme::flatpak;
use crate::theme::model;

pub fn apply_theme(theme: &Theme) -> Result<()> {
    if !theme.wallpaper.is_file() {
        anyhow::bail!("Could not read the wallpaper");
    }

    flatpak::ensure_flatpak_overrides_once()?;

    wallpaper::set_wallpaper(&theme.wallpaper).context("set wallpaper")?;
    gtk_css::write_gtk_css_files(&theme.palette).context("write GTK colors")?;

    let dark = theme.mode == ColorMode::Dark;
    wallpaper::apply_gtk_interface(dark).context("apply GTK interface settings")?;

    if shell_check::user_themes_enabled() {
        shell::write_shell_css(theme.mode, &theme.palette, &theme.glass)?;
        shell::reload_user_theme()?;
    }

    let id = model::theme_id_from_name(&theme.name)?;
    model::set_applied_theme(&id)?;
    Ok(())
}
