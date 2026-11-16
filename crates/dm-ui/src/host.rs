//! 宿主级能力：托盘、全局热键、跨窗口的显隐与命令队列。
//!
//! 托盘与热键跑在各自的线程里。**会碰 Qt 对象**的动作（退出、开设置、开搜索）
//! 用 [`crate::quit_app`] 这类入口直接排到 GUI 线程执行，不再塞进命令队列——
//! 队列是"谁来取谁执行"的，8 个瓦片窗口都在轮询同一个队列，命令会被最先
//! 轮询到的窗口取走；一旦那个窗口没有对应处理分支，请求就永久丢失（这正是
//! "点退出没反应"的原因）。
//!
//! 队列现在只承载新建文件格子这类纯 QML 交互：所有窗口的处理分支完全一致，
//! 被谁取走都能正确执行。

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

/// 内容瓦片是单例，只能从设置页开启；旧的类型新建命令一律引导到设置命令。
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
                        // 内容瓦片是单例，统一从设置页的功能格子开关打开。
                        crate::request_settings_window();
                    }
                }
                // 下面三个都直接排到 GUI 线程执行：走队列会被最先轮询到的
                // 窗口取走，一旦该窗口没有对应分支，请求就丢了。
                dm_platform::tray::TrayAction::Search => crate::request_search_window(),
                dm_platform::tray::TrayAction::Settings => crate::request_settings_window(),
                dm_platform::tray::TrayAction::Quit => crate::quit_app(),
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
