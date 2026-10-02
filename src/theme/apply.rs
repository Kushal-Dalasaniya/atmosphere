use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{Context, Result};

use super::alacritty;
use super::atomic::{backup_next_to, restore_from_backup};
use super::baseline::capture_baseline_once;
use super::gtk_css;
use super::icons;
use super::model::{self, ColorMode, Theme};
use super::shell;
use super::shell_check;
use super::validate::{checked_hex, checked_wallpaper_path};
use super::wallpaper;
use crate::paths::{
    alacritty_colors_path, alacritty_config_path, gtk3_css_path, gtk4_css_path,
    shell_theme_css_path,
};
use crate::theme::flatpak;

/// Single-apply mutex (spec §4.2): applies are strictly sequential.
/// A second apply blocks here until the current job finishes; the UI
/// additionally ignores clicks while a job runs.
static APPLY_LOCK: Mutex<()> = Mutex::new(());

/// Files Atmosphere may rewrite during one apply; backed up before any
/// desktop change and restored when the job fails after backups exist.
fn owned_outputs() -> Vec<PathBuf> {
    vec![
        gtk4_css_path(),
        gtk3_css_path(),
        shell_theme_css_path(),
        alacritty_colors_path(),
        alacritty_config_path(),
    ]
}

/// Back up owned outputs, remembering which existed beforehand so rollback
/// can remove files this job created fresh (spec §4.2 step 10: the desktop
/// must match its pre-apply state, not just its pre-apply backups).
fn backup_owned_outputs() -> Result<Vec<(PathBuf, bool)>> {
    let mut prior = Vec::new();
    for path in owned_outputs() {
        let existed = path.is_file();
        backup_next_to(&path).with_context(|| format!("back up {}", path.display()))?;
        prior.push((path, existed));
    }
    Ok(prior)
}

fn rollback_owned_outputs(prior: &[(PathBuf, bool)]) {
    for (path, existed) in prior {
        if *existed {
            if let Err(e) = restore_from_backup(path) {
                eprintln!("atmosphere: rollback failed for {}: {e:?}", path.display());
            }
        } else if path.is_file() {
            // Created fresh by the failed job: remove it.
            if let Err(e) = fs::remove_file(path) {
                eprintln!("atmosphere: rollback failed for {}: {e:?}", path.display());
            }
        }
    }
}

/// Validate everything before touching the desktop (spec §4.2 step 1):
/// canonical wallpaper path, `#RRGGBB` palette, usable theme id.
fn validate_theme(theme: &Theme) -> Result<()> {
    checked_wallpaper_path(&theme.wallpaper)?;
    let p = &theme.palette;
    checked_hex(&p.background, "background")?;
    checked_hex(&p.surface, "surface")?;
    checked_hex(&p.foreground, "foreground")?;
    checked_hex(&p.accent, "accent")?;
    checked_hex(&p.on_accent, "on_accent")?;
    for (role, value) in [
        ("error", &p.error),
        ("secondary", &p.secondary),
        ("tertiary", &p.tertiary),
    ] {
        if let Some(v) = value {
            checked_hex(v, role)?;
        }
    }
    model::theme_id_from_name(&theme.name)?;
    Ok(())
}

pub fn apply_theme(theme: &Theme) -> Result<()> {
    let started = Instant::now();
    let _guard = APPLY_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("Apply already in progress"))?;

    validate_theme(theme)?;

    // Steps 2–2b: pre-apply backups + one-time baseline, before any change.
    let prior = backup_owned_outputs()?;
    capture_baseline_once().context("capture baseline")?;

    let result = apply_validated(theme);
    match result {
        Ok(()) => {
            let id = model::theme_id_from_name(&theme.name)?;
            model::set_applied_theme(&id)?;
            // Durability: flush gsettings to dconf before returning, so a
            // fast-exiting caller (tests, one-shot apply) never loses writes.
            gio::Settings::sync();
            eprintln!(
                "atmosphere: applied \"{}\" in {:.3}s",
                theme.name,
                started.elapsed().as_secs_f32()
            );
            Ok(())
        }
        Err(e) => {
            // Step 10: restore pre-apply backups (not the baseline).
            // The new wallpaper stays only when its set succeeded; colors
            // are reverted. The UI toast reports the split outcome.
            rollback_owned_outputs(&prior);
            gio::Settings::sync();
            eprintln!(
                "atmosphere: apply failed after {:.3}s, backups restored: {e:?}",
                started.elapsed().as_secs_f32()
            );
            Err(e)
        }
    }
}

/// Steps 3–9 of spec §4.2 on an already-validated theme.
/// All pipeline steps are now integrated (icons, alacritty).
fn apply_validated(theme: &Theme) -> Result<()> {
    // Step 3: wallpaper first — the user sees the image before colors land.
    flatpak::ensure_flatpak_overrides_once()?;
    wallpaper::set_wallpaper(&theme.wallpaper).context("set wallpaper")?;

    // Everything below runs after a successful wallpaper set, so failures
    // here leave the new picture in place with colors reverted — the error
    // annotation carries that split outcome to the UI toast.
    apply_colors(theme).context("wallpaper kept, colors reverted")
}

