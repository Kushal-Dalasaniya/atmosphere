# Research: Atmosphere auto-themer (001)

**Date**: 2026-10-02 | **Spec**: `specs/001-atmosphere-auto-themer/spec.md` | **Plan**: `plan.md`
**Method**: Codebase inspection (`src/`, `Cargo.toml`, `Cargo.lock`), spec §§4–8, constitution v1.2.0. No network calls; crate API claims below are verified against the pinned `Cargo.lock` versions during implementation (`cargo metadata` / build).

## R-1. Palette extractor: matugen CLI vs in-process `material-colors`

- **Decision**: Keep the existing in-process extractor (`material-colors 0.4`, already in `Cargo.toml`/`Cargo.lock`, `src/theme/extract.rs`) as the v1 extraction path. Do NOT shell out to `matugen` in v1. File a spec amendment to §4.4 replacing the `matugen image … --mode/--json/--dry-run/-c` command shape with the in-process mapping (same role table), OR gate the matugen path behind an explicit spec change. Until the amendment lands, `cargo run` on a machine without `matugen` installed must still pass acceptance (spec §7 "Extract fills the five swatches" without changing the desktop).
- **Rationale**: Code already extracts via `CorePalette::ofArgb → Scheme::dark/light → Theme` and maps `surface / surface_container_low / on_surface / primary / on_primary (+error/secondary/tertiary)` exactly per the §4.4 role table; zero external binary, zero `~/.config/matugen/` touch risk, zero `--dry-run`/`-c`/hook footguns, UI never blocks either way. The spec's *intent* ("matugen is a palette extractor only; Atmosphere renders every config itself; never read/write `~/.config/matugen/`") is satisfied more strongly in-process.
- **Alternatives considered**: (a) Shell out to `matugen image <ABS> --mode <dark|light> --json hex --dry-run -c <private>` per §4.4 verbatim — rejected for v1 because it reintroduces a missing-binary failure mode ("matugen is not installed" toast), private-config key drift (`[config.wallpaper] set=false` vs legacy `set_wallpaper`), and stdout-JSON schema drift, with no color-quality gain for 5 swatches. (b) Hybrid (try matugen, fall back in-process) — rejected: two extractors = two palettes for one wallpaper, breaks "reuse saved palette / don't re-extract on apply" determinism.
- **Follow-up**: Spec amendment required by Constitution I before release (constitution already anticipates this: "or in-process extraction if the spec is updated"). Implementation must also fix the current weak source-color (flat average in `extract.rs`) to a quantized/dominant-color pick so busy photos don't wash out — same crate, no new dependency.

## R-2. GTK/libadwaita feature flags: `gnome_50` floor

- **Decision**: Set `gtk4 0.11` and `libadwaita 0.9` features to `["gnome_50"]` (minimum API level per spec §6.1; implies the matching `v4_2x`/`v1_x` APIs). Remove the current `v4_22` / `v1_9`+`gtk_v4_22` pins unless `cargo metadata` shows `gnome_50` does not imply them. Never enable `gnome_4x` older flags. Verify with `cargo build` on the reference stack; newer GNOME keeps building because `gnome_50` is a floor, not a cap.
- **Rationale**: Spec + constitution II require GNOME 50 minimum, runnable on 51/52+. `gnome_50` expresses exactly that; raw `v4_22` pins the GTK version instead of the platform floor and invites "works on my GTK, breaks on 50" drift.
- **Alternatives considered**: Keep `v4_22` — rejected (violates spec §6.1 letter). Use newer `gnome_5x` as minimum — rejected (would refuse the reference machine Ubuntu 26.04/GNOME 50).

## R-3. Two-tone Yaru icon matching (`icons.match_yaru`, default on)

- **Decision**: New `src/theme/icons.rs`: parse theme `accent` → sRGB → hue (standard `rgb→hsl`, achromatic accent → keep current/`Yaru`); compare against a static table of installed-candidate Yaru variant accent hues (`Yaru, Yaru-blue, Yaru-magenta, Yaru-purple, Yaru-red, Yaru-sage, Yaru-olive, Yaru-prussiangreen, Yaru-wartybrown, Yaru-yellow` + `-dark` counterparts); pick minimum circular hue distance among variants **actually installed** (probe `/usr/share/icons/<name>`); `mode==dark` prefers `-dark` names when present, `light` prefers non-dark; if only `Yaru`/`Yaru-dark` exists, use it; `match_yaru=false` → don't touch `org.gnome.desktop.interface icon-theme`; validate the chosen name exists before `gsettings set`; baseline restores `icon-theme`.
- **Rationale**: Spec §4.12 + constitution VI: dual-shade folders preserved, no SVG generation, no flat packs, host-only `icon-theme` write (Flatpak apps that ignore it are documented non-goals).
- **Alternatives considered**: Ship recolored SVGs / Papirus switch — rejected (explicit non-goal). Euclidean RGB distance — rejected (hue distance matches "closest on the color wheel" wording).

