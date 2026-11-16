use crate::geometry::{MonitorId, Rect};
use crate::widget::WidgetConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 磁盘格式版本。任何破坏兼容的改动都必须 +1 并提供迁移。
///
/// v1 → v2：新增 `active_topology` 与 `placements`，用于"按显示器组合记忆布局"。
/// v2 → v3：新增 `layout_resolution` 与 `resolution_layouts`，用于"按主屏
///          分辨率记忆布局"——同一块屏换分辨率（或换屏）后恢复各自归档，
///          分辨率没有归档时把旧布局按新屏重排，而不是把瓦片堆回原点。
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

/// 主屏分辨率键："宽x高"，如 "2560x1600"。只关心分辨率，不关心接口
/// 名称（HDMI-1 / eDP-1 会随热插拔变化）与摆放（上下拼 / 左右拼）。
pub type ResolutionKey = String;

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

/// 一个分辨率下的整套布局归档。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolutionLayout {
    /// 归档时的瓦片摆设（id → 摆放）。
    pub widgets: BTreeMap<String, Placement>,
    /// 归档时的副屏拓扑键（原样记录，供诊断；不参与匹配）。
    #[serde(default)]
    pub topology: String,
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
    /// 当前布局对应的**主屏分辨率**（"宽x高"）。为空表示旧格式文件
    /// 或尚未采集——启动/reconcile 时会按规则收养。
    #[serde(default)]
    pub layout_resolution: ResolutionKey,
    /// 主屏分辨率 → 该分辨率下的整套布局归档。
    #[serde(default)]
    pub resolution_layouts: BTreeMap<ResolutionKey, ResolutionLayout>,
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
            layout_resolution: String::new(),
            resolution_layouts: BTreeMap::new(),
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

    // ------------------------------------------------------------ 分辨率归档

    /// 把当前所有瓦片的摆设归档到指定主屏分辨率名下，并把它标为当前布局
    /// 的分辨率。之后切换分辨率时可以从这份归档原样恢复。
    pub fn archive_to_resolution(&mut self, key: &str, topology: &str) {
        let mut widgets = BTreeMap::new();
        for widget in &self.widgets {
            widgets.insert(
                widget.id.clone(),
                Placement {
                    rect: widget.rect,
                    monitor: widget.monitor.clone(),
                },
            );
        }
        self.resolution_layouts.insert(
            key.to_string(),
            ResolutionLayout {
                widgets,
                topology: topology.to_string(),
            },
        );
        self.layout_resolution = key.to_string();
    }

    /// 当前摆设是否与某分辨率的归档完全一致（一致就不必重复写盘）。
    pub fn matches_resolution_archive(&self, key: &str) -> bool {
        if self.layout_resolution != key {
            return false;
        }
        match self.resolution_layouts.get(key) {
            Some(archive) => {
                archive.widgets.len() == self.widgets.len()
                    && self.widgets.iter().all(|w| {
                        archive
                            .widgets
                            .get(&w.id)
                            .is_some_and(|p| p.rect == w.rect && p.monitor == w.monitor)
                    })
            }
            // 没有归档条目：首次落盘时建立，视为不一致。
            None => false,
        }
    }

    /// 从某分辨率的归档恢复摆设。只恢复文档里现存瓦片；归档里多余的
    /// 条目（瓦片已删除）跳过。返回是否改动过任何瓦片。
    pub fn restore_from_resolution(&mut self, key: &str) -> bool {
        let Some(archive) = self.resolution_layouts.get(key) else {
            return false;
        };
        let mut changed = false;
        for widget in self.widgets.iter_mut() {
            if let Some(placement) = archive.widgets.get(&widget.id) {
                if widget.rect != placement.rect || widget.monitor != placement.monitor {
                    widget.rect = placement.rect;
                    widget.monitor = placement.monitor.clone();
                    changed = true;
                }
            }
        }
        if changed || self.layout_resolution != key {
            self.layout_resolution = key.to_string();
        }
        changed
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
    #[serde(default)]
    layout_resolution: ResolutionKey,
    #[serde(default)]
    resolution_layouts: BTreeMap<ResolutionKey, ResolutionLayout>,
}

impl RawDocument {
    fn migrate(self) -> Result<LayoutDocument, LayoutError> {
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(LayoutError::UnsupportedVersion {
                found: self.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        // v0/v1/v2 -> v3：新增字段都有默认值，只需要补版本号。
        Ok(LayoutDocument {
            schema_version: CURRENT_SCHEMA_VERSION,
            revision: self.revision,
            monitors: self.monitors,
            widgets: self.widgets,
            active_topology: self.active_topology,
            placements: self.placements,
            layout_resolution: self.layout_resolution,
            resolution_layouts: self.resolution_layouts,
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
        assert_eq!(doc.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(doc.active_topology.is_empty());
        assert!(doc.placements.is_empty());
        // v2 → v3 新增字段：旧文件读入后为空，等待启动收养。
        assert!(doc.layout_resolution.is_empty());
        assert!(doc.resolution_layouts.is_empty());
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
