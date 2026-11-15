//! DeskMason 的 QML 桥。
//!
//! 两个类型：
//! - `FileTile`：一个文件夹瓦片的内容与操作。
//! - `AppState`：窗口几何、瓦片创建。
//!
//! 业务规则在 `dm-core`，文件系统与窗口在 `dm-platform`；这里只做类型转换与接线。

#[cxx_qt::bridge]
mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, widget_id, cxx_name = "widgetId")]
        #[qproperty(QString, folder_path, cxx_name = "folderPath")]
        #[qproperty(QString, title)]
        #[qproperty(QStringList, names)]
        #[qproperty(QStringList, icon_paths, cxx_name = "iconPaths")]
        #[qproperty(QStringList, kinds)]
        #[qproperty(QStringList, paths)]
        #[qproperty(QString, status_text, cxx_name = "statusText")]
        #[qproperty(i32, item_count, cxx_name = "itemCount")]
        #[qproperty(i32, icon_size, cxx_name = "iconSize")]
        #[qproperty(i32, name_lines, cxx_name = "nameLines")]
        #[qproperty(QString, density)]
        #[qproperty(QString, sort_mode, cxx_name = "sortMode")]
        #[qproperty(i32, cell_width, cxx_name = "cellWidth")]
        #[qproperty(i32, cell_height, cxx_name = "cellHeight")]
        type FileTile = super::FileTileRust;

        /// 绑定到布局里的某个瓦片，并从磁盘读取目录与设置。
        #[qinvokable]
        fn attach(self: Pin<&mut FileTile>, widget_id: &QString);

        #[qinvokable]
        fn set_folder(self: Pin<&mut FileTile>, path: &QString);

        #[qinvokable]
        fn choose_icon_size(self: Pin<&mut FileTile>, value: i32);

        #[qinvokable]
        fn choose_name_lines(self: Pin<&mut FileTile>, value: i32);

        #[qinvokable]
        fn choose_density(self: Pin<&mut FileTile>, value: &QString);

        #[qinvokable]
        fn choose_sort_mode(self: Pin<&mut FileTile>, value: &QString);

        #[qinvokable]
        fn refresh(self: Pin<&mut FileTile>);

        #[qinvokable]
        fn tick(self: Pin<&mut FileTile>);

        #[qinvokable]
        fn open_item(self: &FileTile, index: i32);

        #[qinvokable]
        fn reveal_item(self: &FileTile, index: i32);

        #[qinvokable]
        fn reveal_folder(&self);

        #[qinvokable]
        fn rename_item(self: Pin<&mut FileTile>, index: i32, new_name: &QString);

        #[qinvokable]
        fn trash_item(self: Pin<&mut FileTile>, index: i32);

        #[qinvokable]
        fn file_url(&self, index: i32) -> QString;

        #[qinvokable]
        fn import_urls(self: Pin<&mut FileTile>, urls: &QStringList);

        /// 拖到某个子文件夹上：移动到该子文件夹。
        #[qinvokable]
        fn import_urls_into(self: Pin<&mut FileTile>, urls: &QStringList, index: i32);

        /// 在当前目录新建文件夹。
        #[qinvokable]
        fn create_folder(self: Pin<&mut FileTile>, name: &QString);

        #[qinvokable]
        fn copy_item(self: Pin<&mut FileTile>, index: i32);

        #[qinvokable]
        fn cut_item(self: Pin<&mut FileTile>, index: i32);

        /// 批量复制。参数是 `file://` URL 列表，避免用下标（刷新后会失效）。
        #[qinvokable]
        fn copy_urls(self: Pin<&mut FileTile>, urls: &QStringList);

        #[qinvokable]
        fn cut_urls(self: Pin<&mut FileTile>, urls: &QStringList);

        /// 批量删除到回收站。参数是绝对路径列表。
        #[qinvokable]
        fn trash_paths(self: Pin<&mut FileTile>, paths: &QStringList);

        /// 把剪贴板里的文件复制/移动进当前目录。
        #[qinvokable]
        fn paste(self: Pin<&mut FileTile>);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, title)]
        #[qproperty(QString, artist)]
        #[qproperty(QString, album)]
        #[qproperty(QString, art_url, cxx_name = "artUrl")]
        #[qproperty(QString, status_text, cxx_name = "statusText")]
        #[qproperty(QString, progress_text, cxx_name = "progressText")]
        #[qproperty(bool, playing)]
        #[qproperty(bool, can_next, cxx_name = "canNext")]
        #[qproperty(bool, can_previous, cxx_name = "canPrevious")]
        type MusicContent = super::MusicContentRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut MusicContent>);

        #[qinvokable]
        fn play_pause(self: Pin<&mut MusicContent>);

        #[qinvokable]
        fn next_track(self: Pin<&mut MusicContent>);

        #[qinvokable]
        fn previous_track(self: Pin<&mut MusicContent>);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QStringList, names)]
        #[qproperty(QStringList, paths)]
        #[qproperty(QStringList, kinds)]
        #[qproperty(QStringList, icon_paths, cxx_name = "iconPaths")]
        #[qproperty(QString, count_text, cxx_name = "countText")]
        #[qproperty(QString, status_text, cxx_name = "statusText")]
        type SearchContent = super::SearchContentRust;

        #[qinvokable]
        fn run(self: Pin<&mut SearchContent>, query: &QString);

        #[qinvokable]
        fn open_result(&self, index: i32);

        #[qinvokable]
        fn reveal_result(&self, index: i32);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QStringList, ids)]
        #[qproperty(QStringList, previews)]
        /// 未截断的完整正文，供「查看单条全文」对话框使用。
        #[qproperty(QStringList, contents)]
        #[qproperty(QStringList, pinned_flags, cxx_name = "pinnedFlags")]
        #[qproperty(QStringList, kinds)]
        #[qproperty(QString, count_text, cxx_name = "countText")]
        #[qproperty(QString, status_text, cxx_name = "statusText")]
        type CaptureContent = super::CaptureContentRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut CaptureContent>);

        #[qinvokable]
        fn add_text(self: Pin<&mut CaptureContent>, content: &QString);

        #[qinvokable]
        fn toggle_pin(self: Pin<&mut CaptureContent>, id: &QString);

        #[qinvokable]
        fn delete_item(self: Pin<&mut CaptureContent>, id: &QString);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QStringList, ids)]
        #[qproperty(QStringList, titles)]
        #[qproperty(QStringList, done_flags, cxx_name = "doneFlags")]
        #[qproperty(QStringList, due_texts, cxx_name = "dueTexts")]
        /// 单条详情用的完整字段（标题在 titles，未截断）。
        #[qproperty(QStringList, notes)]
        #[qproperty(QStringList, colors)]
        #[qproperty(QStringList, step_texts, cxx_name = "stepTexts")]
        #[qproperty(QString, count_text, cxx_name = "countText")]
        #[qproperty(QString, filter)]
        #[qproperty(QString, status_text, cxx_name = "statusText")]
        type TodoContent = super::TodoContentRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut TodoContent>);

        #[qinvokable]
        fn add_task(self: Pin<&mut TodoContent>, title: &QString);

        #[qinvokable]
        fn toggle_task(self: Pin<&mut TodoContent>, id: &QString);

        #[qinvokable]
        fn delete_task(self: Pin<&mut TodoContent>, id: &QString);

        #[qinvokable]
        fn choose_filter(self: Pin<&mut TodoContent>, filter: &QString);

        /// 设置/清除单条任务的颜色标记（`#RRGGBB` / 空串）。
        #[qinvokable]
        fn set_task_color(self: Pin<&mut TodoContent>, id: &QString, color: &QString);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, city)]
        #[qproperty(QString, temperature_text, cxx_name = "temperatureText")]
        #[qproperty(QString, condition_text, cxx_name = "conditionText")]
        #[qproperty(QString, symbol_text, cxx_name = "symbolText")]
        #[qproperty(QString, range_text, cxx_name = "rangeText")]
        #[qproperty(QString, updated_text, cxx_name = "updatedText")]
        #[qproperty(QString, status_text, cxx_name = "statusText")]
        type WeatherContent = super::WeatherContentRust;

        #[qinvokable]
        fn attach(self: Pin<&mut WeatherContent>, widget_id: &QString);

        #[qinvokable]
        fn choose_city(self: Pin<&mut WeatherContent>, city: &QString);

        #[qinvokable]
        fn refresh(self: Pin<&mut WeatherContent>);

        /// 每 200ms 由 QML 调用，取回后台线程的结果。
        #[qinvokable]
        fn poll(self: Pin<&mut WeatherContent>);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, time_text, cxx_name = "timeText")]
        #[qproperty(QString, date_text, cxx_name = "dateText")]
        #[qproperty(QString, weekday_text, cxx_name = "weekdayText")]
        #[qproperty(QString, lunar_text, cxx_name = "lunarText")]
        #[qproperty(QString, term_text, cxx_name = "termText")]
        #[qproperty(QString, festival_text, cxx_name = "festivalText")]
        type GlanceContent = super::GlanceContentRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut GlanceContent>);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, page)]
        #[qproperty(i32, page_count, cxx_name = "pageCount")]
        type Onboarding = super::OnboardingRust;

        #[qinvokable]
        fn next_page(self: Pin<&mut Onboarding>);

        #[qinvokable]
        fn previous_page(self: Pin<&mut Onboarding>);

        #[qinvokable]
        fn finish(self: Pin<&mut Onboarding>);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, hotkey)]
        #[qproperty(bool, autostart)]
        #[qproperty(i32, default_icon_size, cxx_name = "defaultIconSize")]
        #[qproperty(i32, default_name_lines, cxx_name = "defaultNameLines")]
        #[qproperty(QString, default_density, cxx_name = "defaultDensity")]
        #[qproperty(QString, default_sort_mode, cxx_name = "defaultSortMode")]
        #[qproperty(QString, status_text, cxx_name = "statusText")]
        #[qproperty(QString, backup_status, cxx_name = "backupStatus")]
        #[qproperty(QString, version)]
        #[qproperty(i32, current_page, cxx_name = "currentPage")]
        #[qproperty(bool, menu_visible, cxx_name = "menuVisible")]
        #[qproperty(bool, capsule_enabled, cxx_name = "capsuleEnabled")]
        #[qproperty(QString, capsule_expand_mode, cxx_name = "capsuleExpandMode")]
        #[qproperty(QString, capsule_width_mode, cxx_name = "capsuleWidthMode")]
        #[qproperty(
            QString,
            capsule_collapsed_content,
            cxx_name = "capsuleCollapsedContent"
        )]
        #[qproperty(QString, capsule_arrangement, cxx_name = "capsuleArrangement")]
        #[qproperty(bool, capsule_hide_content, cxx_name = "capsuleHideContent")]
        #[qproperty(QString, capsule_hover_response, cxx_name = "capsuleHoverResponse")]
        #[qproperty(QString, tray_icon_style, cxx_name = "trayIconStyle")]
        #[qproperty(bool, use_system_accent, cxx_name = "useSystemAccent")]
        #[qproperty(QString, window_material, cxx_name = "windowMaterial")]
        #[qproperty(QString, display_density, cxx_name = "displayDensity")]
        #[qproperty(QString, titlebar_style, cxx_name = "titlebarStyle")]
        #[qproperty(QString, show_hide_animation, cxx_name = "showHideAnimation")]
        #[qproperty(bool, file_show_previews, cxx_name = "fileShowPreviews")]
        #[qproperty(bool, file_auto_stack, cxx_name = "fileAutoStack")]
        #[qproperty(bool, hover_actions_enabled, cxx_name = "hoverActionsEnabled")]
        #[qproperty(QString, general_language, cxx_name = "generalLanguage")]
        #[qproperty(QString, attachment_mode, cxx_name = "attachmentMode")]
        #[qproperty(QString, app_theme, cxx_name = "appTheme")]
        #[qproperty(bool, tool_capture, cxx_name = "toolCapture")]
        #[qproperty(bool, tool_todo, cxx_name = "toolTodo")]
        #[qproperty(bool, tool_music, cxx_name = "toolMusic")]
        #[qproperty(bool, tool_weather, cxx_name = "toolWeather")]
        #[qproperty(bool, tool_glance, cxx_name = "toolGlance")]
        #[qproperty(bool, tool_search, cxx_name = "toolSearch")]
        /// 时光瓦片背景：空路径表示跟随当前桌面壁纸。
        #[qproperty(bool, glance_bg_enabled, cxx_name = "glanceBgEnabled")]
        #[qproperty(QString, glance_bg_path, cxx_name = "glanceBgPath")]
        type SettingsPanel = super::SettingsPanelRust;

        #[qinvokable]
        fn create_backup(self: Pin<&mut SettingsPanel>);

        #[qinvokable]
        fn restore_latest_backup(self: Pin<&mut SettingsPanel>);

        #[qinvokable]
        fn save(self: Pin<&mut SettingsPanel>, hotkey: &QString, autostart: bool);

        /// 持久化设置窗口的界面状态（当前页、菜单展开、下拉选择）。
        #[qinvokable]
        fn save_ui_state(self: Pin<&mut SettingsPanel>);

        /// 打开首次运行引导窗口（设置 → 常规 → 新手引导"查看"）。
        #[qinvokable]
        fn show_onboarding(self: Pin<&mut SettingsPanel>);

        /// 在文件管理器里打开备份目录。
        #[qinvokable]
        fn reveal_backup_dir(self: Pin<&mut SettingsPanel>);

        /// 开启/关闭一种功能瓦片（单例，即时生效）。
        /// kind_key: quickCapture / todo / music / weather / glance。
        #[qinvokable]
        fn set_tool_enabled(self: Pin<&mut SettingsPanel>, kind_key: &QString, enabled: bool);

        /// 设置时光瓦片背景：path 为空表示使用当前桌面壁纸。
        #[qinvokable]
        fn set_glance_background(self: Pin<&mut SettingsPanel>, path: &QString, enabled: bool);

        /// 当前桌面壁纸的本地路径（供设置页显示）。
        #[qinvokable]
        fn desktop_wallpaper_path(&self) -> QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, widget_id, cxx_name = "widgetId")]
        #[qproperty(i32, window_x, cxx_name = "windowX")]
        #[qproperty(i32, window_y, cxx_name = "windowY")]
        #[qproperty(i32, window_w, cxx_name = "windowW")]
        #[qproperty(i32, window_h, cxx_name = "windowH")]
        #[qproperty(QString, folder_path, cxx_name = "folderPath")]
        type AppState = super::AppStateRust;

        #[qinvokable]
        fn attach(self: Pin<&mut AppState>, widget_id: &QString);

        #[qinvokable]
        fn save_geometry(self: Pin<&mut AppState>, x: i32, y: i32, w: i32, h: i32);

        /// 新建一个文件格子并开窗，必须先由用户选定文件夹。
        /// folder_path 是文件夹选择器返回的本地路径（也兼容 file:// URL）；
        /// 路径无效或不是目录时返回空串（不创建任何东西）。
        #[qinvokable]
        fn create_file_tile(self: Pin<&mut AppState>, folder_path: &QString) -> QString;

        /// 关窗时从布局里移除。
        #[qinvokable]
        fn remove_tile(self: Pin<&mut AppState>);

        /// 重新对齐显示器拓扑；本瓦片位置有变化时返回 true。
        #[qinvokable]
        fn reconcile(self: Pin<&mut AppState>) -> bool;

        /// 宿主显隐版本号（托盘/热键触发时递增）。
        #[qinvokable]
        fn host_epoch(&self) -> i32;

        #[qinvokable]
        fn host_visible(&self) -> bool;

        /// 取走一条宿主命令：`new-tile` / `quit`，没有则返回空串。
        #[qinvokable]
        fn take_host_command(&self) -> QString;

        /// 退出应用（托盘"退出"用；QML 的 Qt.quit() 在本架构下没有接收者）。
        #[qinvokable]
        fn quit_app(&self);

        /// 打开（或前置）设置窗口。
        #[qinvokable]
        fn open_settings(&self);

        /// 打开（或前置）搜索窗口。
        #[qinvokable]
        fn open_search(&self);

        /// 把本瓦片窗口放回 below 层（桌面之上、普通窗口之下）。
        #[qinvokable]
        fn keep_self_below(&self);

        /// 发起窗口管理器原生拖动（KWin 按指针同步移动，不卡顿）。
        /// 返回 false 表示窗管不接手，QML 退回手动改 x/y。
        #[qinvokable]
        fn start_move(&self) -> bool;

        /// 输入框获得焦点时把 X 输入焦点交给本瓦片窗口。
        /// 桌面层窗口默认不抢焦点（fcitx 兼容），键盘输入必须显式接管。
        #[qinvokable]
        fn focus_tile(&self);

        /// 文件夹选择器用的目录浏览接口（不依赖 Qt.labs.folderlistmodel）。
        #[qinvokable]
        fn home_folder(&self) -> QString;

        #[qinvokable]
        fn list_directories(&self, path: &QString) -> QStringList;

        #[qinvokable]
        fn is_directory(&self, path: &QString) -> bool;

        /// 其他瓦片的几何，扁平数组 [x, y, w, h, x, y, w, h, ...]。
        /// 磁吸（resize 边界吸附）用；不含本瓦片自身。
        #[qinvokable]
        fn neighbor_rects(&self) -> QStringList;

        /// 把窗口矩形收进主屏可用区，返回 [x, y, w, h]（字符串数字）。
        /// 双屏场景瓦片只允许在主屏；拖动/缩放路径每帧调用，纯内存计算。
        #[qinvokable]
        fn clamp_rect(&self, x: i32, y: i32, w: i32, h: i32) -> QStringList;

        /// 时光瓦片当前应显示的背景（`file://` URL），关闭时为空串。
        #[qinvokable]
        fn glance_background(&self) -> QString;

        /// 当前桌面壁纸的本地路径，找不到时为空串。
        #[qinvokable]
        fn wallpaper_path(&self) -> QString;

        /// 一次按固定顺序把全部瓦片固定回 below 层；瓦片绝不置顶。
        #[qinvokable]
        fn keep_below(&self);

        /// 全局「松手留缝」距离（像素），唯一来源 `layout_store::RELEASE_GAP`。
        /// QML 拖动 / 缩放边条从这里读，避免各处写死魔法数字。
        #[qinvokable]
        fn release_gap(&self) -> i32;
    }
}

