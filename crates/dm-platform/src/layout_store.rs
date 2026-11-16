//! 布局文件的读写。
//!
//! 所有落盘都走这里：启动加载、窗口几何、瓦片设置。写盘用"临时文件 + rename"，
//! 断电时不会留下半个 JSON。

use crate::display;
use crate::paths;
use crate::settings_store;
use dm_core::file_tile_settings::{Density, FileTileSettings};
use dm_core::geometry::{MonitorId, Rect};
use dm_core::layout::{LayoutDocument, MonitorLayout, Placement};
use dm_core::widget::{WidgetConfig, WidgetKind};
use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::sync::OnceLock;

/// 默认新建瓦片的尺寸。
pub const DEFAULT_TILE_RECT: Rect = Rect {
    x: 160,
    y: 140,
    width: 420,
    height: 520,
};

/// 进程内布局文档缓存。
///
/// 窗口几何在拖动中以每秒几十次的频率保存；如果每次 save_placement 都
/// 读盘 + 解析 JSON，Qt 主线程会被文件 I/O 卡出一帧一顿的观感。
/// 这里缓存最近一次读到的文档（带源文件 mtime），save 时直接在缓存上
/// 改并写穿到磁盘——拖动路径只剩一次小 JSON 写，读路径全部命中内存。
static DOC_CACHE: OnceLock<Mutex<Option<CachedDoc>>> = OnceLock::new();

struct CachedDoc {
    document: LayoutDocument,
    source_mtime: Option<std::time::SystemTime>,
}

fn doc_cache() -> &'static Mutex<Option<CachedDoc>> {
    DOC_CACHE.get_or_init(|| Mutex::new(None))
}

