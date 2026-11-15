//! 简单的 JSON 缓存：一个 key 一个文件，放在 `~/.cache/deskmason` 下。

use crate::error::{PlatformError, PlatformResult};
use std::path::PathBuf;

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("deskmason")
}

fn path_for(key: &str) -> PathBuf {
    // 只保留安全字符，避免 key 里出现路径分隔符。
    let safe: String = key
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    cache_dir().join(format!("{safe}.json"))
}

pub fn write(key: &str, json: &str) -> PlatformResult<()> {
    let path = path_for(key);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(PlatformError::Io)?;
    }
    std::fs::write(&path, json).map_err(PlatformError::Io)
}

pub fn read(key: &str) -> Option<String> {
    std::fs::read_to_string(path_for(key)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_sanitised() {
        let path = path_for("../etc/passwd");
        assert!(!path.to_string_lossy().contains(".."));
        assert!(path.to_string_lossy().ends_with("__etc_passwd.json"));
    }
}
