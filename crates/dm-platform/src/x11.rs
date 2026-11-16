//! X11 后端：桌面层窗口与全局热键。
//!
//! 只在 DDE/KWin 的 X11 会话使用。Wayland（Treeland）走独立模块，
//! 两者都实现同一组能力，宿主不感知差异。

use crate::error::{PlatformError, PlatformResult};
use crate::hotkey::{Hotkey, HotkeyBinding};
use dm_core::Rect;
use std::sync::{Mutex, OnceLock};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::{COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT};

/// 桌面层窗口收到的事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerEvent {
    /// 需要重绘（内容由调用方决定）。
    Expose,
    /// 左键按下，坐标为窗口内逻辑像素。
    Press { x: i16, y: i16 },
    /// 窗口几何被外部改变。
    Geometry { rect: Rect },
}

struct Atoms {
    window_type: u32,
    type_desktop: u32,
    state: u32,
    skip_taskbar: u32,
    skip_pager: u32,
    below: u32,
    motif_hints: u32,
    desktop_hint: u32,
}

/// 一个贴在桌面层的窗口（一块"瓦片"的宿主）。
pub struct X11DesktopLayer {
    conn: RustConnection,
    win: Window,
    atoms: Atoms,
}

impl X11DesktopLayer {
    /// 创建并显示。`title` 只用于调试（`xwininfo`/`wmctrl` 里能认出来）。
    pub fn attach(title: &str, rect: Rect) -> PlatformResult<Self> {
        if std::env::var_os("DISPLAY").is_none() {
            return Err(PlatformError::NoX11Display);
        }
        let (conn, screen_num) = x11rb::connect(None)?;
        let screen = &conn.setup().roots[screen_num];
        let atoms = Atoms {
            window_type: intern(&conn, "_NET_WM_WINDOW_TYPE")?,
            type_desktop: intern(&conn, "_NET_WM_WINDOW_TYPE_DESKTOP")?,
            state: intern(&conn, "_NET_WM_STATE")?,
            skip_taskbar: intern(&conn, "_NET_WM_STATE_SKIP_TASKBAR")?,
            skip_pager: intern(&conn, "_NET_WM_STATE_SKIP_PAGER")?,
            below: intern(&conn, "_NET_WM_STATE_BELOW")?,
            motif_hints: intern(&conn, "_MOTIF_WM_HINTS")?,
            desktop_hint: intern(&conn, "_NET_WM_DESKTOP")?,
        };

        let win = conn.generate_id()?;
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            win,
            screen.root,
            rect.x as i16,
            rect.y as i16,
            rect.width as u16,
            rect.height as u16,
            0,
            WindowClass::INPUT_OUTPUT,
            COPY_FROM_PARENT,
            &CreateWindowAux::new()
                .background_pixel(screen.black_pixel)
                .event_mask(
                    EventMask::EXPOSURE | EventMask::BUTTON_PRESS | EventMask::STRUCTURE_NOTIFY,
                ),
        )?;

        conn.change_property8(
            PropMode::REPLACE,
            win,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        )?;
        conn.change_property8(
            PropMode::REPLACE,
            win,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            b"deskmason\0DeskMason\0",
        )?;

        // 关键三件套：类型=DESKTOP、置底、不进任务栏/分页器。
        conn.change_property32(
            PropMode::REPLACE,
            win,
            atoms.window_type,
            AtomEnum::ATOM,
            &[atoms.type_desktop],
        )?;
        conn.change_property32(
            PropMode::REPLACE,
            win,
            atoms.state,
            AtomEnum::ATOM,
            &[atoms.skip_taskbar, atoms.skip_pager, atoms.below],
        )?;
        conn.change_property32(
            PropMode::REPLACE,
            win,
            atoms.motif_hints,
            atoms.motif_hints,
            &[2, 0, 0, 0, 0], // 只声明"无边框"
        )?;
        conn.change_property32(
            PropMode::REPLACE,
            win,
            atoms.desktop_hint,
            AtomEnum::CARDINAL,
            &[0xFFFF_FFFF], // 所有工作区
        )?;

        conn.map_window(win)?;
        conn.flush()?;

