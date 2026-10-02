use std::path::PathBuf;

pub const USER_THEME_EXTENSION: &str = "user-theme@gnome-shell-extensions.gcampax.github.com";

pub const GTK_CSS_MARKER: &str =
    "/* atmosphere-owned: do not edit; regenerated when a theme is applied */";

pub fn home() -> PathBuf {
    dirs::home_dir().expect("HOME not set")
}

pub fn wallpapers_dir() -> PathBuf {
    home().join("Pictures/Wallpapers")
}

pub fn themes_root() -> PathBuf {
    home().join(".local/share/atmosphere/themes")
}

pub fn applied_state_path() -> PathBuf {
    home().join(".local/share/atmosphere/applied.toml")
}

pub fn thumb_cache_dir() -> PathBuf {
    home().join(".cache/atmosphere/thumbs")
}

pub fn gtk4_css_path() -> PathBuf {
    home().join(".config/gtk-4.0/gtk.css")
}

pub fn gtk3_css_path() -> PathBuf {
    home().join(".config/gtk-3.0/gtk.css")
}

pub fn shell_theme_css_path() -> PathBuf {
    home().join(".themes/Atmosphere/gnome-shell/gnome-shell.css")
}

pub fn baseline_dir() -> PathBuf {
    home().join(".local/share/atmosphere/baseline")
}

pub fn baseline_toml_path() -> PathBuf {
    baseline_dir().join("baseline.toml")
}

pub fn private_matugen_config_path() -> PathBuf {
    home().join(".config/atmosphere/matugen.toml")
}

pub fn alacritty_colors_path() -> PathBuf {
    home().join(".config/alacritty/atmosphere-colors.toml")
}

pub fn alacritty_config_path() -> PathBuf {
    home().join(".config/alacritty/alacritty.toml")
}

pub fn flatpak_done_path() -> PathBuf {
    home().join(".local/share/atmosphere/flatpak-overrides.done")
}

pub fn ensure_data_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(themes_root())?;
    std::fs::create_dir_all(thumb_cache_dir())?;
    std::fs::create_dir_all(wallpapers_dir())?;
    Ok(())
}
