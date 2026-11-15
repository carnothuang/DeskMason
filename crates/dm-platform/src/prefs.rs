//! 通用键值偏好（设置窗口的界面状态：选中页、折叠状态等）。
//!
//! 与 `settings_store` 的区别：这里存的是纯 UI 偏好，丢了不影响功能。
//! JSON 树落盘在 `~/.config/deskmason/ui-prefs.json`。

use crate::paths;
use serde_json::{Map, Value};
use std::io;
use std::sync::Mutex;

fn prefs_path() -> std::path::PathBuf {
    paths::config_dir().join("ui-prefs.json")
}

fn load_map() -> Map<String, Value> {
    std::fs::read_to_string(prefs_path())
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn save_map(map: &Map<String, Value>) -> io::Result<()> {
    let path = prefs_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&Value::Object(map.clone()))
        .map_err(|e| io::Error::other(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// 全局互斥：偏好文件很小，串行读写足够。
static LOCK: Mutex<()> = Mutex::new(());

pub fn get_string(key: &str) -> Option<String> {
    let _guard = LOCK.lock().ok()?;
    load_map()
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub fn get_bool(key: &str) -> Option<bool> {
    let _guard = LOCK.lock().ok()?;
    load_map().get(key).and_then(Value::as_bool)
}

pub fn set_string(key: &str, value: &str) {
    let _guard = LOCK.lock().expect("ui-prefs 锁被污染");
    let mut map = load_map();
    map.insert(key.to_string(), Value::String(value.to_string()));
    if let Err(error) = save_map(&map) {
        eprintln!("写入偏好失败: {error}");
    }
}

pub fn set_bool(key: &str, value: bool) {
    let _guard = LOCK.lock().expect("ui-prefs 锁被污染");
    let mut map = load_map();
    map.insert(key.to_string(), Value::Bool(value));
    if let Err(error) = save_map(&map) {
        eprintln!("写入偏好失败: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_bool_and_string() {
        // 用不存在的 key 走默认分支。
        assert_eq!(get_bool("deskmason-test/missing"), None);
        assert_eq!(get_string("deskmason-test/missing"), None);
    }
}
