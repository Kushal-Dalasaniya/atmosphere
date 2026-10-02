use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
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

/// Accept both the struct form and the legacy string form:
/// `glass = "off"` ≡ disabled; `"subtle"`/`"strong"` ≡ enabled + strength.
fn deserialize_glass<'de, D>(deserializer: D) -> Result<GlassSettings, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct GlassVisitor;

    impl<'de> serde::de::Visitor<'de> for GlassVisitor {
        type Value = GlassSettings;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a glass settings table or one of \"off\", \"subtle\", \"strong\"")
        }

        fn visit_str<E>(self, value: &str) -> Result<GlassSettings, E>
        where
            E: serde::de::Error,
        {
            match value {
                "off" => Ok(GlassSettings {
                    enabled: false,
                    strength: GlassStrength::Subtle,
                }),
                "subtle" => Ok(GlassSettings {
                    enabled: true,
                    strength: GlassStrength::Subtle,
                }),
                "strong" => Ok(GlassSettings {
                    enabled: true,
                    strength: GlassStrength::Strong,
                }),
                other => Err(E::unknown_variant(other, &["off", "subtle", "strong"])),
            }
        }

        fn visit_map<M>(self, map: M) -> Result<GlassSettings, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            #[derive(Deserialize)]
            struct Helper {
                #[serde(default)]
                enabled: bool,
                #[serde(default = "default_glass_strength")]
                strength: GlassStrength,
            }
            let helper = Helper::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
            Ok(GlassSettings {
                enabled: helper.enabled,
                strength: helper.strength,
            })
        }
    }

    deserializer.deserialize_any(GlassVisitor)
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
    #[serde(default, deserialize_with = "deserialize_glass")]
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
    let id = name.trim().to_lowercase().replace(' ', "-");
    if !valid_theme_id(&id) {
        anyhow::bail!(
            "Theme name must start with a letter or digit and use only letters, digits, . _ + -"
        );
    }
    Ok(id)
}

/// Manual `^[a-z0-9][a-z0-9._+-]*$` check: panic-free and allocation-free,
/// unlike a compiled regex on this hot validation path.
fn valid_theme_id(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    id.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '+' | '-'))
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
    for entry in fs::read_dir(root)?.filter_map(Result::ok) {
        let path = entry.path();
        if path.join("theme.toml").is_file()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            ids.push(name.to_string());
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

/// Rename a theme: re-validates the new name, refuses duplicate ids, and
/// moves the directory only when the id changes. Never touches the user's
/// wallpaper file. Returns the (possibly unchanged) id.
pub fn rename_theme(old_id: &str, new_name: &str) -> Result<String> {
    rename_theme_in(&paths::themes_root(), old_id, new_name)
}

fn rename_theme_in(root: &Path, old_id: &str, new_name: &str) -> Result<String> {
    let new_id = theme_id_from_name(new_name)?;
    if new_id != old_id && root.join(&new_id).join("theme.toml").is_file() {
        anyhow::bail!("A theme named \"{new_name}\" already exists");
    }
    let mut theme = load_theme_in(root, old_id)?;
    theme.name = new_name.trim().to_string();
    let dir = root.join(&new_id);
    fs::create_dir_all(&dir)?;
    let data = toml::to_string_pretty(&theme).context("serialize theme")?;
    fs::write(dir.join("theme.toml"), data)?;
    if new_id != old_id {
        // Preserve preview.png and any sibling files.
        for entry in fs::read_dir(root.join(old_id))?.filter_map(Result::ok) {
            let dest = dir.join(entry.file_name());
            if entry.path() != dest && !dest.exists() {
                fs::copy(entry.path(), &dest).ok();
            }
        }
        fs::remove_dir_all(root.join(old_id)).ok();
    }
    Ok(new_id)
}

fn load_theme_in(root: &Path, id: &str) -> Result<Theme> {
    let path = root.join(id).join("theme.toml");
    let data = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&data).context("parse theme.toml")
}

