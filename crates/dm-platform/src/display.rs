//! 显示器拓扑。
//!
//! 身份用 EDID 指纹（制造商+型号+序列号），不用 RandR 输出编号：编号会随
//! 重插、改主屏、睡眠唤醒变化，而 EDID 不会。
//!
//! 缩放用 `Xft.dpi` 估算（DPI/96）。Wayland 会话以后走 wl_output 的 scale，
//! 这里只实现 X11。

use crate::error::{PlatformError, PlatformResult};
use dm_core::geometry::{MonitorId, Rect};
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    pub id: MonitorId,
    pub name: String,
    pub primary: bool,
    /// 整块屏幕。
    pub bounds: Rect,
    /// 扣掉面板/任务栏后的可用区（不含 Dock 覆盖的边）。
    pub work: Rect,
    pub scale: f64,
}

impl MonitorInfo {
    pub fn work(&self) -> Rect {
        self.work
    }
}

pub fn list_monitors() -> PlatformResult<Vec<MonitorInfo>> {
    if std::env::var_os("DISPLAY").is_none() {
        return Err(PlatformError::NoX11Display);
    }
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let scale = xft_scale(&conn, root);

    // RandR 1.5 的 get_monitors：把多个输出合成"显示器"，正是我们要的粒度。
    let reply = match conn.randr_get_monitors(root, true) {
        Ok(cookie) => cookie.reply().ok(),
        Err(_) => None,
    };

    let struts = read_struts(&conn, root);
    let global_work = net_workarea(&conn, root);

    let mut monitors = Vec::new();
    if let Some(reply) = reply {
        for monitor in reply.monitors {
            let name = conn
                .get_atom_name(monitor.name)
                .ok()
                .and_then(|c| c.reply().ok())
                .map(|r| String::from_utf8_lossy(&r.name).to_string())
                .unwrap_or_else(|| format!("monitor-{}", monitor.name));
            let edid = monitor
                .outputs
                .first()
                .and_then(|output| read_edid(&conn, *output));
            let id = match edid {
                Some(bytes) => MonitorId::new(format!("edid:{}", short_hash(&bytes))),
                None => MonitorId::new(format!("name:{name}")),
            };
            let bounds = Rect::new(
                monitor.x as i32,
                monitor.y as i32,
                monitor.width as u32,
                monitor.height as u32,
            );
            let work = compute_work_area(
                bounds,
                &struts,
                global_work,
                screen.width_in_pixels,
                screen.height_in_pixels,
            );
            monitors.push(MonitorInfo {
                id,
                name,
                primary: monitor.primary,
                bounds,
                work,
                scale,
            });
        }
    }

    // 老 X server 或没有 RandR 1.5：退回整个屏幕，保证程序还能用。
    if monitors.is_empty() {
        let bounds = Rect::new(
            0,
            0,
            screen.width_in_pixels as u32,
            screen.height_in_pixels as u32,
        );
        let work = compute_work_area(
            bounds,
            &struts,
            global_work,
            screen.width_in_pixels,
            screen.height_in_pixels,
        );
        monitors.push(MonitorInfo {
            id: MonitorId::new("screen:primary"),
            name: "screen".to_string(),
            primary: true,
            bounds,
            work,
            scale,
        });
    }

    Ok(monitors)
}

/// `_NET_WM_STRUT_PARTIAL` 的 12 个值。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Strut {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
    pub left_start_y: u32,
    pub left_end_y: u32,
    pub right_start_y: u32,
    pub right_end_y: u32,
    pub top_start_x: u32,
    pub top_end_x: u32,
    pub bottom_start_x: u32,
    pub bottom_end_x: u32,
}

