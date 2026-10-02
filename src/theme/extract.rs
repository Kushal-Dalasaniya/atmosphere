use std::path::Path;

use anyhow::{Context, Result};
use image::imageops::FilterType;
use material_colors::color::Argb;
use material_colors::palette::CorePalette;
use material_colors::scheme::Scheme;
use material_colors::theme::Theme as MaterialTheme;

use super::model::{ColorMode, Palette};

/// Built-in Material You extraction — no external matugen binary required.
pub fn extract_palette_from_wallpaper(path: &Path, mode: ColorMode) -> Result<Palette> {
    let source = source_color_from_image(path)?;
    let core = CorePalette::ofArgb(source);
    let scheme = match mode {
        ColorMode::Dark => Scheme::dark(&core),
        ColorMode::Light => Scheme::light(&core),
    };
    let theme = MaterialTheme::from(scheme);
    let s = theme.schemes;

    let pick = |c: material_colors::color::Argb| format!("#{:06x}", c.value() & 0xFFFFFF);

    Ok(Palette {
        background: pick(s.surface),
        surface: pick(s.surface_container_low),
        foreground: pick(s.on_surface),
        accent: pick(s.primary),
        on_accent: pick(s.on_primary),
        error: Some(pick(s.error)),
        secondary: Some(pick(s.secondary)),
        tertiary: Some(pick(s.tertiary)),
    })
}

fn source_color_from_image(path: &Path) -> Result<Argb> {
    let img = image::open(path).with_context(|| format!("open image {}", path.display()))?;
    let thumb = img.resize(128, 128, FilterType::Triangle);
    let mut r = 0u64;
    let mut g = 0u64;
    let mut b = 0u64;
    let mut n = 0u64;
    for pixel in thumb.pixels() {
        let p = pixel.0;
        r += p[0] as u64;
        g += p[1] as u64;
        b += p[2] as u64;
        n += 1;
    }
    if n == 0 {
        anyhow::bail!("empty image");
    }
    let argb = Argb::from_rgb(
        (r / n) as u8,
        (g / n) as u8,
        (b / n) as u8,
    );
    Ok(argb)
}
