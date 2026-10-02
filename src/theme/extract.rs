use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use image::GenericImageView;
use material_colors::color::{Argb, Rgb};
use material_colors::dynamic_color::{DynamicScheme, Variant};

use super::model::{ColorMode, Palette};

/// In-process Material You extraction — no external binary (spec §4.4).
/// Side-effect free: reads the image, returns colors, touches nothing else.
pub fn extract_palette_from_wallpaper(path: &Path, mode: ColorMode) -> Result<Palette> {
    let source = source_color_from_image(path)?;
    let dark = mode == ColorMode::Dark;
    // Vibrant variant: vivid accents suited to desktop theming.
    let scheme = DynamicScheme::by_variant(source, &Variant::Vibrant, dark, None);

    Ok(Palette {
        background: scheme.surface().to_hex_with_pound(),
        surface: scheme.surface_container_low().to_hex_with_pound(),
        foreground: scheme.on_surface().to_hex_with_pound(),
        accent: scheme.primary().to_hex_with_pound(),
        on_accent: scheme.on_primary().to_hex_with_pound(),
        error: Some(scheme.error().to_hex_with_pound()),
        secondary: Some(scheme.secondary().to_hex_with_pound()),
        tertiary: Some(scheme.tertiary().to_hex_with_pound()),
    })
}

/// Dominant color via 5-bit/channel histogram over a downscaled copy.
/// Beats a flat average on busy photos (washed-out grays); cheap and local.
type VoteKey = (u8, u8, u8);
type VoteAcc = (u64, u64, u64, u64);

fn source_color_from_image(path: &Path) -> Result<Argb> {
    let img = image::open(path).with_context(|| format!("open image {}", path.display()))?;
    let thumb = img.thumbnail(128, 128);
    let mut votes: HashMap<VoteKey, VoteAcc> = HashMap::new();
    for (_, _, rgba) in thumb.pixels() {
        let c = rgba.0;
        let key = (c[0] >> 3, c[1] >> 3, c[2] >> 3);
        let slot = votes.entry(key).or_insert((0, 0, 0, 0));
        slot.0 += c[0] as u64;
        slot.1 += c[1] as u64;
        slot.2 += c[2] as u64;
        slot.3 += 1;
    }
    let (_, (r, g, b, n)) = votes
        .into_iter()
        .max_by_key(|(_, (_, _, _, n))| *n)
        .context("empty image")?;
    if n == 0 {
        anyhow::bail!("empty image");
    }
    Ok(Argb::from(Rgb {
        red: (r / n) as u8,
        green: (g / n) as u8,
        blue: (b / n) as u8,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::validate::is_valid_hex;

    /// Manual reference-machine check (quickstart T032 scenario 2).
    /// `cargo test -- --ignored live_` with a real wallpaper library.
    /// Pure computation: asserts the desktop is untouched.
    #[test]
    #[ignore]
    fn live_extract_isolation() {
        use gio::prelude::*;
        let bg = gio::Settings::new("org.gnome.desktop.background");
        let before = (
            bg.string("picture-uri").to_string(),
            bg.string("picture-uri-dark").to_string(),
        );
        let uri = before.0.clone();
        let path = gio::File::for_uri(&uri)
            .path()
            .expect("wallpaper must resolve");
        for mode in [ColorMode::Dark, ColorMode::Light] {
            let started = std::time::Instant::now();
            let palette = extract_palette_from_wallpaper(&path, mode).expect("extracts");
            eprintln!(
                "extract {mode:?} took {:.3}s",
                started.elapsed().as_secs_f32()
            );
            for hex in [
                &palette.background,
                &palette.surface,
                &palette.foreground,
                &palette.accent,
                &palette.on_accent,
            ] {
                assert!(is_valid_hex(hex), "invalid {hex}");
            }
        }
        let after = (
            bg.string("picture-uri").to_string(),
            bg.string("picture-uri-dark").to_string(),
        );
        assert_eq!(before, after, "extract must not touch the desktop");
    }
}
