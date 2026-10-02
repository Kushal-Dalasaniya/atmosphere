<!--
Sync Impact Report
- Version: 1.1.0 → 1.2.0
- Added: Principle VIII (Security release gate); icon guidance in VI
- Spec: §4.12 two-tone Yaru icons; §8 security
- TODOs: add SECURITY.md at implementation phase
-->

# Atmosphere Constitution

## Core Principles

### I. Spec-first delivery

The active feature contract lives in `specs/001-atmosphere-auto-themer/spec.md`. No implementation work may contradict that spec. When the spec and code disagree, fix the code or amend the spec deliberately—never “fix in code only.”

New behavior starts in spec (or a new numbered spec under `specs/`), then plan, then tasks, then code. Agents and contributors MUST NOT add features listed under “v1 does not include” or “Later, not now” without a spec amendment.

**Rationale:** Atmosphere touches the user’s desktop; ambiguous requirements cause broken themes and lost trust.

### II. Platform floor (non-negotiable)

Atmosphere targets **Ubuntu 26.04 LTS and later**, **GNOME 50 and later**, **Wayland** sessions only.

- MUST NOT add support, shims, or tests for Ubuntu &lt; 26 or GNOME &lt; 50.
- MUST NOT support X11 sessions.
- MUST remain runnable on newer GNOME releases without artificial version caps.

Reference QA machine: Ubuntu 26.04 with GNOME 50.

**Rationale:** Shell CSS, libadwaita, and gsettings behavior are version-sensitive; a single floor keeps the product maintainable.

### III. Independence & minimal coupling

Atmosphere MUST be as self-contained as practical for end users and implementers.

- **Only GNOME extension Atmosphere depends on for full shell theming:** User Themes (`user-theme@gnome-shell-extensions.gcampax.github.com`). If it is missing or disabled, the app MUST degrade gracefully (banner + skip shell CSS) and MUST still apply wallpaper and GTK colors.
- MUST NOT install, configure, read, or write settings for **Blur my Shell** or any other blur/theming extension. Users may run those tools separately for overview, lock screen, or app grid; Atmosphere MUST NOT integrate with them.
- MUST NOT read or write `~/.config/matugen/`. Palette extraction uses Atmosphere’s private config path (per spec) or in-process extraction if the spec is updated; the user’s matugen setup stays untouched.
- Atmosphere owns generated files it writes (`gtk.css`, shell overlay, theme data) and MUST back up or preserve user files per spec before replacing them.

**Rationale:** Third-party extensions and dotfile tools change often; coupling creates support burden and “dirty” visuals (e.g. application blur).

### IV. User safety & honesty

- Apply MUST NOT require `sudo`. All changes are per-user under `$HOME` and standard gsettings keys.
- Apply MUST be sequential, one job at a time; the GTK main loop MUST NOT block on long work (use `gio::spawn_blocking` or equivalent).
- On failure after backups exist, restore Atmosphere-owned outputs when the spec requires it; show one clear toast, not a stack trace. The session MUST remain usable (logout, Settings, launching apps).
- UI MUST be honest about limits: already-open GTK 4 apps may not pick up new `gtk.css`; glass is shell tint/translucency, not macOS-style blur behind every window.

**Rationale:** Users theme their daily driver; surprises and irreversible damage are unacceptable.

### VII. Reversible desktop (non-negotiable)

- Before the **first successful Apply**, capture a **baseline snapshot** of every gsettings key and CSS file Atmosphere will modify (spec §4.11).
- **Failed Apply** MUST roll back to the state immediately before that Apply started.
- **Restore Ubuntu defaults** MUST restore the baseline so wallpaper, GTK theme, color-scheme, and shell user-theme behave like **before Atmosphere was ever used** (typical Ubuntu/Yaru behavior). Saved themes in `~/.local/share/atmosphere/themes/` MAY remain on disk; only desktop state is reverted.
- Turning **glass off** and re-applying a theme is NOT a full system revert; only Restore (or failed-apply rollback) returns to baseline.
- MUST NOT leave the shell stuck on a broken `Atmosphere` user-theme or invalid `gtk-theme` after restore or rollback.

**Rationale:** Users will only trust the app if disable/restore truly returns to “normal Ubuntu,” and errors never brick the session.

### V. Simplicity & scope discipline