mod host;

pub use host::{
    epoch as host_epoch, is_visible as host_visible, start as start_host,
    take_command as take_host_command,
};

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use dm_core::file_tile::{scan_dir, FileEntry};
use dm_core::file_tile_settings::{Density, FileTileSettings};
use dm_core::widget::WidgetKind;
use dm_platform::{icons, launch, layout_store, paths, trash, watch::DirWatcher};
use std::path::PathBuf;

// ---------------------------------------------------------------- FileTile

pub struct FileTileRust {
    // --- QML 属性 ---
    widget_id: QString,
    folder_path: QString,
    title: QString,
    names: QStringList,
    icon_paths: QStringList,
    kinds: QStringList,
    paths: QStringList,
    status_text: QString,
    item_count: i32,
    icon_size: i32,
    name_lines: i32,
    density: QString,
    sort_mode: QString,
    cell_width: i32,
    cell_height: i32,
    // --- 内部状态 ---
    folder: PathBuf,
    entries: Vec<FileEntry>,
    watcher: Option<DirWatcher>,
    settings: FileTileSettings,
}

impl Default for FileTileRust {
    fn default() -> Self {
        let folder = paths::default_folder();
        let watcher = DirWatcher::new(&folder).ok();
        let settings = FileTileSettings::default();
        let (cell_width, cell_height) = settings.cell_size();
        Self {
            widget_id: QString::default(),
            folder_path: QString::from(&folder.to_string_lossy().to_string()),
            title: QString::default(),
            names: QStringList::default(),
            icon_paths: QStringList::default(),
            kinds: QStringList::default(),
            paths: QStringList::default(),
            status_text: QString::default(),
            item_count: 0,
            icon_size: settings.icon_size as i32,
            name_lines: settings.name_lines as i32,
            density: QString::from(settings.density.as_str()),
            sort_mode: QString::from(sort_mode_key(settings.sort_mode)),
            cell_width: cell_width as i32,
            cell_height: cell_height as i32,
            folder,
            entries: Vec::new(),
            watcher,
            settings,
        }
    }
}

impl qobject::FileTile {
    fn attach(mut self: std::pin::Pin<&mut Self>, widget_id: &QString) {
        let id = widget_id.to_string();
        self.as_mut().set_widget_id(QString::from(&id));
        let Some(widget) = layout_store::find(&id) else {
            self.as_mut()
                .set_status_text(QString::from(&format!("布局里找不到瓦片 {id}")));
            return;
        };
        let folder = widget
            .data
            .get("folder")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(paths::default_folder);
        let settings = FileTileSettings::from_data(&widget.data);
        self.as_mut().apply_settings(settings.clone());
        {
            let mut rust = self.as_mut().rust_mut();
            rust.settings = settings;
            rust.folder = folder.clone();
            rust.watcher = DirWatcher::new(&folder).ok();
        }
        self.as_mut()
            .set_folder_path(QString::from(&folder.to_string_lossy().to_string()));
        self.as_mut().refresh();
    }

    fn set_folder(mut self: std::pin::Pin<&mut Self>, path: &QString) {
        let text = path.to_string();
        // QML 的 FileDialog 给的是 file:// URL，命令行/测试给的是本地路径。
        let folder = if text.starts_with("file://") {
            dm_platform::files::url_to_path(&text).unwrap_or_default()
        } else {
            PathBuf::from(&text)
        };
        let text = folder.to_string_lossy().to_string();
        if !folder.is_dir() {
            self.as_mut()
                .set_status_text(QString::from(&format!("目录不存在：{text}")));
            return;
        }
        {
            let mut rust = self.as_mut().rust_mut();
            rust.folder = folder.clone();
            rust.watcher = DirWatcher::new(&folder).ok();
            rust.entries.clear();
        }
        self.as_mut().set_folder_path(QString::from(&text));
        self.persist_folder(&text);
        self.as_mut().refresh();
    }

    fn choose_icon_size(mut self: std::pin::Pin<&mut Self>, value: i32) {
        let mut settings = self.rust().settings.clone();
        settings.icon_size = value.clamp(24, 96) as u32;
        self.as_mut().apply_settings_and_persist(settings);
    }

    fn choose_name_lines(mut self: std::pin::Pin<&mut Self>, value: i32) {
        let mut settings = self.rust().settings.clone();
        settings.name_lines = value.clamp(0, 2) as u8;
        self.as_mut().apply_settings_and_persist(settings);
    }

    fn choose_density(mut self: std::pin::Pin<&mut Self>, value: &QString) {
        let mut settings = self.rust().settings.clone();
        settings.density = Density::parse(&value.to_string());
        self.as_mut().apply_settings_and_persist(settings);
    }

    fn choose_sort_mode(mut self: std::pin::Pin<&mut Self>, value: &QString) {
        let mut settings = self.rust().settings.clone();
        settings.sort_mode = parse_sort_mode(&value.to_string());
        self.as_mut().apply_settings_and_persist(settings);
        self.as_mut().refresh();
    }

