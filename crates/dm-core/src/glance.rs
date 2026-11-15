//! 时光瓦片的领域逻辑：时间、日期、星期、农历、节气、节日。
//!
//! 农历换算用 `chinese-lunisolar-calendar`（带 chrono 支持），不自己维护年份表。
//! 节气用通行的近似公式，误差最多一天，对桌面展示足够。

use chinese_lunisolar_calendar::LunisolarDate;
use chrono::{DateTime, Datelike, Local, NaiveDate, Timelike};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlanceSnapshot {
    /// `14:32`
    pub time: String,
    /// `11月15日`
    pub date: String,
    /// `星期日`
    pub weekday: String,
    /// `农历十月初六`（闰月会带"闰"）
    pub lunar: String,
    /// 当天恰好是节气时的名称
    pub term: Option<&'static str>,
    /// 当天是节日时的名称（公历优先，其次农历）
    pub festival: Option<String>,
}

pub fn snapshot_now() -> GlanceSnapshot {
    let now: DateTime<Local> = Local::now();
    snapshot_for(now.date_naive(), now.hour(), now.minute())
}

pub fn snapshot_for(date: NaiveDate, hour: u32, minute: u32) -> GlanceSnapshot {
    let weekday = match date.weekday().num_days_from_monday() {
        0 => "星期一",
        1 => "星期二",
        2 => "星期三",
        3 => "星期四",
        4 => "星期五",
        5 => "星期六",
        _ => "星期日",
    };

    let lunar_parts = lunar_parts(date);

    GlanceSnapshot {
        time: format!("{hour:02}:{minute:02}"),
        date: format!("{}月{}日", date.month(), date.day()),
        weekday: weekday.to_string(),
        lunar: lunar_parts
            .as_ref()
            .map(|(text, _, _)| format!("农历{text}"))
            .unwrap_or_default(),
        term: solar_term(date),
        festival: festival(date, lunar_parts.as_ref()),
    }
}

/// 农历文本与月/日数字（用于节日判断）。
fn lunar_parts(date: NaiveDate) -> Option<(String, u8, u8)> {
    let lunisolar = LunisolarDate::from_date(date).ok()?;
    let raw = lunisolar.to_lunar_month().to_u8_raw();
    let leap = raw >= 100;
    let month = if leap { raw - 100 } else { raw };
    let day = lunisolar.to_lunar_day().to_u8();
    let text = format!(
        "{}{}月{}",
        if leap { "闰" } else { "" },
        month_name(month),
        day_name(day)
    );
    Some((text, month, day))
}

fn month_name(month: u8) -> &'static str {
    match month {
        1 => "正",
        2 => "二",
        3 => "三",
        4 => "四",
        5 => "五",
        6 => "六",
        7 => "七",
        8 => "八",
        9 => "九",
        10 => "十",
        11 => "冬",
        12 => "腊",
        _ => "",
    }
}

