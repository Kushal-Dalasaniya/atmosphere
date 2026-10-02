# Contract: `theme.toml` schema (source of truth)

**Spec**: `../spec.md` §5.1 · **Model**: `../data-model.md` (Theme, Palette)

Applies to `~/.local/share/atmosphere/themes/<id>/theme.toml`. `<id>` derives from `name` (lowercase, spaces→`-`) and MUST match `^[a-z0-9][a-z0-9._+-]*$`. Readers MUST reject ids with `..` or other characters instead of stripping silently. Duplicate ids are an error.

## Full shape (authoritative)

```toml
name = "Sunset Mountains"          # required, non-empty after trim
mode = "dark"                      # required: "dark" | "light"
wallpaper = "/home/user/Pictures/Wallpapers/sunset.jpg"  # required, absolute path
palette_edited = false             # default false; true after any hand edit

[glass]
enabled = false                    # default false for new themes
strength = "subtle"                # "subtle" | "strong"; used only when enabled

[icons]
match_yaru = true                  # default true; false = don't touch icon-theme

[palette]
background = "#1b1b1f"             # required hex
surface = "#201f24"                # required hex
foreground = "#e4e1e6"             # required hex
accent = "#c5c0ff"                 # required hex
on_accent = "#2c0091"              # required hex
error = "#…"                       # optional, stored
secondary = "#…"                   # optional, stored
tertiary = "#…"                    # optional, stored
```

Legacy reader compat: `glass = "off"` ≡ `[glass] enabled=false`; `glass = "subtle"|"strong"` ≡ enabled + that strength.

## Validation contract

- All palette values MUST match `^#[0-9a-fA-F]{6}$` at load AND before render. Violations abort the operation (extract/save/apply) with a one-sentence error; nothing is written.
- `wallpaper` MUST canonicalize to a readable regular file; symlink escapes rejected.
- Unknown top-level keys are ignored (forward compat); missing required keys are errors.
- Writers MUST emit `[glass]` struct form and `[icons]` table; readers MUST accept the legacy string form.
