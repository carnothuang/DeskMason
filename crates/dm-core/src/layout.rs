use crate::geometry::{MonitorId, Rect};
use crate::widget::WidgetConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 磁盘格式版本。任何破坏兼容的改动都必须 +1 并提供迁移。
///
/// v1 → v2：新增 `active_topology` 与 `placements`，用于"按显示器组合记忆布局"。
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MonitorLayout {
    pub bounds: Rect,
    /// 逻辑像素比例（1.0 / 1.25 / 1.5 / 2.0 ...）。
    pub scale: f64,
}

/// 某个瓦片在某个"显示器组合"下的摆放。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub rect: Rect,
    pub monitor: MonitorId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub monitors: BTreeMap<MonitorId, MonitorLayout>,
    #[serde(default)]
    pub widgets: Vec<WidgetConfig>,
    /// 当前生效的显示器组合键（各显示器 id 排序后拼接）。
    #[serde(default)]
    pub active_topology: String,
    /// 组合键 → 瓦片 id → 摆放。换屏组合时整批保存/恢复。
    #[serde(default)]
    pub placements: BTreeMap<String, BTreeMap<String, Placement>>,
}

impl Default for LayoutDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutDocument {
    pub fn new() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            revision: 0,
            monitors: BTreeMap::new(),
            widgets: Vec::new(),
            active_topology: String::new(),
            placements: BTreeMap::new(),
        }
    }

    pub fn from_json(text: &str) -> Result<Self, LayoutError> {
        // 先按无版本号读，兼容 v0（早期格式没有 schemaVersion）。
        let raw: RawDocument = serde_json::from_str(text)?;
        let mut doc = raw.migrate()?;
        doc.normalize();
        Ok(doc)
    }

    pub fn to_json(&self) -> Result<String, LayoutError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 写入后必须递增 revision，宿主用它判断"是否被外部改过"。
    pub fn bump(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    pub fn widget(&self, id: &str) -> Option<&WidgetConfig> {
        self.widgets.iter().find(|w| w.id == id)
    }

    pub fn widget_mut(&mut self, id: &str) -> Option<&mut WidgetConfig> {
        self.widgets.iter_mut().find(|w| w.id == id)
    }

    /// 插入或替换，保留创建顺序。
    pub fn upsert_widget(&mut self, mut widget: WidgetConfig) -> bool {
        if let Some(existing) = self.widget_mut(&widget.id) {
            widget.order = existing.order;
            *existing = widget;
            true
        } else {
            widget.order = self.widgets.iter().map(|w| w.order).max().unwrap_or(-1) + 1;
            self.widgets.push(widget);
            false
        }
    }

    pub fn remove_widget(&mut self, id: &str) -> bool {
        let before = self.widgets.len();
        self.widgets.retain(|w| w.id != id);
        before != self.widgets.len()
    }

    /// 显示器消失后，把它名下的瓦片迁到 fallback，并按可用区域缩放位置。
    pub fn relocate_monitor(&mut self, from: &MonitorId, to: &MonitorId) -> usize {
        let target = self.monitors.get(to).map(|m| m.bounds);
        let mut moved = 0;
        for widget in self.widgets.iter_mut().filter(|w| &w.monitor == from) {
            widget.monitor = to.clone();
            if let Some(bounds) = target {
                widget.rect = widget.rect.clamp_into(&bounds);
            }
            moved += 1;
        }
        moved
    }

    /// 读盘后的自愈：丢掉重复 id、保证顺序稳定。
    fn normalize(&mut self) {
        self.widgets
            .sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));
        let mut seen = std::collections::HashSet::new();
        self.widgets.retain(|w| seen.insert(w.id.clone()));
    }
}

/// 兼容读入的中间形态。
#[derive(Deserialize)]
struct RawDocument {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    monitors: BTreeMap<MonitorId, MonitorLayout>,
    #[serde(default)]
    widgets: Vec<WidgetConfig>,
    #[serde(default)]
    active_topology: String,
    #[serde(default)]
    placements: BTreeMap<String, BTreeMap<String, Placement>>,
}