## R-4. gsettings + wallpaper patterns (correctness fixes)

- **Decision**: Keep `gio::Settings` (no `sh -c`, no user-controlled args). Fixes required: (1) wallpaper URI via `gio::File::for_path → file.uri()` (already in `wallpaper.rs` — preserves percent-encoding for spaces/non-ASCII; never hand-build `file://<PATH>`); set **both** `picture-uri` and `picture-uri-dark` + `picture-options=zoom`. (2) `color-scheme` + `gtk-theme` per mode table (`prefer-dark`/`adw-gtk3-dark`, `prefer-light`/`adw-gtk3`); force GTK3 reload via `Adwaita`→target toggle (already implemented). (3) Never write `accent-color` (fixed enum, not hex). (4) Shell reload via `user-theme name "" → "Atmosphere"` toggle (already implemented); only when `user_themes_enabled()`. (5) Startup check `gnome-extensions info user-theme@…` with the **constant** id from `paths.rs`; banner dismissible per-session; missing/disabled → skip shell writes, continue apply.
- **Rationale**: Every one of these is a spec §§4.3/4.5/4.6/3.3 trap already half-handled in code; making them explicit prevents regression.
- **Alternatives considered**: `std::process::Command gsettings …` with path args — rejected (spec §4.3 mandates `gio`/gsettings API; subprocess invites quoting bugs on spaces).

## R-5. Atomicity, backups, baseline, restore (biggest gap)

- **Decision**: New `src/theme/baseline.rs` + extend `apply.rs` to the §4.2 sequential order: (1) read+validate `theme.toml`, refuse on missing wallpaper **before touching desktop**; (2) per-apply `*.atmosphere.bak` copies next to each owned file; (2b) first-ever apply captures `baseline/` (`baseline.toml` with 7 keys: `picture-uri, picture-uri-dark, picture-options, color-scheme, gtk-theme, icon-theme, user-theme-name` + `gtk-3.0.css`/`gtk-4.0.css` copies or `absent` markers + `README`) — capture **once**, never overwrite; (3) wallpaper first; (4) reuse saved palette (no re-extract unless `palette_edited=false` + explicit regenerate); (5) render to temp-in-same-dir → `rename(2)` per file (extend current `gtk.css` pattern to shell CSS + Alacritty + state); (6–9) gsettings → shell → Alacritty → glass-merge; (10) any failure after step 2 → restore pre-apply backups, keep new wallpaper only if set succeeded AND colors restored, one-sentence toast naming wallpaper-vs-colors outcome; (11) success → write `applied.toml`, never touch baseline. `Restore Ubuntu defaults` reads baseline only (fallback to documented Yaru/`default`/cleared user-theme + `gtk.css.user-saved` with warning when baseline missing), removes Atmosphere-owned `gtk.css` when pre-Atmosphere state was absent, clears `applied.toml`, leaves themes + Blur-my-Shell + Flatpak overrides alone. Single-apply mutex (`std::sync::Mutex`/`RefCell` guard + spinner + ignore clicks) around the whole job.
- **Rationale**: Current `apply.rs` does wallpaper→CSS→gsettings→shell with no backups, no baseline, no mutex, no Alacritty, no icons — i.e. exactly the "half-broken session" spec §4.11 forbids. The order matters: the old "concurrent" draft raced backup-vs-write.
- **Alternatives considered**: Concurrent steps / per-key gsettings backups only — rejected (spec mandates sequential + file backups). Full-system snapshot — rejected (only owned keys/files per §4.11 table).

## R-6. Glass math + GNOME 50 shell selectors

- **Decision**: Keep `src/theme/glass.rs` shape (`enabled=false` default; `subtle α≈0.82`, `strong α≈0.68` within spec bands 0.75–0.88 / 0.60–0.75); tint from `surface` (fallback `background`), text from `foreground`; emit translucent `rgba()` panel/calendar/message-list/notification rules **only when enabled**, else opaque `#panel` block that overwrites stale translucency; shell CSS = `@import` first-existing Yaru-dark/Yaru/generic + short overrides (current `shell.rs` already does this); never a 4-line standalone theme; skip-but-warn when no import path exists; dock selectors best-effort, failures skipped silently; readability QA on busy light+dark wallpapers.
- **Rationale**: Matches §§4.6/4.10 + "clean, not dirty glass" rules; current code is close but needs the off-path overwrite guarantee and band documentation.
- **Alternatives considered**: Compositor blur / Mutter pipelines / Blur-my-Shell driving — rejected (explicit non-goals, constitution III).

