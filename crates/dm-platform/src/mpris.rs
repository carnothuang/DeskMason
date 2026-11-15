//! MPRIS2 客户端：读取播放器状态、控制播放。
//!
//! Linux 上没有 Windows 那种统一的媒体会话 API，MPRIS2 就是事实标准；
//! 几乎所有播放器（Deepin Music、Spotify、浏览器）都实现了它。

use std::collections::HashMap;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedValue, Value};

const PLAYER_PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";
const ROOT_INTERFACE: &str = "org.mpris.MediaPlayer2";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: String,
    pub length_us: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerState {
    pub bus_name: String,
    pub identity: String,
    pub playing: bool,
    pub status: String,
    pub track: TrackMeta,
    pub position_us: i64,
    pub can_next: bool,
    pub can_previous: bool,
}

pub struct MprisClient {
    conn: Connection,
}

impl MprisClient {
    pub fn connect() -> Result<Self, String> {
        Connection::session()
            .map(|conn| Self { conn })
            .map_err(|e| format!("无法连接会话总线：{e}"))
    }

    /// 所有实现了 MPRIS2 的播放器总线名。
    pub fn players(&self) -> Result<Vec<String>, String> {
        let reply = self
            .conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "ListNames",
                &(),
            )
            .map_err(|e| e.to_string())?;
        let names: Vec<String> = reply.body().deserialize().map_err(|e| e.to_string())?;
        Ok(names
            .into_iter()
            .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
            .collect())
    }

    fn player_proxy<'a>(&'a self, bus: &'a str) -> Result<Proxy<'a>, String> {
        Proxy::new(&self.conn, bus, PLAYER_PATH, PLAYER_INTERFACE).map_err(|e| e.to_string())
    }

    pub fn snapshot(&self, bus: &str) -> Result<PlayerState, String> {
        let proxy = self.player_proxy(bus)?;
        let status: String = proxy
            .get_property("PlaybackStatus")
            .map_err(|e| e.to_string())?;
        let position_us: i64 = proxy.get_property("Position").unwrap_or(0);
        let can_next: bool = proxy.get_property("CanGoNext").unwrap_or(false);
        let can_previous: bool = proxy.get_property("CanGoPrevious").unwrap_or(false);
        let metadata: HashMap<String, OwnedValue> =
            proxy.get_property("Metadata").map_err(|e| e.to_string())?;

        let identity = Proxy::new(&self.conn, bus, PLAYER_PATH, ROOT_INTERFACE)
            .ok()
            .and_then(|root| root.get_property::<String>("Identity").ok())
            .unwrap_or_else(|| bus.to_string());

        Ok(PlayerState {
            bus_name: bus.to_string(),
            identity,
            playing: status == "Playing",
            status,
            track: parse_metadata(&metadata),
            position_us,
            can_next,
            can_previous,
        })
    }

    /// 选一个有音乐在放的播放器；都停着就选第一个。
    pub fn active_player(&self) -> Result<Option<PlayerState>, String> {
        let players = self.players()?;
        if players.is_empty() {
            return Ok(None);
        }
        let mut fallback = None;
        for bus in players {
            if let Ok(state) = self.snapshot(&bus) {
                if state.playing {
                    return Ok(Some(state));
                }
                if fallback.is_none() {
                    fallback = Some(state);
                }
            }
        }
        Ok(fallback)
    }

    pub fn play_pause(&self, bus: &str) -> Result<(), String> {
        self.player_proxy(bus)?
            .call_method("PlayPause", &())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn next(&self, bus: &str) -> Result<(), String> {
        self.player_proxy(bus)?
            .call_method("Next", &())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn previous(&self, bus: &str) -> Result<(), String> {
        self.player_proxy(bus)?
            .call_method("Previous", &())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// MPRIS Metadata 是 `a{sv}`，值可能是字符串、数组或整数。
pub fn parse_metadata(metadata: &HashMap<String, OwnedValue>) -> TrackMeta {
    let title = metadata
        .get("xesam:title")
        .and_then(text_value)
        .unwrap_or_default();
    let artist = metadata
        .get("xesam:artist")
        .map(text_list)
        .unwrap_or_default()
        .join("、");
    let album = metadata
        .get("xesam:album")
        .and_then(text_value)
        .unwrap_or_default();
    let art_url = metadata
        .get("mpris:artUrl")
        .and_then(text_value)
        .unwrap_or_default();
    let length_us = metadata
        .get("mpris:length")
        .and_then(|value| value.downcast_ref::<i64>().ok())
        .unwrap_or(0);

    TrackMeta {
        title,
        artist,
        album,
        art_url,
        length_us,
    }
}

fn text_value(value: &OwnedValue) -> Option<String> {
    value
        .downcast_ref::<&str>()
        .ok()
        .map(|s| s.to_string())
        .or_else(|| value.downcast_ref::<String>().ok())
}

fn text_list(value: &OwnedValue) -> Vec<String> {
    match &**value {
        Value::Array(array) => array
            .iter()
            .filter_map(|item| item.downcast_ref::<&str>().ok().map(|s| s.to_string()))
            .collect(),
        _ => text_value(value).into_iter().collect(),
    }
}

/// 进度文案。长度未知（0 或大到离谱，例如 Edge 报 `i64::MAX`）时只显示当前位置。
pub fn progress_text(position_us: i64, length_us: i64) -> String {
    const MAX_SANE_US: i64 = 86_400_000_000; // 24 小时
    if length_us > 0 && length_us <= MAX_SANE_US {
        format!(
            "{} / {}",
            format_position(position_us),
            format_position(length_us)
        )
    } else {
        format_position(position_us)
    }
}

/// 把微秒位置格式化成 `1:23`。
pub fn format_position(position_us: i64) -> String {
    let total = (position_us.max(0) / 1_000_000) as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(value: Value<'static>) -> OwnedValue {
        value.try_into().unwrap()
    }

    #[test]
    fn parses_plain_metadata() {
        let mut map = HashMap::new();
        map.insert("xesam:title".to_string(), owned(Value::from("晴天")));
        map.insert("xesam:album".to_string(), owned(Value::from("叶惠美")));
        map.insert(
            "mpris:artUrl".to_string(),
            owned(Value::from("file:///tmp/cover.png")),
        );
        map.insert(
            "mpris:length".to_string(),
            owned(Value::from(269_000_000i64)),
        );

        let meta = parse_metadata(&map);
        assert_eq!(meta.title, "晴天");
        assert_eq!(meta.album, "叶惠美");
        assert_eq!(meta.art_url, "file:///tmp/cover.png");
        assert_eq!(meta.length_us, 269_000_000);
        assert!(meta.artist.is_empty());
    }

    #[test]
    fn joins_artist_array() {
        let mut map = HashMap::new();
        map.insert(
            "xesam:artist".to_string(),
            owned(Value::from(vec!["周杰伦", "方文山"])),
        );
        let meta = parse_metadata(&map);
        assert_eq!(meta.artist, "周杰伦、方文山");
    }

    #[test]
    fn missing_fields_are_empty() {
        let meta = parse_metadata(&HashMap::new());
        assert_eq!(meta, TrackMeta::default());
    }

    #[test]
    fn progress_handles_unknown_length() {
        assert_eq!(progress_text(83_000_000, 269_000_000), "1:23 / 4:29");
        // Edge 这类播放器会把长度报成 i64::MAX
        assert_eq!(progress_text(83_000_000, i64::MAX), "1:23");
        assert_eq!(progress_text(83_000_000, 0), "1:23");
    }

    #[test]
    fn formats_position() {
        assert_eq!(format_position(0), "0:00");
        assert_eq!(format_position(83_000_000), "1:23");
        assert_eq!(format_position(-5), "0:00");
    }
}
