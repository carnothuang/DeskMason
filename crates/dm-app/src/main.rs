//! DeskMason 桌面进程入口。
//!
//! 启动顺序：注册 QML 模块 → 建 Qt 应用与引擎 → 把引擎指针交给 Rust →
//! 按布局为每个瓦片创建一个桌面层窗口 → 进入事件循环。

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine};

fn main() {
    cxx_qt::init_qml_module!("com.deskmason.probe");

    let mut app = QGuiApplication::new();
    let engine = QQmlApplicationEngine::new();

    dm_ui::apply_light_theme();
    dm_ui::install_translations();
    dm_ui::install_app_translations();

    let engine_ptr = engine.as_ptr() as *mut core::ffi::c_void;
    dm_ui::set_engine(engine_ptr);

    // 托盘与全局热键；失败不阻止启动。
    dm_ui::start_host();

    let spawned = dm_ui::spawn_all_tiles();

    if dm_ui::should_show_onboarding() {
        dm_ui::spawn_onboarding();
    }
    // 开发验证：DESKMASON_OPEN=settings|search 直接拉起对应窗口。
    match std::env::var("DESKMASON_OPEN").as_deref() {
        Ok("settings") => dm_ui::open_settings_dev(),
        Ok("search") => dm_ui::open_search_dev(),
        _ => {}
    }
    if spawned == 0 {
        eprintln!("没有创建任何瓦片窗口；检查 X11 会话与 QML 资源");
    } else {
        eprintln!("已创建 {spawned} 个瓦片窗口");
    }

    let code = app.pin_mut().exec();
    std::process::exit(code);
}
