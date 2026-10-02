# Contract: UI behavior (Themes / Create / banner / toasts)

**Spec**: `../spec.md` §3 · **Research**: `../research.md` R-9, R-11

One libadwaita window; header-bar switch between **Themes** and **Create**.

## Themes view

- Card per saved theme: wallpaper thumbnail, name, palette swatches, glass on/off label.
- Single click = apply (mutex: spinner on running card, further clicks ignored until finish/fail).
- Applied card shows "Applied" (from `applied.toml`).
- Empty library: short explainer + button opening Create (no crash).
- Row actions: Rename (re-validates id uniqueness), Delete (deleting applied theme allowed; desktop unchanged until next apply; never deletes source wallpaper).
- Control: **Restore Ubuntu defaults** with confirmation dialog (behavior per `system-effects.md`).
- Failure: card NOT marked applied; one-sentence toast ("Could not read the wallpaper" / "Could not write GTK colors" / …); no stack trace.

## Create view

- Wallpaper strip from `~/Pictures/Wallpapers` (jpg/jpeg/png/webp; broken files → "could not read" note) + **Add wallpaper** (file chooser; copy into folder, numeric suffix on collision, never overwrite; symlink escapes rejected) + drag-and-drop onto window selects wallpaper.
- **Extract** fills swatches, changes nothing on desktop. Editable swatches in order: Background, Surface, Foreground, Accent, On accent (hex-validated) + reset to last extraction.
- Dark/light control re-extracts for that mode; never inverts.
- **Glass effect**: `Switch` On/Off (default Off for new themes). Off → hide strength, save `enabled=false`. On → show Subtle/Strong segmented (default Subtle on first enable), thumbnail tint preview. Changing glass does not touch the desktop until Save-and-apply / re-apply.
- **Match folder icons (Yaru)** `Switch`, default on, with help text "Keeps Ubuntu's two-color folders; picks the Yaru set closest to your accent."
- Name field (default = wallpaper stem; id rules enforced, duplicates refused).
- **Save** writes theme, returns to Themes. **Save and apply** writes then runs the apply pipeline.

## Startup banner / toasts

- Banner (session-dismissible) when User Themes missing/disabled: "To theme the top bar and notifications, install the "User Themes" extension and turn it on. You may need to log out and back in. Wallpaper and app colors still work."
- Success toast: "Applied. Apps you open from now on use this theme." + honest note that open GTK 4 apps may need restart (dark/light flips live on watching apps).
- Failure-after-wallpaper toast variant states the picture changed while colors were reverted.
