use serde::{Deserialize, Serialize};
use std::fmt;

/// 一个物理显示器的稳定身份。
///
/// 不能用 X11 的 RandR 输出编号：重插、改主屏、睡眠唤醒都会变。
/// 这里存调用方算好的稳定指纹（EDID 制造商+型号+序列号的哈希）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MonitorId(String);

impl MonitorId {
    pub fn new(fingerprint: impl Into<String>) -> Self {
        let value = fingerprint.into();
        assert!(!value.is_empty(), "monitor identity must not be empty");
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MonitorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 屏幕坐标矩形，左上角为原点，单位为设备无关的逻辑像素。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> i32 {
        self.x.saturating_add(self.width as i32)
    }

    pub fn bottom(&self) -> i32 {
        self.y.saturating_add(self.height as i32)
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    pub fn intersection_area(&self, other: &Rect) -> u64 {
        if !self.intersects(other) {
            return 0;
        }
        let w = (self.right().min(other.right()) - self.x.max(other.x)) as u64;
        let h = (self.bottom().min(other.bottom()) - self.y.max(other.y)) as u64;
        w * h
    }

    /// 把矩形完整收进容器；放不下时按容器尺寸截断。
    pub fn clamp_into(&self, container: &Rect) -> Rect {
        let width = self.width.min(container.width);
        let height = self.height.min(container.height);
        let max_x = container.right() - width as i32;
        let max_y = container.bottom() - height as i32;
        Rect {
            x: self.x.clamp(container.x, max_x),
            y: self.y.clamp(container.y, max_y),
            width,
            height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection_area_counts_only_overlap() {
        let a = Rect::new(0, 0, 100, 100);
        let b = Rect::new(50, 50, 100, 100);
        assert_eq!(a.intersection_area(&b), 50 * 50);
        assert_eq!(a.intersection_area(&Rect::new(200, 200, 10, 10)), 0);
    }

    #[test]
    fn clamp_keeps_rect_inside_container() {
        let container = Rect::new(0, 0, 1000, 800);
        let clamped = Rect::new(-50, 700, 200, 400).clamp_into(&container);
        assert_eq!(clamped, Rect::new(0, 400, 200, 400));
    }

    #[test]
    fn clamp_shrinks_oversized_rect() {
        let clamped = Rect::new(0, 0, 5000, 5000).clamp_into(&Rect::new(0, 0, 1000, 800));
        assert_eq!(clamped, Rect::new(0, 0, 1000, 800));
    }
}