        Ok(Self { conn, win, atoms })
    }

    pub fn window_id(&self) -> u32 {
        self.win
    }

    pub fn set_geometry(&mut self, rect: Rect) -> PlatformResult<()> {
        self.conn.configure_window(
            self.win,
            &ConfigureWindowAux::new()
                .x(rect.x)
                .y(rect.y)
                .width(rect.width)
                .height(rect.height),
        )?;
        self.conn.flush()?;
        Ok(())
    }

    /// 纯色填充。QML 接管后这里会换成共享 surface，不再直接画。
    pub fn set_color(&mut self, rgb: u32) -> PlatformResult<()> {
        self.conn.change_window_attributes(
            self.win,
            &ChangeWindowAttributesAux::new().background_pixel(rgb),
        )?;
        self.conn.clear_area(false, self.win, 0, 0, 0, 0)?;
        self.conn.flush()?;
        Ok(())
    }

    /// 非阻塞取事件；宿主每帧调一次。
    pub fn pump(&mut self) -> PlatformResult<Vec<LayerEvent>> {
        let mut out = Vec::new();
        while let Some(event) = self.conn.poll_for_event()? {
            match event {
                Event::Expose(_) => out.push(LayerEvent::Expose),
                Event::ButtonPress(p) => out.push(LayerEvent::Press {
                    x: p.event_x,
                    y: p.event_y,
                }),
                Event::ConfigureNotify(c) => out.push(LayerEvent::Geometry {
                    rect: Rect::new(c.x as i32, c.y as i32, c.width as u32, c.height as u32),
                }),
                _ => {}
            }
        }
        Ok(out)
    }

    pub fn destroy(self) -> PlatformResult<()> {
        self.conn.destroy_window(self.win)?;
        self.conn.flush()?;
        Ok(())
    }

    /// 供探针使用：把窗口设为某种底色并保持一段时间。
    pub fn handle_for_probe(&self) -> Window {
        self.win
    }

    pub fn atoms_below(&self) -> u32 {
        self.atoms.below
    }
}

/// X11 全局热键。
pub struct X11Hotkeys {
    conn: RustConnection,
    root: Window,
    next_id: u64,
    bindings: Vec<HotkeyBinding>,
    keycodes: Vec<(u64, u8, Hotkey)>,
}

impl X11Hotkeys {
    pub fn new() -> PlatformResult<Self> {
        if std::env::var_os("DISPLAY").is_none() {
            return Err(PlatformError::NoX11Display);
        }
        let (conn, screen_num) = x11rb::connect(None)?;
        let root = conn.setup().roots[screen_num].root;
        Ok(Self {
            conn,
            root,
            next_id: 1,
            bindings: Vec::new(),
            keycodes: Vec::new(),
        })
    }

    pub fn grab(&mut self, hotkey: Hotkey) -> PlatformResult<HotkeyBinding> {
        let keycode = self.keycode_for(hotkey.keysym)?;
        let mods = modifiers(hotkey);

        // CapsLock/NumLock 是用户可能开着的状态，两种都要抢，
        // 否则"没反应"的投诉会永远查不出来。
        for extra in [ModMask::from(0u16), ModMask::LOCK | ModMask::M2] {
            if let Err(error) = self
                .conn
                .grab_key(
                    true,
                    self.root,
                    mods | extra,
                    keycode,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                )?
                .check()
            {
                // 回滚已抢到的另一半，避免半注册状态。
                let _ = self.conn.ungrab_key(keycode, self.root, mods | extra);
                for e in [ModMask::from(0u16), ModMask::LOCK | ModMask::M2] {
                    let _ = self.conn.ungrab_key(keycode, self.root, mods | e);
                }
                self.conn.flush()?;
                return Err(PlatformError::HotkeyConflict(format!(
                    "{} ({error})",
                    hotkey.label()
                )));
            }
        }
        self.conn.flush()?;

        let binding = HotkeyBinding {
            id: self.next_id,
            hotkey,
        };
        self.next_id += 1;
        self.bindings.push(binding);
        self.keycodes.push((binding.id, keycode, hotkey));
        Ok(binding)
    }

    pub fn ungrab(&mut self, id: u64) -> PlatformResult<()> {
        if let Some(pos) = self.keycodes.iter().position(|(bid, _, _)| *bid == id) {
            let (_, keycode, hotkey) = self.keycodes.remove(pos);
            for extra in [ModMask::from(0u16), ModMask::LOCK | ModMask::M2] {
                let _ = self
                    .conn
                    .ungrab_key(keycode, self.root, modifiers(hotkey) | extra);
            }
            self.conn.flush()?;
            self.bindings.retain(|b| b.id != id);
        }
        Ok(())
    }