    fn refresh(mut self: std::pin::Pin<&mut Self>) {
        let folder = self.rust().folder.clone();
        let sort = self.rust().settings.sort_mode;
        match scan_dir(&folder, sort, false) {
            Ok(entries) => {
                let names: QStringList = entries.iter().map(|e| QString::from(&e.name)).collect();
                let icon_paths: QStringList = entries
                    .iter()
                    .map(|e| {
                        QString::from(
                            &icons::resolve_icon_with_fallback(e.icon_name())
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        )
                    })
                    .collect();
                let kinds: QStringList = entries
                    .iter()
                    .map(|e| QString::from(if e.is_dir { "dir" } else { "file" }))
                    .collect();
                let paths: QStringList = entries
                    .iter()
                    .map(|e| QString::from(&e.path.to_string_lossy().to_string()))
                    .collect();
                let count = entries.len() as i32;
                let title = folder
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| folder.to_string_lossy().to_string());

                self.as_mut().rust_mut().entries = entries;
                self.as_mut().set_names(names);
                self.as_mut().set_icon_paths(icon_paths);
                self.as_mut().set_kinds(kinds);
                self.as_mut().set_paths(paths);
                self.as_mut().set_item_count(count);
                self.as_mut().set_title(QString::from(&title));
                self.as_mut()
                    .set_status_text(QString::from(&format!("{count} 项")));
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("无法读取目录：{error}")));
            }
        }
    }

    fn tick(mut self: std::pin::Pin<&mut Self>) {
        let dirty = self
            .rust()
            .watcher
            .as_ref()
            .map(|w| w.take_dirty())
            .unwrap_or(false);
        if dirty {
            self.as_mut().refresh();
        }
    }

    fn open_item(&self, index: i32) {
        if let Some(entry) = self.local_entry(index) {
            let _ = launch::open_path(&entry.path);
        }
    }

    fn reveal_item(&self, index: i32) {
        if let Some(entry) = self.local_entry(index) {
            let _ = launch::reveal_path(&entry.path);
        }
    }

    fn reveal_folder(&self) {
        let _ = launch::reveal_path(&self.rust().folder);
    }

    fn rename_item(mut self: std::pin::Pin<&mut Self>, index: i32, new_name: &QString) {
        let Some(entry) = self.rust().entries.get(index as usize).cloned() else {
            return;
        };
        let trimmed = new_name.to_string();
        let trimmed = trimmed.trim();
        if trimmed.is_empty() || trimmed.contains('/') {
            self.as_mut()
                .set_status_text(QString::from("名称不能为空，且不能包含 /"));
            return;
        }
        if trimmed == entry.name {
            return;
        }
        let target = entry.path.with_file_name(trimmed);
        if target.exists() {
            self.as_mut()
                .set_status_text(QString::from(&format!("已存在同名项：{trimmed}")));
            return;
        }
        match std::fs::rename(&entry.path, &target) {
            Ok(()) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("已重命名为 {trimmed}")));
                self.as_mut().refresh();
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("重命名失败：{error}")));
            }
        }
    }

    fn trash_item(mut self: std::pin::Pin<&mut Self>, index: i32) {
        let Some(entry) = self.rust().entries.get(index as usize).cloned() else {
            return;
        };
        match trash::move_to_trash(&entry.path) {
            Ok(_) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("已移到回收站：{}", entry.name)));
                self.as_mut().refresh();
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("删除失败：{error}")));
            }
        }
    }

    fn file_url(&self, index: i32) -> QString {
        let Some(entry) = self.rust().entries.get(index as usize) else {
            return QString::default();
        };
        QString::from(&path_to_file_url(&entry.path))
    }

    fn import_urls(self: std::pin::Pin<&mut Self>, urls: &QStringList) {
        let folder = self.rust().folder.clone();
        self.moved_into(urls, &folder);
    }

    fn import_urls_into(mut self: std::pin::Pin<&mut Self>, urls: &QStringList, index: i32) {
        let Some(target) = self.rust().entries.get(index as usize).cloned() else {
            return;
        };
        if !target.is_dir {
            // 拖到文件上：退回"移进当前目录"。
            self.as_mut().import_urls(urls);
            return;
        }
        let folder = target.path.clone();
        self.moved_into(urls, &folder);
    }

    fn copy_item(mut self: std::pin::Pin<&mut Self>, index: i32) {
        let Some(entry) = self.rust().entries.get(index as usize).cloned() else {
            return;
        };
        let ok = clipboard_write(&[entry.path], false);
        self.as_mut().set_status_text(QString::from(&format!(
            "{}：{}",
            if ok { "已复制" } else { "复制失败" },
            entry.name
        )));
    }

    fn cut_item(mut self: std::pin::Pin<&mut Self>, index: i32) {
        let Some(entry) = self.rust().entries.get(index as usize).cloned() else {
            return;
        };
        let ok = clipboard_write(&[entry.path], true);
        self.as_mut().set_status_text(QString::from(&format!(
            "{}：{}",
            if ok { "已剪切" } else { "剪切失败" },
            entry.name
        )));
    }

    fn copy_urls(mut self: std::pin::Pin<&mut Self>, urls: &QStringList) {
        let paths: Vec<PathBuf> = urls
            .iter()
            .filter_map(|u| dm_platform::files::url_to_path(&u.to_string()))
            .collect();
        if paths.is_empty() {
            self.as_mut()
                .set_status_text(QString::from("没有可复制的项"));
            return;
        }
        let count = paths.len();
        let ok = clipboard_write(&paths, false);
        self.as_mut().set_status_text(QString::from(&format!(
            "{} {count} 项",
            if ok { "已复制" } else { "复制失败" }
        )));
    }

    fn cut_urls(mut self: std::pin::Pin<&mut Self>, urls: &QStringList) {
        let paths: Vec<PathBuf> = urls
            .iter()
            .filter_map(|u| dm_platform::files::url_to_path(&u.to_string()))
            .collect();
        if paths.is_empty() {
            self.as_mut()
                .set_status_text(QString::from("没有可剪切的项"));
            return;
        }
        let count = paths.len();
        let ok = clipboard_write(&paths, true);
        self.as_mut().set_status_text(QString::from(&format!(
            "{} {count} 项",
            if ok { "已剪切" } else { "剪切失败" }
        )));
    }

    fn trash_paths(mut self: std::pin::Pin<&mut Self>, paths: &QStringList) {
        let mut done = 0usize;
        let mut errors: Vec<String> = Vec::new();
        for path in paths.iter() {
            let path = PathBuf::from(path.to_string());
            match trash::move_to_trash(&path) {
                Ok(_) => done += 1,
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            }
        }
        let status = if let Some(first) = errors.first() {
            format!("删除 {done} 项，{} 项失败（{first}）", errors.len())
        } else {
            format!("已移到回收站 {done} 项")
        };
        self.as_mut().set_status_text(QString::from(&status));
        if done > 0 {
            self.as_mut().refresh();
        }
    }

    fn paste(mut self: std::pin::Pin<&mut Self>) {
        let Some((cut, paths)) = clipboard_read() else {
            self.as_mut()
                .set_status_text(QString::from("剪贴板里没有可粘贴的文件"));
            return;
        };
        let folder = self.rust().folder.clone();
        let mut done = 0usize;
        let mut errors: Vec<String> = Vec::new();
        for path in &paths {
            let result = if cut {
                dm_platform::files::move_into(path, &folder)
            } else {
                dm_platform::files::copy_into(path, &folder)
            };
            match result {
                Ok(_) => done += 1,
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            }
        }
        if cut && done > 0 {
            // 剪切内容已消费，清空剪贴板，避免重复粘贴。
            let _ = clipboard_write(&[], true);
        }
        let status = if let Some(first) = errors.first() {
            format!("粘贴 {done} 项，{} 项失败（{first}）", errors.len())
        } else {
            format!("已粘贴 {done} 项")
        };
        self.as_mut().set_status_text(QString::from(&status));
        if done > 0 {
            self.as_mut().refresh();
        }
    }

    fn create_folder(mut self: std::pin::Pin<&mut Self>, name: &QString) {
        let raw = name.to_string();
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.contains('/') {
            self.as_mut()
                .set_status_text(QString::from("名称不能为空，且不能包含 /"));
            return;
        }
        let target = self.rust().folder.join(trimmed);
        if target.exists() {
            self.as_mut()
                .set_status_text(QString::from(&format!("已存在同名项：{trimmed}")));
            return;
        }
        match std::fs::create_dir(&target) {
            Ok(()) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("已新建文件夹 {trimmed}")));
                self.as_mut().refresh();
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("新建失败：{error}")));
            }
        }
    }

    fn moved_into(
        mut self: std::pin::Pin<&mut Self>,
        urls: &QStringList,
        folder: &std::path::Path,
    ) {
        let folder = folder.to_path_buf();
        let mut moved = 0usize;
        let mut skipped = 0usize;
        let mut errors: Vec<String> = Vec::new();

        for url in urls.iter() {
            let text = url.to_string();
            let Some(path) = dm_platform::files::url_to_path(&text) else {
                skipped += 1;
                continue;
            };
            if path.parent() == Some(folder.as_path()) {
                skipped += 1;
                continue;
            }
            match dm_platform::files::move_into(&path, &folder) {
                Ok(_) => moved += 1,
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            }
        }

        let status = if let Some(first) = errors.first() {
            format!("移入 {moved} 项，{} 项失败（{first}）", errors.len())
        } else if moved == 0 && skipped > 0 {
            format!("没有移动：{skipped} 项已经在当前文件夹")
        } else {
            format!("已移入 {moved} 项")
        };
        self.as_mut().set_status_text(QString::from(&status));
        if moved > 0 {
            self.as_mut().refresh();
        }
    }
}

impl qobject::FileTile {
    fn local_entry(&self, index: i32) -> Option<FileEntry> {
        self.rust().entries.get(index as usize).cloned()
    }

    fn apply_settings(mut self: std::pin::Pin<&mut Self>, settings: FileTileSettings) {
        let settings = settings.clamped();
        let (cell_width, cell_height) = settings.cell_size();
        self.as_mut().set_icon_size(settings.icon_size as i32);
        self.as_mut().set_name_lines(settings.name_lines as i32);
        self.as_mut()
            .set_density(QString::from(settings.density.as_str()));
        self.as_mut()
            .set_sort_mode(QString::from(sort_mode_key(settings.sort_mode)));
        self.as_mut().set_cell_width(cell_width as i32);
        self.as_mut().set_cell_height(cell_height as i32);
        self.as_mut().rust_mut().settings = settings;
    }

    fn apply_settings_and_persist(mut self: std::pin::Pin<&mut Self>, settings: FileTileSettings) {
        self.as_mut().apply_settings(settings.clone());
        let widget_id = self.widget_id().to_string();
        if widget_id.is_empty() {
            return;
        }
        layout_store::update_widget(&widget_id, |widget| {
            widget.data.insert("settings".into(), settings.to_value());
        });
    }

    fn persist_folder(&self, folder: &str) {
        let widget_id = self.widget_id().to_string();
        if widget_id.is_empty() {
            return;
        }
        layout_store::update_widget(&widget_id, |widget| {
            widget.data.insert(
                "folder".into(),
                serde_json::Value::String(folder.to_string()),
            );
            widget.title = PathBuf::from(folder)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
        });
    }
}

// ---------------------------------------------------------------- AppState

pub struct AppStateRust {
    widget_id: QString,
    window_x: i32,
    window_y: i32,
    window_w: i32,
    window_h: i32,
    folder_path: QString,
}

impl Default for AppStateRust {
    fn default() -> Self {
        Self {
            widget_id: QString::default(),
            window_x: 0,
            window_y: 0,
            window_w: 420,
            window_h: 520,
            folder_path: QString::default(),
        }
    }
}

