//! 待办存储：SQLite。
//!
//! 表结构只增不改；`schemaVersion` 用 `PRAGMA user_version` 记录，便于以后迁移。

use crate::error::PlatformResult;
use crate::paths;
use dm_core::todo::{TodoStep, TodoTask};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: i64 = 1;

pub fn default_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| paths::home_dir().join(".local/share"))
        .join("deskmason")
        .join("todo.db")
}

pub struct TodoStore {
    conn: Connection,
}

impl TodoStore {
    pub fn open_default() -> PlatformResult<Self> {
        Self::open(&default_path())
    }

    pub fn open(path: &Path) -> PlatformResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(crate::error::PlatformError::Io)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> PlatformResult<()> {
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                 id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 notes TEXT NOT NULL DEFAULT '',
                 done INTEGER NOT NULL DEFAULT 0,
                 due_at INTEGER,
                 color TEXT NOT NULL DEFAULT '',
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS steps (
                 id TEXT PRIMARY KEY,
                 task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                 title TEXT NOT NULL,
                 done INTEGER NOT NULL DEFAULT 0,
                 position INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_tasks_done ON tasks(done);
             CREATE INDEX IF NOT EXISTS idx_steps_task ON steps(task_id);",
        )?;
        self.conn
            .pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(())
    }

