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
    let alpha = panel_alpha(glass);
    if alpha.is_none() {
        return format!(
            "#panel {{
  background-color: {bg};
  color: {fg};
}}
",
            bg = palette.surface,
            fg = palette.foreground,
        );
    }
    let a = alpha.unwrap();
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