pub fn set_applied_theme(id: &str) -> Result<()> {
    let state = AppliedState { id: id.to_string() };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_from_name_rules() {
        assert_eq!(
            theme_id_from_name("Sunset Mountains").unwrap(),
            "sunset-mountains"
        );
        assert_eq!(
            theme_id_from_name("Yaru-Dark_2.0").unwrap(),
            "yaru-dark_2.0"
        );
        assert!(theme_id_from_name("has space ok").is_ok());
        assert!(theme_id_from_name("..").is_err());
        assert!(theme_id_from_name("../escape").is_err());
        assert!(theme_id_from_name("bad!chars").is_err());
        assert!(theme_id_from_name("").is_err());
    }

    #[test]
    fn glass_legacy_strings() {
        let off: Theme = toml::from_str(
            "name = \"T\"\nmode = \"dark\"\nwallpaper = \"/tmp/x.jpg\"\nglass = \"off\"\n[palette]\nbackground = \"#111111\"\nsurface = \"#222222\"\nforeground = \"#eeeeee\"\naccent = \"#112233\"\non_accent = \"#ffffff\"\n",
        )
        .unwrap();
        assert!(!off.glass.enabled);

        let subtle: Theme = toml::from_str(
            "name = \"T\"\nmode = \"dark\"\nwallpaper = \"/tmp/x.jpg\"\nglass = \"subtle\"\n[palette]\nbackground = \"#111111\"\nsurface = \"#222222\"\nforeground = \"#eeeeee\"\naccent = \"#112233\"\non_accent = \"#ffffff\"\n",
        )
        .unwrap();
        assert!(subtle.glass.enabled);
        assert_eq!(subtle.glass.strength, GlassStrength::Subtle);

        let strong: Theme = toml::from_str(
            "name = \"T\"\nmode = \"dark\"\nwallpaper = \"/tmp/x.jpg\"\nglass = \"strong\"\n[palette]\nbackground = \"#111111\"\nsurface = \"#222222\"\nforeground = \"#eeeeee\"\naccent = \"#112233\"\non_accent = \"#ffffff\"\n",
        )
        .unwrap();
        assert!(strong.glass.enabled);
        assert_eq!(strong.glass.strength, GlassStrength::Strong);

        assert!(toml::from_str::<Theme>(
            "name = \"T\"\nmode = \"dark\"\nwallpaper = \"/tmp/x.jpg\"\nglass = \"frosted\"\n[palette]\nbackground = \"#111111\"\nsurface = \"#222222\"\nforeground = \"#eeeeee\"\naccent = \"#112233\"\non_accent = \"#ffffff\"\n",
        )
        .is_err());
    }

    #[test]
    fn glass_struct_and_defaults() {
        let minimal: Theme = toml::from_str(
            "name = \"T\"\nmode = \"light\"\nwallpaper = \"/tmp/x.jpg\"\n[palette]\nbackground = \"#111111\"\nsurface = \"#222222\"\nforeground = \"#eeeeee\"\naccent = \"#112233\"\non_accent = \"#ffffff\"\n",
        )
        .unwrap();
        assert!(!minimal.glass.enabled);
        assert_eq!(minimal.glass.strength, GlassStrength::Subtle);
        assert!(minimal.icons.match_yaru);
        assert!(!minimal.palette_edited);

        let structured: Theme = toml::from_str(
            "name = \"T\"\nmode = \"dark\"\nwallpaper = \"/tmp/x.jpg\"\n[glass]\nenabled = true\nstrength = \"strong\"\n[icons]\nmatch_yaru = false\n[palette]\nbackground = \"#111111\"\nsurface = \"#222222\"\nforeground = \"#eeeeee\"\naccent = \"#112233\"\non_accent = \"#ffffff\"\n",
        )
        .unwrap();
        assert!(structured.glass.enabled);
        assert_eq!(structured.glass.strength, GlassStrength::Strong);
        assert!(!structured.icons.match_yaru);
    }

    #[test]
    fn rename_roundtrip_in_temp_root() {
        let root = std::env::temp_dir().join("atmosphere-rename-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sunset")).unwrap();
        let theme = Theme {
            name: "Sunset".to_string(),
            mode: ColorMode::Dark,
            wallpaper: PathBuf::from("/home/user/Pictures/Wallpapers/sunset.jpg"),
            palette_edited: false,
            glass: GlassSettings::default(),
            icons: IconsSettings::default(),
            palette: Palette {
                background: "#111111".to_string(),
                surface: "#222222".to_string(),
                foreground: "#eeeeee".to_string(),
                accent: "#112233".to_string(),
                on_accent: "#ffffff".to_string(),
                error: None,
                secondary: None,
                tertiary: None,
            },
        };
        std::fs::write(
            root.join("sunset").join("theme.toml"),
            toml::to_string_pretty(&theme).unwrap(),
        )
        .unwrap();
        std::fs::write(root.join("sunset").join("preview.png"), b"fake").unwrap();

        // Same id, new display name: directory stays, preview preserved.
        let same = rename_theme_in(&root, "sunset", "Sunset Overdrive").unwrap();
        assert_eq!(same, "sunset-overdrive");
        assert!(root.join("sunset-overdrive").join("preview.png").is_file());
        assert!(!root.join("sunset").exists());

        // Duplicate id refused.
        std::fs::create_dir_all(root.join("other")).unwrap();
        std::fs::write(
            root.join("other").join("theme.toml"),
            toml::to_string_pretty(&theme).unwrap(),
        )
        .unwrap();
        assert!(rename_theme_in(&root, "sunset-overdrive", "Other").is_err());

        // Invalid names refused instead of stripped.
        assert!(rename_theme_in(&root, "sunset-overdrive", "bad!name").is_err());
        assert!(rename_theme_in(&root, "missing-id", "Fine Name").is_err());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn default_name_from_stem() {
        assert_eq!(
            default_name_from_wallpaper(Path::new("/home/u/Pictures/Wallpapers/sunset.jpg")),
            "sunset"
        );
        assert_eq!(
            default_name_from_wallpaper(Path::new("/home/u/my photo.png")),
            "my photo"
        );
    }

    /// Manual save/load check (quickstart T032 scenario 3, save half).
    /// Run with an isolated HOME so the real library is never touched:
    /// `HOME=/tmp/atmo-fakehome cargo test -- --ignored live_`
    /// Asserts an edited swatch + flag survive a save/load round-trip.
    #[test]
    #[ignore]
    fn live_save_edit_survival() {
        let home = std::env::var("HOME").expect("HOME must be set");
        assert!(
            !home.is_empty() && home != "/home/kushal" && !home.starts_with("/root"),
            "refusing to run against a real HOME ({home})"
        );
        let theme = Theme {
            name: "Live Edit Probe".to_string(),
            mode: ColorMode::Dark,
            wallpaper: PathBuf::from("/tmp/atmo-fakehome/wall.jpg"),
            palette_edited: true,
            glass: GlassSettings::default(),
            icons: IconsSettings::default(),
            palette: Palette {
                background: "#101010".to_string(),
                surface: "#202020".to_string(),
                foreground: "#f0f0f0".to_string(),
                accent: "#abcdef".to_string(),
                on_accent: "#000000".to_string(),
                error: None,
                secondary: None,
                tertiary: None,
            },
        };
        let id = save_theme(&theme).expect("save");
        assert_eq!(id, "live-edit-probe");
        let back = load_theme(&id).expect("load");
        assert_eq!(back.palette.accent, "#abcdef");
        assert!(back.palette_edited);
        delete_theme(&id).expect("delete");
        assert!(load_theme(&id).is_err());
    }

    #[test]
    fn theme_serialization_roundtrip() {
        let theme = Theme {
            name: "Round Trip".to_string(),
            mode: ColorMode::Dark,
            wallpaper: PathBuf::from("/home/user/Pictures/Wallpapers/x.jpg"),
            palette_edited: true,
            glass: GlassSettings {
                enabled: true,
                strength: GlassStrength::Strong,
            },
            icons: IconsSettings::default(),
            palette: Palette {
                background: "#1b1b1f".to_string(),
                surface: "#201f24".to_string(),
                foreground: "#e4e1e6".to_string(),
                accent: "#c5c0ff".to_string(),
                on_accent: "#2c0091".to_string(),
                error: None,
                secondary: None,
                tertiary: None,
            },
        };
        let text = toml::to_string_pretty(&theme).unwrap();
        let back: Theme = toml::from_str(&text).unwrap();
        assert_eq!(back.name, "Round Trip");
        assert_eq!(back.mode, ColorMode::Dark);
        assert!(back.palette_edited);
        assert!(back.glass.enabled);
        assert!(text.contains("glass"));
    }
}
