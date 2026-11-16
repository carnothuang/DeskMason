//! DeskMason 平台层。
//!
//! 硬边界：只有这个 crate 允许直接接触 X11、Wayland、D-Bus、/proc、inotify。
//! 其余 crate 通过这里的接口使用能力，测试里用假实现或临时目录替换。

pub mod autostart;
pub mod backup;
pub mod cache;
pub mod capture_store;
pub mod display;
pub mod error;
pub mod files;
pub mod hotkey;
pub mod http;
pub mod icons;
pub mod launch;
pub mod layout_store;
pub mod mpris;
pub mod paths;
pub mod prefs;
pub mod settings_store;
pub mod todo_store;
pub mod trash;
pub mod tray;
pub mod wallpaper;
pub mod watch;
pub mod x11;

pub use error::{PlatformError, PlatformResult};
pub use hotkey::{Hotkey, HotkeyBinding};