impl qobject::AppState {
    fn attach(mut self: std::pin::Pin<&mut Self>, widget_id: &QString) {
        let id = widget_id.to_string();
        self.as_mut().set_widget_id(QString::from(&id));
        let Some(widget) = layout_store::find(&id) else {
            return;
        };
        let folder = widget
            .data
            .get("folder")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| paths::default_folder().to_string_lossy().to_string());
        // 双屏场景：所有瓦片只在主屏显示，读档时也把旧位置收回来。
        let rect = layout_store::clamp_rect(widget.rect);
        self.as_mut().set_window_x(rect.x);
        self.as_mut().set_window_y(rect.y);
        self.as_mut().set_window_w(rect.width as i32);
        self.as_mut().set_window_h(rect.height as i32);
        self.as_mut().set_folder_path(QString::from(&folder));
    }

    fn save_geometry(mut self: std::pin::Pin<&mut Self>, x: i32, y: i32, w: i32, h: i32) {
        let widget_id = self.widget_id().to_string();
        if widget_id.is_empty() {
            return;
        }
        let requested = layout_store::clamp_rect(dm_core::geometry::Rect::new(
            x,
            y,
            w.max(160) as u32,
            h.max(160) as u32,
        ));
        // 全局「松手留缝」：贴死在邻瓦边缘或主屏四边时统一退开
        // RELEASE_GAP。系统拖动由 KWin 直接改窗口位置，QML 的磁吸和
        // minGap 约束都拦不住，因此所有瓦片在几何落盘时走这一条规则。
        let rect = layout_store::settle_rect(&widget_id, requested);
        // 同时写当前瓦片与"当前显示器组合"的摆放档。
        layout_store::save_placement(&widget_id, rect);
        // 结算把位置挪开了，必须同步回写 Qt 窗口，否则屏幕上仍贴死。
        if rect != requested {
            if let Ok(c_id) = std::ffi::CString::new(widget_id.as_str()) {
                unsafe {
                    dm_set_tile_geometry(
                        c_id.as_ptr(),
                        rect.x,
                        rect.y,
                        rect.width as i32,
                        rect.height as i32,
                    );
                }
            }
        }
        self.as_mut().set_window_x(rect.x);
        self.as_mut().set_window_y(rect.y);
        self.as_mut().set_window_w(rect.width as i32);
        self.as_mut().set_window_h(rect.height as i32);
    }

    /// 唯一的「新建文件格子」入口：用户先在 FolderDialog 里选定文件夹，
    /// 不允许沿用当前格子的目录（那样只会得到两个内容相同的格子）。
    fn create_file_tile(mut self: std::pin::Pin<&mut Self>, folder_url: &QString) -> QString {
        let Some(folder) = dm_platform::files::url_to_path(&folder_url.to_string()) else {
            return QString::default();
        };
        if !folder.is_dir() {
            self.as_mut()
                .set_folder_path(QString::from("所选路径不是文件夹，未创建格子"));
            return QString::default();
        }
        // 新瓦片按主屏网格错开，行列都留 40px 间隔，不会压在旧瓦片上。
        let rect = layout_store::default_tile_rect(layout_store::widgets().len() as i32);
        let Some(widget) = layout_store::add_file_widget(&folder, rect) else {
            return QString::default();
        };
        let id = widget.id.clone();
        if unsafe { spawn_tile(&id) } == 0 {
            self.as_mut()
                .set_folder_path(QString::from("新建格子失败：无法创建窗口"));
        }
        QString::from(&id)
    }

    fn reconcile(mut self: std::pin::Pin<&mut Self>) -> bool {
        let widget_id = self.widget_id().to_string();
        if widget_id.is_empty() {
            return false;
        }
        let before = layout_store::find(&widget_id).map(|w| w.rect);
        // 瓦片只允许在主屏：刷新主屏可用区、收回副屏/越界瓦片、拆开重叠。
        // 不再调用多屏 reconcile_monitors，否则它会把瓦片重新归属到副屏。
        layout_store::enforce_primary_screen();
        let after = layout_store::find(&widget_id).map(|w| w.rect);
        let Some(after) = after else {
            return false;
        };
        if before == Some(after) {
            return false;
        }
        self.as_mut().set_window_x(after.x);
        self.as_mut().set_window_y(after.y);
        self.as_mut().set_window_w(after.width as i32);
        self.as_mut().set_window_h(after.height as i32);
        true
    }

    fn host_epoch(&self) -> i32 {
        host::epoch()
    }

    fn host_visible(&self) -> bool {
        host::is_visible()
    }

    fn quit_app(&self) {
        unsafe { dm_app_quit() };
    }

    fn open_settings(&self) {
        unsafe {
            open_settings_window();
        }
    }

    fn open_search(&self) {
        unsafe {
            open_search_window();
        }
    }

    fn keep_self_below(&self) {
        // 宿主"隐藏全部"时不参与层级调整：隐藏态保持窗口不可见即可。
        if !host::is_visible() {
            return;
        }
        let widget_id = self.widget_id().to_string();
        if widget_id.is_empty() {
            return;
        }
        if let Ok(c_id) = std::ffi::CString::new(widget_id) {
            unsafe { dm_keep_tile_below(c_id.as_ptr()) };
        }
    }

    fn start_move(&self) -> bool {
        let widget_id = self.widget_id().to_string();
        if widget_id.is_empty() {
            return false;
        }
        match std::ffi::CString::new(widget_id) {
            Ok(c_id) => unsafe { dm_start_system_move(c_id.as_ptr()) },
            Err(_) => false,
        }
    }

    fn focus_tile(&self) {
        let widget_id = self.widget_id().to_string();
        if widget_id.is_empty() {
            return;
        }
        if let Ok(c_id) = std::ffi::CString::new(widget_id) {
            unsafe { dm_focus_tile(c_id.as_ptr()) };
        }
    }

    fn home_folder(&self) -> QString {
        QString::from(&paths::home_dir().to_string_lossy().to_string())
    }

    fn is_directory(&self, path: &QString) -> bool {
        std::path::Path::new(&path.to_string()).is_dir()
    }

    fn list_directories(&self, path: &QString) -> QStringList {
        let mut out = QStringList::default();
        let Ok(entries) = std::fs::read_dir(path.to_string()) else {
            return out;
        };
        let mut dirs: Vec<String> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                // 隐藏目录不铺满列表；需要时可在路径栏直接输入。
                !name.starts_with('.') && entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
            })
            .map(|entry| entry.path().to_string_lossy().to_string())
            .collect();
        dirs.sort();
        for dir in &dirs {
            out.append(QString::from(dir));
        }
        out
    }

    fn neighbor_rects(&self) -> QStringList {
        let widget_id = self.widget_id().to_string();
        let mut out = QStringList::default();
        for widget in layout_store::widgets() {
            if widget.id == widget_id {
                continue;
            }
            out.append(QString::from(widget.rect.x.to_string()));
            out.append(QString::from(widget.rect.y.to_string()));
            out.append(QString::from(widget.rect.width.to_string()));
            out.append(QString::from(widget.rect.height.to_string()));
        }
        out
    }

    fn clamp_rect(&self, x: i32, y: i32, w: i32, h: i32) -> QStringList {
        let rect = layout_store::clamp_rect(dm_core::geometry::Rect::new(
            x,
            y,
            w.max(160) as u32,
            h.max(160) as u32,
        ));
        let mut out = QStringList::default();
        out.append(QString::from(rect.x.to_string()));
        out.append(QString::from(rect.y.to_string()));
        out.append(QString::from(rect.width.to_string()));
        out.append(QString::from(rect.height.to_string()));
        out
    }

    fn glance_background(&self) -> QString {
        if !dm_platform::prefs::get_bool("glance/bgEnabled").unwrap_or(true) {
            return QString::default();
        }
        let custom = dm_platform::prefs::get_string("glance/background").unwrap_or_default();
        let path = if custom.is_empty() {
            dm_platform::wallpaper::desktop_wallpaper()
        } else {
            dm_platform::wallpaper::resolve_image(&custom)
        };
        match path {
            Some(path) => QString::from(&format!("file://{}", path.display())),
            None => QString::default(),
        }
    }

    fn wallpaper_path(&self) -> QString {
        match dm_platform::wallpaper::desktop_wallpaper() {
            Some(path) => QString::from(&path.to_string_lossy().to_string()),
            None => QString::default(),
        }
    }

    fn keep_below(&self) {
        if !host::is_visible() {
            return;
        }
        unsafe { dm_keep_tiles_below() };
    }

    fn release_gap(&self) -> i32 {
        layout_store::RELEASE_GAP
    }

    fn take_host_command(&self) -> QString {
        match host::take_command() {
            Some(command) => QString::from(command),
            None => QString::default(),
        }
    }

    fn remove_tile(self: std::pin::Pin<&mut Self>) {
        let widget_id = self.widget_id().to_string();
        if widget_id.is_empty() {
            return;
        }
        layout_store::remove_widget(&widget_id);
    }
}

// ---------------------------------------------------------------- helpers

// ------------------------------------------------------------ 音乐瓦片

pub struct MusicContentRust {
    title: QString,
    artist: QString,
    album: QString,
    art_url: QString,
    status_text: QString,
    progress_text: QString,
    playing: bool,
    can_next: bool,
    can_previous: bool,
    client: Option<dm_platform::mpris::MprisClient>,
    bus_name: String,
}

impl Default for MusicContentRust {
    fn default() -> Self {
        let client = dm_platform::mpris::MprisClient::connect().ok();
        let status = if client.is_none() {
            QString::from("无法连接会话总线")
        } else {
            QString::default()
        };
        Self {
            title: QString::from("没有正在播放的内容"),
            artist: QString::default(),
            album: QString::default(),
            art_url: QString::default(),
            status_text: status,
            progress_text: QString::default(),
            playing: false,
            can_next: false,
            can_previous: false,
            client,
            bus_name: String::new(),
        }
    }
}

impl qobject::MusicContent {
    fn refresh(mut self: std::pin::Pin<&mut Self>) {
        let Some(client) = self.rust().client.as_ref() else {
            return;
        };
        match client.active_player() {
            Ok(Some(state)) => {
                self.as_mut().rust_mut().bus_name = state.bus_name.clone();
                self.as_mut()
                    .set_title(QString::from(if state.track.title.is_empty() {
                        "未知曲目"
                    } else {
                        &state.track.title
                    }));
                self.as_mut().set_artist(QString::from(&state.track.artist));
                self.as_mut().set_album(QString::from(&state.track.album));
                self.as_mut()
                    .set_art_url(QString::from(&state.track.art_url));
                self.as_mut().set_playing(state.playing);
                self.as_mut().set_can_next(state.can_next);
                self.as_mut().set_can_previous(state.can_previous);
                self.as_mut()
                    .set_progress_text(QString::from(&dm_platform::mpris::progress_text(
                        state.position_us,
                        state.track.length_us,
                    )));
                self.as_mut()
                    .set_status_text(QString::from(&state.identity));
            }
            Ok(None) => {
                self.as_mut().set_title(QString::from("没有正在播放的内容"));
                self.as_mut().set_artist(QString::default());
                self.as_mut().set_album(QString::default());
                self.as_mut().set_art_url(QString::default());
                self.as_mut().set_playing(false);
                self.as_mut().set_can_next(false);
                self.as_mut().set_can_previous(false);
                self.as_mut().set_progress_text(QString::default());
                self.as_mut()
                    .set_status_text(QString::from("没有检测到 MPRIS 播放器"));
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("读取播放器失败：{error}")));
            }
        }
    }

    fn play_pause(mut self: std::pin::Pin<&mut Self>) {
        let bus = self.rust().bus_name.clone();
        if bus.is_empty() {
            return;
        }
        if let Some(client) = self.rust().client.as_ref() {
            let _ = client.play_pause(&bus);
        }
        self.as_mut().refresh();
    }

    fn next_track(mut self: std::pin::Pin<&mut Self>) {
        let bus = self.rust().bus_name.clone();
        if bus.is_empty() {
            return;
        }
        if let Some(client) = self.rust().client.as_ref() {
            let _ = client.next(&bus);
        }
        self.as_mut().refresh();
    }

    fn previous_track(mut self: std::pin::Pin<&mut Self>) {
        let bus = self.rust().bus_name.clone();
        if bus.is_empty() {
            return;
        }
        if let Some(client) = self.rust().client.as_ref() {
            let _ = client.previous(&bus);
        }
        self.as_mut().refresh();
    }
}

// ------------------------------------------------------------ 搜索

pub struct SearchContentRust {
    names: QStringList,
    paths: QStringList,
    kinds: QStringList,
    icon_paths: QStringList,
    count_text: QString,
    status_text: QString,
}

impl Default for SearchContentRust {
    fn default() -> Self {
        Self {
            names: QStringList::default(),
            paths: QStringList::default(),
            kinds: QStringList::default(),
            icon_paths: QStringList::default(),
            count_text: QString::from("0"),
            status_text: QString::default(),
        }
    }
}