    /// 返回这次轮询里被触发的绑定 id。
    pub fn poll(&mut self) -> PlatformResult<Vec<u64>> {
        let mut hits = Vec::new();
        while let Some(event) = self.conn.poll_for_event()? {
            if let Event::KeyPress(press) = event {
                if let Some((id, _, _)) = self
                    .keycodes
                    .iter()
                    .find(|(_, keycode, _)| *keycode == press.detail)
                {
                    hits.push(*id);
                }
            }
        }
        Ok(hits)
    }

    fn keycode_for(&self, keysym: u32) -> PlatformResult<u8> {
        let setup = self.conn.setup();
        let count = setup.max_keycode - setup.min_keycode + 1;
        let mapping = self
            .conn
            .get_keyboard_mapping(setup.min_keycode, count)?
            .reply()?;
        let per = mapping.keysyms_per_keycode as usize;
        for i in 0..count as usize {
            let row = &mapping.keysyms[i * per..(i + 1) * per];
            if row.contains(&keysym) {
                return Ok(setup.min_keycode + i as u8);
            }
        }
        Err(PlatformError::HotkeyConflict(format!(
            "键盘映射里找不到 keysym 0x{keysym:X}"
        )))
    }
}

/// 把已存在的窗口设为"普通窗口但永远置底"。
///
/// 与 [`apply_desktop_layer`] 的区别：不写 `_NET_WM_WINDOW_TYPE_DESKTOP`，
/// 因此窗口位于桌面图标层之上、普通窗口之下，可以正常接收鼠标点击。
/// 把窗口稳定保持在底层，是桌面组件的常规做法。
pub fn apply_below_layer(window_id: u32, title: &str) -> PlatformResult<()> {
    if std::env::var_os("DISPLAY").is_none() {
        return Err(PlatformError::NoX11Display);
    }
    let (conn, _screen_num) = x11rb::connect(None)?;
    let state = intern(&conn, "_NET_WM_STATE")?;
    let skip_taskbar = intern(&conn, "_NET_WM_STATE_SKIP_TASKBAR")?;
    let skip_pager = intern(&conn, "_NET_WM_STATE_SKIP_PAGER")?;
    let below = intern(&conn, "_NET_WM_STATE_BELOW")?;
    let motif = intern(&conn, "_MOTIF_WM_HINTS")?;
    let desktop_hint = intern(&conn, "_NET_WM_DESKTOP")?;
    let win = window_id;

    if !title.is_empty() {
        conn.change_property8(
            PropMode::REPLACE,
            win,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        )?;
    }
    conn.change_property8(
        PropMode::REPLACE,
        win,
        AtomEnum::WM_CLASS,
        AtomEnum::STRING,
        b"deskmason\0DeskMason\0",
    )?;
    conn.change_property32(
        PropMode::REPLACE,
        win,
        state,
        AtomEnum::ATOM,
        &[skip_taskbar, skip_pager, below],
    )?;
    conn.change_property32(PropMode::REPLACE, win, motif, motif, &[2, 0, 0, 0, 0])?;
    conn.change_property32(
        PropMode::REPLACE,
        win,
        desktop_hint,
        AtomEnum::CARDINAL,
        &[0xFFFF_FFFF],
    )?;
    conn.flush()?;
    Ok(())
}

