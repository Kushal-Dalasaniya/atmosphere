# Contract: system effects of Apply / Restore (gsettings + files)

**Spec**: `../spec.md` §§4.2–4.12 · **Research**: `../research.md` R-4–R-7, R-10

Apply order is sequential and mandatory. Any failure after backups → restore pre-apply `*.atmosphere.bak` (not baseline) and toast. Success → write `applied.toml`, never touch `baseline/`.

## gsettings writes (via `gio::Settings`, never shell strings)

| Condition | Schema | Key | Value |
|---|---|---|---|
| Always | `org.gnome.desktop.background` | `picture-uri` | `file://` URI from `GFile` (percent-encoded) |
| Always | `org.gnome.desktop.background` | `picture-uri-dark` | same URI |
| Always | `org.gnome.desktop.background` | `picture-options` | `zoom` |
| `mode=dark` | `org.gnome.desktop.interface` | `color-scheme` | `prefer-dark` |
| `mode=dark` | `org.gnome.desktop.interface` | `gtk-theme` | `adw-gtk3-dark` (toggle `Adwaita`→target to force reload) |
| `mode=light` | `org.gnome.desktop.interface` | `color-scheme` | `prefer-light` |
| `mode=light` | `org.gnome.desktop.interface` | `gtk-theme` | `adw-gtk3` (same toggle) |
| User Themes enabled | `org.gnome.shell.extensions.user-theme` | `name` | `""` then `"Atmosphere"` (toggle to force reload) |
| `icons.match_yaru=true` | `org.gnome.desktop.interface` | `icon-theme` | nearest installed Yaru variant (`Yaru[-color][-dark]`); `dark` prefers `-dark`; validate existence first |
| NEVER | `org.gnome.desktop.interface` | `accent-color` | MUST NOT write (fixed enum, not hex) |
| NEVER | `org.gnome.shell.extensions.blur-my-shell` etc. | any | MUST NOT read or write |

User Themes missing/disabled → skip the `user-theme` row, show session banner, continue everything else.

## File writes (all atomic: temp-in-same-dir + rename; `mkdir -p` first)

| Path | Content contract |
|---|---|
| `~/.config/gtk-4.0/gtk.css`, `~/.config/gtk-3.0/gtk.css` | Line 1 `/* atmosphere-owned: do not edit; regenerated when a theme is applied */`; `@define-color accent_bg_color/accent_fg_color/window_bg_color/window_fg_color/view_bg_color/view_fg_color` from palette; optional `atmosphere_glass_surface rgba(…)` only when glass enabled. Pre-existing unmarked file → one-time `gtk.css.user-saved` copy + single notice. |
| `~/.themes/Atmosphere/gnome-shell/gnome-shell.css` | First line(s) `@import url('file://…')` of first-existing `/usr/share/gnome-shell/theme/Yaru-dark/gnome-shell.css` (dark) / `Yaru/gnome-shell.css` (light) / `gnome-shell.css`; then opaque `#panel` block (glass off) or `rgba()` panel+calendar+message-list block (glass on, α subtle 0.75–0.88 / strong 0.60–0.75). No translucent rules when off. Skip+warn when no import exists. |
| `~/.config/alacritty/atmosphere-colors.toml` | Only if `alacritty` on `PATH`. Merge `import = ["~/.config/alacritty/atmosphere-colors.toml"]` + `live_config_reload = true` into `alacritty.toml` only when absent; minimal two-line create when missing. |
| `~/.local/share/atmosphere/applied.toml` | `id = "<theme-id>"` on success only; removed by Restore. |
| `~/.local/share/atmosphere/baseline/` | Written once before first success; never overwritten by later applies. |
| Flatpak | Once per user: `flatpak override --user --filesystem=xdg-config/gtk-4.0:ro` + `…/gtk-3.0:ro` when `flatpak` present and overrides absent. Never `GTK_THEME`. |

## Restore Ubuntu defaults (reads `baseline/` only)

Restores the 7 baseline keys, restores/removes both `gtk.css` per baseline copies/markers, resets shell user-theme to baseline (often `""`), clears `applied.toml`, toasts "Restored your desktop to how it was before Atmosphere." Leaves saved themes, Blur-my-Shell, Flatpak overrides alone. Missing baseline → documented Yaru/`default` fallbacks + warning.
