//! 宿主级能力：托盘、全局热键、跨窗口的显隐与命令队列。
//!
//! 托盘与热键跑在各自的线程里，但它们只改原子变量 / 入队命令；
//! 真正动 Qt 对象的动作（新建窗口、退出）由每个窗口的 QML 定时器取走执行。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

use dm_platform::settings_store;

static VISIBLE: AtomicBool = AtomicBool::new(true);
static EPOCH: AtomicI32 = AtomicI32::new(1);
static COMMANDS: Mutex<VecDeque<&'static str>> = Mutex::new(VecDeque::new());

pub fn toggle_visibility() {
    let next = !VISIBLE.load(Ordering::SeqCst);
    VISIBLE.store(next, Ordering::SeqCst);
    EPOCH.fetch_add(1, Ordering::SeqCst);
}

pub fn is_visible() -> bool {
    VISIBLE.load(Ordering::SeqCst)
}

pub fn epoch() -> i32 {
    EPOCH.load(Ordering::SeqCst)
}

/// 内容瓦片是单例，只能从设置页开启；旧的类型新建命令一律引导到设置。
fn kind_new_command(_kind: &'static str) -> &'static str {
    "settings"
}

pub fn push_command(command: &'static str) {
    if let Ok(mut queue) = COMMANDS.lock() {
        queue.push_back(command);
    }
}

pub fn take_command() -> Option<&'static str> {
    COMMANDS.lock().ok()?.pop_front()
}

/// 注册托盘与全局热键。两者失败都不致命：没有托盘也能靠热键用。
pub fn start() -> (bool, bool) {
    let (tx, rx) = std::sync::mpsc::channel::<dm_platform::tray::TrayAction>();

    let tray_ok = match dm_platform::tray::start(tx) {
        Ok(()) => {
            eprintln!("托盘已就绪");
            true
        }
        Err(error) => {
            eprintln!("托盘不可用（继续运行）: {error}");
            false
        }
    };

    std::thread::spawn(move || {
        while let Ok(action) = rx.recv() {
            match action {
                dm_platform::tray::TrayAction::ToggleVisibility => toggle_visibility(),
                dm_platform::tray::TrayAction::NewTile => push_command("new-tile"),
                dm_platform::tray::TrayAction::NewTileKind(kind) => {
                    if kind == "file" {
                        push_command("new-tile");
                    } else {
                        push_command(kind_new_command(kind));
                    }
                }
                dm_platform::tray::TrayAction::Search => push_command("search"),
                dm_platform::tray::TrayAction::Settings => push_command("settings"),
                dm_platform::tray::TrayAction::Quit => push_command("quit"),
            }
        }
    });

    let settings = settings_store::load();
    let hotkey_text = if settings.hotkey.trim().is_empty() {
        settings_store::DEFAULT_HOTKEY.to_string()
    } else {
        settings.hotkey.clone()
    };

    let hotkey_ok = match dm_platform::hotkey::Hotkey::parse(&hotkey_text) {
        Some(hotkey) => match dm_platform::x11::spawn_hotkey_listener(hotkey, toggle_visibility) {
            Ok(()) => {
                eprintln!("全局热键已注册: {}", hotkey.label());
                true
            }
            Err(error) => {
                eprintln!("热键注册失败（继续运行）: {error}");
                false
            }
        },
        None => false,
    };

    (tray_ok, hotkey_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_flips_state_and_bumps_epoch() {
        let before_visible = is_visible();
        let before_epoch = epoch();
        toggle_visibility();
        assert_eq!(is_visible(), !before_visible);
        assert_eq!(epoch(), before_epoch + 1);
        toggle_visibility();
        assert_eq!(is_visible(), before_visible);
    }

    #[test]
    fn commands_are_fifo() {
        // 其它测试也可能往队列里塞东西，先清空。
        while take_command().is_some() {}
        push_command("new-tile");
        push_command("quit");
        assert_eq!(take_command(), Some("new-tile"));
        assert_eq!(take_command(), Some("quit"));
        assert_eq!(take_command(), None);
    }
}