/// 把已存在的窗口（例如 Qt 创建的 QQuickWindow）设为桌面层。
///
/// 必须在窗口映射**之前**调用：EWMH 只在 MapRequest 时读取窗口类型，
/// 映射后再改会被窗管忽略。
pub fn apply_desktop_layer(window_id: u32, title: &str) -> PlatformResult<()> {
    if std::env::var_os("DISPLAY").is_none() {
        return Err(PlatformError::NoX11Display);
    }
    let (conn, _screen_num) = x11rb::connect(None)?;
    let atoms = Atoms {
        window_type: intern(&conn, "_NET_WM_WINDOW_TYPE")?,
        type_desktop: intern(&conn, "_NET_WM_WINDOW_TYPE_DESKTOP")?,
        state: intern(&conn, "_NET_WM_STATE")?,
        skip_taskbar: intern(&conn, "_NET_WM_STATE_SKIP_TASKBAR")?,
        skip_pager: intern(&conn, "_NET_WM_STATE_SKIP_PAGER")?,
        below: intern(&conn, "_NET_WM_STATE_BELOW")?,
        motif_hints: intern(&conn, "_MOTIF_WM_HINTS")?,
        desktop_hint: intern(&conn, "_NET_WM_DESKTOP")?,
    };
    let win = window_id;

    if !title.is_empty() {
        conn.change_property8(
            PropMode::REPLACE,
            win,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        )?;
    }
    conn.change_property8(
        PropMode::REPLACE,
        win,
        AtomEnum::WM_CLASS,
        AtomEnum::STRING,
        b"deskmason\0DeskMason\0",
    )?;
    conn.change_property32(
        PropMode::REPLACE,
        win,
        atoms.window_type,
        AtomEnum::ATOM,
        &[atoms.type_desktop],
    )?;
    conn.change_property32(
        PropMode::REPLACE,
        win,
        atoms.state,
        AtomEnum::ATOM,
        &[atoms.skip_taskbar, atoms.skip_pager, atoms.below],
    )?;
    conn.change_property32(
        PropMode::REPLACE,
        win,
        atoms.motif_hints,
        atoms.motif_hints,
        &[2, 0, 0, 0, 0],
    )?;
    conn.change_property32(
        PropMode::REPLACE,
        win,
        atoms.desktop_hint,
        AtomEnum::CARDINAL,
        &[0xFFFF_FFFF],
    )?;
    conn.flush()?;
    Ok(())
}

/// 在后台线程常驻监听一个全局热键。
///
/// 给窗口开启 KWin 背景模糊（`_KDE_NET_WM_BLUR_BEHIND`）。
///
/// 用于搜索窗 / 设置窗的"亚克力 / 云母"材质：窗管对窗口后方做高斯模糊，
/// QML 侧再用不同透明度的背景色区分亚克力（透亮）与云母（暗调）。
/// 窗管不支持或非 KWin 时静默失败——效果退化成纯色半透明，不影响功能。
pub fn apply_blur_behind(window_id: u32) -> PlatformResult<()> {
    if std::env::var_os("DISPLAY").is_none() {
        return Err(PlatformError::NoX11Display);
    }
    let (conn, _screen_num) = x11rb::connect(None)?;
    let blur = intern(&conn, "_KDE_NET_WM_BLUR_BEHIND")?;
    conn.change_property32(PropMode::REPLACE, window_id, blur, AtomEnum::CARDINAL, &[1])?;
    conn.flush()?;
    Ok(())
}

static DESKTOP_FRAME: OnceLock<Mutex<Option<Window>>> = OnceLock::new();

fn desktop_frame_cache() -> &'static Mutex<Option<Window>> {
    DESKTOP_FRAME.get_or_init(|| Mutex::new(None))
}

/// 找到 KWin 桌面窗口的 frame（root 的直接子窗口）。
///
/// KWin 把受管窗口重父级到 frame，桌面 client 是 frame 的孙窗口；
/// `_NET_WM_WINDOW_TYPE_DESKTOP` 只写在 client 上，frame 自己没有属性，
/// 所以要往下看两层。`query_tree` 的 children 按 stacking 从底到顶，
/// 取最后一个匹配项 = 最上面那块桌面（双屏各有自己的桌面 frame）。
fn find_desktop_frame(conn: &RustConnection, root: Window) -> Option<Window> {
    let ty = intern(conn, "_NET_WM_WINDOW_TYPE").ok()?;
    let desktop_type = intern(conn, "_NET_WM_WINDOW_TYPE_DESKTOP").ok()?;
    let children = conn.query_tree(root).ok()?.reply().ok()?.children;
    let mut best = None;
    for frame in children {
        let mut candidates = vec![frame];
        if let Some(sub) = conn
            .query_tree(frame)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
        {
            for child in sub.children {
                candidates.push(child);
                if let Some(sub2) = conn
                    .query_tree(child)
                    .ok()
                    .and_then(|cookie| cookie.reply().ok())
                {
                    for grandchild in sub2.children {
                        candidates.push(grandchild);
                    }
                }
            }
        }
        for win in candidates {
            let Some(prop) = conn
                .get_property(false, win, ty, AtomEnum::ATOM, 0, 4)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
            else {
                continue;
            };
            let is_desktop = prop
                .value32()
                .map(|mut atoms| atoms.any(|atom| atom == desktop_type))
                .unwrap_or(false);
            if is_desktop {
                best = Some(frame);
                break;
            }
        }
    }
    best
}