impl RawDocument {
    fn migrate(self) -> Result<LayoutDocument, LayoutError> {
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(LayoutError::UnsupportedVersion {
                found: self.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        // v0/v1 -> v2：字段都有默认值，只需要补版本号。
        Ok(LayoutDocument {
            schema_version: CURRENT_SCHEMA_VERSION,
            revision: self.revision,
            monitors: self.monitors,
            widgets: self.widgets,
            active_topology: self.active_topology,
            placements: self.placements,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("布局 JSON 无效: {0}")]
    Json(#[from] serde_json::Error),
    #[error("布局版本 {found} 高于本程序支持的 {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::WidgetKind;

    fn doc_with_one_widget() -> LayoutDocument {
        let mut doc = LayoutDocument::new();
        doc.monitors.insert(
            MonitorId::new("edid:a"),
            MonitorLayout {
                bounds: Rect::new(0, 0, 1920, 1080),
                scale: 1.0,
            },
        );
        doc.upsert_widget(WidgetConfig::new(
            "w1",
            WidgetKind::File,
            Rect::new(100, 100, 360, 480),
            MonitorId::new("edid:a"),
        ));
        doc
    }

    #[test]
    fn round_trips_through_json() {
        let doc = doc_with_one_widget();
        let text = doc.to_json().unwrap();
        let back = LayoutDocument::from_json(&text).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn migrates_v1_to_v2() {
        let text = r#"{"schema_version":1,"widgets":[],"monitors":{}}"#;
        let doc = LayoutDocument::from_json(text).unwrap();
        assert_eq!(doc.schema_version, 2);
        assert!(doc.active_topology.is_empty());
        assert!(doc.placements.is_empty());
    }

    #[test]
    fn placements_round_trip() {
        let mut doc = doc_with_one_widget();
        let mut placements = BTreeMap::new();
        placements.insert(
            "a|b".to_string(),
            BTreeMap::from([(
                "w1".to_string(),
                Placement {
                    rect: Rect::new(5, 6, 7, 8),
                    monitor: MonitorId::new("edid:a"),
                },
            )]),
        );
        doc.placements = placements;
        doc.active_topology = "a|b".to_string();

        let text = doc.to_json().unwrap();
        let back = LayoutDocument::from_json(&text).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn migrates_v0_without_schema_version() {
        let text = r#"{"widgets":[{"id":"w1","kind":"file","title":"","rect":{"x":0,"y":0,"width":100,"height":100},"monitor":"edid:a","collapsed":false,"order":0}]}"#;
        let doc = LayoutDocument::from_json(text).unwrap();
        assert_eq!(doc.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(doc.widgets.len(), 1);
    }

    #[test]
    fn rejects_future_schema() {
        let text = format!(
            "{{\"schema_version\":{},\"widgets\":[]}}",
            CURRENT_SCHEMA_VERSION + 1
        );
        assert!(matches!(
            LayoutDocument::from_json(&text),
            Err(LayoutError::UnsupportedVersion { .. })
        ));
    }

    #[test]
    fn duplicate_ids_are_dropped_and_order_is_stable() {
        let text = r#"{"schema_version":1,"widgets":[
            {"id":"b","kind":"file","title":"","rect":{"x":0,"y":0,"width":10,"height":10},"monitor":"m","collapsed":false,"order":2},
            {"id":"a","kind":"file","title":"","rect":{"x":0,"y":0,"width":10,"height":10},"monitor":"m","collapsed":false,"order":1},
            {"id":"a","kind":"todo","title":"","rect":{"x":0,"y":0,"width":10,"height":10},"monitor":"m","collapsed":false,"order":3}
        ]}"#;
        let doc = LayoutDocument::from_json(text).unwrap();
        assert_eq!(
            doc.widgets
                .iter()
                .map(|w| w.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn relocating_a_monitor_clamps_widgets_into_target() {
        let mut doc = doc_with_one_widget();
        doc.monitors.insert(
            MonitorId::new("edid:b"),
            MonitorLayout {
                bounds: Rect::new(0, 0, 800, 600),
                scale: 1.0,
            },
        );
        doc.widget("w1").unwrap();
        let moved = doc.relocate_monitor(&MonitorId::new("edid:a"), &MonitorId::new("edid:b"));
        assert_eq!(moved, 1);
        let rect = doc.widget("w1").unwrap().rect;
        assert_eq!(rect, Rect::new(100, 100, 360, 480));
        assert_eq!(doc.widget("w1").unwrap().monitor, MonitorId::new("edid:b"));
    }

    #[test]
    fn upsert_preserves_creation_order() {
        let mut doc = doc_with_one_widget();
        let mut replacement = doc.widget("w1").unwrap().clone();
        replacement.title = "renamed".into();
        let replaced = doc.upsert_widget(replacement);
        assert!(replaced);
        assert_eq!(doc.widget("w1").unwrap().title, "renamed");
        assert_eq!(doc.widgets[0].order, 0);
    }
}
