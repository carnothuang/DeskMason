//! 本地备份：把配置与数据打成 zip。
//!
//! 备份范围：`widget-layout.json`、`settings.json`、`todo.db`（含随记表）。
//! 恢复只做"解压覆盖"，是否生效由调用方提示用户重启。

use crate::paths;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn backup_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| paths::home_dir().join(".local/share"))
        .join("deskmason")
        .join("backups")
}

/// 需要备份的文件：(归档内的名字, 磁盘路径)。
fn sources(config_dir: &Path, data_dir: &Path) -> Vec<(&'static str, PathBuf)> {
    vec![
        (
            "widget-layout.json",
            config_dir.join("deskmason/widget-layout.json"),
        ),
        ("settings.json", config_dir.join("deskmason/settings.json")),
        ("todo.db", data_dir.join("deskmason/todo.db")),
    ]
}

pub fn create_backup() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| paths::home_dir().join(".config"));
    let data_dir = dirs::data_dir().unwrap_or_else(|| paths::home_dir().join(".local/share"));
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    create_backup_at(&config_dir, &data_dir, &backup_dir(), stamp)
        .map_err(|e| format!("备份失败：{e}"))
}

/// 纯路径版本，便于测试。
pub fn create_backup_at(
    config_dir: &Path,
    data_dir: &Path,
    out_dir: &Path,
    stamp: i64,
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(out_dir)?;

    // SQLite 处于 WAL 模式且进程还占着连接时，主文件可能是空的（数据在 -wal 里）。
    // 用 `VACUUM INTO` 生成一致性快照，而不是直接拷主文件。
    let db_path = data_dir.join("deskmason/todo.db");
    let snapshot = out_dir.join(".todo-snapshot.db");
    let _ = fs::remove_file(&snapshot);
    if db_path.is_file() {
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            let _ = conn.execute("VACUUM INTO ?1", [snapshot.to_string_lossy().to_string()]);
        }
    }

    let name = format!("deskmason-{}.zip", format_stamp(stamp));
    let path = out_dir.join(name);
    let file = fs::File::create(&path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut entries = sources(config_dir, data_dir);
    // 用快照替换原始 todo.db。
    if snapshot.is_file() {
        for entry in entries.iter_mut() {
            if entry.0 == "todo.db" {
                entry.1 = snapshot.clone();
            }
        }
    }

    for (archive_name, source) in entries {
        if !source.is_file() {
            continue;
        }
        zip.start_file(archive_name, options)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut input = fs::File::open(&source)?;
        let mut buffer = Vec::new();
        input.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
    }
    zip.finish()
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let _ = fs::remove_file(&snapshot);
    Ok(path)
}

pub fn list_backups() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(backup_dir()) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("zip"))
        .collect();
    files.sort();
    files.reverse();
    files
}

pub fn restore_backup(path: &Path) -> Result<(), String> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| paths::home_dir().join(".config"));
    let data_dir = dirs::data_dir().unwrap_or_else(|| paths::home_dir().join(".local/share"));
    restore_backup_at(path, &config_dir, &data_dir).map_err(|e| format!("恢复失败：{e}"))
}

pub fn restore_backup_at(path: &Path, config_dir: &Path, data_dir: &Path) -> std::io::Result<()> {
    let file = fs::File::open(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| std::io::Error::other(e.to_string()))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let name = entry
            .name()
            .map_err(|e| std::io::Error::other(e.to_string()))?
            .to_string();
        let target = match name.as_str() {
            "widget-layout.json" => config_dir.join("deskmason/widget-layout.json"),
            "settings.json" => config_dir.join("deskmason/settings.json"),
            "todo.db" => data_dir.join("deskmason/todo.db"),
            _ => continue,
        };
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut output)?;
    }
    Ok(())
}

fn format_stamp(unix: i64) -> String {
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempTree(PathBuf);

    impl TempTree {
        fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "deskmason-backup-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn backup_and_restore_round_trip() {
        let root = TempTree::new("roundtrip");
        let config = root.path().join("config");
        let data = root.path().join("data");
        fs::create_dir_all(config.join("deskmason")).unwrap();
        fs::create_dir_all(data.join("deskmason")).unwrap();
        fs::write(
            config.join("deskmason/widget-layout.json"),
            b"{\"widgets\":[]}",
        )
        .unwrap();
        fs::write(
            config.join("deskmason/settings.json"),
            b"{\"hotkey\":\"a\"}",
        )
        .unwrap();
        fs::write(data.join("deskmason/todo.db"), b"fakedb").unwrap();

        let out = root.path().join("backups");
        let archive = create_backup_at(&config, &data, &out, 1_700_000_000).unwrap();
        assert!(archive.is_file());

        // 破坏原文件后恢复
        fs::write(config.join("deskmason/widget-layout.json"), b"broken").unwrap();
        fs::remove_file(data.join("deskmason/todo.db")).unwrap();
        restore_backup_at(&archive, &config, &data).unwrap();

        assert_eq!(
            fs::read(config.join("deskmason/widget-layout.json")).unwrap(),
            b"{\"widgets\":[]}"
        );
        assert_eq!(fs::read(data.join("deskmason/todo.db")).unwrap(), b"fakedb");
    }

    #[test]
    fn missing_files_are_skipped() {
        let root = TempTree::new("missing");
        let config = root.path().join("config");
        let data = root.path().join("data");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(&data).unwrap();
        let out = root.path().join("backups");
        let archive = create_backup_at(&config, &data, &out, 1_700_000_000).unwrap();
        assert!(archive.is_file(), "没有任何源文件时也应生成空备份");
    }

    #[test]
    fn snapshot_contains_wal_data() {
        let root = TempTree::new("wal");
        let config = root.path().join("config");
        let data = root.path().join("data");
        fs::create_dir_all(config.join("deskmason")).unwrap();
        fs::create_dir_all(data.join("deskmason")).unwrap();

        // 模拟应用运行中：连接保持打开，数据只在 WAL 里。
        let db_path = data.join("deskmason/todo.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('hello');")
            .unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM t", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );

        let out = root.path().join("backups");
        let archive = create_backup_at(&config, &data, &out, 1_700_000_000).unwrap();

        // 解压后必须能查到那条数据。
        let file = std::fs::File::open(&archive).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut entry = zip.by_name("todo.db").unwrap();
        let mut buffer = Vec::new();
        entry.read_to_end(&mut buffer).unwrap();
        let restored = root.path().join("restored.db");
        fs::write(&restored, &buffer).unwrap();
        let restored_conn = rusqlite::Connection::open(&restored).unwrap();
        let count: i64 = restored_conn
            .query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "备份里的数据库必须包含 WAL 中已提交的数据");
    }

    #[test]
    fn stamp_is_readable() {
        assert_eq!(format_stamp(1_700_000_000), "20231114-221320");
    }
}
