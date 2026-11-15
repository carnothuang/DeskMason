//! 系统托盘（StatusNotifierItem）。
//!
//! 用 ksni 而不是手写 D-Bus：SNI + dbusmenu 的签名很碎，手写容易在
//! 不同宿主上出现"图标在但菜单打不开"的问题。

use ksni::blocking::TrayMethods;
use std::sync::mpsc::Sender;

/// 托盘菜单能发起的动作。由上层决定怎么执行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ToggleVisibility,
    NewTile,
    /// 按类型新建：storage_key（file/quickCapture/todo/weather/music/glance）。
    NewTileKind(&'static str),
    Search,
    Settings,
    Quit,
}

struct DeskMasonTray {
    tx: Sender<TrayAction>,
}

impl DeskMasonTray {
    fn send(&self, action: TrayAction) {
        // 接收端可能已经退出，忽略错误。
        let _ = self.tx.send(action);
    }
}

impl ksni::Tray for DeskMasonTray {
    fn id(&self) -> String {
        "deskmason".to_string()
    }

    fn title(&self) -> String {
        "DeskMason".to_string()
    }

    fn icon_name(&self) -> String {
        // 主题里没有专属图标，靠下面的 icon_pixmap 兜底。
        "folder".to_string()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![generated_icon(22)]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(TrayAction::ToggleVisibility);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{MenuItem, StandardItem};
        vec![
            StandardItem {
                label: "显示 / 隐藏全部瓦片".to_string(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayAction::ToggleVisibility)),
                ..Default::default()
            }
            .into(),
            // 只保留新建文件格子（会先让用户选文件夹）。内容瓦片是单例，
            // 统一从「设置…」里的功能格子开关开启。
            StandardItem {
                label: "新建文件格子…".to_string(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayAction::NewTileKind("file"))),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "搜索…".to_string(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayAction::Search)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "设置…".to_string(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayAction::Settings)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "退出 DeskMason".to_string(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayAction::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// 启动托盘。失败（比如没有 SNI 宿主）返回错误，调用方决定是否致命。
pub fn start(sender: Sender<TrayAction>) -> Result<(), String> {
    let handle = DeskMasonTray { tx: sender }
        .spawn()
        .map_err(|error| error.to_string())?;
    // 托盘要活到进程结束；Handle 被丢弃可能带走服务线程。
    std::mem::forget(handle);
    Ok(())
}

/// 生成一个 22×22 的图标：深蓝底 + 浅蓝"砖块"网格。
///
/// SNI 的 IconPixmap 是 ARGB32 大端序，这里自己画，避免依赖主题里有哪个图标。
fn generated_icon(size: i32) -> ksni::Icon {
    let size = size.max(8);
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let cell = (size / 4).max(2);

    for y in 0..size {
        for x in 0..size {
            let (r, g, b, a) = if x % cell == 0 && y % cell == 0 {
                // 砖缝
                (0x1B, 0x3A, 0x74, 0xFF)
            } else if x > 1 && y > 1 && x < size - 2 && y < size - 2 {
                (0x4A, 0x8B, 0xF0, 0xFF)
            } else {
                (0x2A, 0x5C, 0xB8, 0xFF)
            };
            // ARGB32，网络字节序
            data.push(a);
            data.push(r);
            data.push(g);
            data.push(b);
        }
    }

    ksni::Icon {
        width: size,
        height: size,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_expected_size() {
        let icon = generated_icon(22);
        assert_eq!(icon.width, 22);
        assert_eq!(icon.height, 22);
        assert_eq!(icon.data.len(), 22 * 22 * 4);
        // alpha 全不透明
        assert!(icon.data.chunks(4).all(|p| p[0] == 0xFF));
    }

    #[test]
    fn icon_clamps_tiny_sizes() {
        let icon = generated_icon(1);
        assert_eq!(icon.width, 8);
        assert_eq!(icon.data.len(), 8 * 8 * 4);
    }
}
