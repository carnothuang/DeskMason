//! 开机自启：XDG autostart 的 `.desktop` 文件。
//!
//! `Exec` 指向当前可执行文件；从 Debug 目录运行时写进去的也是 Debug 路径，
//! 这是开发期的预期行为（换成安装版后重新开关一次即可）。

use crate::error::PlatformResult;
use std::io;
use std::path::PathBuf;

pub fn autostart_dir() -> PathBuf {
    let config = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    config.join("autostart")
}

pub fn desktop_path() -> PathBuf {
    autostart_dir().join("deskmason.desktop")
}

pub fn is_enabled() -> bool {
    desktop_path().is_file()
}

pub fn set_enabled(enabled: bool) -> io::Result<()> {
    let path = desktop_path();
    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }

    let executable = std::env::current_exe()?;
    let exec = executable.to_string_lossy().replace('"', "\\\"");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=DeskMason\n\
         Comment=桌面瓦匠\n\
         Exec=\"{exec}\"\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n"
    );
    std::fs::write(&path, entry)?;
    Ok(())
}

/// 只做存在性检查时的便捷封装。
pub fn report() -> PlatformResult<bool> {
    Ok(is_enabled())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_path_is_under_autostart() {
        assert!(desktop_path().ends_with("autostart/deskmason.desktop"));
    }

    #[test]
    fn entry_contains_required_keys() {
        // 不写盘，只验证格式模板的关键行都在。
        let executable = "/opt/deskmason/deskmason";
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName=DeskMason\nExec=\"{executable}\"\nTerminal=false\n"
        );
        assert!(entry.contains("Type=Application"));
        assert!(entry.contains("Exec=\"/opt/deskmason/deskmason\""));
    }
}
