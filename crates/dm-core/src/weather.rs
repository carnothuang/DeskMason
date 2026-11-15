//! 天气瓦片的领域逻辑：WMO 天气代码映射、Open-Meteo 响应解析、缓存结构。
//!
//! 解析函数是纯函数，测试用固定 JSON，不碰网络。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeatherSnapshot {
    pub city: String,
    pub latitude: f64,
    pub longitude: f64,
    pub temperature: f64,
    pub weather_code: u32,
    pub max: f64,
    pub min: f64,
    /// 观测时间（ISO，来自接口）
    pub observed_at: String,
    /// 写入本地缓存的时间（Unix 秒）
    pub cached_at: i64,
}

impl WeatherSnapshot {
    pub fn condition(&self) -> &'static str {
        weather_text(self.weather_code)
    }

    pub fn symbol(&self) -> &'static str {
        weather_symbol(self.weather_code)
    }

    pub fn temperature_text(&self) -> String {
        format!("{:.0}°", self.temperature)
    }

    pub fn range_text(&self) -> String {
        format!("{:.0}° / {:.0}°", self.min, self.max)
    }
}

/// WMO 天气代码 → 中文描述。
pub fn weather_text(code: u32) -> &'static str {
    match code {
        0 => "晴",
        1 => "大部晴",
        2 => "多云",
        3 => "阴",
        45 | 48 => "雾",
        51 | 53 | 55 => "毛毛雨",
        56 | 57 => "冻雨",
        61 | 63 | 65 => "雨",
        66 | 67 => "冻雨",
        71 | 73 | 75 => "雪",
        77 => "雪粒",
        80..=82 => "阵雨",
        85 | 86 => "阵雪",
        95 => "雷阵雨",
        96 | 99 => "雷暴伴冰雹",
        _ => "未知",
    }
}

/// 用 DejaVu 等常见字体都覆盖的符号，避免依赖彩色 emoji 字体。
pub fn weather_symbol(code: u32) -> &'static str {
    match code {
        0 | 1 => "☀",
        2 | 3 => "☁",
        45 | 48 => "☁",
        51..=57 => "☂",
        61..=67 => "☂",
        71..=77 => "❄",
        80..=82 => "☂",
        85 | 86 => "❄",
        95..=99 => "⚡",
        _ => "☁",
    }
}

#[derive(Debug, Deserialize)]
struct ForecastResponse {
    latitude: f64,
    longitude: f64,
    current: CurrentBlock,
    daily: DailyBlock,
}

#[derive(Debug, Deserialize)]
struct CurrentBlock {
    time: String,
    temperature_2m: f64,
    weather_code: u32,
}

#[derive(Debug, Deserialize)]
struct DailyBlock {
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

/// 解析 Open-Meteo 预报响应。
pub fn parse_forecast(city: &str, json: &str, cached_at: i64) -> Result<WeatherSnapshot, String> {
    let response: ForecastResponse =
        serde_json::from_str(json).map_err(|e| format!("天气数据解析失败: {e}"))?;
    let max = response
        .daily
        .temperature_2m_max
        .first()
        .copied()
        .ok_or_else(|| "天气数据缺少最高温".to_string())?;
    let min = response
        .daily
        .temperature_2m_min
        .first()
        .copied()
        .ok_or_else(|| "天气数据缺少最低温".to_string())?;

    Ok(WeatherSnapshot {
        city: city.to_string(),
        latitude: response.latitude,
        longitude: response.longitude,
        temperature: response.current.temperature_2m,
        weather_code: response.current.weather_code,
        max,
        min,
        observed_at: response.current.time,
        cached_at,
    })
}

#[derive(Debug, Deserialize)]
struct GeocodeResponse {
    results: Option<Vec<GeocodeResult>>,
}

#[derive(Debug, Deserialize)]
struct GeocodeResult {
    name: String,
    latitude: f64,
    longitude: f64,
}

/// 解析 Open-Meteo 地理编码响应，返回（规范名, 纬度, 经度）。
pub fn parse_geocode(json: &str) -> Result<(String, f64, f64), String> {
    let response: GeocodeResponse =
        serde_json::from_str(json).map_err(|e| format!("城市数据解析失败: {e}"))?;
    let results = response.results.unwrap_or_default();
    let first = results
        .into_iter()
        .next()
        .ok_or_else(|| "没有找到这个城市".to_string())?;
    Ok((first.name, first.latitude, first.longitude))
}

/// 缓存是否还算新鲜（分钟）。
pub fn is_fresh(snapshot: &WeatherSnapshot, now_unix: i64, minutes: i64) -> bool {
    now_unix.saturating_sub(snapshot.cached_at) < minutes * 60
}

/// 把观测时间格式化成 `11:30 更新`。
pub fn updated_text(snapshot: &WeatherSnapshot) -> String {
    snapshot
        .observed_at
        .split('T')
        .nth(1)
        .map(|time| format!("{time} 更新"))
        .unwrap_or_else(|| "—".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORECAST: &str = r#"{
        "latitude": 39.89455,
        "longitude": 116.35983,
        "current": {"time": "2026-11-15T11:30", "interval": 900, "temperature_2m": 10.2, "weather_code": 3},
        "daily": {"time": ["2026-11-15"], "temperature_2m_max": [10.7], "temperature_2m_min": [4.1]}
    }"#;

    const GEOCODE: &str =
        r#"{"results":[{"id":1816670,"name":"北京","latitude":39.9075,"longitude":116.39723}]}"#;

    #[test]
    fn parses_forecast() {
        let snapshot = parse_forecast("北京", FORECAST, 1000).unwrap();
        assert_eq!(snapshot.temperature, 10.2);
        assert_eq!(snapshot.temperature_text(), "10°");
        assert_eq!(snapshot.range_text(), "4° / 11°");
        assert_eq!(snapshot.condition(), "阴");
        assert_eq!(snapshot.symbol(), "☁");
        assert_eq!(updated_text(&snapshot), "11:30 更新");
    }

    #[test]
    fn parses_geocode() {
        let (name, lat, lon) = parse_geocode(GEOCODE).unwrap();
        assert_eq!(name, "北京");
        assert!((lat - 39.9075).abs() < 1e-6);
        assert!((lon - 116.39723).abs() < 1e-6);
    }

    #[test]
    fn empty_geocode_is_an_error() {
        let error = parse_geocode(r#"{"results":[]}"#).unwrap_err();
        assert!(error.contains("没有找到"));
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(parse_forecast("北京", "not json", 0).is_err());
    }

    #[test]
    fn freshness_window() {
        let snapshot = parse_forecast("北京", FORECAST, 1_000).unwrap();
        assert!(is_fresh(&snapshot, 1_000 + 60, 15));
        assert!(!is_fresh(&snapshot, 1_000 + 15 * 60 + 1, 15));
    }

    #[test]
    fn wmo_codes_have_text_and_symbol() {
        for code in [0, 2, 3, 45, 61, 71, 80, 95, 99, 123] {
            assert!(!weather_text(code).is_empty());
            assert!(!weather_symbol(code).is_empty());
        }
        assert_eq!(weather_text(0), "晴");
        assert_eq!(weather_text(95), "雷阵雨");
    }
}