impl qobject::SearchContent {
    fn run(mut self: std::pin::Pin<&mut Self>, query: &QString) {
        let text = query.to_string();
        if text.trim().is_empty() {
            self.as_mut().set_names(QStringList::default());
            self.as_mut().set_paths(QStringList::default());
            self.as_mut().set_kinds(QStringList::default());
            self.as_mut().set_icon_paths(QStringList::default());
            self.as_mut().set_count_text(QString::from("0"));
            self.as_mut().set_status_text(QString::default());
            return;
        }

        let hits = dm_platform::search::search(&text, 120);
        let count = hits.len();
        self.as_mut().set_names(
            hits.iter()
                .map(|h| QString::from(&h.name))
                .collect::<QStringList>(),
        );
        self.as_mut().set_paths(
            hits.iter()
                .map(|h| QString::from(&h.path.to_string_lossy().to_string()))
                .collect::<QStringList>(),
        );
        self.as_mut().set_kinds(
            hits.iter()
                .map(|h| QString::from(if h.is_dir { "dir" } else { "file" }))
                .collect::<QStringList>(),
        );
        // 与文件瓦片一致：走系统图标主题（bloom），避免 emoji 字体渲染问题。
        self.as_mut().set_icon_paths(
            hits.iter()
                .map(|h| {
                    let icon_name = if h.is_dir {
                        "folder"
                    } else {
                        dm_core::file_tile::icon_name_for_file(&h.name)
                    };
                    QString::from(
                        &dm_platform::icons::resolve_icon_with_fallback(icon_name)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    )
                })
                .collect::<QStringList>(),
        );
        self.as_mut()
            .set_count_text(QString::from(&count.to_string()));
        self.as_mut().set_status_text(QString::from(if count == 0 {
            "没有找到匹配项（当前为本地遍历后端）"
        } else {
            ""
        }));
    }

    fn open_result(&self, index: i32) {
        if let Some(path) = self.paths().iter().nth(index as usize) {
            let _ = dm_platform::launch::open_path(std::path::Path::new(&path.to_string()));
        }
    }

    fn reveal_result(&self, index: i32) {
        if let Some(path) = self.paths().iter().nth(index as usize) {
            let _ = dm_platform::launch::reveal_path(std::path::Path::new(&path.to_string()));
        }
    }
}

// ------------------------------------------------------------ 随记瓦片

pub struct CaptureContentRust {
    ids: QStringList,
    previews: QStringList,
    contents: QStringList,
    pinned_flags: QStringList,
    kinds: QStringList,
    count_text: QString,
    status_text: QString,
    store: Option<dm_platform::capture_store::CaptureStore>,
}

impl Default for CaptureContentRust {
    fn default() -> Self {
        let store = dm_platform::capture_store::CaptureStore::open_default();
        let status = match &store {
            Ok(_) => QString::default(),
            Err(error) => QString::from(&format!("随记数据库打不开：{error}")),
        };
        Self {
            ids: QStringList::default(),
            previews: QStringList::default(),
            contents: QStringList::default(),
            pinned_flags: QStringList::default(),
            kinds: QStringList::default(),
            count_text: QString::from("0"),
            status_text: status,
            store: store.ok(),
        }
    }
}

impl qobject::CaptureContent {
    fn refresh(mut self: std::pin::Pin<&mut Self>) {
        let Some(store) = self.rust().store.as_ref() else {
            return;
        };
        match store.list() {
            Ok(mut items) => {
                dm_core::quick_capture::sort_items(&mut items);
                let count = items.len();
                self.as_mut().set_ids(
                    items
                        .iter()
                        .map(|i| QString::from(&i.id))
                        .collect::<QStringList>(),
                );
                self.as_mut().set_previews(
                    items
                        .iter()
                        .map(|i| QString::from(&dm_core::quick_capture::preview(&i.content, 40)))
                        .collect::<QStringList>(),
                );
                self.as_mut().set_contents(
                    items
                        .iter()
                        .map(|i| QString::from(&i.content))
                        .collect::<QStringList>(),
                );
                self.as_mut().set_pinned_flags(
                    items
                        .iter()
                        .map(|i| QString::from(if i.pinned { "1" } else { "0" }))
                        .collect::<QStringList>(),
                );
                self.as_mut().set_kinds(
                    items
                        .iter()
                        .map(|i| QString::from(i.kind.as_str()))
                        .collect::<QStringList>(),
                );
                self.as_mut()
                    .set_count_text(QString::from(&count.to_string()));
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("读取随记失败：{error}")));
            }
        }
    }

    fn add_text(mut self: std::pin::Pin<&mut Self>, content: &QString) {
        let text = content.to_string();
        if text.trim().is_empty() {
            return;
        }
        let Some(store) = self.rust().store.as_ref() else {
            return;
        };
        match store.create(&text, now_unix()) {
            Ok(_) => {
                self.as_mut().set_status_text(QString::from("已记录"));
                self.as_mut().refresh();
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("记录失败：{error}")));
            }
        }
    }

    fn toggle_pin(mut self: std::pin::Pin<&mut Self>, id: &QString) {
        let item_id = id.to_string();
        let Some(store) = self.rust().store.as_ref() else {
            return;
        };
        let Ok(items) = store.list() else {
            return;
        };
        let Some(item) = items.iter().find(|i| i.id == item_id) else {
            return;
        };
        let next = !item.pinned;
        if let Err(error) = store.set_pinned(&item_id, next) {
            self.as_mut()
                .set_status_text(QString::from(&format!("置顶失败：{error}")));
            return;
        }
        self.as_mut().refresh();
    }

    fn delete_item(mut self: std::pin::Pin<&mut Self>, id: &QString) {
        let item_id = id.to_string();
        let Some(store) = self.rust().store.as_ref() else {
            return;
        };
        match store.delete(&item_id) {
            Ok(()) => {
                self.as_mut().set_status_text(QString::from("已删除"));
                self.as_mut().refresh();
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("删除失败：{error}")));
            }
        }
    }
}

// ------------------------------------------------------------ 待办瓦片

pub struct TodoContentRust {
    ids: QStringList,
    titles: QStringList,
    done_flags: QStringList,
    due_texts: QStringList,
    notes: QStringList,
    colors: QStringList,
    step_texts: QStringList,
    count_text: QString,
    filter: QString,
    status_text: QString,
    store: Option<dm_platform::todo_store::TodoStore>,
}

impl Default for TodoContentRust {
    fn default() -> Self {
        let store = dm_platform::todo_store::TodoStore::open_default();
        let status = match &store {
            Ok(_) => QString::default(),
            Err(error) => QString::from(&format!("待办数据库打不开：{error}")),
        };
        Self {
            ids: QStringList::default(),
            titles: QStringList::default(),
            done_flags: QStringList::default(),
            due_texts: QStringList::default(),
            notes: QStringList::default(),
            colors: QStringList::default(),
            step_texts: QStringList::default(),
            count_text: QString::from("0"),
            filter: QString::from("open"),
            status_text: status,
            store: store.ok(),
        }
    }
}

impl qobject::TodoContent {
    fn refresh(mut self: std::pin::Pin<&mut Self>) {
        let filter = dm_core::todo::TodoFilter::parse(&self.filter().to_string());
        let Some(store) = self.rust().store.as_ref() else {
            return;
        };

        match store.list() {
            Ok(mut tasks) => {
                tasks.retain(|task| filter.matches(task));
                dm_core::todo::sort_tasks(&mut tasks);

                let today = chrono::Local::now().date_naive();
                let total = tasks.len();
                let open = tasks.iter().filter(|t| !t.done).count();

                self.as_mut().set_ids(
                    tasks
                        .iter()
                        .map(|t| QString::from(&t.id))
                        .collect::<QStringList>(),
                );
                self.as_mut().set_titles(
                    tasks
                        .iter()
                        .map(|t| QString::from(&t.title))
                        .collect::<QStringList>(),
                );
                self.as_mut().set_done_flags(
                    tasks
                        .iter()
                        .map(|t| QString::from(if t.done { "1" } else { "0" }))
                        .collect::<QStringList>(),
                );
                self.as_mut().set_due_texts(
                    tasks
                        .iter()
                        .map(|t| QString::from(&dm_core::todo::due_text(t.due_at, today)))
                        .collect::<QStringList>(),
                );
                self.as_mut().set_notes(
                    tasks
                        .iter()
                        .map(|t| QString::from(&t.notes))
                        .collect::<QStringList>(),
                );
                self.as_mut().set_colors(
                    tasks
                        .iter()
                        .map(|t| QString::from(&t.color))
                        .collect::<QStringList>(),
                );
                // 步骤列表拼成多行文本，详情对话框里原样换行显示。
                self.as_mut().set_step_texts(
                    tasks
                        .iter()
                        .map(|t| {
                            QString::from(
                                &t.steps
                                    .iter()
                                    .map(|s| {
                                        format!("{} {}", if s.done { "✓" } else { "○" }, s.title)
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                            )
                        })
                        .collect::<QStringList>(),
                );
                self.as_mut()
                    .set_count_text(QString::from(&format!("{open}/{total}")));
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("读取待办失败：{error}")));
            }
        }
    }

