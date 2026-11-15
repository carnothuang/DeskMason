//! 随记存储：SQLite（与待办同一个数据库文件，独立表）。

use crate::error::PlatformResult;
use crate::todo_store;
use dm_core::quick_capture::{detect_kind, CaptureItem, CaptureKind};
use rusqlite::{params, Connection};
use std::path::Path;

const SCHEMA_VERSION: i64 = 1;

pub struct CaptureStore {
    conn: Connection,
}

impl CaptureStore {
    pub fn open_default() -> PlatformResult<Self> {
        Self::open(&todo_store::default_path())
    }

    pub fn open(path: &Path) -> PlatformResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(crate::error::PlatformError::Io)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> PlatformResult<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS captures (
                 id TEXT PRIMARY KEY,
                 kind TEXT NOT NULL DEFAULT 'text',
                 content TEXT NOT NULL,
                 pinned INTEGER NOT NULL DEFAULT 0,
                 created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_captures_pinned ON captures(pinned);",
        )?;
        // 与待办共用 user_version：待办迁移到 1 之后，这里只需要保证表存在。
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < SCHEMA_VERSION {
            self.conn
                .pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        Ok(())
    }

    pub fn list(&self) -> PlatformResult<Vec<CaptureItem>> {
        let mut statement = self
            .conn
            .prepare("SELECT id, kind, content, pinned, created_at FROM captures")?;
        let rows = statement.query_map([], |row| {
            Ok(CaptureItem {
                id: row.get(0)?,
                kind: CaptureKind::parse(&row.get::<_, String>(1)?),
                content: row.get(2)?,
                pinned: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn create(&self, content: &str, now: i64) -> PlatformResult<String> {
        let id = format!("{now:x}-{}", now % 1000);
        let kind = detect_kind(content);
        self.conn.execute(
            "INSERT INTO captures (id, kind, content, pinned, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
            params![id, kind.as_str(), content, now],
        )?;
        Ok(id)
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> PlatformResult<()> {
        self.conn.execute(
            "UPDATE captures SET pinned = ?2 WHERE id = ?1",
            params![id, pinned as i64],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> PlatformResult<()> {
        self.conn
            .execute("DELETE FROM captures WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDb(PathBuf);

    impl TempDb {
        fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "deskmason-capture-{tag}-{}-{}.db",
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
        }
    }

    #[test]
    fn create_list_pin_delete() {
        let db = TempDb::new("crud");
        let store = CaptureStore::open(db.path()).unwrap();
        let id = store.create("随手记一条", 100).unwrap();
        store.create("https://example.com", 101).unwrap();

        let items = store.list().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.kind == CaptureKind::Link));

        store.set_pinned(&id, true).unwrap();
        assert!(store.list().unwrap().iter().any(|i| i.id == id && i.pinned));

        store.delete(&id).unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
    }

    #[test]
    fn survives_reopen() {
        let db = TempDb::new("reopen");
        {
            let store = CaptureStore::open(db.path()).unwrap();
            store.create("持久化", 100).unwrap();
        }
        let store = CaptureStore::open(db.path()).unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
    }
}