pub fn load() -> LayoutDocument {
    let path = paths::layout_path();
    let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();

    // 缓存命中：源文件没被外部改动时直接复用内存文档。
    if let Ok(guard) = doc_cache().lock() {
        if let Some(cached) = guard.as_ref() {
            if cached.source_mtime == mtime {
                return cached.document.clone();
            }
        }
    }

    let mut document = None;
    if let Ok(text) = std::fs::read_to_string(&path) {
        match LayoutDocument::from_json(&text) {
            Ok(parsed) => document = Some(parsed),
            Err(_) => {
                // 坏文件不覆盖，改名留档，避免用户配置被静默清空。
                let backup = path.with_extension(format!(
                    "broken-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                ));
                let _ = std::fs::rename(&path, backup);
            }
        }
    }

    let doc = document.unwrap_or_else(|| {
        let mut doc = LayoutDocument::new();
        doc.monitors.insert(
            MonitorId::new("primary"),
            MonitorLayout {
                bounds: Rect::new(0, 0, 1920, 1080),
                scale: 1.0,
            },
        );
        let folder = paths::default_folder();
        let mut widget = new_file_widget(&folder, DEFAULT_TILE_RECT, "file-1");
        widget.title = folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        doc.upsert_widget(widget);
        let _ = save(&doc);
        doc
    });

    // 命中磁盘的最新文档回填缓存。
    if let Ok(mut guard) = doc_cache().lock() {
        *guard = Some(CachedDoc {
            document: doc.clone(),
            source_mtime: std::fs::metadata(&path).and_then(|m| m.modified()).ok(),
        });
    }
    doc
}

pub fn save(document: &LayoutDocument) -> io::Result<()> {
    let path = paths::layout_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = document
        .to_json()
        .map_err(|e| io::Error::other(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;

    // 写穿后同步缓存，下一次 load 直接命中内存。
    if let Ok(mut guard) = doc_cache().lock() {
        *guard = Some(CachedDoc {
            document: document.clone(),
            source_mtime: std::fs::metadata(&path).and_then(|m| m.modified()).ok(),
        });
    }
    Ok(())
}

/// 修改单个瓦片并落盘。返回是否找到了该瓦片。
pub fn update_widget<F>(widget_id: &str, mutate: F) -> bool
where
    F: FnOnce(&mut WidgetConfig),
{
    let mut document = load();
    let Some(widget) = document.widget_mut(widget_id) else {
        return false;
    };
    mutate(widget);
    document.bump();
    let _ = save(&document);
    true
}

/// 用当前显示器拓扑修正布局，返回是否发生了修改。
///
/// - 同步 `monitors` 表（分辨率/缩放会变）；
/// - 显示器被拔掉时，把它名下的瓦片迁到"重叠面积最大"的那块，没有就主屏；
/// - 所有矩形收进所属显示器的边界，避免留在屏幕外。
pub fn reconcile_monitors() -> bool {
    let Ok(monitors) = display::list_monitors() else {
        return false;
    };
    if monitors.is_empty() {
        return false;
    }
    let mut document = load();
    let key = topology_key(&monitors);
    let mut changed = apply_topology(&mut document, &key);
    changed |= reconcile_document(&mut document, &monitors);
    if !changed {
        return false;
    }
    document.bump();
    let _ = save(&document);
    true
}

/// 显示器组合键：各显示器 id 排序后拼接。同一组屏 → 同一个键。
pub fn topology_key(monitors: &[display::MonitorInfo]) -> String {
    let mut ids: Vec<String> = monitors.iter().map(|m| m.id.as_str().to_string()).collect();
    ids.sort();
    ids.dedup();
    ids.join("|")
}

/// 把当前摆放写进某个组合的档位。
fn remember_placements(document: &mut LayoutDocument, key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    let mut changed = false;
    let mut snapshot: Vec<(String, Placement)> = Vec::new();
    for widget in &document.widgets {
        snapshot.push((
            widget.id.clone(),
            Placement {
                rect: widget.rect,
                monitor: widget.monitor.clone(),
            },
        ));
    }
    let entry = document.placements.entry(key.to_string()).or_default();
    for (id, placement) in snapshot {
        if entry.get(&id) != Some(&placement) {
            entry.insert(id, placement);
            changed = true;
        }
    }
    changed
}

/// 切换显示器组合：先把旧组合的摆放存档，再恢复新组合的摆放。
///
/// 新组合没有存档时保持当前位置（随后由 `reconcile_document` 收进屏幕内）。
pub fn apply_topology(document: &mut LayoutDocument, key: &str) -> bool {
    if key.is_empty() || document.active_topology == key {
        return false;
    }
    let previous = document.active_topology.clone();
    if !previous.is_empty() {
        remember_placements(document, &previous);
    }
    document.active_topology = key.to_string();

    if let Some(saved) = document.placements.get(key).cloned() {
        for widget in document.widgets.iter_mut() {
            if let Some(placement) = saved.get(&widget.id) {
                if widget.rect != placement.rect || widget.monitor != placement.monitor {
                    widget.rect = placement.rect;
                    widget.monitor = placement.monitor.clone();
                }
            }
        }
    }
    true
}

/// 保存一次窗口几何：同时写当前瓦片、当前显示器组合的摆放档与
/// 当前主屏分辨率的布局归档。
pub fn save_placement(widget_id: &str, rect: Rect) {
    let mut document = load();
    // 瓦片已被移除（设置页关闭功能瓦片）时，窗口关闭信号仍会带着旧几何
    // 过来；不要写回孤儿摆放记录。
    if document.widget(widget_id).is_none() {
        return;
    }
    let active = document.active_topology.clone();
    let mut changed = false;
    if let Some(widget) = document.widget_mut(widget_id) {
        if widget.rect != rect {
            widget.rect = rect;
            changed = true;
        }
    }
    if !active.is_empty() {
        let monitor = document
            .widget(widget_id)
            .map(|w| w.monitor.clone())
            .unwrap_or_else(|| MonitorId::new("primary"));
        let entry = document.placements.entry(active).or_default();
        let placement = Placement { rect, monitor };
        if entry.get(widget_id) != Some(&placement) {
            entry.insert(widget_id.to_string(), placement);
            changed = true;
        }
    }
    // 分辨率归档：摆设一旦与归档不一致就刷新。旧格式文件（无分辨率
    // 标记）在这里被就地收养——补上标记与首个归档。
    if document.layout_resolution.is_empty() {
        let key = current_resolution_key();
        if !key.is_empty() {
            let topology = document.active_topology.clone();
            document.archive_to_resolution(&key, &topology);
            changed = true;
        }
    } else {
        let key = document.layout_resolution.clone();
        if !document.matches_resolution_archive(&key) {
            let topology = document.active_topology.clone();
            document.archive_to_resolution(&key, &topology);
            changed = true;
        }
    }
    if changed {
        document.bump();
        let _ = save(&document);
    }
}

/// 当前主屏分辨率键（"宽x高"）。X11 不可用时返回空串。
fn current_resolution_key() -> String {
    let area = refresh_primary_area();
    format!("{}x{}", area.width, area.height)
}

/// 纯逻辑部分：给定拓扑修正布局，返回是否发生修改。单独抽出来是为了能用
/// 合成拓扑做测试（Xvfb 不支持虚拟多显示器）。
pub fn reconcile_document(
    document: &mut LayoutDocument,
    monitors: &[display::MonitorInfo],
) -> bool {
    if monitors.is_empty() {
        return false;
    }
    let mut changed = false;

    let mut table = std::collections::BTreeMap::new();
    for monitor in monitors {
        table.insert(
            monitor.id.clone(),
            MonitorLayout {
                bounds: monitor.work(),
                scale: monitor.scale,
            },
        );
    }
    if document.monitors != table {
        document.monitors = table;
        changed = true;
    }

    let Some(primary) = monitors
        .iter()
        .find(|m| m.primary)
        .or_else(|| monitors.first())
        .cloned()
    else {
        return false;
    };

    for widget in document.widgets.iter_mut() {
        // 归属跟着矩形走：拖到哪块屏，那块屏就是它的归属。
        // 用重叠面积最大而不是中心点，边界附近的抖动更少。
        let target = monitors
            .iter()
            .max_by_key(|m| m.work().intersection_area(&widget.rect))
            .filter(|m| m.work().intersection_area(&widget.rect) > 0)
            .unwrap_or(&primary);
        if widget.monitor != target.id {
            widget.monitor = target.id.clone();
            changed = true;
        }
        let clamped = widget.rect.clamp_into(&target.work());
        if clamped != widget.rect {
            widget.rect = clamped;
            changed = true;
        }
    }

    changed
}

pub fn find(widget_id: &str) -> Option<WidgetConfig> {
    load().widget(widget_id).cloned()
}

/// 新建一个文件瓦片并写入布局，返回新 id。
pub fn add_file_widget(folder: &Path, rect: Rect) -> Option<WidgetConfig> {
    let mut document = load();
    let id = next_widget_id(&document);
    let mut widget = new_file_widget(folder, rect, &id);
    widget.title = folder
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    document.upsert_widget(widget.clone());
    document.bump();
    save(&document).ok()?;
    Some(widget)
}

/// 按类型新建瓦片。文件瓦片走 `add_file_widget`（要绑目录），
/// 其余类型直接落布局：默认尺寸取 registry，data 留空由 QML 侧自建存储。
///
/// 内容瓦片（随记/待办/天气/音乐/时光）是单例：每种最多一个，且只能
/// 从设置页开关创建。文件格子不受限制，可以建多个。
pub fn add_widget(kind: WidgetKind, rect: Rect) -> Option<WidgetConfig> {
    if kind == WidgetKind::File {
        return add_file_widget(&paths::default_folder(), rect);
    }
    let registry = dm_core::registry::WidgetRegistry::builtin();
    let (width, height) = registry.describe(kind).default_size;
    let rect = Rect::new(rect.x, rect.y, width, height);
    let mut document = load();
    if document.widgets.iter().any(|w| w.kind == kind) {
        return None;
    }
    let id = next_widget_id_for(&document, kind);
    let mut widget = WidgetConfig::new(id, kind, rect, MonitorId::new("primary"));
    widget.title = kind_title(kind).to_string();
    document.upsert_widget(widget.clone());
    document.bump();
    save(&document).ok()?;
    Some(widget)
}

/// 类型显示名（中文，与 QML 瓦片标题一致）。
fn kind_title(kind: WidgetKind) -> &'static str {
    match kind {
        WidgetKind::File => "文件",
        WidgetKind::QuickCapture => "随记",
        WidgetKind::Weather => "天气",
        WidgetKind::Todo => "待办",
        WidgetKind::Music => "音乐",
        WidgetKind::Glance => "时光",
    }
}

fn next_widget_id_for(document: &LayoutDocument, kind: WidgetKind) -> String {
    let prefix = match kind {
        WidgetKind::File => "file",
        WidgetKind::QuickCapture => "capture",
        WidgetKind::Weather => "weather",
        WidgetKind::Todo => "todo",
        WidgetKind::Music => "music",
        WidgetKind::Glance => "glance",
    };
    let mut index = document.widgets.len() + 1;
    loop {
        let candidate = format!("{prefix}-{index}");
        if document.widget(&candidate).is_none() {
            return candidate;
        }
        index += 1;
    }
}

pub fn remove_widget(widget_id: &str) -> bool {
    let mut document = load();
    if !document.remove_widget(widget_id) {
        return false;
    }
    document.bump();
    save(&document).is_ok()
}

pub fn file_widgets() -> Vec<WidgetConfig> {
    load()
        .widgets
        .into_iter()
        .filter(|w| w.kind == WidgetKind::File)
        .collect()
}

/// 该类型是否已有瓦片（单例判断用）。
pub fn has_kind(kind: WidgetKind) -> bool {
    load().widgets.iter().any(|w| w.kind == kind)
}

/// 找到该类型的第一个瓦片（设置页开关关闭时用）。
pub fn widget_of_kind(kind: WidgetKind) -> Option<WidgetConfig> {
    load().widgets.into_iter().find(|w| w.kind == kind)
}

/// 纯逻辑：内容瓦片每种只保留第一个，返回被移除的 id。
/// 单独抽出来是为了能在不碰磁盘的前提下测试。
fn dedupe_singletons(document: &mut LayoutDocument) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut removed = Vec::new();
    document.widgets.retain(|widget| {
        if widget.kind == WidgetKind::File {
            return true;
        }
        if seen.insert(widget.kind) {
            true
        } else {
            removed.push(widget.id.clone());
            false
        }
    });
    removed
}

/// 启动时归一化历史布局：内容瓦片每种只保留第一个。
/// 旧版本允许从「+」重复添加，升级后要自动收敛到单例。
/// 返回被移除的 id（仅供日志）。
pub fn enforce_singletons() -> Vec<String> {
    let mut document = load();
    let removed = dedupe_singletons(&mut document);
    if !removed.is_empty() {
        document.bump();
        let _ = save(&document);
    }
    removed
}

/// 所有瓦片（不区分类型）。开窗顺序按 order。
pub fn widgets() -> Vec<WidgetConfig> {
    load().widgets
}

fn next_widget_id(document: &LayoutDocument) -> String {
    let mut index = document.widgets.len() + 1;
    loop {
        let candidate = format!("file-{index}");
        if document.widget(&candidate).is_none() {
            return candidate;
        }
        index += 1;
    }
}

fn new_file_widget(folder: &Path, rect: Rect, id: &str) -> WidgetConfig {
    let mut widget = WidgetConfig::new(id, WidgetKind::File, rect, MonitorId::new("primary"));
    widget.data.insert(
        "folder".into(),
        serde_json::Value::String(folder.to_string_lossy().to_string()),
    );
    // 新瓦片继承宿主设置里的默认外观。
    let defaults = settings_store::load().tile_defaults.clamped();
    let settings = FileTileSettings {
        icon_size: defaults.icon_size,
        name_lines: defaults.name_lines,
        density: match defaults.density.as_str() {
            "compact" => Density::Compact,
            "comfortable" => Density::Comfortable,
            _ => Density::Normal,
        },
        sort_mode: match defaults.sort_mode.as_str() {
            "modified" => dm_core::file_tile::FileSortMode::Modified,
            "size" => dm_core::file_tile::FileSortMode::Size,
            "kind" => dm_core::file_tile::FileSortMode::Kind,
            _ => dm_core::file_tile::FileSortMode::Name,
        },
    };
    widget.data.insert("settings".into(), settings.to_value());
    widget
}

// ---------------------------------------------------------------- 屏幕约束

/// 新建瓦片 / 边界磁吸 / 重叠重排的目标间隔：原吸附间隔 8px 的 5 倍。
pub const MIN_TILE_GAP: i32 = 40;

/// 全局「松手留缝」距离。
///
/// 瓦片边缘贴到相邻瓦片边缘或所在屏幕四边时，松手后统一退开 10px，
/// 否则两条边完全重合，下次再也抓不住拖动/缩放边条。这是运行时允许的
/// 最小可操作间隔；40px 只是默认摆位与磁吸的目标，不用来覆盖用户
/// 主动贴住后留下的 10px 缝。
pub const RELEASE_GAP: i32 = 10;

static PRIMARY_WORK: OnceLock<Mutex<Option<Rect>>> = OnceLock::new();

fn primary_work_cache() -> &'static Mutex<Option<Rect>> {
    PRIMARY_WORK.get_or_init(|| Mutex::new(None))
}

/// 主屏可用区。仅用于新建瓦片的默认摆位（新瓦片落在主屏网格上）。
pub fn primary_work_area() -> Rect {
    if let Ok(guard) = primary_work_cache().lock() {
        if let Some(rect) = *guard {
            return rect;
        }
    }
    refresh_primary_area()
}

pub fn refresh_primary_area() -> Rect {
    let rect = detect_primary_work_area();
    if let Ok(mut guard) = primary_work_cache().lock() {
        *guard = Some(rect);
    }
    rect
}

fn detect_primary_work_area() -> Rect {
    if let Ok(monitors) = display::list_monitors() {
        let chosen = monitors
            .iter()
            .find(|m| m.primary)
            .or_else(|| monitors.first());
        if let Some(monitor) = chosen {
            let work = monitor.work();
            if work.width >= 320 && work.height >= 240 {
                return work;
            }
            if monitor.bounds.width >= 320 && monitor.bounds.height >= 240 {
                return monitor.bounds;
            }
        }
    }
    Rect::new(0, 0, 1920, 1080)
}

static SCREEN_AREAS: OnceLock<Mutex<Option<Vec<Rect>>>> = OnceLock::new();

fn screen_areas_cache() -> &'static Mutex<Option<Vec<Rect>>> {
    SCREEN_AREAS.get_or_init(|| Mutex::new(None))
}