fn day_name(day: u8) -> String {
    const DIGITS: [&str; 10] = ["十", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    match day {
        1..=10 => format!("初{}", DIGITS[day as usize]),
        11..=19 => format!("十{}", DIGITS[(day - 10) as usize]),
        20 => "二十".to_string(),
        21..=29 => format!("廿{}", DIGITS[(day - 20) as usize]),
        30 => "三十".to_string(),
        _ => day.to_string(),
    }
}

fn festival(date: NaiveDate, lunar: Option<&(String, u8, u8)>) -> Option<String> {
    let solar = match (date.month(), date.day()) {
        (1, 1) => Some("元旦"),
        (2, 14) => Some("情人节"),
        (3, 8) => Some("妇女节"),
        (5, 1) => Some("劳动节"),
        (6, 1) => Some("儿童节"),
        (9, 10) => Some("教师节"),
        (10, 1) => Some("国庆节"),
        (12, 25) => Some("圣诞节"),
        _ => None,
    };
    if let Some(name) = solar {
        return Some(name.to_string());
    }

    let (_, month, day) = lunar?;
    let name = match (*month, *day) {
        (1, 1) => "春节",
        (1, 15) => "元宵节",
        (5, 5) => "端午节",
        (7, 7) => "七夕",
        (8, 15) => "中秋节",
        (9, 9) => "重阳节",
        (12, 8) => "腊八节",
        (12, 23) => "小年",
        _ => return None,
    };
    Some(name.to_string())
}

/// 24 节气名称，按公历顺序（小寒、大寒、立春……）。
const TERM_NAMES: [&str; 24] = [
    "小寒", "大寒", "立春", "雨水", "惊蛰", "春分", "清明", "谷雨", "立夏", "小满", "芒种", "夏至",
    "小暑", "大暑", "立秋", "处暑", "白露", "秋分", "寒露", "霜降", "立冬", "小雪", "大雪", "冬至",
];

/// 1900 年以来每个节气的分钟偏移（通行的近似表）。
const TERM_MINUTES: [i64; 24] = [
    0, 21208, 42467, 63836, 85337, 107014, 128867, 150921, 173149, 195551, 218072, 240693, 263343,
    285989, 308563, 331033, 353350, 375494, 397447, 419210, 440795, 462224, 483532, 504758,
];

pub fn solar_term(date: NaiveDate) -> Option<&'static str> {
    let month_index = (date.month() as usize).saturating_sub(1);
    for slot in [month_index * 2, month_index * 2 + 1] {
        if slot >= TERM_MINUTES.len() {
            continue;
        }
        if term_day(date.year(), slot) == date.day() {
            return Some(TERM_NAMES[slot]);
        }
    }
    None
}

fn term_day(year: i32, slot: usize) -> u32 {
    // 基准：1900-01-06 02:05（北京时间）。
    let base = NaiveDate::from_ymd_opt(1900, 1, 6)
        .expect("基准日期合法")
        .and_hms_opt(2, 5, 0)
        .expect("基准时间合法");
    let minutes = 31_556_925_974.7_f64
        .mul_add(year as f64 - 1900.0, TERM_MINUTES[slot] as f64 * 60_000.0)
        / 60_000.0;
    let moment = base + chrono::Duration::minutes(minutes.round() as i64);
    moment.day()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn weekday_is_correct() {
        // 2026-11-15 是星期日
        let snap = snapshot_for(date(2026, 11, 15), 9, 5);
        assert_eq!(snap.weekday, "星期日");
        assert_eq!(snap.date, "11月15日");
        assert_eq!(snap.time, "09:05");
    }

    #[test]
    fn spring_festival_2024() {
        let snap = snapshot_for(date(2024, 2, 10), 12, 0);
        assert!(snap.lunar.contains("正月初一"), "lunar={}", snap.lunar);
        assert_eq!(snap.festival.as_deref(), Some("春节"));
    }

    #[test]
    fn mid_autumn_2024() {
        let snap = snapshot_for(date(2024, 9, 17), 20, 0);
        assert!(snap.lunar.contains("八月十五"), "lunar={}", snap.lunar);
        assert_eq!(snap.festival.as_deref(), Some("中秋节"));
    }

    #[test]
    fn leap_month_is_marked() {
        // 2023 年闰二月
        let snap = snapshot_for(date(2023, 3, 22), 8, 0);
        assert!(snap.lunar.contains("闰"), "lunar={}", snap.lunar);
    }

    #[test]
    fn solar_terms_match_known_dates() {
        assert_eq!(solar_term(date(2024, 2, 4)), Some("立春"));
        assert_eq!(solar_term(date(2024, 12, 21)), Some("冬至"));
        assert_eq!(solar_term(date(2025, 2, 3)), Some("立春"));
        assert_eq!(solar_term(date(2025, 12, 21)), Some("冬至"));
    }

    #[test]
    fn ordinary_day_has_no_term_or_festival() {
        let snap = snapshot_for(date(2026, 11, 15), 10, 0);
        assert_eq!(snap.term, None);
        assert_eq!(snap.festival, None);
    }

    #[test]
    fn solar_festival_takes_priority() {
        let snap = snapshot_for(date(2024, 10, 1), 10, 0);
        assert_eq!(snap.festival.as_deref(), Some("国庆节"));
    }
}
