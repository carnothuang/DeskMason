//! 文件瓦片的显示设置：图标尺寸、文件名行数、密度、排序。
//!
//! 存在 `WidgetConfig.data["settings"]` 里，随布局文件一起持久化。
//! 新增字段必须是可缺省的，否则旧配置会被判为无效。

use crate::file_tile::FileSortMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Density {
    Compact,
    #[default]
    Normal,
    Comfortable,
}

impl Density {
    pub fn parse(text: &str) -> Self {
        match text {
            "compact" => Density::Compact,
            "comfortable" => Density::Comfortable,
            _ => Density::Normal,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Density::Compact => "compact",
            Density::Normal => "normal",
            Density::Comfortable => "comfortable",
        }
    }

    /// 单元格内边距（图标与文字之外的留白）。
    fn padding(self) -> u32 {
        match self {
            Density::Compact => 6,
            Density::Normal => 12,
            Density::Comfortable => 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileTileSettings {
    /// 图标边长，逻辑像素。
    pub icon_size: u32,
    /// 文件名显示行数：0 隐藏、1 单行、2 双行。
    pub name_lines: u8,
    pub density: Density,
    pub sort_mode: FileSortMode,
}

impl Default for FileTileSettings {
    fn default() -> Self {
        Self {
            icon_size: 48,
            name_lines: 2,
            density: Density::Normal,
            sort_mode: FileSortMode::Name,
        }
    }
}

impl FileTileSettings {
    /// 把任意输入收进合法区间。UI 和磁盘数据都要经过它。
    pub fn clamped(mut self) -> Self {
        self.icon_size = self.icon_size.clamp(24, 96);
        self.name_lines = self.name_lines.min(2);
        self
    }

    /// 单元格尺寸。QML 的 GridView 直接用这两个值，保证 Rust 是唯一真源。
    pub fn cell_size(&self) -> (u32, u32) {
        let icon = self.icon_size;
        let padding = self.density.padding();
        let text_height = match self.name_lines {
            0 => 0,
            1 => 14,
            _ => 28,
        };
        let gaps = if self.name_lines == 0 { 0 } else { 4 };
        (icon + padding * 2, icon + gaps + text_height + padding)
    }

    /// 从 `WidgetConfig.data["settings"]` 读取；缺失或损坏时回默认。
    pub fn from_data(data: &serde_json::Map<String, serde_json::Value>) -> Self {
        data.get("settings")
            .and_then(|value| serde_json::from_value::<FileTileSettings>(value.clone()).ok())
            .unwrap_or_default()
            .clamped()
    }

    pub fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let s = FileTileSettings::default();
        assert_eq!(s.icon_size, 48);
        assert_eq!(s.name_lines, 2);
        assert_eq!(s.density, Density::Normal);
        assert_eq!(s.sort_mode, FileSortMode::Name);
    }

    #[test]
    fn clamps_out_of_range_values() {
        let s = FileTileSettings {
            icon_size: 500,
            name_lines: 9,
            ..Default::default()
        }
        .clamped();
        assert_eq!(s.icon_size, 96);
        assert_eq!(s.name_lines, 2);

        let s = FileTileSettings {
            icon_size: 1,
            ..Default::default()
        }
        .clamped();
        assert_eq!(s.icon_size, 24);
    }

    #[test]
    fn cell_size_shrinks_when_names_hidden() {
        let with_names = FileTileSettings::default().cell_size();
        let without = FileTileSettings {
            name_lines: 0,
            ..Default::default()
        }
        .cell_size();
        assert!(without.1 < with_names.1);
    }

    #[test]
    fn cell_size_grows_with_density() {
        let compact = FileTileSettings {
            density: Density::Compact,
            ..Default::default()
        }
        .cell_size();
        let comfortable = FileTileSettings {
            density: Density::Comfortable,
            ..Default::default()
        }
        .cell_size();
        assert!(comfortable.0 > compact.0);
    }

    #[test]
    fn round_trips_through_widget_data() {
        let settings = FileTileSettings {
            icon_size: 64,
            name_lines: 1,
            density: Density::Compact,
            sort_mode: FileSortMode::Modified,
        };
        let mut data = serde_json::Map::new();
        data.insert("settings".into(), settings.to_value());
        assert_eq!(FileTileSettings::from_data(&data), settings);
    }

    #[test]
    fn missing_or_corrupt_settings_fall_back_to_defaults() {
        let empty = serde_json::Map::new();
        assert_eq!(
            FileTileSettings::from_data(&empty),
            FileTileSettings::default()
        );

        let mut broken = serde_json::Map::new();
        broken.insert("settings".into(), serde_json::json!("not an object"));
        assert_eq!(
            FileTileSettings::from_data(&broken),
            FileTileSettings::default()
        );
    }

    #[test]
    fn partial_settings_keep_other_defaults() {
        let mut data = serde_json::Map::new();
        data.insert("settings".into(), serde_json::json!({"iconSize": 32}));
        let parsed = FileTileSettings::from_data(&data);
        assert_eq!(parsed.icon_size, 32);
        assert_eq!(parsed.name_lines, 2);
    }
}