    pub fn list(&self) -> PlatformResult<Vec<TodoTask>> {
        let mut statement = self.conn.prepare(
            "SELECT id, title, notes, done, due_at, color, created_at, updated_at FROM tasks",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(TodoTask {
                id: row.get(0)?,
                title: row.get(1)?,
                notes: row.get(2)?,
                done: row.get::<_, i64>(3)? != 0,
                due_at: row.get(4)?,
                color: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                steps: Vec::new(),
            })
        })?;
        let mut tasks: Vec<TodoTask> = rows.collect::<Result<_, _>>()?;
        for task in tasks.iter_mut() {
            task.steps = self.steps(&task.id)?;
        }
        Ok(tasks)
    }

    pub fn steps(&self, task_id: &str) -> PlatformResult<Vec<TodoStep>> {
        let mut statement = self.conn.prepare(
            "SELECT id, title, done, position FROM steps WHERE task_id = ?1 ORDER BY position",
        )?;
        let rows = statement.query_map([task_id], |row| {
            Ok(TodoStep {
                id: row.get(0)?,
                title: row.get(1)?,
                done: row.get::<_, i64>(2)? != 0,
                position: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn create(&self, title: &str, now: i64) -> PlatformResult<String> {
        let id = new_id(now, 0);
        self.conn.execute(
            "INSERT INTO tasks (id, title, notes, done, due_at, color, created_at, updated_at)
             VALUES (?1, ?2, '', 0, NULL, '', ?3, ?3)",
            params![id, title, now],
        )?;
        Ok(id)
    }

    pub fn set_done(&self, id: &str, done: bool, now: i64) -> PlatformResult<()> {
        self.conn.execute(
            "UPDATE tasks SET done = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, done as i64, now],
        )?;
        Ok(())
    }

    pub fn set_title(&self, id: &str, title: &str, now: i64) -> PlatformResult<()> {
        self.conn.execute(
            "UPDATE tasks SET title = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, title, now],
        )?;
        Ok(())
    }

    pub fn set_notes(&self, id: &str, notes: &str, now: i64) -> PlatformResult<()> {
        self.conn.execute(
            "UPDATE tasks SET notes = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, notes, now],
        )?;
        Ok(())
    }

    pub fn set_due(&self, id: &str, due_at: Option<i64>, now: i64) -> PlatformResult<()> {
        self.conn.execute(
            "UPDATE tasks SET due_at = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, due_at, now],
        )?;
        Ok(())
    }

    /// 设定任务颜色。`color` 为 `#RRGGBB`，空串表示清除颜色标记。
    pub fn set_color(&self, id: &str, color: &str, now: i64) -> PlatformResult<()> {
        let value = normalize_color(color);
        self.conn.execute(
            "UPDATE tasks SET color = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, value, now],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> PlatformResult<()> {
        self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn add_step(&self, task_id: &str, title: &str, now: i64) -> PlatformResult<String> {
        let position: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM steps WHERE task_id = ?1",
            [task_id],
            |row| row.get(0),
        )?;
        let id = new_id(now, position as u32);
        self.conn.execute(
            "INSERT INTO steps (id, task_id, title, done, position) VALUES (?1, ?2, ?3, 0, ?4)",
            params![id, task_id, title, position],
        )?;
        Ok(id)
    }

    pub fn set_step_done(&self, step_id: &str, done: bool) -> PlatformResult<()> {
        self.conn.execute(
            "UPDATE steps SET done = ?2 WHERE id = ?1",
            params![step_id, done as i64],
        )?;
        Ok(())
    }

    pub fn delete_step(&self, step_id: &str) -> PlatformResult<()> {
        self.conn
            .execute("DELETE FROM steps WHERE id = ?1", [step_id])?;
        Ok(())
    }
}

/// 只接受 `#RRGGBB` / `RRGGBB`，其余一律视为"无颜色"。
/// QML 会把这个值直接绑到 color 属性，先在这里收窄输入。
fn normalize_color(color: &str) -> String {
    let raw = color.strip_prefix('#').unwrap_or(color);
    if raw.is_empty() {
        return String::new();
    }
    if raw.len() != 6 || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return String::new();
    }
    format!("#{raw}")
}

fn new_id(now: i64, salt: u32) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let tick = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{now:x}-{salt:x}-{tick:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDb(PathBuf);

    impl TempDb {
        fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "deskmason-todo-{tag}-{}-{}.db",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            Self(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_file(self.0.with_extension("db-wal"));
            let _ = std::fs::remove_file(self.0.with_extension("db-shm"));
        }
    }

    #[test]
    fn create_and_list() {
        let db = TempDb::new("create");
        let store = TodoStore::open(db.path()).unwrap();
        let id = store.create("买牛奶", 100).unwrap();
        let tasks = store.list().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, id);
        assert_eq!(tasks[0].title, "买牛奶");
        assert!(!tasks[0].done);
    }

    #[test]
    fn toggle_and_delete() {
        let db = TempDb::new("toggle");
        let store = TodoStore::open(db.path()).unwrap();
        let id = store.create("任务", 100).unwrap();
        store.set_done(&id, true, 101).unwrap();
        assert!(store.list().unwrap()[0].done);
        store.delete(&id).unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn notes_and_due_persist() {
        let db = TempDb::new("notes");
        let store = TodoStore::open(db.path()).unwrap();
        let id = store.create("任务", 100).unwrap();
        store.set_notes(&id, "备注内容", 101).unwrap();
        store.set_due(&id, Some(1_700_000_000), 101).unwrap();
        let task = store.list().unwrap().pop().unwrap();
        assert_eq!(task.notes, "备注内容");
        assert_eq!(task.due_at, Some(1_700_000_000));
    }

    #[test]
    fn color_is_normalized_and_persisted() {
        let db = TempDb::new("color");
        let store = TodoStore::open(db.path()).unwrap();
        let id = store.create("任务", 100).unwrap();
        store.set_color(&id, "A94DC1", 101).unwrap();
        assert_eq!(store.list().unwrap()[0].color, "#A94DC1");
        store.set_color(&id, "#3477E8", 102).unwrap();
        assert_eq!(store.list().unwrap()[0].color, "#3477E8");
        store
            .set_color(&id, "red; background: url(x)", 103)
            .unwrap();
        assert_eq!(store.list().unwrap()[0].color, "");
        assert_eq!(normalize_color("#abc"), "");
        assert_eq!(normalize_color(""), "");
    }

    #[test]
    fn steps_are_ordered_and_cascade_deleted() {
        let db = TempDb::new("steps");
        let store = TodoStore::open(db.path()).unwrap();
        let id = store.create("任务", 100).unwrap();
        store.add_step(&id, "第一步", 101).unwrap();
        store.add_step(&id, "第二步", 102).unwrap();
        let steps = store.steps(&id).unwrap();
        assert_eq!(
            steps.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(),
            ["第一步", "第二步"]
        );

        store.delete(&id).unwrap();
        let count: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM steps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "删除任务应级联删除步骤");
    }

    #[test]
    fn reopening_keeps_data() {
        let db = TempDb::new("reopen");
        {
            let store = TodoStore::open(db.path()).unwrap();
            store.create("持久化", 100).unwrap();
        }
        let store = TodoStore::open(db.path()).unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
    }
}