/// 所有显示器的可用区。拖动路径每帧都会问一次，结果缓存；显示器变化由
/// [`refresh_screen_areas`] 刷新，避免每帧连一次 X server。
/// 多屏布局是合法摆位：瓦片允许停在任何一块屏上。
pub fn screen_areas() -> Vec<Rect> {
    if let Ok(guard) = screen_areas_cache().lock() {
        if let Some(areas) = guard.as_ref() {
            return areas.clone();
        }
    }
    refresh_screen_areas()
}

pub fn refresh_screen_areas() -> Vec<Rect> {
    let areas = detect_screen_areas();
    if let Ok(mut guard) = screen_areas_cache().lock() {
        *guard = Some(areas.clone());
    }
    areas
}

fn detect_screen_areas() -> Vec<Rect> {
    if let Ok(monitors) = display::list_monitors() {
        let mut areas: Vec<Rect> = monitors
            .iter()
            .map(|m| {
                let work = m.work();
                if work.width >= 320 && work.height >= 240 {
                    work
                } else {
                    m.bounds
                }
            })
            .filter(|r| r.width >= 320 && r.height >= 240)
            .collect();
        // 克隆/镜像模式会给出完全相同的矩形，去重避免候选重复。
        areas.dedup();
        if !areas.is_empty() {
            return areas;
        }
    }
    vec![Rect::new(0, 0, 1920, 1080)]
}

