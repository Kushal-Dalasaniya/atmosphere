use super::model::{GlassSettings, GlassStrength, Palette};

pub fn panel_alpha(glass: &GlassSettings) -> Option<f32> {
    if !glass.enabled {
        return None;
    }
    match glass.strength {
        GlassStrength::Subtle => Some(0.82),
        GlassStrength::Strong => Some(0.68),
    }
}

pub fn hex_to_rgba(hex: &str, alpha: f32) -> String {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return format!("rgba(0,0,0,{alpha})");
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
    format!("rgba({r}, {g}, {b}, {alpha})")
}

pub fn shell_glass_block(palette: &Palette, glass: &GlassSettings) -> String {
    let Some(a) = panel_alpha(glass) else {
        return format!(
            "#panel {{
  background-color: {bg};
  color: {fg};
}}
",
            bg = palette.surface,
            fg = palette.foreground,
        );
    };
    let panel_bg = hex_to_rgba(&palette.surface, a);
    format!(
        "#panel {{
  background-color: {panel_bg};
  color: {fg};
}}
.calendar,
.message-list-section,
.notification-banner {{
  background-color: {panel_bg};
  color: {fg};
}}
",
        panel_bg = panel_bg,
        fg = palette.foreground,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::model::Palette;

    fn sample() -> Palette {
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

    fn glass(enabled: bool, strength: GlassStrength) -> GlassSettings {
        GlassSettings { enabled, strength }
    }

    #[test]
    fn alpha_bands_match_spec() {
        // Spec §4.10: subtle 0.75–0.88, strong 0.60–0.75.
        assert_eq!(panel_alpha(&glass(false, GlassStrength::Subtle)), None);
        let subtle = panel_alpha(&glass(true, GlassStrength::Subtle)).unwrap();
        assert!((0.75..=0.88).contains(&subtle), "subtle {subtle}");
        let strong = panel_alpha(&glass(true, GlassStrength::Strong)).unwrap();
        assert!((0.60..=0.75).contains(&strong), "strong {strong}");
        assert!(strong < subtle, "strong must be more translucent");
    }

    #[test]
    fn hex_to_rgba_math() {
        assert_eq!(hex_to_rgba("#ffffff", 1.0), "rgba(255, 255, 255, 1)");
        assert_eq!(hex_to_rgba("#201f24", 0.82), "rgba(32, 31, 36, 0.82)");
        // Invalid input degrades to opaque black, never breaks CSS syntax.
        assert_eq!(hex_to_rgba("not-a-color", 0.5), "rgba(0,0,0,0.5)");
    }

    #[test]
    fn off_path_is_opaque_and_overwrites() {
        let css = shell_glass_block(&sample(), &glass(false, GlassStrength::Subtle));
        assert!(css.contains("#panel"));
        assert!(css.contains("#201f24"), "opaque surface color");
        assert!(
            !css.contains("rgba("),
            "off must emit zero translucent rules"
        );
    }

    #[test]
    fn on_path_tints_panel_and_popups() {
        let css = shell_glass_block(&sample(), &glass(true, GlassStrength::Strong));
        assert!(css.contains("rgba(32, 31, 36, 0.68)"));
        for selector in [
            "#panel",
            ".calendar",
            ".message-list-section",
            ".notification-banner",
        ] {
            assert!(css.contains(selector), "missing {selector}");
        }
        // Theme colors still drive text on the tint.
        assert!(css.contains("#e4e1e6"));
    }
}
