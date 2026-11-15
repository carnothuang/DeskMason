//! 宿主设置（与瓦片设置分开）：热键、开机自启等全局项。

use crate::paths;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

pub const DEFAULT_HOTKEY: &str = "ctrl+alt+d";

/// 新建文件瓦片时使用的默认外观。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TileDefaults {
    pub icon_size: u32,
    pub name_lines: u8,
    /// compact / normal / comfortable
    pub density: String,
    /// name / modified / size / kind
    pub sort_mode: String,
}

impl Default for TileDefaults {
    fn default() -> Self {
        Self {
            icon_size: 48,
            name_lines: 2,
            density: "normal".to_string(),
            sort_mode: "name".to_string(),
        }
    }
}

impl TileDefaults {
    pub fn clamped(mut self) -> Self {
        self.icon_size = self.icon_size.clamp(24, 96);
        self.name_lines = self.name_lines.min(2);
        if !matches!(self.density.as_str(), "compact" | "normal" | "comfortable") {
            self.density = "normal".to_string();
        }
        if !matches!(
            self.sort_mode.as_str(),
            "name" | "modified" | "size" | "kind"
        ) {
            self.sort_mode = "name".to_string();
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HostSettings {
    /// 显示/隐藏全部瓦片的热键。
    pub hotkey: String,
    /// 是否随桌面登录自启。
    pub autostart: bool,
    /// 新建瓦片的默认外观。
    pub tile_defaults: TileDefaults,
    /// 是否已看过首次运行引导。
    pub onboarded: bool,
}

impl Default for HostSettings {
    fn default() -> Self {
        Self {
            hotkey: DEFAULT_HOTKEY.to_string(),
            autostart: false,
            tile_defaults: TileDefaults::default(),
            onboarded: false,
        }
    }
}

pub fn settings_path() -> PathBuf {
    paths::config_dir().join("settings.json")
}

pub fn load() -> HostSettings {
    let path = settings_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(settings: &HostSettings) -> io::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json =
        serde_json::to_string_pretty(settings).map_err(|e| io::Error::other(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_stable() {
        let settings = HostSettings::default();
        assert_eq!(settings.hotkey, "ctrl+alt+d");
        assert!(!settings.autostart);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let parsed: HostSettings =
            serde_json::from_str(r#"{"hotkey":"super+space","unknown":1}"#).unwrap();
        assert_eq!(parsed.hotkey, "super+space");
    }

    #[test]
    fn partial_settings_keep_defaults() {
        let parsed: HostSettings = serde_json::from_str(r#"{"autostart":true}"#).unwrap();
        assert_eq!(parsed.hotkey, DEFAULT_HOTKEY);
        assert!(parsed.autostart);
    }

    #[test]
    fn tile_defaults_are_clamped() {
        let defaults = TileDefaults {
            icon_size: 500,
            name_lines: 9,
            density: "weird".into(),
            sort_mode: "unknown".into(),
        }
        .clamped();
        assert_eq!(defaults.icon_size, 96);
        assert_eq!(defaults.name_lines, 2);
        assert_eq!(defaults.density, "normal");
        assert_eq!(defaults.sort_mode, "name");
    }

    #[test]
    fn old_settings_without_tile_defaults_still_load() {
        let parsed: HostSettings =
            serde_json::from_str(r#"{"hotkey":"ctrl+alt+d","autostart":true}"#).unwrap();
        assert_eq!(parsed.tile_defaults, TileDefaults::default());
    }

    #[test]
    fn settings_path_lives_under_config() {
        assert!(settings_path().ends_with("deskmason/settings.json"));
    }
}