- Prefer the smallest correct change. Match existing module boundaries in the spec (`theme/`, `ui/`, etc.).
- YAGNI: no plugin systems, cloud sync, wallpaper editors, or “while we’re here” app templates unless a spec says so.
- **Glass** is optional per theme (default **off**): user toggle on/off; when off, no translucent shell rules. When on, only Atmosphere shell CSS (`subtle` / `strong`)—no compositor blur pipelines in v1.
- Source of truth for a saved look is **`theme.toml`** under `~/.local/share/atmosphere/themes/`, not live matugen output or a wallpaper folder alone.

**Rationale:** Omarchy/Aether inspire the product; Atmosphere v1 is a focused GNOME theme switcher and creator, not a full desktop environment.

### VI. Native GNOME craft

- UI: **Rust**, **GTK 4**, **libadwaita**—one main window, **Themes** and **Create** views.
- Shell styling: import Ubuntu **Yaru** (or documented fallback), then override; never ship a trivial stylesheet as the entire shell theme.
- **Icons:** preserve Ubuntu’s **two-tone Yaru folder** look; match accent to the nearest installed Yaru icon variant via `icon-theme` (spec §4.12). MUST NOT switch users to flat monochrome icon packs as the default behavior.
- Packaging (future `.deb`): runtime `Depends` on GTK/libadwaita and theme assets; end users MUST NOT need `-dev` packages or Rust to install a release build.

**Rationale:** The app should feel like part of Ubuntu GNOME, not a cross-desktop experiment.

### VIII. Security release gate (highest priority)

- Treat security as a **release blocker** (spec §8). Ship with **no known Critical/High** `cargo audit` findings in dependencies, or documented waivers in `SECURITY.md`.
- MUST NOT execute user-controlled shell, load executable hooks from theme files, or write outside the user’s home for normal operations.
- MUST validate hex colors and paths before writing CSS/TOML; MUST NOT allow path traversal or symlink escape on wallpaper import.
- MUST use rollback/baseline (Principle VII) so security or logic failures do not brick GNOME Shell.

**Rationale:** A theming app that runs arbitrary commands or leaves the session broken is worse than no app at all.

## Platform & Dependencies

| Area | Rule |
| --- | --- |
| Language / UI | Rust (edition per spec), gtk4-rs, libadwaita for GNOME 50+ APIs |
| Inspiration | Omarchy-style **switch** + Aether-style **create**; behavior defined in spec, not by copying Hyprland/Omarchy internals |
| Extensions | User Themes: optional for shell; detect and banner only |
| Blur extensions | Out of scope for Atmosphere code |
| GTK 3 apps | `adw-gtk3` / `adw-gtk3-dark` via gsettings when installed; document meson install on Ubuntu 26 if not in apt |
| Dev packages (Ubuntu 26) | Use `libgdk-pixbuf-2.0-dev`, not obsolete `libgdk-pixbuf2.0-dev`; see spec §6 |

## Spec & Delivery Workflow

1. **Constitution** (this file) — principles and gates for all Spec Kit commands.
2. **Feature spec** — `specs/001-atmosphere-auto-themer/spec.md` — acceptance criteria and pipelines.
3. **Plan / tasks** — produced under the feature directory via `/speckit-plan` and `/speckit-tasks`.
4. **Implement** — `/speckit-implement` only after spec and tasks align with constitution checks.
5. **Analyze / converge** — use `/speckit-analyze` and `/speckit-converge` when spec, plan, tasks, or code drift.

Constitution checks in plans MUST explicitly verify: platform floor, extension independence, glass opt-in, user safety, and spec non-goals.

## Governance

- This constitution supersedes informal chat and agent defaults when they conflict.
- Feature details and acceptance tests live in the numbered spec; amending behavior requires updating that spec and, if principles change, this constitution.
- **Amendments:** edit `.specify/memory/constitution.md`, bump **Version** (semver: MAJOR = principle removal/redefinition; MINOR = new principle or material expansion; PATCH = clarity only), set **Last Amended** to the change date, and note the change in the sync impact comment at the top until commit.
- **Compliance:** Every implementation PR or agent session SHOULD be checked against sections I–VI and the active feature spec before merge.
- **Ratification:** treat version 1.0.0 as the baseline for the Atmosphere project.

**Version**: 1.2.0 | **Ratified**: 2026-10-02 | **Last Amended**: 2026-10-02