/// 收集所有 DOCK 窗口的 strut。DDE 的 dock 虽然设了 strut，但 `_NET_WORKAREA`
/// 只给一个全局矩形，多屏时不够用，所以两者都要。
fn read_struts(conn: &RustConnection, root: u32) -> Vec<Strut> {
    let mut out = Vec::new();
    let Ok(tree_cookie) = conn.query_tree(root) else {
        return out;
    };
    let Ok(tree) = tree_cookie.reply() else {
        return out;
    };
    let Some(type_atom) = intern(conn, "_NET_WM_WINDOW_TYPE") else {
        return out;
    };
    let Some(dock_atom) = intern(conn, "_NET_WM_WINDOW_TYPE_DOCK") else {
        return out;
    };
    let Some(strut_atom) = intern(conn, "_NET_WM_STRUT_PARTIAL") else {
        return out;
    };

    for window in tree.children {
        let Ok(types_cookie) = conn.get_property(false, window, type_atom, AtomEnum::ATOM, 0, 32)
        else {
            continue;
        };
        let Ok(types) = types_cookie.reply() else {
            continue;
        };
        let Some(type_list) = types.value32() else {
            continue;
        };
        if !type_list.into_iter().any(|t| t == dock_atom) {
            continue;
        }
        let Ok(property_cookie) =
            conn.get_property(false, window, strut_atom, AtomEnum::CARDINAL, 0, 12)
        else {
            continue;
        };
        let Ok(property) = property_cookie.reply() else {
            continue;
        };
        let values: Vec<u32> = property
            .value32()
            .map(|it| it.collect())
            .unwrap_or_default();
        if values.len() < 12 {
            continue;
        }
        let s = Strut {
            left: values[0],
            right: values[1],
            top: values[2],
            bottom: values[3],
            left_start_y: values[4],
            left_end_y: values[5],
            right_start_y: values[6],
            right_end_y: values[7],
            top_start_x: values[8],
            top_end_x: values[9],
            bottom_start_x: values[10],
            bottom_end_x: values[11],
        };
        if s.left + s.right + s.top + s.bottom > 0 {
            out.push(s);
        }
    }
    out
}

fn intern(conn: &RustConnection, name: &str) -> Option<u32> {
    Some(
        conn.intern_atom(false, name.as_bytes())
            .ok()?
            .reply()
            .ok()?
            .atom,
    )
}

fn net_workarea(conn: &RustConnection, root: u32) -> Option<Rect> {
    let atom = intern(conn, "_NET_WORKAREA")?;
    let reply = conn
        .get_property(false, root, atom, AtomEnum::CARDINAL, 0, 4)
        .ok()?
        .reply()
        .ok()?;
    let values: Vec<u32> = reply.value32().map(|it| it.collect())?;
    if values.len() < 4 {
        return None;
    }
    Some(Rect::new(
        values[0] as i32,
        values[1] as i32,
        values[2],
        values[3],
    ))
}

fn compute_work_area(
    bounds: Rect,
    struts: &[Strut],
    global_work: Option<Rect>,
    screen_w: u16,
    screen_h: u16,
) -> Rect {
    let mut work = bounds;
    if let Some(global) = global_work {
        let clipped = intersect(work, global);
        if clipped.width > 0 && clipped.height > 0 {
            work = clipped;
        }
    }
    for strut in struts {
        work = apply_strut(work, strut, screen_w, screen_h);
    }
    work
}

