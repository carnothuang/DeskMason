//! DeskMason 宿主。M0 阶段只有两个探针，QML 界面在 `dm-ui` 接好之后进来。

use anyhow::Result;
use dm_core::Rect;
use dm_platform::hotkey::Hotkey;
use dm_platform::x11::{LayerEvent, X11DesktopLayer, X11Hotkeys};
use std::thread;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--probe-desktop-layer") => probe_desktop_layer(&args[1..]),
        Some("--probe-hotkey") => probe_hotkey(&args[1..]),
        Some("--version") | Some("-V") => {
            println!("deskmason {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!(
        "deskmason {}\n\n\
         用法:\n  \
         deskmason --probe-desktop-layer [--seconds N]\n  \
         deskmason --probe-hotkey [--combo ctrl+alt+j] [--seconds N]\n  \
         deskmason --version",
        env!("CARGO_PKG_VERSION")
    );
}

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn seconds_from(args: &[String], default: u64) -> u64 {
    flag_value(args, "--seconds")
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn probe_desktop_layer(args: &[String]) -> Result<()> {
    let seconds = seconds_from(args, 20);
    let rect = Rect::new(120, 120, 640, 360);
    let mut layer = X11DesktopLayer::attach("DeskMason desktop-layer probe", rect)?;
    println!(
        "桌面层窗口已创建: id=0x{:x} rect={:?} 保持 {seconds}s",
        layer.window_id(),
        rect
    );
    layer.set_color(0x0033_6699)?;

    let started = Instant::now();
    let mut clicks = 0u32;
    let mut exposes = 0u32;
    while started.elapsed() < Duration::from_secs(seconds) {
        for event in layer.pump()? {
            match event {
                LayerEvent::Expose => {
                    exposes += 1;
                    layer.set_color(0x0033_6699)?;
                }
                LayerEvent::Press { x, y } => {
                    clicks += 1;
                    println!("点击 #{clicks} @ {x},{y}");
                    layer.set_color(0x00CC_6633)?;
                }
                LayerEvent::Geometry { rect } => println!("几何变化: {rect:?}"),
            }
        }
        thread::sleep(Duration::from_millis(16));
    }

    println!("RESULT exposes={exposes} clicks={clicks}");
    layer.destroy()?;
    Ok(())
}

fn probe_hotkey(args: &[String]) -> Result<()> {
    let seconds = seconds_from(args, 20);
    let combo = flag_value(args, "--combo").unwrap_or("ctrl+alt+j");
    let hotkey = Hotkey::parse(combo).ok_or_else(|| anyhow::anyhow!("无法解析热键: {combo}"))?;

    let mut hotkeys = X11Hotkeys::new()?;
    let binding = hotkeys.grab(hotkey)?;
    println!(
        "已注册 {} (id={})，保持 {seconds}s",
        hotkey.label(),
        binding.id
    );

    let started = Instant::now();
    let mut hits = 0u32;
    while started.elapsed() < Duration::from_secs(seconds) {
        for id in hotkeys.poll()? {
            if id == binding.id {
                hits += 1;
                println!("触发 #{hits} @ {:.1}s", started.elapsed().as_secs_f32());
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    hotkeys.ungrab(binding.id)?;
    println!("RESULT hits={hits}");
    if hits == 0 {
        eprintln!("提示: 若在真实会话里为 0，先确认会话未锁屏（dde-lock 可能持有键盘 grab）");
        return Ok(());
    }
    Ok(())
}