/// 矩形归属的屏幕下标：与该屏交集最大者；与任何屏都无交集时取
/// 中心最近的一块。`areas` 为空返回 None。
fn max_intersection_owner(rect: Rect, areas: &[Rect]) -> Option<usize> {
    if areas.is_empty() {
        return None;
    }
    let mut best: Option<usize> = None;
    let mut best_area = 0u64;
    for (i, area) in areas.iter().enumerate() {
        let inter = rect.intersection_area(area);
        if inter > best_area {
            best_area = inter;
            best = Some(i);
        }
    }
    if best.is_some() {
        return best;
    }
    let center = rect.center();
    let mut nearest: Option<usize> = None;
    let mut best_dist = i64::MAX;
    for (i, area) in areas.iter().enumerate() {
        let c = area.center();
        let dx = (c.0 - center.0) as i64;
        let dy = (c.1 - center.1) as i64;
        let dist = dx * dx + dy * dy;
        if dist < best_dist {
            best_dist = dist;
            nearest = Some(i);
        }
    }
    nearest
}

/// 把矩形收进屏幕集合：
/// - 已完整落在某块屏内 → 不动（用户摆好的位置）；
/// - 与某块屏有交集 → 收进交集最大的那块（归属屏）；
/// - 完全在所有屏之外 → 收进中心最近的一块。
pub fn clamp_rect_into_screens(rect: Rect, areas: &[Rect]) -> Rect {
    if areas.is_empty() {
        return rect;
    }
    for area in areas {
        let inside = rect.x >= area.x
            && rect.y >= area.y
            && rect.right() <= area.right()
            && rect.bottom() <= area.bottom();
        if inside {
            return rect;
        }
    }
    match max_intersection_owner(rect, areas) {
        Some(i) => clamp_rect_to(rect, areas[i]),
        None => rect,
    }
}

/// 把矩形收进指定可用区（同时限制最大尺寸）。
fn clamp_rect_to(rect: Rect, area: Rect) -> Rect {
    let w = (rect.width as i32).clamp(160, area.width as i32);
    let h = (rect.height as i32).clamp(160, area.height as i32);
    let max_x = (area.x + area.width as i32 - w).max(area.x);
    let max_y = (area.y + area.height as i32 - h).max(area.y);
    Rect::new(
        rect.x.clamp(area.x, max_x),
        rect.y.clamp(area.y, max_y),
        w as u32,
        h as u32,
    )
}

/// 拖动/缩放/落盘路径的边界钳制：收进"所有屏幕"的合法范围。
/// 瓦片停在哪块屏都行，但不许悬在屏幕外。
pub fn clamp_rect(rect: Rect) -> Rect {
    clamp_rect_into_screens(rect, &screen_areas())
}

/// 两条边缘是否算「贴死」：完全接触、几乎接触或轻微侵入。
fn edges_touching(gap: i32) -> bool {
    (-RELEASE_GAP..RELEASE_GAP).contains(&gap)
}

/// 全局「松手留缝」结算（纯逻辑，便于合成屏幕集合做测试）。
///
/// 系统拖动（`startSystemMove`）由 KWin 直接移动窗口，QML 的磁吸与
/// `minGap` 约束都拦不住瓦片贴死；因此所有瓦片统一在几何落盘时走这一条
/// 规则，不在每个 QML 里各写一份。四个方向一视同仁：
/// - 我的顶边贴住上方瓦片底边 → 下移 [`RELEASE_GAP`]；
/// - 我的底边贴住下方瓦片顶边 → 上移；
/// - 我的左/右边贴住左/右邻瓦 → 右移 / 左移；
/// - 贴住**所属屏幕**四边 → 向屏幕内侧退同样的距离。
///
/// 已经拉开距离（含用户上一轮要的 40px 对齐间隔）的位置完全不动。
/// 多屏布局是合法摆位：结算只在瓦片归属屏的四边退缝，不把瓦片拽回主屏。
pub fn settle_rect_in(
    widget_id: &str,
    rect: Rect,
    widgets: &[WidgetConfig],
    areas: &[Rect],
) -> Rect {
    let mut out = clamp_rect_into_screens(rect, areas);
    // 两轮足够让「同时贴住邻居 + 屏幕边」的情况收敛。
    for _ in 0..2 {
        let mut moved = false;
        for other in widgets {
            if other.id == widget_id {
                continue;
            }
            let o = other.rect;
            // 横向投影有交集 → 上下相邻。
            if out.x < o.right() && o.x < out.right() {
                // 我在下方：顶边贴住对方底边 → 下移 10px。
                if edges_touching(out.y - o.bottom()) {
                    let ny = o.bottom() + RELEASE_GAP;
                    if ny != out.y {
                        out.y = ny;
                        moved = true;
                    }
                }
                // 我在上方：底边贴住对方顶边 → 上移 10px。
                if edges_touching(o.y - out.bottom()) {
                    let ny = o.y - RELEASE_GAP - out.height as i32;
                    if ny != out.y {
                        out.y = ny;
                        moved = true;
                    }
                }
            }
            // 纵向投影有交集 → 左右相邻。
            if out.y < o.bottom() && o.y < out.bottom() {
                // 我在右侧：左边贴住对方右边 → 右移 10px。
                if edges_touching(out.x - o.right()) {
                    let nx = o.right() + RELEASE_GAP;
                    if nx != out.x {
                        out.x = nx;
                        moved = true;
                    }
                }
                // 我在左侧：右边贴住对方左边 → 左移 10px。
                if edges_touching(o.x - out.right()) {
                    let nx = o.x - RELEASE_GAP - out.width as i32;
                    if nx != out.x {
                        out.x = nx;
                        moved = true;
                    }
                }
            }
        }

        // 所属屏幕四条边：贴住就往屏幕内侧退 10px。归属按当前矩形
        // 重新判断——邻瓦推挤可能把它挪到另一块屏附近。
        if let Some(area) = max_intersection_owner(out, areas).and_then(|i| areas.get(i)) {
            let top_gap = out.y - area.y;
            let bottom_gap = area.bottom() - out.bottom();
            let left_gap = out.x - area.x;
            let right_gap = area.right() - out.right();
            if (0..RELEASE_GAP).contains(&top_gap) {
                out.y = area.y + RELEASE_GAP;
                moved = true;
            }
            if (0..RELEASE_GAP).contains(&bottom_gap) {
                out.y = area.bottom() - RELEASE_GAP - out.height as i32;
                moved = true;
            }
            if (0..RELEASE_GAP).contains(&left_gap) {
                out.x = area.x + RELEASE_GAP;
                moved = true;
            }
            if (0..RELEASE_GAP).contains(&right_gap) {
                out.x = area.right() - RELEASE_GAP - out.width as i32;
                moved = true;
            }
        }

        let clamped = clamp_rect_into_screens(out, areas);
        if clamped != out {
            out = clamped;
            moved = true;
        }
        if !moved {
            break;
        }
    }
    out
}

