use std::fs;
use std::path::Path;

use anyhow::Result;

use super::atomic::atomic_write;
use super::glass::{hex_to_rgba, panel_alpha};
use super::model::{GlassSettings, Palette};
use super::validate::checked_hex;
use crate::paths::{GTK_CSS_MARKER, gtk3_css_path, gtk4_css_path, user_saved_backup_path};

pub fn render_gtk_css(palette: &Palette, glass: &GlassSettings) -> Result<String> {
    let background = checked_hex(&palette.background, "background")?;
    let surface = checked_hex(&palette.surface, "surface")?;
    let foreground = checked_hex(&palette.foreground, "foreground")?;
    let accent = checked_hex(&palette.accent, "accent")?;
    let on_accent = checked_hex(&palette.on_accent, "on accent")?;

    // Best-effort translucent surface token, only when glass is enabled
    // (spec §4.9). Glass off emits zero translucent rules.
    let glass_token = match panel_alpha(glass) {
        Some(alpha) => format!(
            "/* Derived from palette; alpha depends on glass strength */\n@define-color atmosphere_glass_surface {};\n",
            hex_to_rgba(&surface, alpha)
        ),
        None => String::new(),
    };

    Ok(format!(
        "{marker}\n\
@define-color accent_bg_color {accent};\n\
@define-color accent_fg_color {on_accent};\n\
@define-color window_bg_color {background};\n\
@define-color window_fg_color {foreground};\n\
@define-color view_bg_color {surface};\n\
@define-color view_fg_color {foreground};\n\
{glass_token}",
        marker = GTK_CSS_MARKER,
        accent = accent,
        on_accent = on_accent,
        background = background,
        foreground = foreground,
        surface = surface,
        glass_token = glass_token,
    ))
}

pub fn write_gtk_css_files(palette: &Palette, glass: &GlassSettings) -> Result<()> {
    let body = render_gtk_css(palette, glass)?;

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
            let backup = user_saved_backup_path(path);
            if !backup.is_file() {
                fs::copy(path, &backup)?;
            }
        }
    }
    atomic_write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::model::GlassStrength;

    fn sample_palette() -> Palette {
        Palette {
            background: "#1b1b1f".to_string(),
            surface: "#201f24".to_string(),
            foreground: "#e4e1e6".to_string(),
            accent: "#c5c0ff".to_string(),
            on_accent: "#2c0091".to_string(),
            error: None,
            secondary: None,
            tertiary: None,
        }
    }

    #[test]
    fn render_has_marker_and_theme_hexes() {
        let css = render_gtk_css(&sample_palette(), &GlassSettings::default()).unwrap();
        let lines: Vec<&str> = css.lines().collect();
        assert_eq!(lines[0], GTK_CSS_MARKER);
        for var in [
            "accent_bg_color",
            "accent_fg_color",
            "window_bg_color",
            "window_fg_color",
            "view_bg_color",
            "view_fg_color",
        ] {
            assert!(css.contains(var), "missing {var}");
        }
        for hex in ["#c5c0ff", "#2c0091", "#1b1b1f", "#e4e1e6", "#201f24"] {
            assert!(css.contains(hex), "missing {hex}");
        }
        assert!(!css.contains("{{"), "template braces leaked");
    }

    #[test]
    fn glass_off_emits_no_translucency() {
        let css = render_gtk_css(&sample_palette(), &GlassSettings::default()).unwrap();
        assert!(!css.contains("rgba("));
        assert!(!css.contains("atmosphere_glass_surface"));
    }

    #[test]
    fn glass_on_emits_surface_token() {
        for (strength, _) in [
            (GlassStrength::Subtle, 0.82f32),
            (GlassStrength::Strong, 0.68f32),
        ] {
            let css = render_gtk_css(
                &sample_palette(),
                &GlassSettings {
                    enabled: true,
                    strength,
                },
            )
            .unwrap();
            assert!(css.contains("@define-color atmosphere_glass_surface rgba("));
        }
    }

    #[test]
    fn invalid_hex_rejected_before_write() {
        let mut bad = sample_palette();
        bad.accent = "#000\"; url(javascript:1)".to_string();
        assert!(render_gtk_css(&bad, &GlassSettings::default()).is_err());
    }
}