/// 从 client 窗口向上爬到 root 的直接子窗口（KWin 的 frame）。
/// X11 的 `sibling` restack 要求两个窗口同父；瓦片被 KWin 重父级后，
/// 拿 client id 直接 restack 会 BadMatch，必须换成它的 frame。
fn toplevel_frame(conn: &RustConnection, win: Window, root: Window) -> Window {
    let mut current = win;
    for _ in 0..4 {
        let Some(reply) = conn
            .query_tree(current)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
        else {
            break;
        };
        if reply.parent == root || reply.parent == 0 || reply.parent == current {
            break;
        }
        current = reply.parent;
    }
    current
}

/// 把瓦片保持在「桌面之上、普通窗口之下」。
///
/// 之前周期性用全局 `StackMode::ABOVE` 把瓦片抬到整个栈顶，虽然能避免点击
/// 桌面空白处后被桌面盖住，却让待办/随记/天气/时光压住微信、文件管理器等
/// 普通窗口。也不能改成全局 `BELOW`——那样瓦片会掉到桌面 frame 之下而消失。
///
/// 正确做法是指定 sibling：只把瓦片 frame 放到桌面 frame 正上方。桌面 frame
/// 本来就在所有普通窗口 frame 之下，于是瓦片既不遮挡别的窗口，也不会被桌面
/// 盖住。另外用 EWMH client message 请求 `_NET_WM_STATE_BELOW`，让 KWin
/// 自己也把窗口归回 below 层（直接写属性只在 MapRequest 时生效）。
pub fn keep_tile_below(window_id: u32) -> PlatformResult<()> {
    if std::env::var_os("DISPLAY").is_none() {
        return Err(PlatformError::NoX11Display);
    }
    let (conn, screen_num) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen_num].root;
    let state = intern(&conn, "_NET_WM_STATE")?;
    let skip_taskbar = intern(&conn, "_NET_WM_STATE_SKIP_TASKBAR")?;
    let skip_pager = intern(&conn, "_NET_WM_STATE_SKIP_PAGER")?;
    let below = intern(&conn, "_NET_WM_STATE_BELOW")?;
    conn.change_property32(
        PropMode::REPLACE,
        window_id,
        state,
        AtomEnum::ATOM,
        &[skip_taskbar, skip_pager, below],
    )?;

    // EWMH 规定：已映射窗口的状态变更必须发 client message 给 root，
    // 窗管才会重新分层；change_property 只是补齐属性本身。
    const NET_WM_STATE_ADD: u32 = 1;
    let notice = ClientMessageEvent::new(32, window_id, state, [NET_WM_STATE_ADD, below, 0, 0, 0]);
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        notice,
    )?;

    let cached = desktop_frame_cache().lock().ok().and_then(|guard| *guard);
    let desktop_frame = match cached.or_else(|| find_desktop_frame(&conn, root)) {
        Some(id) => {
            if cached.is_none() {
                if let Ok(mut guard) = desktop_frame_cache().lock() {
                    *guard = Some(id);
                }
            }
            id
        }
        // 找不到桌面 frame 就只保留状态，绝不发会压住普通窗口的全局 restack。
        None => {
            conn.flush()?;
            return Ok(());
        }
    };
    let tile_frame = toplevel_frame(&conn, window_id, root);
    conn.configure_window(
        tile_frame,
        &ConfigureWindowAux::new()
            .sibling(desktop_frame)
            .stack_mode(StackMode::ABOVE),
    )?;
    conn.flush()?;
    Ok(())
}

/// 把瓦片窗口的 EWMH 窗口类型写回统一值。
///
/// Qt 会在窗口标志变化时重写 `_NET_WM_WINDOW_TYPE`：放行激活焦点的窗口会被
/// 写成 `_NET_WM_WINDOW_TYPE_DESKTOP`，而不抢焦点的窗口是
/// `_KDE_NET_WM_WINDOW_TYPE_OVERRIDE` + `_NET_WM_WINDOW_TYPE_NORMAL`。
/// 后者才是 KWin 按透明/圆角合成的那一类。窗口映射前统一不抢焦点已让窗管
/// 记住正确类型；映射后放开焦点时再写回一次，保证 6 块瓦片属性一致。
pub fn apply_tile_window_type(window_id: u32) -> PlatformResult<()> {
    if std::env::var_os("DISPLAY").is_none() {
        return Err(PlatformError::NoX11Display);
    }
    let (conn, _screen_num) = x11rb::connect(None)?;
    let ty = intern(&conn, "_NET_WM_WINDOW_TYPE")?;
    let override_type = intern(&conn, "_KDE_NET_WM_WINDOW_TYPE_OVERRIDE")?;
    let normal = intern(&conn, "_NET_WM_WINDOW_TYPE_NORMAL")?;
    conn.change_property32(
        PropMode::REPLACE,
        window_id,
        ty,
        AtomEnum::ATOM,
        &[override_type, normal],
    )?;
    conn.flush()?;
    Ok(())
}

