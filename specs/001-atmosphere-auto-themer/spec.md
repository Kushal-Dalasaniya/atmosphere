# 001 - Atmosphere: theme switcher and theme creator for Ubuntu 26 and later

**Status:** Implementation contract (replaces the 2026-10-01 draft)
**Date:** 2026-10-01
**Target:** Ubuntu 26.04 LTS and later, GNOME 50 and later, Wayland session
**Inspiration:** [Omarchy](https://omarchy.org/) theme switching, plus [Aether](https://github.com/omacom/aether) (the theme creator shipped with current Omarchy)

> Pick a saved theme and the desktop follows. Or make a new theme from a wallpaper, tweak the colors, save it, and apply it. No terminal required.

This file is the contract for implementation. If an example command in an older note conflicts with this file, follow this file.

### Support floor

Atmosphere is for **Ubuntu 26 and later** only. That means **GNOME 50 and later**.

- Ubuntu 26.04 ships GNOME 50. That pair is the oldest system this project supports.
- Newer Ubuntu releases and newer GNOME (51, 52, …) stay in scope. Do not hardcode “50 only” in a way that refuses to run on a newer shell.
- Ubuntu 24.04, 25.04, 25.10, and any GNOME older than 50 are out of scope. Do not add shims, fallbacks, or tests for them.
- There is no X11 session to support. Ubuntu 26’s GNOME session is Wayland.

The reference machine for v1 is Ubuntu 26.04 with GNOME 50. Passing there is required. A newer GNOME is a supported target, not a port.

---

## 1. What we are building

Omarchy has two separate jobs:

1. **Switch.** A menu of named themes. One choice applies that theme’s wallpaper and palette to the shell, apps, and terminal.
2. **Create.** Aether: choose a wallpaper, extract a palette, adjust colors, save the result as a named theme, apply it.

Atmosphere is those two jobs on Ubuntu GNOME. It is not a Hyprland port, and it is not “click a loose jpg and hope matugen rewrote every config on the machine.”

### Who it is for

- Someone who wants Ubuntu to look cohesive without editing CSS.
- Someone who wants one saved look they can reapply, then another, without losing the first.

### v1 includes

- A theme library: saved themes as cards. One click applies one theme.
- A creator: add a wallpaper, extract colors, edit the main colors, choose dark or light, name the theme, save, apply.
- Apply updates wallpaper, GTK 3, GTK 4 / libadwaita, and (when the extension is on) GNOME Shell top bar and notifications.
- **Glass look (optional, user-controlled):** a clear **on/off** control per theme. When **on**, clean translucent shell styling on the top bar (and related shell surfaces in CSS), tinted from the palette, with **Subtle** or **Strong** strength. When **off**, shell uses opaque theme colors only. Independent of Blur my Shell and any other blur extension.
- Optional Alacritty colors, only when Alacritty is installed and its config can import a colors file.
- A warning banner when the **User Themes** extension is missing or disabled. The rest of the app still works (colors and wallpaper without shell CSS).
- **Restore Ubuntu defaults:** one explicit action that puts the desktop back to how it was **before Atmosphere’s first successful Apply** (wallpaper, GTK theme, color-scheme, shell user-theme, and owned CSS files). The session must never be left in a half-broken state.
- **Two-tone folder icons (Yaru-style):** keep Ubuntu’s default **dual-shade** folder look (not flat single-color icons). On Apply, pick the closest installed **Yaru** icon theme variant so folder primary/secondary tones harmonize with the theme accent (see §4.12). User can turn this off per theme.

### v1 does not include

- KDE, XFCE, or Hyprland
- AI wallpaper generation, Wallhaven, online theme install, or cloud sync
- VS Code, Neovim, Kitty, Foot, browser, or btop templates
- Building or shipping custom SVG icon packs (use installed **Yaru** variants only; see §4.12)
- A full wallpaper editor (crop, grain, filters on the image file itself)
- Configuring, installing, or driving **Blur my Shell** (or any blur extension): users may keep such extensions for overview, lock screen, or app grid on their own; Atmosphere never writes their settings
- Compositor blur behind the panel (Mutter blur pipelines). Atmosphere uses **tint + translucency in shell CSS only** for a clean look, not extension-managed “application glass”
- Frosted title bars on every GTK app
- Promising that already-open GTK 4 apps recolor without a restart
- Replacing the user’s existing `~/.config/matugen/config.toml`

Those can wait. Do not implement them “while you are here.”

---

## 2. Decisions that are already made

Do not reopen these during implementation.

| Topic | Decision |
| --- | --- |
| Desktop | Ubuntu 26 and later, GNOME 50 and later, Wayland. No Ubuntu older than 26. No GNOME older than 50. |
| UI toolkit | Rust, GTK 4, libadwaita. Minimum API level is the `gnome_50` feature. Code must still run on newer GNOME. |
| Color engine | Palette extraction is in-process (`material-colors` crate). Atmosphere renders every config file itself; no external extractor binary is required or executed. |
| Extractor config | None — in-process extraction takes no config file. Never read or write `~/.config/matugen/`. |
| Source of truth | A saved `theme.toml`, not a live wallpaper folder and not extractor output files. |
| Dark / light | A property of the theme (`mode = "dark"` or `"light"`). The window toggle edits the theme being previewed. It does not hardcode `adw-gtk3-dark`. |
| Shell theme | Best-effort overlay on top of Ubuntu’s Yaru shell stylesheet. Never install a 4-line CSS file as the whole shell theme. |
| Apply speed | Wallpaper changes immediately. The UI never blocks on extraction. “Under 1 second” means the background job finishes in under 1 second after the first extraction has been run once in that session, on a normal photo. Shell animation time does not count. |
| Open apps | Say so in the UI: new apps pick up colors now; already-open GTK 4 apps may need a restart for the new accent. Dark/light can still flip on apps that watch `color-scheme`. |
| Glass / frost | **Optional per theme** with an explicit user toggle (default **off** for new themes). When off, Apply must not emit translucent shell rules. When on: `subtle` or `strong` in `gnome-shell.css` only. **Never** read or write Blur my Shell (or any blur extension). |
| Reversibility | Before the **first** successful Apply, capture a **baseline snapshot** of every gsettings key and file Atmosphere will touch. Failed Apply restores the pre-apply backup. **Restore Ubuntu defaults** restores the baseline so GNOME behaves like before Atmosphere (Yaru / Ubuntu gtk-theme, prior wallpaper, shell theme name cleared or reset to pre-Atmosphere value). |
| Icons | Dual-tone **Yaru** folder icons via `icon-theme` gsettings only; map accent → nearest Yaru color variant; never flatten to monochrome icon sets. |
| Security | No shell execution of user/theme content; validate paths; **zero known Critical/High** issues at release (`cargo audit`, review). See §8. |

---

## 3. UX

One libadwaita window. Two views, switched from the header bar: **Themes** and **Create**.

### 3.1 Themes

Cards for every saved theme. Each card shows a wallpaper thumbnail, the theme name, and a few palette swatches.

- Single click applies that theme.
- The applied card shows “Applied”.
- While a job is running, the card shows a spinner and further clicks are ignored until it finishes or fails.
- Empty library: short explanation plus a button that opens **Create**.
- Right-click or a secondary button: Rename, Delete. Deleting the applied theme is allowed; desktop files stay as they were until another theme is applied. Do not delete the user’s original wallpaper file.
- Each theme card shows whether **Glass** is on or off (small label or icon). No global force-enable: glass follows the saved theme.

### 3.2 Create

- Wallpaper strip from `~/Pictures/Wallpapers`, plus **Add wallpaper** (file chooser, copies or references the file; see §5.3).
- **Extract** runs the in-process extractor on the selected image and fills the swatches. Extract does not apply.
- Editable swatches, in this order: Background, Surface, Foreground, Accent, On accent. A reset control restores the last extraction.
- Dark / light control. Switching it re-runs extraction for that mode. It does not invent colors by inverting the other mode.
- **Glass effect** (see §4.10):
  - Primary control: **libadwaita `Switch`** — “Glass effect” **On / Off**. Default for **new** themes: **Off** (opt-in).
  - When the switch is **Off**, hide strength controls and save `glass.enabled = false` (or `glass = "off"`).
  - When **On**, show **Subtle** / **Strong** (segmented control or radio). Default strength when turning on: **Subtle**.
  - Preview: when On, show a light tinted overlay on the wallpaper thumbnail; when Off, no overlay.
  - Changing glass does not apply to the desktop until **Save and apply** (or re-apply an existing theme).
- Name field. Default name comes from the wallpaper file name.
- **Save** writes a theme and returns to Themes.
- **Save and apply** writes it and runs the apply pipeline.
- Drag-and-drop of an image onto the window selects it as the wallpaper.

### 3.3 Startup banner

On startup, run `gnome-extensions info user-theme@gnome-shell-extensions.gcampax.github.com` (or `gnome-extensions list --enabled`).

Show a dismissible info bar when the extension is missing **or** installed but disabled:

> To theme the top bar and notifications, install the “User Themes” extension and turn it on. You may need to log out and back in. Wallpaper and app colors still work.

Remember dismissal for that session only. If the extension is missing, skip shell steps on apply. Do not crash and do not fail the rest of the apply.

### 3.4 Failure toast

If apply fails, the theme card is not marked applied. The toast says what failed in one sentence (“Could not read the wallpaper”, “Could not extract colors from this image”, “Could not write GTK colors”). Do not dump a stack trace into the UI.

After a failed Apply, the desktop MUST match the state immediately before that Apply started (colors, CSS, gsettings keys Atmosphere owns). The session must remain usable (normal Settings, logout, and app launch still work).

### 3.5 Restore Ubuntu defaults

Provide a clear control (e.g. **Themes** view → **Restore Ubuntu defaults**, with a confirmation dialog).

**What it does:**

1. Restore all keys in the **baseline snapshot** (§4.11) — wallpaper URIs, `color-scheme`, `gtk-theme`, and `org.gnome.shell.extensions.user-theme name` as they were before the first Atmosphere Apply.
2. Restore `~/.config/gtk-3.0/gtk.css` and `gtk-4.0/gtk.css` from the baseline file backup when present; if the user had no custom CSS before Atmosphere, remove Atmosphere-owned `gtk.css` (marker present) so GTK falls back to theme defaults.
3. Set shell user-theme to the baseline value (often empty string → default Yaru shell). Do not leave `Atmosphere` selected after restore.
4. Clear `~/.local/share/atmosphere/applied.toml` so no theme shows as “Applied”.
5. Show toast: “Restored your desktop to how it was before Atmosphere.”

**What it does not do:** uninstall the app, delete saved themes under `~/.local/share/atmosphere/themes/`, or change Blur my Shell or other third-party extensions.

**Glass off vs restore:** Turning **glass off** on a theme and re-applying only changes that theme’s shell CSS to opaque colors; it does **not** replace **Restore Ubuntu defaults**, which returns the whole desktop to the pre-Atmosphere baseline.

---

## 4. How apply works

Apply is one background job per click. A new click waits until the current job finishes (a simple mutex). Do not run two applies at once.

### 4.1 Threading

GTK runs on the GLib main loop. Do not block it and do not drive the UI from a Tokio worker.

- Start work with `gio::spawn_blocking` (or one helper thread).
- Return to the UI with `glib::idle_add` / `glib::spawn_future_local`.
- Tokio is optional. It is not required for v1. If used, it stays off the main thread.

### 4.2 Order

Do these steps in this order. The old draft said “concurrent” and also “wallpaper, then extraction, then Flatpak.” That race would back up the wrong files. Sequential order is mandatory.

1. Read `theme.toml`. If the wallpaper path is missing, stop with an error. Do not change the desktop.
2. Copy current output files to `*.atmosphere.bak` next to each file (only files Atmosphere owns; see §6).
3. Set the wallpaper (step 4.3). The user should see the image before color extraction finishes.
4. If this apply is from **Extract**, the palette is already in the theme. If the theme says the palette is generated and the user has not hand-edited it, reuse the saved palette. Do **not** re-run extraction on every apply.
5. Render templates from the saved palette into temp files in the same directory, then rename each into place. Readers must never see a half-written `gtk.css`.
6. Apply GTK and color-scheme settings (step 4.5).
7. If User Themes is enabled, write the shell CSS and reload it (step 4.6). If not, skip this step and continue.
8. If Alacritty is present, update its import (step 4.7). If not, skip.
9. Apply glass styling (step 4.10): merge translucent shell rules into the CSS from §4.6. Glass **Off** uses opaque panel colors only.
10. On any failure after step 2, restore the **pre-apply** backups from step 2 (not the baseline), then show the failure toast. The desktop must be usable. Leave the new wallpaper in place only when the failure happened after a successful wallpaper set **and** the color restore succeeded; the toast must say that the picture changed and the colors were reverted.
11. On success, update `applied.toml`. Never delete or overwrite the **baseline** snapshot (§4.11).

Flatpak filesystem overrides are **not** part of apply. They run once, when the user first applies a theme, and are skipped when already set (§4.8). Restoring Ubuntu defaults does **not** remove Flatpak overrides (harmless read-only mounts).

### 4.11 Baseline snapshot & safe revert (non-negotiable)

Atmosphere MUST NOT leave the user stuck with a broken top bar, unreadable panel, or invalid gtk-theme.

**When:** Immediately before the **first successful Apply** in this user account (if `baseline.toml` does not exist yet).

**Where:** `~/.local/share/atmosphere/baseline/`

| Artifact | Purpose |
| --- | --- |
| `baseline.toml` | gsettings values: `picture-uri`, `picture-uri-dark`, `picture-options`, `color-scheme`, `gtk-theme`, `icon-theme`, `user-theme-name` |
| `gtk-3.0.css` | Copy of `~/.config/gtk-3.0/gtk.css` if it existed (or a marker file `absent` if not) |
| `gtk-4.0.css` | Same for gtk-4.0 |
| `README` | One line: “Created before first Atmosphere Apply; used by Restore Ubuntu defaults.” |

**Rules:**

- Capture baseline **once**; later Applies only use per-apply `*.atmosphere.bak` next to files being written.
- **Restore Ubuntu defaults** (§3.5) reads baseline only — never guess Yaru settings.
- If baseline is missing (user deleted it), Restore offers to apply **documented Ubuntu 26 fallbacks** (`gtk-theme` `Yaru` / `Yaru-dark`, `color-scheme` `default`, clear user-theme name) and restore CSS from `gtk.css.user-saved` if present; show a warning that this may not match their exact pre-Atmosphere look.
- Apply pipeline steps must be **atomic per resource**: write temp → rename; on error, restore backups so GNOME Shell and Settings keep working.
- Disabling **glass** on a theme and applying again is not a full revert; only **Restore Ubuntu defaults** or successful failure-rollback returns to baseline behavior.

### 4.3 Wallpaper

Use `gio` / `gsettings`, not a shell string.

```text
org.gnome.desktop.background picture-uri
org.gnome.desktop.background picture-uri-dark
org.gnome.desktop.background picture-options = zoom
```

Both URI keys get the same `file://` URI from `GFile`. Percent-encode spaces and non-ASCII characters. A hand-built `'file://<PATH>'` breaks on spaces.

Set both URI keys. Ubuntu’s dark and light wallpapers are independent; setting only one leaves the other mode on the old image.

### 4.4 Palette extraction

Extraction is in-process via the `material-colors` crate. No external extractor
binary is required or executed, and `~/.config/matugen/` is never read or written.

- Derive a representative source color from a downscaled copy of the wallpaper,
  then build the scheme for `mode` (`dark` or `light`) from the theme. Never
  leave a hardcoded dark scheme in the light path.
- Extraction reads the image and returns colors. It does not write templates,
  set a wallpaper, run hooks, or write gsettings. Atmosphere owns every
  gsettings write, so no hook can undo light mode.
- There is no extractor config file, no template syntax, and no post-hook.

Map the scheme into the theme palette:

| Atmosphere role | Scheme field (under the selected mode) |
| --- | --- |
| `background` | `surface` |
| `surface` | `surface_container_low` |
| `foreground` | `on_surface` |
| `accent` | `primary` |
| `on_accent` | `on_primary` |

Also store `error`, `secondary`, and `tertiary` for later templates, even if the creator does not show them yet.

If the user edits a swatch, set `palette_edited = true` and keep their hex values. Later applies use those values.

### 4.5 GTK settings

After the CSS files are in place:

| `mode` | `org.gnome.desktop.interface color-scheme` | `org.gnome.desktop.interface gtk-theme` |
| --- | --- | --- |
| `dark` | `prefer-dark` | `adw-gtk3-dark` |
| `light` | `prefer-light` | `adw-gtk3` |

`adw-gtk3-theme` must be installed or GTK 3 apps will not follow. Check once at startup; if the theme name is missing, warn, and still write the CSS files.

To force GTK 3 to re-read `gtk.css`, set `gtk-theme` to `Adwaita`, then immediately set it to the target name above. Setting the same value it already has does not reload.

Do **not** write a hex color into `org.gnome.desktop.interface accent-color`. That key is a fixed set of names (`blue`, `teal`, `green`, `yellow`, `orange`, `red`, `pink`, `purple`, `slate`), not an arbitrary Material color. Custom accents live only in `gtk.css`.

What actually updates:

- Wallpaper: immediate.
- `color-scheme`: many open libadwaita apps flip between light and dark.
- Custom `@define-color` values in `~/.config/gtk-4.0/gtk.css`: applied to apps started **after** the write. Already-open GTK 4 apps do not watch that file.
- GTK 3: the theme-name toggle reloads `~/.config/gtk-3.0/gtk.css` for apps that honor `gtk-theme`.

The success toast is: “Applied. Apps you open from now on use this theme.”

### 4.6 GNOME Shell

Only when User Themes is enabled.

Atmosphere’s shell stylesheet must start by importing Ubuntu’s real shell theme, then override a short list of selectors (panel, panel buttons, calendar, message list / notifications). A file that only contains libadwaita `@define-color` lines will replace Yaru and leave the top bar unstyled. That is a broken theme, not a small theme.

At runtime, use the first path that exists:

1. `/usr/share/gnome-shell/theme/Yaru-dark/gnome-shell.css` when `mode` is dark
2. `/usr/share/gnome-shell/theme/Yaru/gnome-shell.css` when `mode` is light
3. `/usr/share/gnome-shell/theme/gnome-shell.css`

If none exist, skip shell theming and warn. Do not invent a path.

Write:

```text
~/.themes/Atmosphere/gnome-shell/gnome-shell.css
```

Reload by setting `org.gnome.shell.extensions.user-theme name` to `''`, then to `Atmosphere`. Setting `Atmosphere` when it is already `Atmosphere` does not reload CSS.

Shell CSS is version-sensitive. The oldest stylesheet this project supports is GNOME 50 on Ubuntu 26.04. Do not special-case GNOME 49 or older. If a selector does nothing on GNOME 50, leave it out. Do not copy a random older shell theme into the repo. On a newer GNOME, keep the same import-and-override approach; if that release moved the Yaru file, the runtime path search in this section is what adapts, not a second theme for old Ubuntu.

### 4.7 Alacritty

Skip the whole step when `alacritty` is not on `PATH`.

Write `~/.config/alacritty/atmosphere-colors.toml`. Ensure `~/.config/alacritty/alacritty.toml` contains:

```toml
live_config_reload = true
import = ["~/.config/alacritty/atmosphere-colors.toml"]
```

If the user’s file exists, add the import only when that exact line is absent. Do not rewrite the rest of the file. If the user has no Alacritty config, create a minimal one with those two lines.

Alacritty does not read `~/.config/alacritty/colors.toml` by itself. Writing that file alone does nothing.

Ubuntu’s default terminal follows GTK, so it is covered by §4.5. Do not add a special case for Ptyxis or Console.

### 4.8 Flatpak

Once per user, before the first successful apply returns:

```bash
flatpak override --user --filesystem=xdg-config/gtk-4.0:ro
flatpak override --user --filesystem=xdg-config/gtk-3.0:ro
```

If `flatpak` is missing, skip and continue. If the overrides are already present, do not run them again.

This lets Flatpak GTK apps **started afterward** read the same CSS. It does not recolor a Flatpak app that is already open. Do not set `GTK_THEME` globally; that fights libadwaita.

### 4.9 GTK CSS content

`~/.config/gtk-4.0/gtk.css` and `~/.config/gtk-3.0/gtk.css` are **owned by Atmosphere**. On first apply, if either file exists and does not contain the marker below, copy it to `gtk.css.user-saved` and then replace it. Tell the user once: “Your previous GTK CSS was saved as gtk.css.user-saved.”

Marker on line 1:

```css
/* atmosphere-owned: do not edit; regenerated when a theme is applied */
```

Minimum variables that must be set from the palette:

```css
@define-color accent_bg_color <accent>;
@define-color accent_fg_color <on_accent>;
@define-color window_bg_color <background>;
@define-color window_fg_color <foreground>;
@define-color view_bg_color <surface>;
@define-color view_fg_color <foreground>;
```

Hex values include the leading `#`. No `{{…}}` template braces in the output file.

When glass is not **Off**, also emit libadwaita-friendly translucent surface tokens where GTK supports them on GNOME 50 (best-effort; do not fail apply if a property is ignored):

```css
/* Derived from palette; alpha depends on glass strength — see §4.10 */
@define-color atmosphere_glass_surface rgba(<surface RGB>, <alpha>);
```

Document in the UI that most GTK apps only pick up accent/background colors, not full window frosted chrome.

### 4.10 Glass styling (clean, Atmosphere-only)

**Goal:** A cohesive, **clean** look: the top bar and notification area use a soft translucent tint from the active theme, not muddy “application blur” or extension-driven frosted windows. Wallpaper and palette stay in sync; shell chrome feels lighter than a solid Yaru strip.

**Independence from blur extensions**

- Atmosphere **must not** install, enable, configure, or back up **Blur my Shell** or any similar extension.
- Users who like Blur my Shell for **overview, app grid, or lock screen** keep managing that extension themselves. Atmosphere Apply **never** writes under `/org/gnome/shell/extensions/blur-my-shell/`.
- Do not document Atmosphere glass as “install Blur my Shell.” Do not use extension blur for GTK apps (Files, etc.).

**What v1 covers**

| Surface | Mechanism |
| --- | --- |
| GNOME top panel, calendar popup, message list (where selectors work on GNOME 50) | `rgba()` backgrounds and borders in `gnome-shell.css` (§4.6), strength from `glass` |
| Ubuntu dock | Only if targetable via the same shell CSS import (best-effort; if a selector fails on 50, skip without error) |
| Overview / app grid | **Not** driven by Atmosphere in v1 (user’s blur extension, if any, stays separate) |
| Atmosphere app window | libadwaita styling in our own UI only (e.g. consistent accent / optional light translucency in the control center window) |

**Per-theme setting** (`theme.toml` §5.1):

Use either shape (implement one; prefer the struct for clarity):

```toml
[glass]
enabled = false   # master on/off — default false for new themes
strength = "subtle"   # only when enabled: "subtle" | "strong"
```

Legacy equivalent: `glass = "off"` means disabled; `glass = "subtle"` or `"strong"` means enabled with that strength.

| State | Meaning |
| --- | --- |
| **Off** (`enabled = false` / `glass = "off"`) | Opaque shell colors from the palette. No translucency. User chose no glass effect. |
| **On + subtle** | Soft tint; panel alpha ~0.75–0.88; readable on busy wallpapers. |
| **On + strong** | Stronger tint; panel alpha ~0.60–0.75; still readable. |

Apply **must** treat Off as a first-class path: do not leave stale translucent CSS from a previous theme when the newly applied theme has glass off.

**Tint math:** Convert theme `surface` (or `background` when `surface` is too close to wallpaper) to `rgba`. Use `foreground` / `on_surface` for labels and buttons. Recompute when swatches change.

**Design rules (avoid “dirty glass”):**

- Prefer **one** translucent panel background color, not stacked blurs + heavy transparency.
- Avoid whitelisting apps or lowering window opacity via third-party extensions.
- QA on Ubuntu 26.04: clock, system icons, and notification text must meet a minimum **4.5:1 contrast ratio** between text and its panel/notification background, verified on one light and one dark busy reference wallpaper (see quickstart reference inputs).

**Order in apply:** Apply §4.10 as part of generating `gnome-shell.css` in step 4.6 / before user-theme reload. No extra extension reload beyond §4.6.

### 4.12 Icons — two-tone Yaru folders (theme-matched)

Users expect **Files (Nautilus)** to keep Ubuntu’s familiar **two-color** folder icons (lighter flap + darker body), not a flat single-fill icon theme.

**Approach (v1):**

- Do **not** generate or download icon packs.
- On Apply, when the theme has **Match folder icons** enabled (default **on**), set `org.gnome.desktop.interface icon-theme` to the best-matching **installed** Yaru variant, for example: `Yaru`, `Yaru-blue`, `Yaru-magenta`, `Yaru-purple`, `Yaru-red`, `Yaru-sage`, `Yaru-olive`, `Yaru-prussiangreen`, `Yaru-wartybrown`, `Yaru-yellow`, and dark counterparts where Ubuntu provides them (`Yaru-dark`, `Yaru-blue-dark`, …).
- **Matching:** derive hue from theme `accent` (or `primary` from extraction) and choose the variant whose documented accent color is closest on the color wheel. If only `Yaru` / `Yaru-dark` is installed, use that and do not fail Apply.
- **Dark/light:** when `mode` is `dark`, prefer `*-dark` icon theme names when they exist; when `light`, prefer non-dark Yaru variants.
- **Off:** when the user disables **Match folder icons** for a theme, Apply does not change `icon-theme` (leave current or baseline on restore).
- **Restore Ubuntu defaults** (§3.5) restores `icon-theme` from baseline.

**Create UI:** `Switch` — “Match folder icons (Yaru)” — default **on**. Help text: “Keeps Ubuntu’s two-color folders; picks the Yaru set closest to your accent.”

**Non-goals:** Papirus, Numix, or other flat icon themes; recoloring individual SVGs; icon changes inside Flatpak apps that ignore host `icon-theme`.

---

## 5. Files

### 5.1 Saved themes

```text
~/.local/share/atmosphere/themes/<id>/theme.toml
~/.local/share/atmosphere/themes/<id>/preview.png
```

`<id>` is the name lowercased, with spaces turned into `-`. It must match `^[a-z0-9][a-z0-9._+-]*$`. Refuse other characters in the UI instead of stripping them silently. Two themes cannot share an id.

`theme.toml` shape:

```toml
name = "Sunset Mountains"
mode = "dark"                 # "dark" or "light"
wallpaper = "/home/user/Pictures/Wallpapers/sunset.jpg"
palette_edited = false
[glass]
enabled = false
strength = "subtle"           # used only when enabled = true

[icons]
match_yaru = true             # dual-tone Yaru variant from accent; false = do not touch icon-theme

[palette]
background = "#1b1b1f"
surface = "#201f24"
foreground = "#e4e1e6"
accent = "#c5c0ff"
on_accent = "#2c0091"
```

`wallpaper` is an absolute path. The theme stores the path; it does not move the user’s image.

### 5.2 Other Atmosphere files

```text
~/.local/share/atmosphere/applied.toml     # id of the last successfully applied theme
~/.local/share/atmosphere/baseline/        # snapshot before first Apply; Restore uses this
~/.cache/atmosphere/thumbs/                # 256px thumbnails
~/.themes/Atmosphere/gnome-shell/gnome-shell.css
~/.config/gtk-4.0/gtk.css                  # owned after first apply; see §4.9
~/.config/gtk-3.0/gtk.css
~/.config/alacritty/atmosphere-colors.toml # only if Alacritty exists
```

Create directories with the equivalent of `mkdir -p` before writing. Never write into `~/.config/matugen/`.

### 5.3 Wallpaper library

Scan `~/Pictures/Wallpapers` for `.jpg`, `.jpeg`, `.png`, and `.webp`.

- Thumbnails are decoded off the main thread, longest side 256px, cached by path + mtime.
- Broken images are skipped and listed in a small “could not read” note, not a crash.
- `.webp` is shown only when gdk-pixbuf can decode it. GNOME wallpaper may still reject webp; if `gsettings` rejects the URI, report that and do not mark the theme applied.
- **Add wallpaper** copies the chosen file into `~/Pictures/Wallpapers` when it lives outside that folder. If a file with the same name exists, add a numeric suffix. Do not overwrite.

### 5.4 Repo layout

```text
atmosphere/
├── specs/001-atmosphere-auto-themer/spec.md
├── Cargo.toml
├── src/
│   ├── main.rs                 # Adw application
│   ├── ui/
│   │   ├── window.rs
│   │   ├── themes_view.rs
│   │   └── create_view.rs
│   ├── theme/
│   │   ├── model.rs            # theme.toml read/write
│   │   ├── validate.rs         # hex/path/id validators (§8)
│   │   ├── atomic.rs           # temp→rename writes, *.atmosphere.bak backup/restore
│   │   ├── extract.rs          # in-process Material You extraction (§4.4)
│   │   ├── apply.rs            # the pipeline in §4
│   │   ├── baseline.rs         # baseline snapshot + restore (§4.11, §3.5)
│   │   ├── gtk_css.rs
│   │   ├── shell.rs
│   │   ├── shell_check.rs      # User Themes detection (§3.3)
│   │   ├── glass.rs            # shell rgba tints for gnome-shell.css only
│   │   ├── icons.rs            # accent→Yaru variant matching (§4.12)
│   │   ├── alacritty.rs        # optional Alacritty colors (§4.7)
│   │   ├── wallpaper.rs
│   │   └── flatpak.rs
│   └── thumbs.rs
├── assets/                     # icons only; no matugen template pack
└── README.md
```

Ship templates as Rust functions or included strings that write final files. Do not copy a matugen template pack into `~/.config/matugen/templates` on first run.

---

## 6. Toolchain

### 6.1 Rust

- rustup stable (1.85 or newer)
- Edition 2024
- `rustup component add rust-analyzer clippy rustfmt`

Crate lines for the GNOME 50 minimum (confirm versions on crates.io at implementation time; do not use the old gtk4 0.9 / libadwaita 0.7 / glib 0.20 set). `gnome_50` is the oldest feature flag this project may require. Do not enable an older `gnome_4x` flag to run on Ubuntu before 26. On a newer Ubuntu, a newer `gnome_*` flag is fine when the installed GTK matches it, as long as the app still builds against the GNOME 50 baseline.

```toml
gtk4 = { version = "0.11", features = ["gnome_50"] }
libadwaita = { version = "0.9", features = ["gnome_50"] }
glib = "0.22"
gio = "0.22"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
image = "0.25"
```

Add a JSON crate only if `serde_json` is not already pulled in. Do not add Tokio unless a later change needs it.

### 6.2 System packages

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config git curl \
  libgtk-4-dev libadwaita-1-dev \
  libglib2.0-dev libcairo2-dev libpango1.0-dev \
  libgdk-pixbuf-2.0-dev libgraphene-1.0-dev \
  gsettings-desktop-schemas gnome-shell \
  gnome-shell-extensions \
  adw-gtk3-theme
```

`gnome-shell-extensions` contains User Themes (`user-theme@gnome-shell-extensions.gcampax.github.com`). Enable it with:

```bash
gnome-extensions enable user-theme@gnome-shell-extensions.gcampax.github.com
```

Then log out and back in. Extension Manager is optional UI, not a compile dependency. Its apt name is not required by this spec.

`flatpak` is optional. Install it only when testing Flatpak apps.

### 6.3 Color extraction (no external binary)

Palette extraction is in-process via the `material-colors` crate (see §4.4).
There is nothing to install: no extractor binary, no missing-binary path, no
`~/.config/matugen/` involvement. Do not vendor an external extractor in v1.

### 6.4 Dev check

```bash
rustc --version
pkg-config --modversion gtk4
pkg-config --modversion libadwaita-1
echo "$XDG_SESSION_TYPE"          # wayland
gnome-shell --version             # GNOME Shell 50 or newer
lsb_release -rs                   # 26.04 or newer
gsettings --version
```

---

## 7. Acceptance

- [ ] `cargo run` opens a libadwaita window with Themes and Create.
- [ ] An empty theme library shows the empty state, not a crash.
- [ ] Create → Extract fills the five swatches from the in-process extractor without changing the desktop.
- [ ] Editing a swatch survives Save. Apply uses the edited hex, not a fresh extraction.
- [ ] Save and apply sets both wallpaper URI keys to a percent-encoded `file://` URI and sets `picture-options` to `zoom`.
- [ ] Dark theme writes `prefer-dark` and `adw-gtk3-dark`. Light theme writes `prefer-light` and `adw-gtk3`.
- [ ] Both `gtk.css` files are replaced atomically, contain the ownership marker, and contain the theme hex values.
- [ ] A pre-existing user `gtk.css` without the marker is preserved as `gtk.css.user-saved` once.
- [ ] `~/.config/matugen/` is unchanged after extract and apply.
- [ ] With User Themes disabled, apply succeeds, the banner can be shown, and no shell gsettings key is written.
- [ ] With User Themes enabled, shell CSS imports a real Yaru (or fallback) stylesheet, then sets the user-theme name to empty and then `Atmosphere`.
- [ ] A second apply is ignored until the first job finishes. The window stays responsive during extraction.
- [ ] A missing wallpaper or failing extraction restores the previous CSS and does not mark the theme applied.
- [ ] Paths with spaces work.
- [ ] `cargo clippy` and `cargo fmt --check` are clean.
- [ ] New themes default to **glass off** (`enabled = false`).
- [ ] UI: Glass **Switch** turns effect off without deleting strength; turning on restores last strength (default Subtle).
- [ ] `theme.toml` saves `glass.enabled` and `glass.strength` (or equivalent `off` / `subtle` / `strong`).
- [ ] Apply with glass **off** writes opaque shell CSS only; no translucent rules left over from a prior theme (fresh-off case).
- [ ] Apply with glass **on** (`subtle` / `strong`) and User Themes enabled uses translucent `rgba` panel backgrounds from the palette.
- [ ] Apply does not modify dconf/gsettings keys belonging to Blur my Shell or other blur extensions.
- [ ] First Apply creates `baseline/` with gsettings and gtk.css copies.
- [ ] Failed Apply after partial work restores pre-apply backups; session remains usable.
- [ ] **Restore Ubuntu defaults** returns wallpaper, gtk-theme, color-scheme, and user-theme name to baseline; clears applied state; GTK CSS matches baseline or is removed when Atmosphere-owned.
- [ ] Theme with glass **off** and Apply does not leave translucent shell rules from a previous glass-on apply of another theme (off-after-glass-on case).
- [ ] With `icons.match_yaru = true`, Apply sets a Yaru variant `icon-theme`; Files shows **two-tone** folders harmonizing with accent; with `false`, `icon-theme` unchanged.
- [ ] Security §8.4 checklist passed for the release candidate.

Manual check on Ubuntu 26.04 Wayland (GNOME 50): note desktop state → Apply a theme → **Restore Ubuntu defaults** → desktop matches pre-Atmosphere behavior (wallpaper, Yaru gtk, shell). Then: glass off → on subtle → off on same theme (solid bar). Confirm an already-open GTK 4 app is not required to recolor for the test to pass. Do not spend time making this pass on Ubuntu older than 26.

---

## 8. Security (release gate — highest priority)

Atmosphere runs on the user’s desktop and writes config under `$HOME`. Security is a **release blocker**, not a nice-to-have.

### 8.1 Threat model (v1)

- **Trust boundary:** same user running the GUI; `theme.toml` and wallpapers are user-owned data.
- **Untrusted input:** wallpaper file paths, theme names/ids, pasted paths, future imported themes.
- **Not in scope v1:** remote attackers over the network (no network stack in core app), multi-user privilege escalation (no `sudo`).

### 8.2 MUST NOT (non-negotiable)

- Execute shell commands built from theme files, wallpaper names, or user text (`sh -c`, `bash`, `eval`, `gnome-extensions` with user-controlled extension ids beyond fixed constants).
- Load or run Lua, `.desktop` `Exec=`, or binary hooks from theme directories.
- Write outside the user’s home except standard XDG paths documented in §5.2 (no `/etc`, no `/usr` writes).
- Ship setuid binaries or request polkit/root for normal Apply/Restore.
- Embed API keys, tokens, or phone-home telemetry in the app.
- Follow symlinks when copying **Add wallpaper** into `~/Pictures/Wallpapers` if the target resolves outside the source directory (copy regular files only; reject symlink escapes).
- Accept theme `id` or names that fail `^[a-z0-9][a-z0-9._+-]*$` or contain `..` path segments.

### 8.3 MUST (design & implementation)

- **Generated CSS/TOML only:** shell and GTK output is produced from Rust templates; palette values must be validated as `#RRGGBB` hex before interpolation (reject `{`, `}`, `url(`, `javascript`, newlines in color fields).
- **Paths:** canonicalize wallpaper paths; Apply refuses paths that are not readable regular files under the user’s home (or explicit allowlist: `~/Pictures/Wallpapers` and paths already stored in saved themes).
- **Subprocesses:** only fixed commands (`gnome-extensions info` with constant extension id); no user-controlled arguments.
- **Dependencies:** pin versions in `Cargo.lock`; run **`cargo audit`** in CI; **no known Critical or High** advisories in direct dependencies at release tag (document exceptions with CVE id and mitigation in `SECURITY.md`).
- **Unsafe Rust:** avoid `unsafe` unless justified in code review; no `unwrap()` on user-driven I/O paths in Apply/Restore — use errors and rollback (§4.11).
- **Flatpak overrides:** only the documented read-only `gtk-3.0` / `gtk-4.0` config mounts (§4.8).
- **Restore & failure paths:** must not leave invalid gsettings values that could crash Shell (validate theme names exist before setting `gtk-theme` / `icon-theme`; fall back to baseline or Yaru).

### 8.4 Release checklist (security)

- [ ] `cargo audit` clean for Critical/High (or documented waivers)
- [ ] `cargo clippy` with warnings addressed for Apply/Restore/icon code paths
- [ ] Manual test: malicious `theme.toml` with `accent = "#000\"; url(javascript:1)"` does not appear verbatim in `gtk.css`
- [ ] Manual test: symlink wallpaper escape rejected
- [ ] No secrets in repository; `.env` not required for runtime

### 8.5 Honest expectation

“Zero vulnerabilities forever” is not provable. The project **target** is: **no known Critical/High issues at ship time**, fast dependency updates, and safe failure modes so a bad theme cannot brick the session (§4.11, §3.5).

---

## 9. Later, not now

- Lock screen theming coordinated with the active theme
- Optional integration hooks for third-party blur extensions (explicitly out of scope; users configure those extensions themselves)
- Auto glass strength from wallpaper luminance (light vs dark photo)
- More templates (Kitty, Foot, VS Code) using the same saved palette
- Time-of-day light/dark
- Sharing a theme directory
- Super keybinding to open Atmosphere

---

## 10. References

- Omarchy theme creation: https://omarchy.org/manual/making-your-own-theme/
- Aether (Omarchy’s theme creator): https://github.com/omacom/aether
- matugen: https://github.com/InioX/matugen
- gtk4-rs: https://gtk-rs.org/gtk4-rs/stable/latest/book/
- User Themes: https://extensions.gnome.org/extension/19/user-themes/
- Blur my Shell (optional, user-managed; **not** used by Atmosphere): https://extensions.gnome.org/extension/3193/blur-my-shell/
