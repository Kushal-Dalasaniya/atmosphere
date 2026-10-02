# Quickstart: validate Atmosphere v1 end-to-end (001)

**Spec**: `spec.md` §7 · **Plan**: `plan.md` · **Contracts**: `contracts/` · **Model**: `data-model.md`

Run on the reference machine: **Ubuntu 26.04, GNOME 50, Wayland**. Do not attempt on Ubuntu < 26.

## Prerequisites

```bash
rustc --version            # 1.85+
pkg-config --modversion gtk4
pkg-config --modversion libadwaita-1
echo "$XDG_SESSION_TYPE"   # wayland
gnome-shell --version      # 50+
lsb_release -rs            # 26.04+
gsettings --version
```

Install dev packages (note `libgdk-pixbuf-2.0-dev`, not the old `2.0` infix):

```bash
sudo apt update && sudo apt install -y build-essential pkg-config git curl \
  libgtk-4-dev libadwaita-1-dev libglib2.0-dev libcairo2-dev libpango1.0-dev \
  libgdk-pixbuf-2.0-dev libgraphene-1.0-dev \
  gsettings-desktop-schemas gnome-shell gnome-shell-extensions adw-gtk3-theme
```

User Themes extension (for shell steps; app must also pass with it disabled):

```bash
gnome-extensions enable user-theme@gnome-shell-extensions.gcampax.github.com
# then log out and back in
```

## Run

```bash
cargo fmt --check && cargo clippy -- -D warnings
cargo test
cargo run
```

Put 2–3 photos in `~/Pictures/Wallpapers`. Reference inputs for reproducible QA:

- One 3840×2160 JPEG photo (timing reference for scenario 14; one copy with a space in the filename for scenario 9).
- One light busy wallpaper + one dark busy wallpaper (readability QA for scenario 10).
- One `.webp` if available (scenario 2 notes webp handling).

## Validation scenarios (expected outcomes)

1. **Empty state**: fresh `~/.local/share/atmosphere/` → Themes shows explainer + open-Create, no crash.
2. **Extract isolation**: Create → select wallpaper → Extract fills 5 swatches; `gsettings get org.gnome.desktop.background picture-uri` unchanged; `~/.config/matugen/` untouched (or absent).
3. **Edit survival**: change Accent → Save → reopen `theme.toml`: edited hex present, `palette_edited=true`; re-apply uses it (check `gtk.css` contains it).
4. **Apply dark**: Save and apply (dark) → both `picture-uri`/`picture-uri-dark` = same percent-encoded `file://` URI; `picture-options` = `zoom`; `color-scheme` = `prefer-dark`; `gtk-theme` = `adw-gtk3-dark`; both `gtk.css` have marker + theme hexes; atomic (no half-written file); toast "Applied. Apps you open from now on use this theme."
5. **Apply light**: same with `prefer-light` / `adw-gtk3`; light path contains no hardcoded `dark`.
6. **User-Themes-off**: `gnome-extensions disable …` → restart app → banner visible → apply succeeds, no `user-theme` key written.
7. **User-Themes-on**: enable + relogin → apply → `gnome-shell.css` starts with real Yaru `@import`, `user-theme name` = `Atmosphere` (via `""`→`"Atmosphere"` toggle).
8. **Concurrency**: double-click a card fast → second apply ignored until first finishes; window stays responsive.
9. **Failure rollback**: theme with deleted wallpaper → apply fails with one-sentence toast, CSS/gsettings match pre-apply state, not marked applied. Paths with spaces apply cleanly.
10. **Glass**: new theme defaults `enabled=false`; Switch off→on defaults Subtle; off-apply leaves zero `rgba` panel rules (clears prior translucency); on-subtle/strong apply writes `rgba()` panel tint; `gnome-shell` extension keys untouched; panel/notification text meets **4.5:1 contrast** on both reference busy wallpapers (spec §4.10).
11. **Baseline + Restore**: note wallpaper/gtk-theme/color-scheme/user-theme → first Apply creates `baseline/` (7 keys + CSS copies/markers + README) → **Restore Ubuntu defaults** → desktop matches noted state, `applied.toml` gone, shell theme deselected, saved themes still on disk.
12. **Icons**: `match_yaru=true` → `icon-theme` becomes nearest installed Yaru variant (dark mode prefers `-dark`); Files shows two-tone folders; `false` → `icon-theme` untouched.
13. **Security probes** (must all pass): malicious `theme.toml` accent (`#000"; url(javascript:1)`) rejected, never verbatim in CSS; symlink wallpaper outside home rejected; `cargo audit` shows zero Critical/High (or CVE waiver in `SECURITY.md`).
14. **Apply timing**: apply the reference 3840×2160 JPEG twice; the second (post-first-extraction) background job finishes in **<1 s** per the info-level duration log (shell animation time excluded), per spec §2.

Details for each outcome live in `contracts/system-effects.md`, `contracts/theme-toml.md`, `contracts/ui.md`, and `data-model.md` — this guide does not duplicate them.
