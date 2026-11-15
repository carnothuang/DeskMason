/// 与后端无关的热键描述。
///
/// `keysym` 使用 X11 keysym 值（字母/数字就是 ASCII，功能键在 0xFF00 段），
/// 这样 X11 与 Wayland 后端可以共用同一份配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
    pub keysym: u32,
}

impl Hotkey {
    pub const fn new(ctrl: bool, alt: bool, shift: bool, super_key: bool, keysym: u32) -> Self {
        Self {
            ctrl,
            alt,
            shift,
            super_key,
            keysym,
        }
    }

    /// 解析 `ctrl+alt+f9` / `super+space` 这类配置串。
    pub fn parse(text: &str) -> Option<Self> {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut super_key = false;
        let mut keysym = None;

        for part in text.split('+').map(|p| p.trim().to_ascii_lowercase()) {
            match part.as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" | "mod1" => alt = true,
                "shift" => shift = true,
                "super" | "win" | "meta" | "mod4" => super_key = true,
                "" => return None,
                key => keysym = Some(keysym_from_name(key)?),
            }
        }

        Some(Self {
            ctrl,
            alt,
            shift,
            super_key,
            keysym: keysym?,
        })
    }

    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.super_key {
            parts.push("Super".to_string());
        }
        parts.push(name_from_keysym(self.keysym));
        parts.join("+")
    }
}

/// 一条已注册的绑定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HotkeyBinding {
    pub id: u64,
    pub hotkey: Hotkey,
}

fn keysym_from_name(name: &str) -> Option<u32> {
    // 单字符（ASCII）
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            return Some(c as u32);
        }
    }

    if let Some(rest) = name.strip_prefix('f') {
        if let Ok(n) = rest.parse::<u32>() {
            if (1..=24).contains(&n) {
                return Some(0xFFBE + n - 1);
            }
        }
    }

    Some(match name {
        "space" => 0x0020,
        "escape" | "esc" => 0xFF1B,
        "return" | "enter" => 0xFF0D,
        "tab" => 0xFF09,
        "backspace" => 0xFF08,
        "delete" => 0xFFFF,
        "home" => 0xFF50,
        "end" => 0xFF57,
        "left" => 0xFF51,
        "up" => 0xFF52,
        "right" => 0xFF53,
        "down" => 0xFF54,
        _ => return None,
    })
}

fn name_from_keysym(keysym: u32) -> String {
    match keysym {
        0x0020 => "Space".into(),
        0xFF1B => "Esc".into(),
        0xFF0D => "Enter".into(),
        0xFF09 => "Tab".into(),
        0xFF08 => "Backspace".into(),
        0xFFFF => "Delete".into(),
        0xFF50 => "Home".into(),
        0xFF57 => "End".into(),
        0xFF51 => "Left".into(),
        0xFF52 => "Up".into(),
        0xFF53 => "Right".into(),
        0xFF54 => "Down".into(),
        0xFFBE..=0xFFD5 => format!("F{}", keysym - 0xFFBE + 1),
        k if k <= 0x7E => (k as u8 as char).to_ascii_uppercase().to_string(),
        other => format!("0x{other:X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_combinations() {
        let hk = Hotkey::parse("ctrl+alt+f9").unwrap();
        assert!(hk.ctrl && hk.alt && !hk.shift && !hk.super_key);
        assert_eq!(hk.keysym, 0xFFC6);
        assert_eq!(hk.label(), "Ctrl+Alt+F9");

        let hk = Hotkey::parse("Super+Space").unwrap();
        assert!(hk.super_key);
        assert_eq!(hk.keysym, 0x0020);
    }

    #[test]
    fn rejects_empty_and_unknown() {
        assert!(Hotkey::parse("").is_none());
        assert!(Hotkey::parse("ctrl+").is_none());
        assert!(Hotkey::parse("ctrl+f99").is_none());
        assert!(Hotkey::parse("ctrl+nosuchkey").is_none());
    }

    #[test]
    fn label_round_trips() {
        for text in ["ctrl+alt+j", "super+space", "ctrl+shift+f12", "alt+left"] {
            let hk = Hotkey::parse(text).unwrap();
            let again = Hotkey::parse(&hk.label()).unwrap();
            assert_eq!(hk, again, "round trip failed for {text}");
        }
    }
}