/// 给普通应用窗口（设置/搜索/引导）补 SKIP_TASKBAR / SKIP_PAGER。
///
/// DeskMason 的常驻入口是托盘（SNI），不该像普通应用一样在任务栏
/// 占一个条块。瓦片窗口在 apply_desktop_layer 里已带上这些状态；
/// 这三个工具窗走普通 Window 路径创建，需要单独补。
/// 映射前调用才可靠（KWin 在 MapRequest 时读取任务栏策略）。
pub fn skip_taskbar(window_id: u32) -> PlatformResult<()> {
    if std::env::var_os("DISPLAY").is_none() {
        return Err(PlatformError::NoX11Display);
    }
    let (conn, _screen_num) = x11rb::connect(None)?;
    let state = intern(&conn, "_NET_WM_STATE")?;
    let skip_taskbar = intern(&conn, "_NET_WM_STATE_SKIP_TASKBAR")?;
    let skip_pager = intern(&conn, "_NET_WM_STATE_SKIP_PAGER")?;
    conn.change_property32(
        PropMode::REPLACE,
        window_id,
        state,
        AtomEnum::ATOM,
        &[skip_taskbar, skip_pager],
    )?;
    conn.flush()?;
    Ok(())
}

/// 把 X 输入焦点交给指定的桌面层窗口。
///
/// 瓦片窗口设了 `Qt::WindowDoesNotAcceptFocus`（避免 fcitx5 在点击瓦片时
/// 反复切换输入上下文），副作用是：点击输入框后 Qt 内部有 activeFocus，
/// 但 X 输入焦点仍在别的应用，键盘事件根本进不来。这里在用户明确要输入时
/// 显式把 X 焦点设给该窗口（override-redirect 窗管不会主动激活）。
pub fn focus_window(window_id: u32) -> PlatformResult<()> {
    if std::env::var_os("DISPLAY").is_none() {
        return Err(PlatformError::NoX11Display);
    }
    let (conn, _screen_num) = x11rb::connect(None)?;
    set_input_focus(
        &conn,
        InputFocus::POINTER_ROOT,
        window_id,
        x11rb::CURRENT_TIME,
    )?;
    conn.flush()?;
    Ok(())
}

/// 注册（grab）在调用线程同步完成，失败会立即返回错误；之后线程只负责轮询。
pub fn spawn_hotkey_listener(
    hotkey: Hotkey,
    on_press: impl Fn() + Send + 'static,
) -> PlatformResult<()> {
    let mut service = X11Hotkeys::new()?;
    service.grab(hotkey)?;
    std::thread::spawn(move || loop {
        match service.poll() {
            Ok(hits) if !hits.is_empty() => on_press(),
            Ok(_) => {}
            Err(_) => break,
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    });
    Ok(())
}

fn modifiers(hotkey: Hotkey) -> ModMask {
    let mut mods = ModMask::from(0u16);
    if hotkey.ctrl {
        mods |= ModMask::CONTROL;
    }
    if hotkey.alt {
        mods |= ModMask::M1;
    }
    if hotkey.shift {
        mods |= ModMask::SHIFT;
    }
    if hotkey.super_key {
        mods |= ModMask::M4;
    }
    mods
}

fn intern(conn: &RustConnection, name: &str) -> PlatformResult<u32> {
    Ok(conn.intern_atom(false, name.as_bytes())?.reply()?.atom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_mask_maps_every_flag() {
        let hk = Hotkey::new(true, true, true, true, 0x006a);
        let mods = modifiers(hk);
        assert!(mods.contains(ModMask::CONTROL));
        assert!(mods.contains(ModMask::M1));
        assert!(mods.contains(ModMask::SHIFT));
        assert!(mods.contains(ModMask::M4));
    }

    #[test]
    fn modifier_mask_is_empty_without_flags() {
        assert_eq!(
            modifiers(Hotkey::new(false, false, false, false, 0x6a)),
            ModMask::from(0u16)
        );
    }
}