## R-7. Alacritty (`atmosphere-colors.toml` + import merge)

- **Decision**: New `src/theme/alacritty.rs`: skip entirely when `alacritty` not on `PATH`; else write `~/.config/alacritty/atmosphere-colors.toml` from palette (`primary/background/foreground` mapping), then ensure `alacritty.toml` contains exactly `live_config_reload = true` and `import = ["~/.config/alacritty/atmosphere-colors.toml"]` — append-merge only when the import line is absent, never rewrite user content; create minimal two-line file when none exists. Never write legacy `colors.toml` alone (Alacritty ignores it).
- **Rationale**: Spec §4.7 verbatim; current codebase has zero Alacritty handling.
- **Alternatives considered**: Full config templating — rejected (would clobber user keybindings/fonts).

## R-8. Security validation (release gate)

- **Decision**: Central validators (in `model.rs` + renderers): palette fields must match `^#[0-9a-fA-F]{6}$` before any CSS/TOML interpolation (reject `{ } url( javascript` newlines — the §8.4 `accent = "#000"; url(javascript:1)"` probe must not appear verbatim in output); theme id must match `^[a-z0-9][a-z0-9._+-]*$`, reject `..` segments (already enforced in `theme_id_from_name`, extend to load paths); wallpaper paths canonicalized (`std::fs::canonicalize`), must be readable regular files under `$HOME` (allowlist: `~/Pictures/Wallpapers` + already-saved theme paths), symlink-escape rejected on Add-wallpaper copy (copy regular files only, `same-file`/metadata check); subprocesses fixed-arg only (`gnome-extensions info <const>`, `flatpak override …` constants); no `sh -c`/`bash`/`eval`/Lua/`Exec=` hooks; no writes outside `$HOME`; no `unsafe`, no `unwrap()` on Apply/Restore I/O (use `Result` + rollback); pin `Cargo.lock`, `cargo audit` clean for Critical/High or CVE waiver in `SECURITY.md`.
- **Rationale**: Spec §8 + constitution VIII are release blockers; theming app that injects CSS or bricks Shell is worse than none.
- **Alternatives considered**: Allowlist-free paths / shell-string gsettings — rejected (breaks spaces + injection).

## R-9. Thumbnails, webp, threading

- **Decision**: Keep `image 0.25` + `thumbs.rs` cache-by-(filename+mtime) design; move decode+resize off the main thread (`gio::spawn_blocking`, longest side 256px — fix current `resize(256,256)` square distortion to `thumbnail(256,256)` aspect-preserving); broken images skipped with "could not read" note; `.webp` shown only when gdk-pixbuf decodes it, and if `gsettings` rejects the URI at apply time, report + don't mark applied. Create-view swatches (Background/Surface/Foreground/Accent/On-accent) are `Entry`-validated hex + color preview; reset restores last extraction; dark/light switch re-extracts for that mode (never invert); drag-and-drop selects wallpaper; Add-wallpaper copies into `~/Pictures/Wallpapers` with numeric suffix, never overwrite.
- **Rationale**: Spec §§5.3/3.2 + performance goals; current `thumbs.rs` + `create_view.rs` lack swatch editors, reset, DnD, chooser-copy, and off-thread decode.
- **Alternatives considered**: Full image editor (crop/grain/filters) — rejected (non-goal).

## R-10. Flatpak overrides idempotence

- **Decision**: Keep `flatpak.rs` once-per-user semantics but replace the local flag-file-only check with: if `flatpak` missing → skip; else query `flatpak override --user --show` (fixed args) and run the two `--filesystem=…:ro` overrides only when absent; write the flag file after. Never set global `GTK_THEME`. Document that already-open Flatpak apps don't recolor.
- **Rationale**: Spec §4.8 + current code gap (flag file can desync from real overrides if user resets them).
- **Alternatives considered**: Run on every apply — rejected (wasteful, spec says once).

## R-11. Honest UI limits + TK deltas

- **Decision**: Success toast `"Applied. Apps you open from now on use this theme."`; failure toasts one sentence, no stack traces; already-open GTK4 note in UI; `adw-gtk3-theme` presence checked once at startup with warning (CSS still written); Cards show glass on/off, Applied state, spinner+ignore during job, rename/delete (delete-applied allowed, files stay until next apply, never delete source wallpaper); empty library → explainer + open-Create; Restore control with confirmation dialog. Fix `Cargo.toml` dev-package doc: `libgdk-pixbuf-2.0-dev` (constitution table) not `libgdk-pixbuf2.0-dev`.
- **Rationale**: Spec §§3–4 + acceptance list; closes doc drift that breaks `apt install` on Ubuntu 26.