    fn add_task(mut self: std::pin::Pin<&mut Self>, title: &QString) {
        let text = title.to_string();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let Some(store) = self.rust().store.as_ref() else {
            return;
        };
        let now = now_unix();
        match store.create(trimmed, now) {
            Ok(_) => {
                self.as_mut().set_status_text(QString::from("已添加"));
                self.as_mut().refresh();
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("添加失败：{error}")));
            }
        }
    }

    fn toggle_task(mut self: std::pin::Pin<&mut Self>, id: &QString) {
        let task_id = id.to_string();
        let Some(store) = self.rust().store.as_ref() else {
            return;
        };
        let Ok(tasks) = store.list() else {
            return;
        };
        let Some(task) = tasks.iter().find(|t| t.id == task_id) else {
            return;
        };
        let next = !task.done;
        if let Err(error) = store.set_done(&task_id, next, now_unix()) {
            self.as_mut()
                .set_status_text(QString::from(&format!("更新失败：{error}")));
            return;
        }
        self.as_mut().refresh();
    }

    fn delete_task(mut self: std::pin::Pin<&mut Self>, id: &QString) {
        let task_id = id.to_string();
        let Some(store) = self.rust().store.as_ref() else {
            return;
        };
        match store.delete(&task_id) {
            Ok(()) => {
                self.as_mut().set_status_text(QString::from("已删除"));
                self.as_mut().refresh();
            }
            Err(error) => {
                self.as_mut()
                    .set_status_text(QString::from(&format!("删除失败：{error}")));
            }
        }
    }

    fn choose_filter(mut self: std::pin::Pin<&mut Self>, filter: &QString) {
        let value = filter.to_string();
        let parsed = dm_core::todo::TodoFilter::parse(&value);
        self.as_mut().set_filter(QString::from(parsed.as_str()));
        self.as_mut().refresh();
    }

    fn set_task_color(mut self: std::pin::Pin<&mut Self>, id: &QString, color: &QString) {
        let task_id = id.to_string();
        let Some(store) = self.rust().store.as_ref() else {
            return;
        };
        match store.set_color(&task_id, &color.to_string(), now_unix()) {
            Ok(()) => {
                self.as_mut().set_status_text(QString::from("已设置颜色"));
                self.as_mut().refresh();
            }
            Err(error) => self
                .as_mut()
                .set_status_text(QString::from(&format!("设置颜色失败：{error}"))),
        }
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ------------------------------------------------------------ 天气瓦片

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

type WeatherJob = Arc<Mutex<Option<Result<(dm_core::weather::WeatherSnapshot, bool), String>>>>;

pub struct WeatherContentRust {
    city: QString,
    temperature_text: QString,
    condition_text: QString,
    symbol_text: QString,
    range_text: QString,
    updated_text: QString,
    status_text: QString,
    widget_id: String,
    latitude: f64,
    longitude: f64,
    job: WeatherJob,
    busy: Arc<AtomicBool>,
}

impl Default for WeatherContentRust {
    fn default() -> Self {
        Self {
            city: QString::from("北京"),
            temperature_text: QString::from("—"),
            condition_text: QString::default(),
            symbol_text: QString::from("☁"),
            range_text: QString::default(),
            updated_text: QString::default(),
            status_text: QString::default(),
            widget_id: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            job: Arc::new(Mutex::new(None)),
            busy: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl qobject::WeatherContent {
    fn attach(mut self: std::pin::Pin<&mut Self>, widget_id: &QString) {
        let id = widget_id.to_string();
        self.as_mut().rust_mut().widget_id = id.clone();
        if let Some(widget) = layout_store::find(&id) {
            if let Some(city) = widget.data.get("city").and_then(|v| v.as_str()) {
                self.as_mut().set_city(QString::from(city));
            }
        }
        self.as_mut().refresh();
    }

    fn choose_city(mut self: std::pin::Pin<&mut Self>, city: &QString) {
        let text = city.to_string();
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            self.as_mut().set_status_text(QString::from("城市不能为空"));
            return;
        }
        self.as_mut().set_city(QString::from(&trimmed));
        {
            let mut rust = self.as_mut().rust_mut();
            rust.latitude = 0.0;
            rust.longitude = 0.0;
        }
        let widget_id = self.rust().widget_id.clone();
        if !widget_id.is_empty() {
            layout_store::update_widget(&widget_id, |widget| {
                widget
                    .data
                    .insert("city".into(), serde_json::Value::String(trimmed.clone()));
            });
        }
        self.as_mut().refresh();
    }

    fn refresh(mut self: std::pin::Pin<&mut Self>) {
        if self.rust().busy.swap(true, Ordering::SeqCst) {
            return;
        }
        let city = self.city().to_string();
        if city.is_empty() {
            self.as_mut().rust_mut().busy.store(false, Ordering::SeqCst);
            self.as_mut().set_status_text(QString::from("请先设置城市"));
            return;
        }
        let latitude = self.rust().latitude;
        let longitude = self.rust().longitude;
        let job = self.rust().job.clone();
        let busy = self.rust().busy.clone();
        self.as_mut().set_status_text(QString::from("正在更新…"));

        std::thread::spawn(move || {
            let result = fetch_weather(&city, latitude, longitude);
            if let Ok(mut slot) = job.lock() {
                *slot = Some(result);
            }
            busy.store(false, Ordering::SeqCst);
        });
    }

    fn poll(mut self: std::pin::Pin<&mut Self>) {
        let result = self.rust().job.lock().ok().and_then(|mut slot| slot.take());
        let Some(result) = result else {
            return;
        };

        match result {
            Ok((snapshot, offline)) => {
                {
                    let mut rust = self.as_mut().rust_mut();
                    rust.latitude = snapshot.latitude;
                    rust.longitude = snapshot.longitude;
                }
                self.as_mut().set_city(QString::from(&snapshot.city));
                self.as_mut()
                    .set_temperature_text(QString::from(&snapshot.temperature_text()));
                self.as_mut()
                    .set_condition_text(QString::from(snapshot.condition()));
                self.as_mut()
                    .set_symbol_text(QString::from(snapshot.symbol()));
                self.as_mut()
                    .set_range_text(QString::from(&snapshot.range_text()));
                self.as_mut()
                    .set_updated_text(QString::from(&dm_core::weather::updated_text(&snapshot)));
                self.as_mut().set_status_text(QString::from(if offline {
                    "离线：显示上次数据"
                } else {
                    ""
                }));
            }
            Err(error) => {
                self.as_mut().set_status_text(QString::from(&error));
            }
        }
    }
}

fn fetch_weather(
    city: &str,
    latitude: f64,
    longitude: f64,
) -> Result<(dm_core::weather::WeatherSnapshot, bool), String> {
    use dm_core::weather;

    let (lat, lon) = if latitude == 0.0 || longitude == 0.0 {
        let url = format!(
            "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=zh",
            url_encode(city)
        );
        let body = dm_platform::http::get_json(&url, std::time::Duration::from_secs(8))
            .map_err(|e| format!("{e} [{url}]"))?;
        let (_, lat, lon) = weather::parse_geocode(&body)?;
        (lat, lon)
    } else {
        (latitude, longitude)
    };

    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat:.4}&longitude={lon:.4}&current=temperature_2m,weather_code&daily=temperature_2m_max,temperature_2m_min&timezone=auto&forecast_days=1"
    );
    let key = format!("weather-{city}");

    match dm_platform::http::get_json(&url, std::time::Duration::from_secs(8)) {
        Ok(body) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let snapshot = weather::parse_forecast(city, &body, now)?;
            if let Ok(json) = serde_json::to_string(&snapshot) {
                let _ = dm_platform::cache::write(&key, &json);
            }
            Ok((snapshot, false))
        }
        Err(error) => {
            // 网络失败时退回缓存，避免瓦片变成一片空白。
            if let Some(text) = dm_platform::cache::read(&key) {
                if let Ok(mut snapshot) = serde_json::from_str::<weather::WeatherSnapshot>(&text) {
                    snapshot.city = city.to_string();
                    return Ok((snapshot, true));
                }
            }
            Err(format!("天气获取失败：{error}"))
        }
    }
}

fn url_encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

// ------------------------------------------------------------ 时光瓦片

pub struct GlanceContentRust {
    time_text: QString,
    date_text: QString,
    weekday_text: QString,
    lunar_text: QString,
    term_text: QString,
    festival_text: QString,
}

impl Default for GlanceContentRust {
    fn default() -> Self {
        let snapshot = dm_core::glance::snapshot_now();
        Self {
            time_text: QString::from(&snapshot.time),
            date_text: QString::from(&snapshot.date),
            weekday_text: QString::from(&snapshot.weekday),
            lunar_text: QString::from(&snapshot.lunar),
            term_text: QString::from(snapshot.term.unwrap_or("")),
            festival_text: QString::from(snapshot.festival.as_deref().unwrap_or("")),
        }
    }
}

impl qobject::GlanceContent {
    fn refresh(mut self: std::pin::Pin<&mut Self>) {
        let snapshot = dm_core::glance::snapshot_now();
        self.as_mut().set_time_text(QString::from(&snapshot.time));
        self.as_mut().set_date_text(QString::from(&snapshot.date));
        self.as_mut()
            .set_weekday_text(QString::from(&snapshot.weekday));
        self.as_mut().set_lunar_text(QString::from(&snapshot.lunar));
        self.as_mut()
            .set_term_text(QString::from(snapshot.term.unwrap_or("")));
        self.as_mut()
            .set_festival_text(QString::from(snapshot.festival.as_deref().unwrap_or("")));
    }
}

// ------------------------------------------------------------ 首次引导

pub struct OnboardingRust {
    page: i32,
    page_count: i32,
}

impl Default for OnboardingRust {
    fn default() -> Self {
        Self {
            page: 0,
            page_count: 3,
        }
    }
}

impl qobject::Onboarding {
    fn next_page(mut self: std::pin::Pin<&mut Self>) {
        let next = (*self.page() + 1).min(*self.page_count() - 1);
        self.as_mut().set_page(next);
    }

    fn previous_page(mut self: std::pin::Pin<&mut Self>) {
        let previous = (*self.page() - 1).max(0);
        self.as_mut().set_page(previous);
    }

    fn finish(self: std::pin::Pin<&mut Self>) {
        let mut settings = dm_platform::settings_store::load();
        settings.onboarded = true;
        let _ = dm_platform::settings_store::save(&settings);
    }
}

/// 首次启动时是否需要展示引导。
pub fn should_show_onboarding() -> bool {
    !dm_platform::settings_store::load().onboarded
}

unsafe fn open_onboarding_window() -> u64 {
    let engine = engine_slot().load(std::sync::atomic::Ordering::SeqCst);
    if engine.is_null() {
        return 0;
    }
    unsafe { dm_spawn_onboarding_window(engine) }
}

pub fn spawn_onboarding() -> bool {
    unsafe { open_onboarding_window() != 0 }
}

/// 开发验证入口：启动时直接打开设置窗口（DESKMASON_OPEN=settings）。
pub fn open_settings_dev() {
    unsafe { open_settings_window() };
}

/// 开发验证入口：启动时直接打开搜索窗口（DESKMASON_OPEN=search）。
pub fn open_search_dev() {
    unsafe { open_search_window() };
}

// ------------------------------------------------------------ 设置面板

pub struct SettingsPanelRust {
    hotkey: QString,
    autostart: bool,
    default_icon_size: i32,
    default_name_lines: i32,
    default_density: QString,
    default_sort_mode: QString,
    status_text: QString,
    backup_status: QString,
    version: QString,
    current_page: i32,
    menu_visible: bool,
    capsule_enabled: bool,
    capsule_expand_mode: QString,
    capsule_width_mode: QString,
    capsule_collapsed_content: QString,
    capsule_arrangement: QString,
    capsule_hide_content: bool,
    capsule_hover_response: QString,
    tray_icon_style: QString,
    use_system_accent: bool,
    window_material: QString,
    display_density: QString,
    titlebar_style: QString,
    show_hide_animation: QString,
    file_show_previews: bool,
    file_auto_stack: bool,
    hover_actions_enabled: bool,
    general_language: QString,
    attachment_mode: QString,
    app_theme: QString,
    tool_capture: bool,
    tool_todo: bool,
    tool_music: bool,
    tool_weather: bool,
    tool_glance: bool,
    tool_search: bool,
    glance_bg_enabled: bool,
    glance_bg_path: QString,
}

impl SettingsPanelRust {
    fn load_ui_prefs() -> Self {
        let settings = dm_platform::settings_store::load();
        let defaults = settings.tile_defaults.clamped();
        let page = dm_platform::prefs::get_string("settings/page")
            .and_then(|text| text.parse::<i32>().ok())
            .unwrap_or(
                std::env::var("DESKMASON_PAGE")
                    .ok()
                    .and_then(|text| text.parse::<i32>().ok())
                    .unwrap_or(0),
            );
        Self {
            hotkey: QString::from(&settings.hotkey),
            autostart: dm_platform::autostart::is_enabled(),
            default_icon_size: defaults.icon_size as i32,
            default_name_lines: defaults.name_lines as i32,
            default_density: QString::from(&defaults.density),
            default_sort_mode: QString::from(&defaults.sort_mode),
            status_text: QString::default(),
            backup_status: QString::default(),
            version: QString::from(env!("CARGO_PKG_VERSION")),
            current_page: page.clamp(0, 7),
            menu_visible: dm_platform::prefs::get_bool("settings/menuOpen").unwrap_or(true),
            capsule_enabled: dm_platform::prefs::get_bool("capsule/enabled").unwrap_or(true),
            capsule_expand_mode: QString::from(
                &dm_platform::prefs::get_string("capsule/expandMode")
                    .unwrap_or_else(|| "hover".into()),
            ),
            capsule_width_mode: QString::from(
                &dm_platform::prefs::get_string("capsule/widthMode")
                    .unwrap_or_else(|| "same".into()),
            ),
            capsule_collapsed_content: QString::from(
                &dm_platform::prefs::get_string("capsule/collapsedContent")
                    .unwrap_or_else(|| "key".into()),
            ),
            capsule_arrangement: QString::from(
                &dm_platform::prefs::get_string("capsule/arrangement")
                    .unwrap_or_else(|| "independent".into()),
            ),
            capsule_hide_content: dm_platform::prefs::get_bool("capsule/hideContent")
                .unwrap_or(false),
            capsule_hover_response: QString::from(
                &dm_platform::prefs::get_string("capsule/hoverResponse")
                    .unwrap_or_else(|| "fast".into()),
            ),
            tray_icon_style: QString::from(
                &dm_platform::prefs::get_string("appearance/trayIconStyle")
                    .unwrap_or_else(|| "color".into()),
            ),
            use_system_accent: dm_platform::prefs::get_bool("appearance/systemAccent")
                .unwrap_or(true),
            window_material: QString::from(
                &dm_platform::prefs::get_string("appearance/material")
                    .unwrap_or_else(|| "mica".into()),
            ),
            display_density: QString::from(
                &dm_platform::prefs::get_string("appearance/density")
                    .unwrap_or_else(|| "normal".into()),
            ),
            titlebar_style: QString::from(
                &dm_platform::prefs::get_string("appearance/titlebar")
                    .unwrap_or_else(|| "floating".into()),
            ),
            show_hide_animation: QString::from(
                &dm_platform::prefs::get_string("appearance/animation")
                    .unwrap_or_else(|| "standard".into()),
            ),
            file_show_previews: dm_platform::prefs::get_bool("files/showPreviews").unwrap_or(true),
            file_auto_stack: dm_platform::prefs::get_bool("files/autoStack").unwrap_or(false),
            hover_actions_enabled: dm_platform::prefs::get_bool("interaction/hoverActions")
                .unwrap_or(true),
            general_language: QString::from(
                &dm_platform::prefs::get_string("general/language")
                    .unwrap_or_else(|| "system".into()),
            ),
            attachment_mode: QString::from(
                &dm_platform::prefs::get_string("general/attachmentMode")
                    .unwrap_or_else(|| "linked".into()),
            ),
            app_theme: QString::from(
                &dm_platform::prefs::get_string("general/appTheme")
                    .unwrap_or_else(|| "system".into()),
            ),
            // 功能瓦片开关以布局文档为唯一真相：有这个类型的瓦片就是"开"。
            // 旧版本把这些开关当偏好存，导致开关状态和桌面上的格子不一致。
            tool_capture: layout_store::has_kind(WidgetKind::QuickCapture),
            tool_todo: layout_store::has_kind(WidgetKind::Todo),
            tool_music: layout_store::has_kind(WidgetKind::Music),
            tool_weather: layout_store::has_kind(WidgetKind::Weather),
            tool_glance: layout_store::has_kind(WidgetKind::Glance),
            tool_search: dm_platform::prefs::get_bool("tools/search").unwrap_or(true),
            glance_bg_enabled: dm_platform::prefs::get_bool("glance/bgEnabled").unwrap_or(true),
            glance_bg_path: QString::from(
                &dm_platform::prefs::get_string("glance/background").unwrap_or_default(),
            ),
        }
    }
}

