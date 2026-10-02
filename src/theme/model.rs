use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlassStrength {
    Subtle,
    Strong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlassSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_glass_strength")]
    pub strength: GlassStrength,
}

fn default_glass_strength() -> GlassStrength {
    GlassStrength::Subtle
}

impl Default for GlassSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: GlassStrength::Subtle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub background: String,
    pub surface: String,
    pub foreground: String,
    pub accent: String,
    pub on_accent: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub secondary: Option<String>,
    #[serde(default)]
    pub tertiary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconsSettings {
    #[serde(default = "default_true")]
    pub match_yaru: bool,
}

fn default_true() -> bool {
    true
}

impl Default for IconsSettings {
    fn default() -> Self {
        Self { match_yaru: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub mode: ColorMode,
    pub wallpaper: PathBuf,
    #[serde(default)]
    pub palette_edited: bool,
    #[serde(default)]
    pub glass: GlassSettings,
    #[serde(default)]
    pub icons: IconsSettings,
    pub palette: Palette,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppliedState {
    id: String,
}

pub fn theme_id_from_name(name: &str) -> Result<String> {
    let re = Regex::new(r"^[a-z0-9][a-z0-9._+-]*$").unwrap();
    let id = name.trim().to_lowercase().replace(' ', "-");
    if !re.is_match(&id) {
        anyhow::bail!(
            "Theme name must start with a letter or digit and use only letters, digits, . _ + -"
        );
    }
    Ok(id)
}

pub fn theme_dir(id: &str) -> PathBuf {
    paths::themes_root().join(id)
}

pub fn theme_file_path(id: &str) -> PathBuf {
    theme_dir(id).join("theme.toml")
}

pub fn save_theme(theme: &Theme) -> Result<String> {
    let id = theme_id_from_name(&theme.name)?;
    let dir = theme_dir(&id);
    fs::create_dir_all(&dir)?;
    let data = toml::to_string_pretty(theme).context("serialize theme")?;
    fs::write(theme_file_path(&id), data)?;
    Ok(id)
}

pub fn load_theme(id: &str) -> Result<Theme> {
    let path = theme_file_path(id);
    let data = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&data).context("parse theme.toml")
}

pub fn list_theme_ids() -> Result<Vec<String>> {
    let root = paths::themes_root();
    if !root.is_dir() {
        return Ok(vec![]);
    }
    let mut ids = vec![];
    for entry in fs::read_dir(root)? {
        let path = entry.path();
        if path.join("theme.toml").is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                ids.push(name.to_string());
            }
        }
    }
    ids.sort();
    Ok(ids)
}

pub fn delete_theme(id: &str) -> Result<()> {
    let dir = theme_dir(id);
    if dir.is_dir() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}

pub fn set_applied_theme(id: &str) -> Result<()> {
    let state = AppliedState {
        id: id.to_string(),
    };
    let path = paths::applied_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, toml::to_string_pretty(&state)?)?;
    Ok(())
}

pub fn applied_theme_id() -> Option<String> {
    let path = paths::applied_state_path();
    let data = fs::read_to_string(path).ok()?;
    toml::from_str::<AppliedState>(&data).ok().map(|s| s.id)
}

pub fn default_name_from_wallpaper(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("My Theme")
        .to_string()
}
