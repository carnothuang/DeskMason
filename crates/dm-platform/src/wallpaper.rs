//! 当前桌面壁纸（deepin / DDE）。
//!
//! DDE 把每块屏、每个工作区的壁纸写在
//! `~/.config/dde-appearance/config.json` 的 `wallpaperInfo` 里，
//! 值是 `file://` URI。这里只做只读解析，不依赖 D-Bus / gsettings
//! （容器里没有对应的 schema 与会话总线）。
//!
//! 沙盒里宿主的 `/var`、`/usr` 挂在 `/host` 下，所以原始路径不存在时
//! 会再试一次带 `/host` 前缀的路径。

use crate::paths;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// 返回当前桌面壁纸在本地可直接读取的图片路径。
pub fn desktop_wallpaper() -> Option<PathBuf> {
    let config = paths::home_dir().join(".config/dde-appearance/config.json");
    let text = std::fs::read_to_string(config).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    for uri in candidate_uris(&value) {
        if let Some(path) = resolve_uri(&uri) {
            return Some(path);
        }
    }
    None
}

/// 校验用户自定义背景路径；只接受已存在的本地图片文件。
pub fn resolve_image(path: &str) -> Option<PathBuf> {
    if path.is_empty() {
        return None;
    }
    let resolved = resolve_uri(path)?;
    let ext = resolved
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif"
    ) {
        return None;
    }
    Some(resolved)
}

/// 按优先级收集候选 URI：当前工作区(1+)的用户自定义壁纸 > 任意自定义 >
/// 当前工作区的系统壁纸 > 第一条。
fn candidate_uris(value: &Value) -> Vec<String> {
    let mut workspace = Vec::new();
    let mut custom = Vec::new();
    let mut system = Vec::new();

    let Some(sections) = value.as_array() else {
        return Vec::new();
    };
    for section in sections {
        let Some(entries) = section.get("wallpaperInfo").and_then(Value::as_array) else {
            continue;
        };
        for entry in entries {
            let Some(uri) = entry.get("uri").and_then(Value::as_str) else {
                continue;
            };
            let index = entry.get("wpIndex").and_then(Value::as_str).unwrap_or("");
            let is_custom = entry
                .get("custom")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let is_workspace = index.starts_with("1+");
            if is_custom && is_workspace {
                workspace.push(uri.to_string());
            } else if is_custom {
                custom.push(uri.to_string());
            } else {
                // 系统壁纸不区分工作区索引，按配置顺序作为兜底。
                system.push(uri.to_string());
            }
        }
    }
    workspace.extend(custom);
    workspace.extend(system);
    workspace
}

/// `file://` URI / 普通路径 → 本地可读路径（含 `/host` 兜底）。
fn resolve_uri(uri: &str) -> Option<PathBuf> {
    let raw = uri.strip_prefix("file://").unwrap_or(uri);
    let raw = raw.replace("%20", " ");
    let path = PathBuf::from(&raw);
    if path.is_file() {
        return Some(path);
    }
    let hosted = Path::new("/host").join(raw.trim_start_matches('/'));
    if hosted.is_file() {
        return Some(hosted);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
        {"type":"index+monitorName","wallpaperInfo":[
            {"custom":false,"uri":"file:///usr/share/wallpapers/deepin/other.jpg","wpIndex":"2+eDP-1"},
            {"custom":true,"uri":"file:///var/cache/wallpapers/can/mine.jpg","wpIndex":"1+eDP-1"},
            {"custom":true,"uri":"file:///var/cache/wallpapers/can/other.jpg","wpIndex":"1+DP-1"}
        ]}
    ]"#;

    #[test]
    fn prefers_current_workspace_custom_wallpaper() {
        let value: Value = serde_json::from_str(SAMPLE).unwrap();
        let uris = candidate_uris(&value);
        assert_eq!(uris[0], "file:///var/cache/wallpapers/can/mine.jpg");
        assert_eq!(uris[1], "file:///var/cache/wallpapers/can/other.jpg");
    }

    #[test]
    fn falls_back_to_system_wallpaper() {
        let value: Value = serde_json::from_str(
            r#"[{"wallpaperInfo":[{"custom":false,"uri":"file:///usr/share/wallpapers/a.jpg","wpIndex":"2+eDP-1"}]}]"#,
        )
        .unwrap();
        let uris = candidate_uris(&value);
        assert_eq!(uris.len(), 1);
        assert_eq!(uris[0], "file:///usr/share/wallpapers/a.jpg");
    }

    #[test]
    fn bad_config_yields_no_candidates() {
        let value: Value = serde_json::from_str(r#"{"wallpaperInfo":[]}"#).unwrap();
        assert!(candidate_uris(&value).is_empty());
    }

    #[test]
    fn missing_file_is_not_resolved() {
        assert!(resolve_uri("file:///definitely/not/here.jpg").is_none());
        assert!(resolve_image("").is_none());
    }
}