impl Default for SettingsPanelRust {
    fn default() -> Self {
        Self::load_ui_prefs()
    }
}

impl qobject::SettingsPanel {
    fn create_backup(mut self: std::pin::Pin<&mut Self>) {
        match dm_platform::backup::create_backup() {
            Ok(path) => self
                .as_mut()
                .set_backup_status(QString::from(&format!("已备份到 {}", path.display()))),
            Err(error) => self.as_mut().set_backup_status(QString::from(&error)),
        }
    }

    fn restore_latest_backup(mut self: std::pin::Pin<&mut Self>) {
        let Some(latest) = dm_platform::backup::list_backups().into_iter().next() else {
            self.as_mut()
                .set_backup_status(QString::from("没有找到备份文件"));
            return;
        };
        match dm_platform::backup::restore_backup(&latest) {
            Ok(()) => self.as_mut().set_backup_status(QString::from(&format!(
                "已恢复 {}，请重启 DeskMason 生效",
                latest.display()
            ))),
            Err(error) => self.as_mut().set_backup_status(QString::from(&error)),
        }
    }

    fn save(mut self: std::pin::Pin<&mut Self>, hotkey: &QString, autostart: bool) {
        let text = hotkey.to_string();
        let trimmed = text.trim().to_string();
        if dm_platform::hotkey::Hotkey::parse(&trimmed).is_none() {
            self.as_mut()
                .set_status_text(QString::from("热键格式无效，例如 ctrl+alt+d"));
            return;
        }

        let settings = dm_platform::settings_store::HostSettings {
            hotkey: trimmed.clone(),
            autostart,
            tile_defaults: dm_platform::settings_store::TileDefaults {
                icon_size: (*self.default_icon_size()).clamp(24, 96) as u32,
                name_lines: (*self.default_name_lines()).clamp(0, 2) as u8,
                density: self.default_density().to_string(),
                sort_mode: self.default_sort_mode().to_string(),
            }
            .clamped(),
            onboarded: dm_platform::settings_store::load().onboarded,
        };
        if let Err(error) = dm_platform::settings_store::save(&settings) {
            self.as_mut()
                .set_status_text(QString::from(&format!("保存设置失败：{error}")));
            return;
        }
        if let Err(error) = dm_platform::autostart::set_enabled(autostart) {
            self.as_mut().set_status_text(QString::from(&format!(
                "热键已保存，但自启设置失败：{error}"
            )));
        } else {
            self.as_mut()
                .set_status_text(QString::from("已保存。热键在下次启动后生效。"));
        }
        self.as_mut().set_hotkey(QString::from(&trimmed));
        self.as_mut().set_autostart(autostart);
        self.as_mut()
            .set_default_icon_size(settings.tile_defaults.icon_size as i32);
        self.as_mut()
            .set_default_name_lines(settings.tile_defaults.name_lines as i32);
        self.as_mut()
            .set_default_density(QString::from(&settings.tile_defaults.density));
        self.as_mut()
            .set_default_sort_mode(QString::from(&settings.tile_defaults.sort_mode));
    }

    /// 把界面状态写入偏好文件。QML 在页面切换/菜单开合/下拉变更时调用。
    fn save_ui_state(self: std::pin::Pin<&mut Self>) {
        dm_platform::prefs::set_string("settings/page", &self.current_page().to_string());
        dm_platform::prefs::set_bool("settings/menuOpen", *self.menu_visible());
        dm_platform::prefs::set_bool("capsule/enabled", *self.capsule_enabled());
        dm_platform::prefs::set_string(
            "capsule/expandMode",
            &self.capsule_expand_mode().to_string(),
        );
        dm_platform::prefs::set_string("capsule/widthMode", &self.capsule_width_mode().to_string());
        dm_platform::prefs::set_string(
            "capsule/collapsedContent",
            &self.capsule_collapsed_content().to_string(),
        );
        dm_platform::prefs::set_string(
            "capsule/arrangement",
            &self.capsule_arrangement().to_string(),
        );
        dm_platform::prefs::set_bool("capsule/hideContent", *self.capsule_hide_content());
        dm_platform::prefs::set_string(
            "capsule/hoverResponse",
            &self.capsule_hover_response().to_string(),
        );
        dm_platform::prefs::set_string(
            "appearance/trayIconStyle",
            &self.tray_icon_style().to_string(),
        );
        dm_platform::prefs::set_bool("appearance/systemAccent", *self.use_system_accent());
        dm_platform::prefs::set_string("appearance/material", &self.window_material().to_string());
        dm_platform::prefs::set_string("appearance/density", &self.display_density().to_string());
        dm_platform::prefs::set_string("appearance/titlebar", &self.titlebar_style().to_string());
        dm_platform::prefs::set_string(
            "appearance/animation",
            &self.show_hide_animation().to_string(),
        );
        dm_platform::prefs::set_bool("files/showPreviews", *self.file_show_previews());
        dm_platform::prefs::set_bool("files/autoStack", *self.file_auto_stack());
        dm_platform::prefs::set_bool("interaction/hoverActions", *self.hover_actions_enabled());
        dm_platform::prefs::set_string("general/language", &self.general_language().to_string());
        dm_platform::prefs::set_string(
            "general/attachmentMode",
            &self.attachment_mode().to_string(),
        );
        dm_platform::prefs::set_string("general/appTheme", &self.app_theme().to_string());
        // 功能瓦片开关以布局为唯一真相，不再写偏好；只有"搜索"仍是偏好。
        dm_platform::prefs::set_bool("tools/search", *self.tool_search());
        dm_platform::prefs::set_bool("glance/bgEnabled", *self.glance_bg_enabled());
        dm_platform::prefs::set_string("glance/background", &self.glance_bg_path().to_string());
    }

    fn show_onboarding(self: std::pin::Pin<&mut Self>) {
        spawn_onboarding();
    }

    /// 设置页的功能瓦片开关：即时创建/移除对应瓦片窗口。
    /// 内容瓦片是单例，布局层 add_widget 会挡住重复创建。
    fn set_tool_enabled(mut self: std::pin::Pin<&mut Self>, kind_key: &QString, enabled: bool) {
        use dm_core::widget::WidgetKind;
        let kind = match kind_key.to_string().as_str() {
            "quickCapture" => WidgetKind::QuickCapture,
            "todo" => WidgetKind::Todo,
            "music" => WidgetKind::Music,
            "weather" => WidgetKind::Weather,
            "glance" => WidgetKind::Glance,
            _ => return,
        };

        if enabled {
            if layout_store::has_kind(kind) {
                return;
            }
            let rect = layout_store::default_tile_rect(layout_store::widgets().len() as i32);
            if let Some(widget) = layout_store::add_widget(kind, rect) {
                unsafe { spawn_tile(&widget.id) };
            }
        } else if let Some(widget) = layout_store::widget_of_kind(kind) {
            // 先从布局里删，再关窗口：窗口的 onClosing 会保存几何，
            // 此时 widget 已不存在，save_placement 会跳过，不会写回孤儿记录。
            layout_store::remove_widget(&widget.id);
            if let Ok(c_id) = std::ffi::CString::new(widget.id) {
                unsafe { dm_close_tile(c_id.as_ptr()) };
            }
        }

        let now = layout_store::has_kind(kind);
        match kind {
            WidgetKind::QuickCapture => self.as_mut().set_tool_capture(now),
            WidgetKind::Todo => self.as_mut().set_tool_todo(now),
            WidgetKind::Music => self.as_mut().set_tool_music(now),
            WidgetKind::Weather => self.as_mut().set_tool_weather(now),
            WidgetKind::Glance => self.as_mut().set_tool_glance(now),
            _ => {}
        }
        self.as_mut().set_status_text(QString::from(if now {
            "已开启，桌面格子即时生效"
        } else {
            "已关闭"
        }));
    }

    /// 时光瓦片背景：空路径 = 跟随桌面壁纸；自定义路径必须是本地图片。
    fn set_glance_background(mut self: std::pin::Pin<&mut Self>, path: &QString, enabled: bool) {
        let raw = path.to_string().trim().to_string();
        let resolved = if raw.is_empty() {
            None
        } else {
            dm_platform::wallpaper::resolve_image(&raw)
        };
        if !raw.is_empty() && resolved.is_none() {
            self.as_mut()
                .set_status_text(QString::from("图片路径无效，背景未更改"));
            return;
        }
        let stored = resolved
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        dm_platform::prefs::set_bool("glance/bgEnabled", enabled);
        dm_platform::prefs::set_string("glance/background", &stored);
        self.as_mut().set_glance_bg_enabled(enabled);
        self.as_mut().set_glance_bg_path(QString::from(&stored));
        self.as_mut()
            .set_status_text(QString::from(if stored.is_empty() {
                "已使用当前桌面壁纸，稍后生效"
            } else {
                "背景已更新，稍后生效"
            }));
    }

    fn desktop_wallpaper_path(&self) -> QString {
        match dm_platform::wallpaper::desktop_wallpaper() {
            Some(path) => QString::from(&path.to_string_lossy().to_string()),
            None => QString::default(),
        }
    }

    fn reveal_backup_dir(self: std::pin::Pin<&mut Self>) {
        let _ = dm_platform::launch::reveal_path(&dm_platform::backup::backup_dir());
    }
}

fn sort_mode_key(mode: dm_core::file_tile::FileSortMode) -> &'static str {
    use dm_core::file_tile::FileSortMode::*;
    match mode {
        Name => "name",
        Modified => "modified",
        Size => "size",
        Kind => "kind",
    }
}

fn parse_sort_mode(text: &str) -> dm_core::file_tile::FileSortMode {
    use dm_core::file_tile::FileSortMode::*;
    match text {
        "modified" => Modified,
        "size" => Size,
        "kind" => Kind,
        _ => Name,
    }
}

