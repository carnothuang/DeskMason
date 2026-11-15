//! 随记的领域规则：条目模型、排序。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CaptureKind {
    #[default]
    Text,
    Link,
}

impl CaptureKind {
    pub fn parse(text: &str) -> Self {
        match text {
            "link" => CaptureKind::Link,
            _ => CaptureKind::Text,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            CaptureKind::Text => "text",
            CaptureKind::Link => "link",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureItem {
    pub id: String,
    pub kind: CaptureKind,
    pub content: String,
    pub pinned: bool,
    pub created_at: i64,
}

/// 识别纯链接，方便给不同的图标/排版。
pub fn detect_kind(content: &str) -> CaptureKind {
    let trimmed = content.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        CaptureKind::Link
    } else {
        CaptureKind::Text
    }
}

/// 置顶在前，其余按创建时间倒序。
pub fn sort_items(items: &mut [CaptureItem]) {
    items.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| b.created_at.cmp(&a.created_at))
    });
}

/// 列表里显示的一行摘要：压平换行、截断。
pub fn preview(content: &str, max_chars: usize) -> String {
    let flat: String = content
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let trimmed = flat.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, pinned: bool, created_at: i64) -> CaptureItem {
        CaptureItem {
            id: id.to_string(),
            kind: CaptureKind::Text,
            content: id.to_string(),
            pinned,
            created_at,
        }
    }

    #[test]
    fn links_are_detected() {
        assert_eq!(detect_kind("https://example.com"), CaptureKind::Link);
        assert_eq!(detect_kind("  http://a.b"), CaptureKind::Link);
        assert_eq!(detect_kind("普通文本"), CaptureKind::Text);
    }

    #[test]
    fn pinned_items_come_first() {
        let mut items = vec![
            item("old", false, 10),
            item("pinned", true, 1),
            item("new", false, 20),
        ];
        sort_items(&mut items);
        let ids: Vec<_> = items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["pinned", "new", "old"]);
    }

    #[test]
    fn preview_flattens_and_truncates() {
        assert_eq!(preview("a\nb", 10), "a b");
        assert_eq!(preview("一二三四五六", 4), "一二三…");
        assert_eq!(preview("  pad  ", 10), "pad");
    }

    #[test]
    fn kind_round_trips() {
        for kind in [CaptureKind::Text, CaptureKind::Link] {
            assert_eq!(CaptureKind::parse(kind.as_str()), kind);
        }
    }
}