/// 把一条 strut 覆盖的那条边从工作区里切掉。
fn apply_strut(work: Rect, strut: &Strut, screen_w: u16, screen_h: u16) -> Rect {
    let mut result = work;
    let sw = screen_w as i32;
    let sh = screen_h as i32;

    if strut.bottom > 0 {
        let start_x = strut.bottom_start_x as i32;
        let end_x = if strut.bottom_end_x == 0 {
            sw
        } else {
            strut.bottom_end_x as i32
        };
        let reserved = Rect::new(
            start_x,
            sh - strut.bottom as i32,
            (end_x - start_x).max(0) as u32,
            strut.bottom,
        );
        result = cut_edge(result, reserved, Edge::Bottom);
    }
    if strut.top > 0 {
        let start_x = strut.top_start_x as i32;
        let end_x = if strut.top_end_x == 0 {
            sw
        } else {
            strut.top_end_x as i32
        };
        let reserved = Rect::new(start_x, 0, (end_x - start_x).max(0) as u32, strut.top);
        result = cut_edge(result, reserved, Edge::Top);
    }
    if strut.left > 0 {
        let start_y = strut.left_start_y as i32;
        let end_y = if strut.left_end_y == 0 {
            sh
        } else {
            strut.left_end_y as i32
        };
        let reserved = Rect::new(0, start_y, strut.left, (end_y - start_y).max(0) as u32);
        result = cut_edge(result, reserved, Edge::Left);
    }
    if strut.right > 0 {
        let start_y = strut.right_start_y as i32;
        let end_y = if strut.right_end_y == 0 {
            sh
        } else {
            strut.right_end_y as i32
        };
        let reserved = Rect::new(
            sw - strut.right as i32,
            start_y,
            strut.right,
            (end_y - start_y).max(0) as u32,
        );
        result = cut_edge(result, reserved, Edge::Right);
    }
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

fn cut_edge(work: Rect, reserved: Rect, edge: Edge) -> Rect {
    if !work.intersects(&reserved) {
        return work;
    }
    match edge {
        Edge::Bottom => {
            let bottom = reserved.y.min(work.bottom());
            Rect::new(work.x, work.y, work.width, (bottom - work.y).max(0) as u32)
        }
        Edge::Top => {
            let top = reserved.bottom().max(work.y);
            Rect::new(work.x, top, work.width, (work.bottom() - top).max(0) as u32)
        }
        Edge::Left => {
            let left = reserved.right().max(work.x);
            Rect::new(
                left,
                work.y,
                (work.right() - left).max(0) as u32,
                work.height,
            )
        }
        Edge::Right => {
            let right = reserved.x.min(work.right());
            Rect::new(work.x, work.y, (right - work.x).max(0) as u32, work.height)
        }
    }
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = a.right().min(b.right());
    let bottom = a.bottom().min(b.bottom());
    Rect::new(x, y, (right - x).max(0) as u32, (bottom - y).max(0) as u32)
}

fn read_edid(conn: &RustConnection, output: u32) -> Option<Vec<u8>> {
    let atom = conn.intern_atom(false, b"EDID").ok()?.reply().ok()?.atom;
    let reply = conn
        .randr_get_output_property(output, atom, AtomEnum::ANY, 0, 128, false, false)
        .ok()?
        .reply()
        .ok()?;
    if reply.data.is_empty() {
        None
    } else {
        Some(reply.data)
    }
}

/// 取前 16 字节做指纹：EDID 头 + 制造商 + 型号 + 序列号足够区分同型号不同屏。
fn short_hash(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(16);
    for byte in bytes.iter().take(16) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// 从 root 的 RESOURCE_MANAGER 里读 `Xft.dpi`，换算成缩放比。
fn xft_scale(conn: &RustConnection, root: u32) -> f64 {
    let Ok(cookie) = conn.get_property(
        false,
        root,
        AtomEnum::RESOURCE_MANAGER,
        AtomEnum::STRING,
        0,
        8192,
    ) else {
        return 1.0;
    };
    let Ok(reply) = cookie.reply() else {
        return 1.0;
    };
    let text = String::from_utf8_lossy(&reply.value);
    for line in text.lines() {
        if let Some(value) = line.trim().strip_prefix("Xft.dpi:") {
            if let Ok(dpi) = value.trim().parse::<f64>() {
                if dpi > 0.0 {
                    return (dpi / 96.0 * 100.0).round() / 100.0;
                }
            }
        }
    }
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Rect {
        Rect::new(0, 0, 1600, 1000)
    }

    #[test]
    fn bottom_dock_shrinks_work_area() {
        let strut = Strut {
            bottom: 37,
            bottom_start_x: 0,
            bottom_end_x: 1600,
            ..Default::default()
        };
        let work = compute_work_area(bounds(), &[strut], None, 1600, 1000);
        assert_eq!(work, Rect::new(0, 0, 1600, 963));
    }

    #[test]
    fn left_dock_shrinks_only_its_rows() {
        let strut = Strut {
            left: 60,
            left_start_y: 0,
            left_end_y: 500,
            ..Default::default()
        };
        let work = compute_work_area(bounds(), &[strut], None, 1600, 1000);
        assert_eq!(work, Rect::new(60, 0, 1540, 1000));
    }

    #[test]
    fn dock_outside_monitor_is_ignored() {
        // 屏幕是 0..1600，strut 在 2000..2100，不应影响。
        let strut = Strut {
            bottom: 40,
            bottom_start_x: 2000,
            bottom_end_x: 2100,
            ..Default::default()
        };
        let work = compute_work_area(Rect::new(0, 0, 1600, 1000), &[strut], None, 2400, 1000);
        assert_eq!(work, Rect::new(0, 0, 1600, 1000));
    }

    #[test]
    fn global_workarea_clips_monitor() {
        // 第二块屏在 y=1440..3040，全局工作区只到 3003。
        let monitor = Rect::new(0, 1440, 2560, 1600);
        let work = compute_work_area(monitor, &[], Some(Rect::new(0, 0, 3440, 3003)), 3440, 3040);
        assert_eq!(work, Rect::new(0, 1440, 2560, 1563));
    }

    #[test]
    fn hash_is_deterministic_and_short() {
        let bytes: Vec<u8> = (0..64u8).collect();
        let a = short_hash(&bytes);
        let b = short_hash(&bytes);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn hash_differs_for_different_panels() {
        let mut left = vec![0u8; 32];
        let mut right = vec![0u8; 32];
        left[8] = 1;
        right[8] = 2;
        assert_ne!(short_hash(&left), short_hash(&right));
    }
}