fn path_to_file_url(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    let mut out = String::from("file://");
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

// ---------------------------------------------------------------- 剪贴板

extern "C" {
    fn dm_clipboard_write(uri_list: *const core::ffi::c_char, cut: bool) -> bool;
    fn dm_clipboard_read() -> *mut core::ffi::c_char;
    fn dm_free_string(ptr: *mut core::ffi::c_char);
}

fn clipboard_write(paths: &[PathBuf], cut: bool) -> bool {
    let joined = paths
        .iter()
        .map(|p| path_to_file_url(p))
        .collect::<Vec<_>>()
        .join("\n");
    let Ok(c_text) = std::ffi::CString::new(joined) else {
        return false;
    };
    unsafe { dm_clipboard_write(c_text.as_ptr(), cut) }
}

fn clipboard_read() -> Option<(bool, Vec<PathBuf>)> {
    unsafe {
        let ptr = dm_clipboard_read();
        if ptr.is_null() {
            return None;
        }
        let text = std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string();
        dm_free_string(ptr);
        let mut lines = text.lines();
        let mode = lines.next().unwrap_or("copy");
        let paths: Vec<PathBuf> = lines
            .filter(|line| !line.is_empty())
            .filter_map(dm_platform::files::url_to_path)
            .collect();
        if paths.is_empty() {
            return None;
        }
        Some((mode == "cut", paths))
    }
}

// ------------------------------------------------------- 桌面层窗口胶水

/// 供 C++ 胶水调用：把 Qt 创建的原生窗口设为桌面层。
#[no_mangle]
pub extern "C" fn dm_apply_desktop_layer_atoms(window_id: u64) -> bool {
    let mode = std::env::var("DESKMANSON_LAYER").unwrap_or_default();
    if mode == "normal" {
        return true;
    }
    let result = if mode == "below" {
        dm_platform::x11::apply_below_layer(window_id as u32, "DeskMason")
    } else {
        dm_platform::x11::apply_desktop_layer(window_id as u32, "DeskMason")
    };
    result.is_ok()
}

/// 供 C++ 胶水调用：给弹窗类窗口（搜索 / 设置）开启 KWin 背景模糊。
/// 材质档位由设置里的 `appearance/material` 决定：opaque 时不加模糊。
#[no_mangle]
pub extern "C" fn dm_apply_window_blur(window_id: u64) -> bool {
    let material =
        dm_platform::prefs::get_string("appearance/material").unwrap_or_else(|| "mica".to_string());
    if material == "opaque" {
        return true;
    }
    dm_platform::x11::apply_blur_behind(window_id as u32).is_ok()
}

extern "C" {
    /// 加载 Qt 自带翻译（标准按钮文案）。
    fn dm_install_translations();

    /// 强制浅色主题（Fusion 样式 + 浅色调色板）。
    fn dm_apply_light_theme();

    /// 加载应用翻译（.qm 文件路径）。
    fn dm_install_translation_file(path: *const core::ffi::c_char) -> bool;

    /// 打开（或前置）设置窗口，返回窗口 id；0 表示失败。
    fn dm_spawn_settings_window(engine_ptr: *mut core::ffi::c_void) -> u64;

    /// 打开（或前置）搜索窗口，返回窗口 id；0 表示失败。
    fn dm_spawn_search_window(engine_ptr: *mut core::ffi::c_void) -> u64;

    /// 打开首次运行引导窗口。
    fn dm_spawn_onboarding_window(engine_ptr: *mut core::ffi::c_void) -> u64;

    /// 退出 QCoreApplication 事件循环。
    fn dm_app_quit();

    /// 由 C++ 胶水实现：用 QML 组件创建一个瓦片窗口并贴到桌面层。
    /// 所有瓦片统一 WindowDoesNotAcceptFocus 保证外观一致；`accepts_focus`
    /// 只决定该瓦片是否需要在鼠标按下时由 C++ 显式接管 X 输入焦点。
    fn dm_spawn_tile(
        engine_ptr: *mut core::ffi::c_void,
        widget_id: *const core::ffi::c_char,
        qml_path: *const core::ffi::c_char,
        accepts_focus: bool,
    ) -> u64;

    /// 由 C++ 胶水实现：把单个瓦片放回 below 层（普通窗口之下）。
    fn dm_keep_tile_below(widget_id: *const core::ffi::c_char);

    /// 按固定顺序把全部瓦片固定回 below 层（内部节流）。
    fn dm_keep_tiles_below();

    /// 由 C++ 胶水实现：把结算后的几何写回瓦片窗口（松手留缝用）。
    fn dm_set_tile_geometry(widget_id: *const core::ffi::c_char, x: i32, y: i32, w: i32, h: i32);

    /// 由 C++ 胶水实现：让瓦片窗口取得 X 输入焦点（输入框用）。
    fn dm_focus_tile(widget_id: *const core::ffi::c_char);

    /// 由 C++ 胶水实现：关闭并释放指定瓦片窗口（设置页开关用）。
    fn dm_close_tile(widget_id: *const core::ffi::c_char);

    /// 由 C++ 胶水实现：对瓦片窗口发起窗口管理器原生拖动。
    /// 返回 false 表示窗管不接手（QML 退回手动拖动）。
    fn dm_start_system_move(widget_id: *const core::ffi::c_char) -> bool;
}

/// C++ 胶水回调：把原生窗口固定回 below 层（BELOW 状态 + restack）。
///
/// 瓦片绝不能压在桌面上的普通窗口（微信、文件管理器等）之上；
/// `StackMode::ABOVE` 的旧做法已废弃。
///
/// # Safety
/// 由 window_glue.cpp 调用，`window_id` 必须是有效的 X11 窗口 id。
#[no_mangle]
pub extern "C" fn dm_keep_window_below_x11(window_id: u64) {
    let _ = dm_platform::x11::keep_tile_below(window_id as u32);
}

/// C++ 胶水回调：映射后放开激活焦点时，把窗口类型写回统一值。
///
/// # Safety
/// 由 window_glue.cpp 调用，`window_id` 必须是有效的 X11 窗口 id。
#[no_mangle]
pub extern "C" fn dm_apply_tile_window_type(window_id: u64) {
    let _ = dm_platform::x11::apply_tile_window_type(window_id as u32);
}

/// C++ 胶水回调：把 X 输入焦点设给指定瓦片窗口。
///
/// # Safety
/// 由 window_glue.cpp 调用，`window_id` 必须是有效的 X11 窗口 id。
#[no_mangle]
pub extern "C" fn dm_focus_window_x11(window_id: u64) {
    let _ = dm_platform::x11::focus_window(window_id as u32);
}

/// 创建并显示一个瓦片窗口；返回 0 表示失败。
///
/// # Safety
/// `engine` 必须是有效的 `QQmlApplicationEngine` 指针。
pub unsafe fn spawn_tile_with_engine(engine: *mut core::ffi::c_void, widget_id: &str) -> u64 {
    let Ok(c_id) = std::ffi::CString::new(widget_id) else {
        return 0;
    };
    let kind = layout_store::find(widget_id)
        .map(|w| w.kind)
        .unwrap_or(dm_core::widget::WidgetKind::File);
    let Ok(c_qml) = std::ffi::CString::new(qml_path_for(kind)) else {
        return 0;
    };
    // 只有含输入框的瓦片需要在鼠标按下时接管 X 焦点；纯展示瓦片保持
    // 完全不参与焦点切换，避免 fcitx5 输入上下文反复横跳。
    let accepts_focus = matches!(
        kind,
        dm_core::widget::WidgetKind::Todo
            | dm_core::widget::WidgetKind::QuickCapture
            | dm_core::widget::WidgetKind::Weather
    );
    unsafe { dm_spawn_tile(engine, c_id.as_ptr(), c_qml.as_ptr(), accepts_focus) }
}

/// 瓦片类型 → QML 文件。新增一种类型只改这里的一行 + 一个新 QML。
fn qml_path_for(kind: dm_core::widget::WidgetKind) -> &'static str {
    use dm_core::widget::WidgetKind::*;
    match kind {
        File => "qrc:/qt/qml/com/deskmason/probe/qml/main.qml",
        // M2 还没实现的内容类型先给占位，保证窗口能出来、能拖动、能删除。
        Glance => "qrc:/qt/qml/com/deskmason/probe/qml/glance.qml",
        Weather => "qrc:/qt/qml/com/deskmason/probe/qml/weather.qml",
        Todo => "qrc:/qt/qml/com/deskmason/probe/qml/todo.qml",
        QuickCapture => "qrc:/qt/qml/com/deskmason/probe/qml/quickcapture.qml",
        Search => "qrc:/qt/qml/com/deskmason/probe/qml/search.qml",
        Music => "qrc:/qt/qml/com/deskmason/probe/qml/music.qml",
    }
}

fn engine_slot() -> &'static std::sync::atomic::AtomicPtr<core::ffi::c_void> {
    static ENGINE: std::sync::atomic::AtomicPtr<core::ffi::c_void> =
        std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
    &ENGINE
}

pub fn install_translations() {
    unsafe { dm_install_translations() };
}

pub fn apply_light_theme() {
    unsafe { dm_apply_light_theme() };
}

/// 内置的英文翻译。中文是源语言，没有对应文件时直接显示源文案。
static TRANSLATION_EN: &[u8] = include_bytes!("../../../translations/deskmason_en_US.qm");

/// 按系统语言加载应用自己的翻译。目前只有 en-US；其它语言回落到中文源文案。
pub fn install_app_translations() -> bool {
    let locale = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default()
        .to_lowercase();
    if !locale.starts_with("en") {
        return false;
    }

    let path = dm_platform::paths::home_dir()
        .join(".cache/deskmason/i18n")
        .join("deskmason_en_US.qm");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&path, TRANSLATION_EN).is_err() {
        return false;
    }
    let Ok(c_path) = std::ffi::CString::new(path.to_string_lossy().to_string()) else {
        return false;
    };
    unsafe { dm_install_translation_file(c_path.as_ptr()) }
}

pub fn set_engine(engine: *mut core::ffi::c_void) {
    engine_slot().store(engine, std::sync::atomic::Ordering::SeqCst);
}

unsafe fn open_settings_window() -> u64 {
    let engine = engine_slot().load(std::sync::atomic::Ordering::SeqCst);
    if engine.is_null() {
        return 0;
    }
    unsafe { dm_spawn_settings_window(engine) }
}

unsafe fn open_search_window() -> u64 {
    let engine = engine_slot().load(std::sync::atomic::Ordering::SeqCst);
    if engine.is_null() {
        return 0;
    }
    unsafe { dm_spawn_search_window(engine) }
}

unsafe fn spawn_tile(widget_id: &str) -> u64 {
    let engine = engine_slot().load(std::sync::atomic::Ordering::SeqCst);
    if engine.is_null() {
        return 0;
    }
    unsafe { spawn_tile_with_engine(engine, widget_id) }
}

/// 启动时按布局创建所有文件瓦片窗口。
///
/// 布局为空时补一个默认瓦片：否则用户删光瓦片后，下次启动没有窗口、
/// 进程会立刻退出，而且再也进不来。
pub fn spawn_all_tiles() -> usize {
    // 旧版本允许从「+」重复添加内容瓦片，启动时收敛到每种一个。
    let removed = layout_store::enforce_singletons();
    if !removed.is_empty() {
        eprintln!("已合并重复的功能瓦片: {}", removed.join(", "));
    }
    // 只允许主屏：先把副屏/越界瓦片收回主屏并拆开重叠，再建窗口。
    // 不走多屏 reconcile_monitors，否则会把瓦片重新归属到副屏。
    let moved = layout_store::enforce_primary_screen();
    if !moved.is_empty() {
        eprintln!("已把瓦片收回主屏: {}", moved.join(", "));
    }
    let mut widgets = layout_store::widgets();
    if widgets.is_empty() {
        let folder = paths::default_folder();
        if let Some(widget) =
            layout_store::add_file_widget(&folder, layout_store::default_tile_rect(0))
        {
            widgets.push(widget);
        }
    }

    let mut count = 0;
    for widget in widgets {
        if unsafe { spawn_tile(&widget.id) } != 0 {
            count += 1;
        }
    }
    count
}

#[allow(dead_code)]
fn _assert_kind_used() {
    let _ = WidgetKind::File;
}
