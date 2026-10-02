# Data Model: Atmosphere auto-themer (001)

**Spec**: `spec.md` §§4–5 · **Research**: `research.md` R-1–R-11 · **Plan**: `plan.md`

Source of truth for a saved look is `theme.toml` (spec §2). No database. All paths absolute; all writes atomic (temp-in-same-dir + rename).

## Entities

### 1. Theme (`~/.local/share/atmosphere/themes/<id>/theme.toml`)

`<id>` = name lowercased, spaces → `-`, must match `^[a-z0-9][a-z0-9._+-]*$` (reject `..`, reject silently-stripped chars — refuse in UI). Two themes cannot share an id. `wallpaper` is an absolute path; theme stores the path, never moves the image.

| Field | Type | Rules |
|---|---|---|
| `name` | string | Non-empty, trimmed. Default from wallpaper file stem. Drives `<id>` via `theme_id_from_name`. |
| `mode` | `"dark"` \| `"light"` | Property of the theme. Dark/light toggle in Create re-extracts for that mode, never inverts. |
| `wallpaper` | absolute path string | Must canonicalize to a readable regular file under `$HOME` (allowlist `~/Pictures/Wallpapers` + saved-theme paths). Missing at apply → stop before touching desktop. |
| `palette_edited` | bool, default `false` | Set `true` on any hand-edited swatch. Later applies reuse saved hex; never re-extract when `true`. |
| `glass.enabled` | bool, default `false` | Master on/off. New themes default **off**. Off → zero translucent rules in shell CSS (must overwrite stale translucency). |
| `glass.strength` | `"subtle"` \| `"strong"`, default `"subtle"` | Used only when `enabled=true`. Turning on restores last strength (default Subtle). Legacy `glass = "off"/"subtle"/"strong"` string form reads equivalently. |
| `icons.match_yaru` | bool, default `true` | **NEW — missing in current `model.rs`, must add.** `true` → Apply sets nearest installed Yaru variant; `false` → Apply leaves `icon-theme` untouched. |
| `palette.*` | hex colors | See Palette below. |

Example (spec §5.1 + `icons`):

```toml
name = "Sunset Mountains"
mode = "dark"
wallpaper = "/home/user/Pictures/Wallpapers/sunset.jpg"
palette_edited = false

[glass]
enabled = false
strength = "subtle"

[icons]
match_yaru = true

[palette]
background = "#1b1b1f"
surface = "#201f24"
foreground = "#e4e1e6"
accent = "#c5c0ff"
on_accent = "#2c0091"
```

### 2. Palette (embedded in Theme)

| Role | Source (extraction) | Used for |
|---|---|---|
| `background` | `surface` | `window_bg_color`, glass fallback tint |
| `surface` | `surface_container_low` | `view_bg_color`, panel tint base |
| `foreground` | `on_surface` | `window/view_fg_color`, shell text |
| `accent` | `primary` | `accent_bg_color`, icon-hue match input |
| `on_accent` | `on_primary` | `accent_fg_color` |
| `error` / `secondary` / `tertiary` | same-named scheme slots | Stored for later templates; not shown in Create v1 |

**Validation**: every color must match `^#[0-9a-fA-F]{6}$` before interpolation into CSS/TOML. Reject `{`, `}`, `url(`, `javascript`, newlines. The §8.4 probe (`accent = "#000\"; url(javascript:1)"`) must fail validation and never appear verbatim in `gtk.css`. Hex always emitted with leading `#`; no matugen `{{…}}` braces in output.

**State transitions**: Extract fills all 8 slots → user may edit the 5 visible swatches (`palette_edited=true`) → Reset restores last extraction (`palette_edited=false` only if values equal extraction) → mode switch re-extracts for the new mode (discards unedited values for that mode, keeps `palette_edited` semantics per mode draft) → Save persists.

### 3. Preview (`.../<id>/preview.png`) + thumb cache (`~/.cache/atmosphere/thumbs/`)

256px longest side, decoded off main thread, cached by path+mtime. Broken images skipped with "could not read" note. Glass-on shows tinted overlay on the thumbnail; glass-off shows none. No state beyond cache invalidation.

### 4. AppliedState (`~/.local/share/atmosphere/applied.toml`)

```toml
id = "<theme-id>"
```

Written only on successful apply. Cleared (file removed) by Restore. Deleting the applied theme is allowed — desktop files stay until next apply; no theme shows "Applied" once its id is gone.

### 5. Baseline (`~/.local/share/atmosphere/baseline/` — NEW module)

Captured **once**, immediately before the first successful Apply (if `baseline.toml` absent). Later applies never overwrite it.

| Artifact | Content |
|---|---|
| `baseline.toml` | 7 gsettings values: `picture-uri`, `picture-uri-dark`, `picture-options`, `color-scheme`, `gtk-theme`, `icon-theme`, `user-theme-name` |
| `gtk-3.0.css` / `gtk-4.0.css` | Byte copies of pre-Atmosphere `~/.config/gtk-{3.0,4.0}/gtk.css`, or `absent` marker files when none existed |
| `README` | One line: "Created before first Atmosphere Apply; used by Restore Ubuntu defaults." |

**Restore** reads baseline only (fallback to `Yaru`/`Yaru-dark`, `default`, cleared user-theme + `gtk.css.user-saved` with warning when baseline missing). Restores keys, restores/removes CSS per markers, clears `applied.toml`, deselects `Atmosphere` shell theme. Never deletes saved themes, never touches Blur-my-Shell or Flatpak overrides.

### 6. Owned outputs (generated, never hand-edited)

- `~/.config/gtk-4.0/gtk.css`, `~/.config/gtk-3.0/gtk.css` — line 1 marker `/* atmosphere-owned: do not edit; regenerated when a theme is applied */`; 6 `@define-color` vars minimum + optional `atmosphere_glass_surface rgba(…)` token (see contracts). Pre-existing unmarked files → one-time copy to `gtk.css.user-saved` beside each, then replace, with one user notice.
- `~/.themes/Atmosphere/gnome-shell/gnome-shell.css` — `@import` first-existing Yaru-dark/Yaru/generic + panel/calendar/message-list overrides (+ glass `rgba` merge when enabled).
- `~/.config/alacritty/atmosphere-colors.toml` — only when `alacritty` on `PATH`; `alacritty.toml` import-merged, never rewritten.
- Per-apply `*.atmosphere.bak` beside each owned file (pre-apply copies; restored on failure; distinct from baseline).

## Relationships

```text
Theme 1──1 Palette (embedded)      Theme 1──1 GlassSettings (embedded [glass])
Theme 1──1 IconsSettings (embedded [icons], to add)
Theme *──1 Wallpaper file (referenced path, not owned)
AppliedState *──0..1 Theme (last-applied id, cleared on restore/delete-miss)
Baseline 1──1 pre-Atmosphere desktop (captured once, read by Restore + missing-baseline fallback)
Theme apply ──writes──▶ Owned outputs (*.css, shell css, alacritty) + gsettings + applied.toml
Failed apply ──restores──▶ pre-apply *.atmosphere.bak (not baseline)
Restore ──restores──▶ baseline snapshot (never guesses Yaru values when baseline exists)
```

## Validation summary (all enforced before any desktop write)

1. id regex + no `..`; 2. hex `^#[0-9a-fA-F]{6}$`; 3. wallpaper canonicalize + home-allowlist + regular-file + symlink-escape reject; 4. `gtk-theme`/`icon-theme` names exist before `gsettings set` (fall back to baseline/Yaru); 5. no `unwrap()` on Apply/Restore I/O; 6. fixed-arg subprocesses only.