/// Steps 4–9 of spec §4.2: palette reuse, render, settings, shell.
fn apply_colors(theme: &Theme) -> Result<()> {
    // Step 4: palette reuse is structural — the saved (possibly hand-edited)
    // palette travels in the theme; extraction never re-runs here.

    // Step 5: render through atomic temp→rename writers.
    gtk_css::write_gtk_css_files(&theme.palette, &theme.glass).context("write GTK colors")?;

    // Step 6: GTK + color-scheme settings.
    let dark = theme.mode == ColorMode::Dark;
    wallpaper::apply_gtk_interface(dark).context("apply GTK interface settings")?;

    // Icons: nearest installed Yaru variant (no-op when disabled or when
    // nothing suitable is installed — never fails the apply for that).
    if let Some(name) =
        icons::apply_icon_match(theme.icons.match_yaru, theme.mode, &theme.palette.accent)?
    {
        eprintln!("atmosphere: icon theme {name}");
    }

    // Step 7: shell overlay only when User Themes is enabled.
    if shell_check::user_themes_enabled() {
        shell::write_shell_css(theme.mode, &theme.palette, &theme.glass)?;
        shell::reload_user_theme()?;
    }

    // Alacritty: skipped entirely when it is not on PATH.
    let p = &theme.palette;
    if alacritty::ensure_alacritty_colors(
        &p.background,
        &p.foreground,
        &p.accent,
        &p.on_accent,
        &p.surface,
    )? {
        eprintln!("atmosphere: alacritty colors updated");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::baseline::{RestoreOutcome, restore_baseline};
    use crate::theme::model::{GlassSettings, IconsSettings, Palette};
    use gio::prelude::*;

    fn live_palette() -> Palette {
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

    /// Manual reference-machine validation (quickstart T032, scenarios
    /// 4/9/11/12/14). NEVER runs in CI: `cargo test -- --ignored live_`
    /// on Ubuntu 26.04 / GNOME 50 / Wayland only. Mutates the live desktop,
    /// then restores it through the baseline snapshot and asserts equality.
    #[test]
    #[ignore]
    fn live_apply_restore_cycle() {
        let bg = gio::Settings::new("org.gnome.desktop.background");
        let iface = gio::Settings::new("org.gnome.desktop.interface");
        let snapshot = [
            bg.string("picture-uri").to_string(),
            bg.string("picture-uri-dark").to_string(),
            bg.string("picture-options").to_string(),
            iface.string("color-scheme").to_string(),
            iface.string("gtk-theme").to_string(),
            iface.string("icon-theme").to_string(),
        ];
        let gtk4 = crate::paths::gtk4_css_path();
        let gtk3 = crate::paths::gtk3_css_path();
        let css_existed = (gtk4.is_file(), gtk3.is_file());

        // Theme under test: the CURRENT wallpaper, dark, glass subtle on.
        let uri = snapshot[0].clone();
        let wallpaper = gio::File::for_uri(&uri)
            .path()
            .expect("current wallpaper must resolve to a file");
        assert!(wallpaper.is_file(), "wallpaper file must exist");
        let theme = Theme {
            name: "Live Cycle Probe".to_string(),
            mode: ColorMode::Dark,
            wallpaper,
            palette_edited: false,
            glass: GlassSettings {
                enabled: true,
                strength: crate::theme::model::GlassStrength::Subtle,
            },
            icons: IconsSettings::default(),
            palette: live_palette(),
        };

        let started = Instant::now();
        apply_theme(&theme).expect("first apply succeeds");
        eprintln!(
            "live cycle: first apply took {:.3}s (one-time setup: baseline + flatpak)",
            started.elapsed().as_secs_f32()
        );

        // Scenario 14 measures the steady-state job: everything one-time
        // (baseline capture, flatpak overrides) is already done, mirroring
        // the spec's "after extraction has been run once" budget.
        let started = Instant::now();
        apply_theme(&theme).expect("apply succeeds");
        let elapsed = started.elapsed();
        eprintln!("live cycle: apply took {:.3}s", elapsed.as_secs_f32());
        assert!(elapsed.as_secs_f32() < 1.0, "scenario 14: <1 s");

        // Scenario 4/5: URIs, options, scheme, CSS contents, applied state.
        assert_eq!(bg.string("picture-uri").to_string(), uri);
        assert_eq!(bg.string("picture-uri-dark").to_string(), uri);
        assert_eq!(bg.string("picture-options").to_string(), "zoom");
        assert_eq!(iface.string("color-scheme").to_string(), "prefer-dark");
        assert_eq!(iface.string("gtk-theme").to_string(), "adw-gtk3-dark");
        for css in [&gtk4, &gtk3] {
            let body = std::fs::read_to_string(css).expect("css written");
            assert!(body.lines().next().unwrap() == crate::paths::GTK_CSS_MARKER);
            assert!(body.contains("#c5c0ff"));
            assert!(body.contains("atmosphere_glass_surface"));
        }
        assert_eq!(
            model::applied_theme_id().as_deref(),
            Some("live-cycle-probe")
        );
        // Scenario 12: a real installed Yaru variant (or unchanged when off).
        assert!(super::super::icons::icon_theme_installed(
            &iface.string("icon-theme").to_string()
        ));
        // Shell overlay only when User Themes is enabled.
        if shell_check::user_themes_enabled() {
            let shell = crate::paths::shell_theme_css_path();
            let body = std::fs::read_to_string(&shell).expect("shell css written");
            assert!(body.contains("@import"));
            assert!(body.contains("rgba("));
        }

        // Scenario 11 + 9: restore returns every key and file.
        assert_eq!(
            restore_baseline().expect("restore"),
            RestoreOutcome::Baseline
        );
        let after = [
            bg.string("picture-uri").to_string(),
            bg.string("picture-uri-dark").to_string(),
            bg.string("picture-options").to_string(),
            iface.string("color-scheme").to_string(),
            iface.string("gtk-theme").to_string(),
            iface.string("icon-theme").to_string(),
        ];
        assert_eq!(snapshot, after, "desktop must match pre-apply state");
        eprintln!("live cycle: post-restore keys = {after:?}");
        assert_eq!((gtk4.is_file(), gtk3.is_file()), css_existed);
        assert_eq!(model::applied_theme_id(), None);
    }
}
