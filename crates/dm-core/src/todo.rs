//! 待办的领域规则：任务/步骤模型、筛选、排序、到期文案。

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoStep {
    pub id: String,
    pub title: String,
    pub done: bool,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoTask {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub done: bool,
    /// Unix 秒；None 表示没有截止时间。
    pub due_at: Option<i64>,
    pub color: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub steps: Vec<TodoStep>,
}

impl TodoTask {
    pub fn open_steps(&self) -> usize {
        self.steps.iter().filter(|s| !s.done).count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TodoFilter {
    #[default]
    All,
    Open,
    Done,
}

impl TodoFilter {
    pub fn parse(text: &str) -> Self {
        match text {
            "open" => TodoFilter::Open,
            "done" => TodoFilter::Done,
            _ => TodoFilter::All,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TodoFilter::All => "all",
            TodoFilter::Open => "open",
            TodoFilter::Done => "done",
        }
    }

    pub fn matches(self, task: &TodoTask) -> bool {
        match self {
            TodoFilter::All => true,
            TodoFilter::Open => !task.done,
            TodoFilter::Done => task.done,
        }
    }
}

/// 未完成在前；有截止时间的按时间升序；同键按创建时间倒序。
pub fn sort_tasks(tasks: &mut [TodoTask]) {
    tasks.sort_by(|a, b| {
        a.done
            .cmp(&b.done)
            .then_with(|| match (a.due_at, b.due_at) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| b.created_at.cmp(&a.created_at))
    });
}

/// 到期文案：全部基于日期差，不依赖具体时区，便于测试。
pub fn due_text_between(due: NaiveDate, today: NaiveDate) -> String {
    let days = (due - today).num_days();
    match days {
        0 => "今天".to_string(),
        1 => "明天".to_string(),
        2 => "后天".to_string(),
        -1 => "昨天".to_string(),
        d if d < 0 => format!("逾期 {} 天", -d),
        d if d <= 7 => format!("{d} 天后"),
        _ => format!("{}月{}日", due.month(), due.day()),
    }
}

pub fn due_date_from_unix(due_at: i64) -> Option<NaiveDate> {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(due_at, 0)
        .single()
        .map(|dt| dt.date_naive())
}

pub fn due_text(due_at: Option<i64>, today: NaiveDate) -> String {
    let Some(due) = due_at.and_then(due_date_from_unix) else {
        return String::new();
    };
    due_text_between(due, today)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn task(id: &str, done: bool, due_at: Option<i64>, created_at: i64) -> TodoTask {
        TodoTask {
            id: id.to_string(),
            title: id.to_string(),
            notes: String::new(),
            done,
            due_at,
            color: String::new(),
            created_at,
            updated_at: created_at,
            steps: Vec::new(),
        }
    }

    #[test]
    fn filters_match_completion() {
        let open = task("a", false, None, 0);
        let done = task("b", true, None, 0);
        assert!(TodoFilter::All.matches(&open) && TodoFilter::All.matches(&done));
        assert!(TodoFilter::Open.matches(&open) && !TodoFilter::Open.matches(&done));
        assert!(!TodoFilter::Done.matches(&open) && TodoFilter::Done.matches(&done));
    }

    #[test]
    fn sorting_puts_open_and_due_first() {
        let mut tasks = vec![
            task("done", true, Some(100), 1),
            task("no-due", false, None, 5),
            task("later", false, Some(200), 2),
            task("sooner", false, Some(100), 3),
        ];
        sort_tasks(&mut tasks);
        let ids: Vec<_> = tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["sooner", "later", "no-due", "done"]);
    }

    #[test]
    fn due_text_covers_near_and_far_dates() {
        let today = date(2026, 11, 15);
        assert_eq!(due_text_between(today, today), "今天");
        assert_eq!(due_text_between(date(2026, 11, 16), today), "明天");
        assert_eq!(due_text_between(date(2026, 11, 17), today), "后天");
        assert_eq!(due_text_between(date(2026, 11, 14), today), "昨天");
        assert_eq!(due_text_between(date(2026, 11, 10), today), "逾期 5 天");
        assert_eq!(due_text_between(date(2026, 11, 20), today), "5 天后");
        assert_eq!(due_text_between(date(2026, 12, 1), today), "12月1日");
    }

    #[test]
    fn missing_due_has_empty_text() {
        assert_eq!(due_text(None, date(2026, 11, 15)), "");
    }

    #[test]
    fn open_step_count() {
        let mut t = task("a", false, None, 0);
        t.steps = vec![
            TodoStep {
                id: "1".into(),
                title: "x".into(),
                done: false,
                position: 0,
            },
            TodoStep {
                id: "2".into(),
                title: "y".into(),
                done: true,
                position: 1,
            },
        ];
        assert_eq!(t.open_steps(), 1);
    }
}
