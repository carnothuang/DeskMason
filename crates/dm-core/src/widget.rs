use crate::geometry::{MonitorId, Rect};
use serde::{Deserialize, Serialize};

/// 瓦片类型。
///
/// 瓦片类型枚举，去掉了只用于旧设置迁移的 `Productivity`
/// 与从未落地的占位项。新增类型必须同时提供 `WidgetDescriptor`，否则
/// `WidgetRegistry` 的覆盖测试会失败。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WidgetKind {
    /// 文件夹支撑的文件格子。
    File,
    /// 随记。
    QuickCapture,
    /// 天气。
    Weather,
    /// 待办。
    Todo,
    /// 音乐。
    Music,
    /// 时光。
    Glance,
}

impl WidgetKind {
    pub const ALL: [WidgetKind; 6] = [
        WidgetKind::File,
        WidgetKind::QuickCapture,
        WidgetKind::Weather,
        WidgetKind::Todo,
        WidgetKind::Music,
        WidgetKind::Glance,
    ];

    /// 落盘键。一旦发布就不可更改，否则旧配置全部失效。
    pub fn storage_key(self) -> &'static str {
        match self {
            WidgetKind::File => "file",
            WidgetKind::QuickCapture => "quickCapture",
            WidgetKind::Weather => "weather",
            WidgetKind::Todo => "todo",
            WidgetKind::Music => "music",
            WidgetKind::Glance => "glance",
        }
    }

    /// 本地化键前缀，文案表在 QML 侧。
    pub fn display_name_key(self) -> &'static str {
        match self {
            WidgetKind::File => "widget.file.name",
            WidgetKind::QuickCapture => "widget.quickCapture.name",
            WidgetKind::Weather => "widget.weather.name",
            WidgetKind::Todo => "widget.todo.name",
            WidgetKind::Music => "widget.music.name",
            WidgetKind::Glance => "widget.glance.name",
        }
    }
}

/// 一个瓦片实例的持久化状态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: String,
    pub kind: WidgetKind,
    pub title: String,
    pub rect: Rect,
    pub monitor: MonitorId,
    /// 收藏/置顶等类型私有设置，按类型自行解释，宿主不解析。
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub data: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub collapsed: bool,
    /// 创建顺序，用于同层稳定排序。
    #[serde(default)]
    pub order: i64,
}

impl WidgetConfig {
    pub fn new(id: impl Into<String>, kind: WidgetKind, rect: Rect, monitor: MonitorId) -> Self {
        Self {
            id: id.into(),
            kind,
            title: String::new(),
            rect,
            monitor,
            data: serde_json::Map::new(),
            collapsed: false,
            order: 0,
        }
    }

    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        self.rect.contains(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_keys_are_unique_and_stable() {
        let mut keys: Vec<_> = WidgetKind::ALL.iter().map(|k| k.storage_key()).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len(), "storage_key 出现重复");

        // 这些字符串是磁盘格式的一部分：改动等于破坏旧数据。
        assert_eq!(WidgetKind::File.storage_key(), "file");
        assert_eq!(WidgetKind::QuickCapture.storage_key(), "quickCapture");
        assert_eq!(WidgetKind::Glance.storage_key(), "glance");
    }

    #[test]
    fn kind_serializes_as_camel_case() {
        let value = serde_json::to_string(&WidgetKind::QuickCapture).unwrap();
        assert_eq!(value, "\"quickCapture\"");
        let back: WidgetKind = serde_json::from_str(&value).unwrap();
        assert_eq!(back, WidgetKind::QuickCapture);
    }

    #[test]
    fn widget_config_round_trips_without_data() {
        let config = WidgetConfig::new(
            "w1",
            WidgetKind::File,
            Rect::new(10, 20, 300, 200),
            MonitorId::new("edid:abc"),
        );
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("\"data\""));
        let back: WidgetConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, config);
    }
}
