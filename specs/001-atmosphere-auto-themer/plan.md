# Implementation Plan: Atmosphere auto-themer (001)

**Branch**: `001-atmosphere-auto-themer` | **Date**: 2026-10-02 | **Spec**: `specs/001-atmosphere-auto-themer/spec.md`

**Input**: Feature specification from `/specs/001-atmosphere-auto-themer/spec.md`

## Summary

Build the v1 GNOME theme switcher + creator on Ubuntu 26.04+/GNOME 50+/Wayland: a Rust GTK4/libadwaita app with **Themes** (card library, one-click apply, rename/delete, glass label, restore defaults) and **Create** (wallpaper strip, extract palette, edit 5 swatches, dark/light, glass on/off + subtle/strong, save / save-and-apply) views. Apply pipeline is strictly sequential (wallpaper → palette reuse → atomic CSS render → gsettings → optional shell overlay → optional Alacritty → glass merge), with per-apply `*.atmosphere.bak` rollback, one-time `baseline/` snapshot, `applied.toml` state, two-tone Yaru icon matching, Flatpak overrides once, and security hardening (hex/path/id validation, no user-controlled shell, `cargo audit` gate). Research resolves the matugen-vs-in-process extractor divergence, `gnome_50` feature flags, Yaru matching, gsettings/shell/rollback patterns, and validation rules.

## Technical Context

**Language/Version**: Rust stable 1.85+ (current toolchain 1.99.0), Edition 2024

**Primary Dependencies**: `gtk4 0.11`, `libadwaita 0.9` (both must use `gnome_50` minimum feature — see research R-2; current `Cargo.toml` uses `v4_22`/`v1_9`+`gtk_v4_22` and must be corrected), `glib 0.22`, `gio 0.22`, `serde 1 + derive`, `toml 0.8`, `serde_json 1`, `image 0.25`, `material-colors 0.4` (in-process extractor; matugen decision in R-1), `anyhow 1`, `dirs 6`, `regex 1`. No Tokio in v1 (optional, off main thread only).

**Storage**: Files only, no DB. `~/.local/share/atmosphere/themes/<id>/{theme.toml,preview.png}`, `~/.local/share/atmosphere/{applied.toml,baseline/{baseline.toml,gtk-3.0.css,gtk-4.0.css,README},flatpak-overrides.done}`, `~/.cache/atmosphere/thumbs/`, `~/.themes/Atmosphere/gnome-shell/gnome-shell.css`, `~/.config/gtk-{4.0,3.0}/gtk.css` (+ `gtk.css.user-saved` one-time backup), `~/.config/alacritty/{atmosphere-colors.toml,alacritty.toml}` import. All writes atomic via temp-in-same-dir + rename; `mkdir -p` before write.

**Testing**: `cargo test` (unit: model validation, glass math, icon matching, CSS rendering, atomic write; integration: apply-order with mocked gsettings/fs), `cargo clippy` + `cargo fmt --check` clean (acceptance gate), `cargo audit` zero Critical/High (release gate), manual QA on Ubuntu 26.04 Wayland GNOME 50 per spec §7 (wallpaper URIs, gtk-theme flips, shell import+reload, glass off/on, baseline/restore, icon match, malicious-input tests in §8.4).

**Target Platform**: Ubuntu 26.04 LTS and later, GNOME 50 and later, Wayland session only. No Ubuntu < 26, no GNOME < 50, no X11. Reference QA: Ubuntu 26.04 + GNOME 50; must keep running on newer GNOME without version caps.

**Project Type**: Desktop app — single binary (`src/main.rs`, Adw application `com.atmosphere.App`).

**Performance Goals**: Wallpaper visible immediately on apply (set before extraction/render); full background job < 1 s after first extraction on a normal photo (shell animation excluded); UI never blocks — `gio::spawn_blocking` + `glib::idle_add`/`spawn_future_local`, one apply mutex, spinner + ignore extra clicks; thumbnails 256px longest side, decoded off main thread, cached by path+mtime.

**Constraints**: Sequential apply only (no concurrent applies); never block GTK main loop; no `sudo`/polkit/setuid; no writes outside `$HOME` + documented XDG paths; no Blur-my-Shell coupling (never read/write `/org/gnome/shell/extensions/blur-my-shell/`); glass default off, off-path emits zero translucent rules and clears stale ones; User Themes missing → skip shell steps, banner, continue; light path must not hardcode `-m dark`/`adw-gtk3-dark`; accent never written to `accent-color` gsettings enum key.

**Scale/Scope**: Single local user; ~10–50 saved themes, dozens of wallpapers in `~/Pictures/Wallpapers` (jpg/jpeg/png/webp); one window, two views; ~12 source modules per spec §5.4.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Derived from `.specify/memory/constitution.md` v1.2.0:

| # | Principle / Gate | Status | Evidence / Plan |
|---|---|---|---|
| I | Spec-first; no "v1 does not include / Later, not now" without spec amendment | PASS | Plan implements only spec §§3–8; non-goals (§1 list, VS Code/Kitty/btop, icon-pack building, wallpaper editor, Blur driving, Mutter blur, frosted GTK bars, matugen config replacement) explicitly excluded in contracts + quickstart scope |
| II | Platform floor: Ubuntu 26+/GNOME 50+/Wayland; no shims/tests for older; no X11; runnable on newer GNOME | PASS | `gnome_50` minimum feature; runtime Yaru path search adapts to newer GNOME; QA only on 26.04/GNOME 50; no `gnome_4x` flags, no X11 branches |
| III | Independence: only User Themes coupling, graceful degrade; no Blur-my-Shell R/W; no `~/.config/matugen/` R/W; own generated files with backup | PASS | `shell_check` banner + skip; glass in own CSS only; in-process extraction with no extractor config file (R-1); `gtk.css.user-saved` + `*.atmosphere.bak` + baseline ownership |
| IV | Safety & honesty: no sudo, sequential + non-blocking, clear toast + usable session on failure, honest limits | PASS | `gio::spawn_blocking`, apply mutex, one-sentence toasts, pre-apply backup rollback, UI copy for open-app limits |
| VII | Reversible desktop: baseline once, failed-apply rollback, Restore-to-baseline, glass-off ≠ restore, never stuck on broken theme | PASS | `baseline/` capture, per-resource temp→rename, restore clears `applied.toml` + user-theme name, fallback Yaru/defaults when baseline missing |
| V | Simplicity/scope: smallest change, YAGNI, glass opt-in default off, `theme.toml` source of truth | PASS | Module layout = spec §5.4; no plugin/cloud/editor systems; glass struct `enabled=false` default; palette from saved theme, no re-extract on apply |
| VI | Native craft: Rust/GTK4/libadwaita, Yaru import+override shell, two-tone Yaru icons via `icon-theme` only | PASS | Shell CSS `@import` Yaru-dark/Yaru/fallback + short selector overrides; icon matching = nearest installed Yaru variant, dark-preferring; no flat packs, no SVG gen |
| VIII | Security gate (highest): `cargo audit` clean, no user-controlled exec, hex/path validation, no traversal/symlink escape, rollback on failure | PASS | Fixed-command subprocesses only, `#RRGGBB` validation before CSS interpolation, canonicalize + home-allowlist + symlink-escape reject, `^[a-z0-9][a-z0-9._+-]*$` ids, no `unwrap()` on Apply/Restore I/O, Flatpak mounts ro-only |

**Gate result: PASS — proceed to Phase 0.** Post-design re-check required after contracts/data-model.

### Post-design re-check (2026-10-02, after Phase 1) — updated: amendment complete

All gates re-evaluated against `research.md`, `data-model.md`, `contracts/`, `quickstart.md`: **PASS, no violations.**

- ~~One outstanding obligation (not a violation): research R-1 keeps the in-process `material-colors` extractor, which requires a deliberate spec amendment to §4.4 before release per Constitution I~~ **RESOLVED 2026-10-02 (tasks T001):** spec §§2, 3.4, 4.2, 4.4, 5.2, 5.4, 6.3, 6.4, 7 are now normatively in-process; remaining `matugen` mentions are prohibitions (`~/.config/matugen/` never touched), non-goals, and the §10 prior-art link. Exactly one extraction path remains.
- New modules (`validate.rs`, `atomic.rs`, `baseline.rs`, `icons.rs`, `alacritty.rs`) stay inside the spec §5.4 `theme/` family — no scope expansion (V).
- Contracts enforce glass default-off, Blur-my-Shell non-touch, baseline-once/restore semantics, and the §8 validation + `cargo audit` gate (III, VII, VIII).

## Project Structure

### Documentation (this feature)

```text
specs/001-atmosphere-auto-themer/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

Single-project desktop app (spec §5.4 + existing `src/`). Delivered plan uses real paths; no Option labels.

```text
atmosphere/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs                 # Adw application entry
│   ├── paths.rs                # XDG/home paths, marker, extension id, baseline/alacritty paths (to extend)
│   ├── thumbs.rs               # wallpaper scan, 256px off-thread thumbs, mtime cache
│   ├── theme/
│   │   ├── mod.rs
│   │   ├── model.rs            # theme.toml read/write + AppliedState, IconsSettings
│   │   ├── validate.rs         # NEW: hex/path/id validators, symlink-escape reject (spec §8)
│   │   ├── atomic.rs           # NEW: temp→rename writes, *.atmosphere.bak backup/restore
│   │   ├── extract.rs          # palette extraction (R-1 decision point)
│   │   ├── apply.rs            # sequential pipeline + mutex + rollback (to extend)
│   │   ├── baseline.rs         # NEW: baseline capture/restore (spec §4.11, §3.5)
│   │   ├── gtk_css.rs          # gtk.css render + ownership + atomic write (to harden)
│   │   ├── shell.rs            # shell CSS import+override + reload toggle (to harden)
│   │   ├── shell_check.rs      # User Themes detect (exists)
│   │   ├── glass.rs            # rgba tint math, subtle/strong (exists, to validate)
│   │   ├── icons.rs            # NEW: accent→Yaru variant matching (spec §4.12)
│   │   ├── alacritty.rs        # NEW: colors.toml + import merge (spec §4.7)
│   │   ├── wallpaper.rs        # GFile URI + gsettings (exists, to harden)
│   │   └── flatpak.rs          # once-per-user overrides (exists, to check idempotence)
│   └── ui/
│       ├── mod.rs
│       ├── window.rs           # headerbar Themes/Create switch, banner, toasts
│       ├── themes_view.rs      # cards, spinner/mutex, rename/delete, restore control
│       └── create_view.rs      # wallpaper strip, extract, 5 swatches, mode, glass switch, DnD
├── assets/                     # icons only; no matugen template pack
└── specs/001-atmosphere-auto-themer/
    ├── spec.md
    ├── plan.md
    ├── research.md
    ├── data-model.md
    ├── quickstart.md
    └── contracts/
```

**Structure Decision**: Single binary desktop-app layout. Keep existing module boundaries; add five shared modules (`validate.rs`, `atomic.rs`, `baseline.rs`, `icons.rs`, `alacritty.rs`) exactly where spec §5.4 anticipates them (`theme/` family). Ship templates as Rust functions/included strings; never copy a matugen template pack.

## Complexity Tracking

> No constitution violations to justify. Table intentionally empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| — | — | — |
