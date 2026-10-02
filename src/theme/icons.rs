use std::path::Path;

use anyhow::Result;
use gio::Settings;

use super::model::ColorMode;
use super::validate::checked_hex;

/// Candidate Yaru variants: (theme name, accent hue in degrees, is dark).
/// Hues are anchors for "closest on the color wheel" matching (spec §4.12);
/// only variants actually installed on the system are ever selected.
const YARU_VARIANTS: &[(&str, f32, bool)] = &[
    ("Yaru", 25.0, false),
    ("Yaru-dark", 25.0, true),
    ("Yaru-blue", 212.0, false),
    ("Yaru-blue-dark", 212.0, true),
    ("Yaru-magenta", 312.0, false),
    ("Yaru-magenta-dark", 312.0, true),
    ("Yaru-purple", 276.0, false),
    ("Yaru-purple-dark", 276.0, true),
    ("Yaru-red", 4.0, false),
    ("Yaru-red-dark", 4.0, true),
    ("Yaru-sage", 152.0, false),
    ("Yaru-sage-dark", 152.0, true),
    ("Yaru-olive", 72.0, false),
    ("Yaru-olive-dark", 72.0, true),
    ("Yaru-prussiangreen", 172.0, false),
    ("Yaru-prussiangreen-dark", 172.0, true),
    ("Yaru-wartybrown", 18.0, false),
    ("Yaru-wartybrown-dark", 18.0, true),
    ("Yaru-yellow", 48.0, false),
    ("Yaru-yellow-dark", 48.0, true),
];

pub fn icon_theme_installed(name: &str) -> bool {
    if name.is_empty() || name.contains('/') || name.contains("..") {
        return false;
    }
    for base in [
        Path::new("/usr/share/icons"),
        &crate::paths::home().join(".local/share/icons"),
        &crate::paths::home().join(".icons"),
    ] {
        if base.join(name).join("index.theme").is_file() {
            return true;
        }
    }
    false
}

fn hex_to_hsl(hex: &str) -> Option<(f32, f32)> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < f32::EPSILON {
        return Some((0.0, 0.0));
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let hue = if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    Some(((hue + 360.0) % 360.0, s))
}

fn circular_distance(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    if d > 180.0 {
        360.0 - d
    } else {
        d
    }
}

/// Pick the nearest installed Yaru variant for `accent_hex` in `mode`.
/// Returns None when nothing suitable is installed.
pub fn nearest_yaru_variant(accent_hex: &str, mode: ColorMode) -> Option<String> {
    let (hue, sat) = hex_to_hsl(accent_hex)?;
    let want_dark = mode == ColorMode::Dark;
    // Achromatic accents: keep Ubuntu default for the mode.
    if sat < 0.12 {
        let fallback = if want_dark { "Yaru-dark" } else { "Yaru" };
        if icon_theme_installed(fallback) {
            return Some(fallback.to_string());
        }
    }
    let mut best: Option<(&str, f32)> = None;
    for (name, anchor, is_dark) in YARU_VARIANTS {
        if !icon_theme_installed(name) {
            continue;
        }
        let mut dist = circular_distance(hue, *anchor);
        // Prefer the mode-matching brightness; small penalty otherwise.
        if *is_dark != want_dark {
            dist += 15.0;
        }
        if best.map(|(_, d)| dist < d).unwrap_or(true) {
            best = Some((name, dist));
        }
    }
    best.map(|(n, _)| n.to_string())
}

/// Apply the Yaru icon match. Returns the theme name set, or None when
/// matching is disabled, the accent is invalid, or nothing is installed.
pub fn apply_icon_match(
    match_yaru: bool,
    mode: ColorMode,
    accent_hex: &str,
) -> Result<Option<String>> {
    if !match_yaru {
        return Ok(None);
    }
    let accent = checked_hex(accent_hex, "accent")?;
    let Some(name) = nearest_yaru_variant(&accent, mode) else {
        return Ok(None);
    };
    let settings =
        Settings::new("org.gnome.desktop.interface").map_err(|e| anyhow::anyhow!("{e:?}"))?;
    settings.set_string("icon-theme", &name)?;
    Ok(Some(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hue_math_sanity() {
        let (h, s) = hex_to_hsl("#ff0000").unwrap();
        assert!((h - 0.0).abs() < 1.0 && s > 0.9);
        let (h, _) = hex_to_hsl("#0000ff").unwrap();
        assert!((h - 240.0).abs() < 1.0);
        assert!(hex_to_hsl("not-a-color").is_none());
    }

    #[test]
    fn rejects_bad_names() {
        assert!(!icon_theme_installed(""));
        assert!(!icon_theme_installed("../x"));
        assert!(!icon_theme_installed("a/b"));
    }
}