/// 松手结算入口：读进程内布局缓存（不碰磁盘）与全部屏幕可用区。
pub fn settle_rect(widget_id: &str, rect: Rect) -> Rect {
    let document = load();
    settle_rect_in(widget_id, rect, &document.widgets, &screen_areas())
}

fn default_tile_rect_in(index: i32, area: Rect) -> Rect {
    let w = (DEFAULT_TILE_RECT.width as i32)
        .min(area.width as i32)
        .max(160);
    let h = (DEFAULT_TILE_RECT.height as i32)
        .min(area.height as i32)
        .max(160);
    let columns = ((area.width as i32) / (w + MIN_TILE_GAP)).max(1);
    let col = index.rem_euclid(columns);
    let row = index.div_euclid(columns);
    clamp_rect_to(
        Rect::new(
            area.x + MIN_TILE_GAP / 2 + col * (w + MIN_TILE_GAP),
            area.y + MIN_TILE_GAP / 2 + row * (h + MIN_TILE_GAP),
            w as u32,
            h as u32,
        ),
        area,
    )
}

/// 新建瓦片的位置：以主屏为原点按网格错开，间隔 [`MIN_TILE_GAP`]。
pub fn default_tile_rect(index: i32) -> Rect {
    default_tile_rect_in(index, primary_work_area())
}

/// 启动/拓扑变化时的分辨率匹配（纯逻辑，不碰磁盘）。
///
/// 规则（`key` 是当前主屏分辨率 "宽x高"）：
/// 1. 文档已记录同一分辨率 → 什么都不做，用户摆好的布局原样生效；
/// 2. 文档没有分辨率标记（v2 旧文件 / 首次运行）→ **就地收养**：
///    瓦片位置不动，只补上归档与标记；
/// 3. 文档记录的是另一分辨率 → 先把旧布局按旧分辨率归档保存，
///    再查新分辨率有没有归档：
///    - 有 → 原样恢复（瓦片回到该分辨率下上次的位置）；
///    - 没有 → 瓦片按新屏网格重排（默认初始化），归档为该分辨率的首个布局。
///
/// 返回 (被调整的瓦片 id, 是否需要落盘)。
pub fn reconcile_resolution_in(
    document: &mut LayoutDocument,
    key: &str,
    topology: &str,
) -> (Vec<String>, bool) {
    if key.is_empty() {
        return (Vec::new(), false);
    }
    // 规则 1：同一分辨率，当前摆设即归档（不一致时补写归档，位置不动）。
    if document.layout_resolution == key {
        if document.matches_resolution_archive(key) {
            return (Vec::new(), false);
        }
        document.archive_to_resolution(key, topology);
        return (Vec::new(), true);
    }

    let previous_resolution = document.layout_resolution.clone();
    let has_marker = !previous_resolution.is_empty();

    // 规则 3 前置：有旧分辨率标记时，先把当前摆设归档到旧分辨率名下。
    if has_marker && !document.matches_resolution_archive(&previous_resolution) {
        document.archive_to_resolution(&previous_resolution, topology);
    }

    if !has_marker {
        // 规则 2：旧格式文件就地收养——位置不动，只补标记与归档。
        document.archive_to_resolution(key, topology);
        return (Vec::new(), true);
    }

    if document.restore_from_resolution(key) {
        // 规则 3a：新分辨率有归档，原样恢复。
        (Vec::new(), true)
    } else {
        // 规则 3b：新分辨率没有归档 → 按新屏重排（默认初始化网格）。
        let area = primary_work_area();
        let mut changed = Vec::new();
        for (index, widget) in document.widgets.iter_mut().enumerate() {
            let next = default_tile_rect_in(index as i32, area);
            if widget.rect != next {
                widget.rect = next;
                changed.push(widget.id.clone());
            }
        }
        document.archive_to_resolution(key, topology);
        (changed, true)
    }
}

