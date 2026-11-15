//! XDG 路径。所有落盘位置只能从这里取，方便测试与打包时替换。

use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// `~/.config/deskmason`
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("deskmason")
}

/// `~/.config/deskmason/widget-layout.json`
pub fn layout_path() -> PathBuf {
    config_dir().join("widget-layout.json")
}

/// 新建瓦片时的默认目录：桌面存在就用桌面，否则用主目录。
pub fn default_folder() -> PathBuf {
    let desktop = dirs::desktop_dir().unwrap_or_else(|| home_dir().join("Desktop"));
    if desktop.is_dir() {
        desktop
    } else {
        home_dir()
    }
}

/// `~/.local/share/Trash`
pub fn trash_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| home_dir().join(".local/share"))
        .join("Trash")
}

/// 图标主题的搜索根：先用户目录，再系统目录。
///
/// 沙盒/容器里 `/usr/share/icons` 可能只有残缺的 hicolor（Deepin 的 bloom
/// 在宿主 `/host/usr/share/icons`），DDE 桌面图标全靠 bloom；把常见宿主
/// 挂载点也排进搜索链，目录不存在时自动跳过，真实环境下无副作用。
pub fn icon_roots() -> Vec<PathBuf> {
    let home = home_dir();
    let data = dirs::data_dir().unwrap_or_else(|| home.join(".local/share"));
    let mut roots = vec![
        home.join(".icons"),
        data.join("icons"),
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/local/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
    ];
    // 容器/沙盒兜底：宿主的 icons 与 pixmaps（存在才加入）。
    for extra in ["/host/usr/share/icons", "/host/usr/share/pixmaps"] {
        let path = PathBuf::from(extra);
        if path.is_dir() {
            roots.push(path);
        }
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_lives_under_config_dir() {
        let layout = layout_path();
        assert!(layout.ends_with("deskmason/widget-layout.json"));
    }

    #[test]
    fn icon_roots_start_with_user_dirs() {
        let roots = icon_roots();
        assert!(roots.iter().any(|p| p.ends_with(".icons")));
        assert!(roots
            .iter()
            .any(|p| p == &PathBuf::from("/usr/share/icons")));
    }
}