/// 分辨率匹配入口：读当前主屏分辨率，跑纯逻辑并按需落盘。
/// 返回被调整的瓦片 id（供日志排查）。
pub fn reconcile_resolution() -> Vec<String> {
    let area = refresh_primary_area();
    let key = format!("{}x{}", area.width, area.height);
    let topology = display::list_monitors()
        .map(|monitors| topology_key(&monitors))
        .unwrap_or_default();
    let mut document = load();
    let (changed, dirty) = reconcile_resolution_in(&mut document, &key, &topology);
    if dirty {
        document.bump();
        let _ = save(&document);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::display::MonitorInfo;

    fn monitor(id: &str, x: i32, w: u32, primary: bool) -> MonitorInfo {
        let bounds = Rect::new(x, 0, w, 1000);
        MonitorInfo {
            id: MonitorId::new(id),
            name: id.to_string(),
            primary,
            bounds,
            work: bounds,
            scale: 1.0,
        }
    }

    fn doc_with(rect: Rect, monitor_id: &str) -> LayoutDocument {
        let mut doc = LayoutDocument::new();
        let folder = std::env::temp_dir();
        let mut widget = new_file_widget(&folder, rect, "file-1");
        widget.monitor = MonitorId::new(monitor_id);
        doc.upsert_widget(widget);
        doc
    }

    #[test]
    fn singleton_dedupe_keeps_files_but_only_first_of_each_content_kind() {
        use dm_core::widget::WidgetConfig;
        let mut doc = LayoutDocument::new();
        let rect = Rect::new(0, 0, 420, 520);
        let monitor = MonitorId::new("primary");
        for (id, kind) in [
            ("file-1", WidgetKind::File),
            ("file-2", WidgetKind::File),
            ("todo-1", WidgetKind::Todo),
            ("todo-2", WidgetKind::Todo),
            ("glance-1", WidgetKind::Glance),
            ("glance-2", WidgetKind::Glance),
            ("music-1", WidgetKind::Music),
        ] {
            doc.upsert_widget(WidgetConfig::new(id, kind, rect, monitor.clone()));
        }

        let removed = dedupe_singletons(&mut doc);
        assert_eq!(removed, vec!["todo-2".to_string(), "glance-2".to_string()]);
        // 两个文件格子都留着，其余每种只剩一个。
        assert_eq!(doc.widgets.len(), 5);
        assert_eq!(
            doc.widgets
                .iter()
                .filter(|w| w.kind == WidgetKind::File)
                .count(),
            2
        );
        for kind in [WidgetKind::Todo, WidgetKind::Glance, WidgetKind::Music] {
            assert_eq!(doc.widgets.iter().filter(|w| w.kind == kind).count(), 1);
        }
        // 再次执行是幂等的。
        assert!(dedupe_singletons(&mut doc).is_empty());
    }

    #[test]
    fn topology_key_is_order_independent() {
        let a = monitor("edid:a", 0, 800, true);
        let b = monitor("edid:b", 800, 800, false);
        assert_eq!(topology_key(&[a.clone(), b.clone()]), topology_key(&[b, a]));
    }

    #[test]
    fn topology_switch_saves_and_restores_placements() {
        let mut doc = doc_with(Rect::new(10, 10, 300, 300), "edid:a");
        assert!(apply_topology(&mut doc, "a"));
        assert_eq!(doc.widgets[0].rect, Rect::new(10, 10, 300, 300));

        // 在组合 a 下移动瓦片
        doc.widgets[0].rect = Rect::new(100, 100, 300, 300);
        // 切到组合 a|b：存档 a，新组合无档 → 保持原状
        assert!(apply_topology(&mut doc, "a|b"));
        assert_eq!(doc.widgets[0].rect, Rect::new(100, 100, 300, 300));
        assert_eq!(
            doc.placements["a"]["file-1"].rect,
            Rect::new(100, 100, 300, 300)
        );

        // 在 a|b 下再移动
        doc.widgets[0].rect = Rect::new(500, 200, 300, 300);
        // 切回 a：应恢复 a 组合的摆放
        assert!(apply_topology(&mut doc, "a"));
        assert_eq!(doc.widgets[0].rect, Rect::new(100, 100, 300, 300));
        assert_eq!(
            doc.placements["a|b"]["file-1"].rect,
            Rect::new(500, 200, 300, 300)
        );
    }

    #[test]
    fn same_topology_is_a_no_op() {
        let mut doc = doc_with(Rect::new(10, 10, 300, 300), "edid:a");
        apply_topology(&mut doc, "a");
        assert!(!apply_topology(&mut doc, "a"));
    }

    #[test]
    fn ownership_follows_geometry_when_dragged_to_other_monitor() {
        let monitors = vec![
            monitor("edid:left", 0, 800, true),
            monitor("edid:right", 800, 800, false),
        ];
        let mut doc = doc_with(Rect::new(900, 100, 300, 300), "edid:left");
        assert!(reconcile_document(&mut doc, &monitors));
        assert_eq!(doc.widgets[0].monitor, MonitorId::new("edid:right"));
        assert_eq!(doc.widgets[0].rect, Rect::new(900, 100, 300, 300));
    }

    #[test]
    fn reconcile_syncs_monitor_table() {
        let monitors = vec![monitor("edid:a", 0, 1600, true)];
        let mut doc = doc_with(Rect::new(100, 100, 300, 300), "edid:a");
        assert!(reconcile_document(&mut doc, &monitors));
        assert_eq!(doc.monitors.len(), 1);
        assert_eq!(doc.monitors[&MonitorId::new("edid:a")].bounds.width, 1600);
    }

    #[test]
    fn reconcile_is_idempotent() {
        let monitors = vec![monitor("edid:a", 0, 1600, true)];
        let mut doc = doc_with(Rect::new(100, 100, 300, 300), "edid:a");
        assert!(reconcile_document(&mut doc, &monitors));
        assert!(!reconcile_document(&mut doc, &monitors), "第二次不应再改");
    }

    #[test]
    fn removed_monitor_moves_widget_to_overlapping_one() {
        // 原瓦片在右屏（x>=800），右屏拔掉后只剩左屏，应迁到左屏并收进边界。
        let monitors = vec![monitor("edid:left", 0, 800, true)];
        let mut doc = doc_with(Rect::new(900, 100, 300, 300), "edid:right");
        assert!(reconcile_document(&mut doc, &monitors));
        let widget = &doc.widgets[0];
        assert_eq!(widget.monitor, MonitorId::new("edid:left"));
        assert_eq!(widget.rect, Rect::new(500, 100, 300, 300));
    }

    #[test]
    fn resolution_shrink_clamps_widget_inside() {
        let monitors = vec![monitor("edid:a", 0, 800, true)];
        let mut doc = doc_with(Rect::new(600, 100, 400, 300), "edid:a");
        assert!(reconcile_document(&mut doc, &monitors));
        assert_eq!(doc.widgets[0].rect, Rect::new(400, 100, 400, 300));
    }

    #[test]
    fn widget_on_valid_monitor_keeps_position() {
        let monitors = vec![monitor("edid:a", 0, 1600, true)];
        let mut doc = doc_with(Rect::new(100, 100, 300, 300), "edid:a");
        let before = doc.widgets[0].rect;
        reconcile_document(&mut doc, &monitors);
        assert_eq!(doc.widgets[0].rect, before);
    }

    #[test]
    fn next_id_skips_existing() {
        let mut document = LayoutDocument::new();
        let folder = std::env::temp_dir();
        document.upsert_widget(new_file_widget(&folder, DEFAULT_TILE_RECT, "file-1"));
        document.upsert_widget(new_file_widget(&folder, DEFAULT_TILE_RECT, "file-2"));
        assert_eq!(next_widget_id(&document), "file-3");
    }

    #[test]
    fn new_widget_has_folder_and_settings() {
        let folder = std::env::temp_dir();
        let widget = new_file_widget(&folder, DEFAULT_TILE_RECT, "file-9");
        assert_eq!(widget.kind, WidgetKind::File);
        assert!(widget.data.contains_key("folder"));
        assert!(widget.data.contains_key("settings"));
    }

    #[test]
    fn clamp_keeps_rect_inside_primary_area() {
        let area = Rect::new(55, 1440, 2560, 1563);
        let clamped = clamp_rect_to(Rect::new(272, 100, 420, 520), area);
        assert!(clamped.x >= area.x && clamped.y >= area.y);
        assert!(clamped.right() <= area.right());
        assert!(clamped.bottom() <= area.bottom());
        assert_eq!(clamped.width, 420);
        assert_eq!(clamped.height, 520);
    }

    #[test]
    fn clamp_into_screens_keeps_tile_on_secondary_screen() {
        // 顶屏 0..1080、主屏 0+1080..2680 的双屏布局（实际机器的拓扑）。
        // 副屏上的瓦片是合法摆位，不得被拽回主屏。
        let areas = vec![Rect::new(0, 0, 1920, 1080), Rect::new(0, 1080, 2560, 1600)];
        let on_top = Rect::new(335, 500, 420, 520);
        assert_eq!(clamp_rect_into_screens(on_top, &areas), on_top);
        let on_bottom = Rect::new(1600, 2000, 420, 520);
        assert_eq!(clamp_rect_into_screens(on_bottom, &areas), on_bottom);
        // 越界矩形收进交集最大的那块屏。
        let straddling = Rect::new(1800, 900, 400, 300);
        let clamped = clamp_rect_into_screens(straddling, &areas);
        let owner = max_intersection_owner(clamped, &areas).unwrap();
        let area = areas[owner];
        assert!(clamped.right() <= area.right() && clamped.bottom() <= area.bottom());
    }

    #[test]
    fn clamp_into_screens_rescues_rect_outside_all_screens() {
        let areas = vec![Rect::new(0, 0, 1920, 1080), Rect::new(0, 1080, 2560, 1600)];
        // 完全在所有屏之外的矩形：收进中心最近的一块。
        let lost = Rect::new(3000, 2000, 420, 520);
        let rescued = clamp_rect_into_screens(lost, &areas);
        assert_ne!(rescued, lost);
        let owner = max_intersection_owner(rescued, &areas);
        assert!(owner.is_some());
    }

    #[test]
    fn clamp_shrinks_oversized_window() {
        let area = Rect::new(0, 0, 800, 600);
        let clamped = clamp_rect_to(Rect::new(900, 900, 1200, 900), area);
        assert_eq!(clamped.width, 800);
        assert_eq!(clamped.height, 600);
        assert_eq!(clamped.x, 0);
        assert_eq!(clamped.y, 0);
    }

    #[test]
    fn default_tiles_keep_five_times_gap() {
        let area = Rect::new(55, 1440, 2560, 1563);
        let a = default_tile_rect_in(0, area);
        let b = default_tile_rect_in(1, area);
        assert_eq!(b.x - a.right(), MIN_TILE_GAP);
        let c = default_tile_rect_in(5, area);
        assert_eq!(c.y - a.bottom(), MIN_TILE_GAP);
    }

    #[test]
    fn settle_pushes_lower_and_right_tiles_away() {
        let folder = std::env::temp_dir();
        let areas = vec![Rect::new(0, 0, 1600, 1200)];
        let widgets = vec![
            new_file_widget(&folder, Rect::new(100, 100, 300, 300), "top"),
            new_file_widget(&folder, Rect::new(100, 400, 300, 300), "bottom"),
            new_file_widget(&folder, Rect::new(400, 100, 300, 300), "right"),
        ];
        // 下方瓦片顶边贴住上方瓦片底边 → 下移 10px。
        let lower = settle_rect_in("bottom", widgets[1].rect, &widgets, &areas);
        assert_eq!(lower.y - widgets[0].rect.bottom(), RELEASE_GAP);
        // 右侧瓦片左边贴住左侧瓦片右边 → 右移 10px。
        let right = settle_rect_in("right", widgets[2].rect, &widgets, &areas);
        assert_eq!(right.x - widgets[0].rect.right(), RELEASE_GAP);
    }

    #[test]
    fn settle_pushes_upper_and_left_tiles_back() {
        let folder = std::env::temp_dir();
        let areas = vec![Rect::new(0, 0, 1600, 1200)];
        let widgets = vec![
            new_file_widget(&folder, Rect::new(500, 500, 300, 300), "anchor"),
            new_file_widget(&folder, Rect::new(500, 200, 300, 300), "upper"),
            new_file_widget(&folder, Rect::new(200, 500, 300, 300), "left"),
        ];
        // 我在上方：底边贴住对方顶边 → 上移 10px。
        let upper = settle_rect_in("upper", widgets[1].rect, &widgets, &areas);
        assert_eq!(widgets[0].rect.y - upper.bottom(), RELEASE_GAP);
        // 我在左侧：右边贴住对方左边 → 左移 10px。
        let left = settle_rect_in("left", widgets[2].rect, &widgets, &areas);
        assert_eq!(widgets[0].rect.x - left.right(), RELEASE_GAP);
    }

    #[test]
    fn settle_pushes_off_all_four_screen_edges() {
        let areas = vec![Rect::new(0, 0, 1000, 800)];
        let top_left = settle_rect_in("tl", Rect::new(0, 0, 300, 300), &[], &areas);
        assert_eq!(top_left.x, RELEASE_GAP);
        assert_eq!(top_left.y, RELEASE_GAP);
        let bottom_right = settle_rect_in("br", Rect::new(700, 500, 300, 300), &[], &areas);
        assert_eq!(areas[0].right() - bottom_right.right(), RELEASE_GAP);
        assert_eq!(areas[0].bottom() - bottom_right.bottom(), RELEASE_GAP);
    }

    #[test]
    fn settle_on_secondary_screen_pushes_off_its_edges() {
        // 双屏：瓦片贴住**副屏**底边（1080+1600=2680）时，退缝针对副屏
        // 而不是主屏。
        let areas = vec![Rect::new(0, 0, 1920, 1080), Rect::new(0, 1080, 2560, 1600)];
        let tile = Rect::new(100, 2580, 300, 100); // 底边恰好在 2680。
        let out = settle_rect_in("t", tile, &[], &areas);
        assert_eq!(areas[1].bottom() - out.bottom(), RELEASE_GAP);
        assert_eq!(out.y, 2580 - RELEASE_GAP);
    }

    #[test]
    fn settle_keeps_already_separated_tiles() {
        let folder = std::env::temp_dir();
        let areas = vec![Rect::new(0, 0, 1600, 1200)];
        let widgets = vec![
            new_file_widget(&folder, Rect::new(100, 100, 300, 300), "top"),
            new_file_widget(&folder, Rect::new(100, 440, 300, 300), "bottom"),
        ];
        let out = settle_rect_in("bottom", widgets[1].rect, &widgets, &areas);
        assert_eq!(out.y, 440);
        assert_eq!(out.y - widgets[0].rect.bottom(), MIN_TILE_GAP);
    }

    // ------------------------------------------------------------ 分辨率匹配

    fn widget(id: &str, rect: Rect) -> WidgetConfig {
        let folder = std::env::temp_dir();
        let mut widget = new_file_widget(&folder, rect, id);
        widget.title = id.to_string();
        widget
    }

    #[test]
    fn resolution_match_same_key_is_a_no_op() {
        let mut doc = LayoutDocument::new();
        doc.upsert_widget(widget("file-1", Rect::new(100, 200, 420, 520)));
        doc.archive_to_resolution("2560x1600", "edid:a");
        let before = doc.widgets[0].rect;
        let (changed, dirty) = reconcile_resolution_in(&mut doc, "2560x1600", "edid:a");
        assert!(changed.is_empty() && !dirty);
        assert_eq!(doc.widgets[0].rect, before);
    }

    #[test]
    fn resolution_match_adopts_legacy_file_without_marker() {
        let mut doc = LayoutDocument::new();
        doc.upsert_widget(widget("file-1", Rect::new(100, 200, 420, 520)));
        let (changed, dirty) = reconcile_resolution_in(&mut doc, "2560x1600", "edid:a");
        assert!(changed.is_empty() && dirty);
        assert_eq!(doc.layout_resolution, "2560x1600");
        assert!(doc.matches_resolution_archive("2560x1600"));
        // 位置不动：旧文件的用户摆设原样生效。
        assert_eq!(doc.widgets[0].rect, Rect::new(100, 200, 420, 520));
    }

    #[test]
    fn resolution_switch_restores_archived_layout() {
        let mut doc = LayoutDocument::new();
        doc.upsert_widget(widget("file-1", Rect::new(100, 200, 420, 520)));
        doc.archive_to_resolution("2560x1600", "edid:a");

        // 分辨率换成 1920x1080：瓦片位置还没动（归档与摆设一致）。
        let (changed, dirty) = reconcile_resolution_in(&mut doc, "1920x1080", "edid:a");
        // 新分辨率没有归档 → 默认网格重排（id 出现在 changed 里）。
        assert_eq!(changed, vec!["file-1".to_string()]);
        assert!(dirty);
        // 旧布局仍归档在 2560x1600 名下。
        assert_eq!(doc.layout_resolution, "1920x1080");
        assert_eq!(
            doc.resolution_layouts["2560x1600"].widgets["file-1"].rect,
            Rect::new(100, 200, 420, 520)
        );

        // 在 1920x1080 下摆好后归档（模拟 save_placement），切回 2560x1600
        // → 原样恢复。
        doc.archive_to_resolution("1920x1080", "edid:a");
        let (changed, dirty) = reconcile_resolution_in(&mut doc, "2560x1600", "edid:a");
        assert!(changed.is_empty() && dirty);
        assert_eq!(doc.widgets[0].rect, Rect::new(100, 200, 420, 520));
    }

    #[test]
    fn resolution_switch_saves_unarchived_move_first() {
        let mut doc = LayoutDocument::new();
        doc.upsert_widget(widget("file-1", Rect::new(100, 200, 420, 520)));
        doc.archive_to_resolution("2560x1600", "edid:a");

        // 用户在 2560x1600 下拖动了瓦片但归档还没跟上（异常退出场景）。
        doc.widgets[0].rect = Rect::new(10, 10, 420, 520);
        let (changed, dirty) = reconcile_resolution_in(&mut doc, "1920x1080", "edid:a");
        assert_eq!(changed, vec!["file-1".to_string()]);
        assert!(dirty);
        // 切换前先把未归档的移动保存到旧分辨率名下，不丢用户摆设。
        assert_eq!(
            doc.resolution_layouts["2560x1600"].widgets["file-1"].rect,
            Rect::new(10, 10, 420, 520)
        );
    }

    #[test]
    fn resolution_archive_tracks_moves() {
        let mut doc = LayoutDocument::new();
        doc.upsert_widget(widget("file-1", Rect::new(100, 200, 420, 520)));
        doc.archive_to_resolution("2560x1600", "edid:a");

        // 拖动后落盘（模拟 save_placement 的归档同步）。
        doc.widgets[0].rect = Rect::new(300, 400, 420, 520);
        doc.archive_to_resolution("2560x1600", "edid:a");
        assert!(doc.matches_resolution_archive("2560x1600"));

        // 再做一次匹配是幂等的。
        let (changed, dirty) = reconcile_resolution_in(&mut doc, "2560x1600", "edid:a");
        assert!(changed.is_empty() && !dirty);
        assert_eq!(doc.widgets[0].rect, Rect::new(300, 400, 420, 520));
    }
}
