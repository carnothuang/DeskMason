//! 按 freedesktop 规范把文件移入回收站。
//!
//! 不依赖 `gio` 进程：直接写 `~/.local/share/Trash/{files,info}`，这样行为可测、
//! 错误可分类，也便于以后做"精确恢复"。

use crate::error::{PlatformError, PlatformResult};
use crate::files::move_path;
use crate::paths;
use std::fs;
use std::path::{Path, PathBuf};

pub fn move_to_trash(path: &Path) -> PlatformResult<PathBuf> {
    move_to_trash_in(&paths::trash_dir(), path)
}

/// 指定回收站根目录的版本，测试用。
pub fn move_to_trash_in(trash_root: &Path, path: &Path) -> PlatformResult<PathBuf> {
    let metadata = fs::symlink_metadata(path).map_err(PlatformError::Io)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| PlatformError::InvalidPath(path.display().to_string()))?;

    let files_dir = trash_root.join("files");
    let info_dir = trash_root.join("info");
    fs::create_dir_all(&files_dir).map_err(PlatformError::Io)?;
    fs::create_dir_all(&info_dir).map_err(PlatformError::Io)?;

    let (stored_name, target) = unique_target(&files_dir, &name);
    move_path(path, &target, metadata.is_dir())?;

    // 先落 info，再算成功：没有 info 的文件在回收站里是"孤儿"，恢复工具看不到。
    let info_path = info_dir.join(format!("{stored_name}.trashinfo"));
    let info = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        percent_encode(&path.to_string_lossy()),
        utc_timestamp_now()
    );
    if let Err(error) = fs::write(&info_path, info) {
        // 回滚，避免留下无法恢复的文件。
        let _ = fs::rename(&target, path);
        return Err(PlatformError::Io(error));
    }

    Ok(target)
}

fn unique_target(files_dir: &Path, name: &str) -> (String, PathBuf) {
    let mut candidate = name.to_string();
    let mut counter = 1u32;
    loop {
        let target = files_dir.join(&candidate);
        let info = files_dir
            .parent()
            .unwrap_or(files_dir)
            .join("info")
            .join(format!("{candidate}.trashinfo"));
        if !target.exists() && !info.exists() {
            return (candidate, target);
        }
        candidate = format!("{name}.{counter}");
        counter += 1;
    }
}

fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// RFC 3339（UTC）。回收站规范只要求时间可解析，不做本地时区换算。
pub fn utc_timestamp_now() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_timestamp(seconds)
}

pub fn format_timestamp(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let secs_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant 的 civil_from_days。
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

    fn temp_root(tag: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "deskmason-trash-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn moves_file_and_writes_trashinfo() {
        let root = temp_root("file");
        let work = root.join("work");
        fs::create_dir_all(&work).unwrap();
        let file = work.join("report (draft).pdf");
        fs::write(&file, b"hello").unwrap();

        let trash = root.join("Trash");
        let stored = move_to_trash_in(&trash, &file).unwrap();

        assert!(!file.exists());
        assert_eq!(fs::read(&stored).unwrap(), b"hello");

        let info = fs::read_to_string(trash.join("info/report (draft).pdf.trashinfo")).unwrap();
        assert!(info.contains("[Trash Info]"));
        assert!(info.contains("Path="));
        assert!(info.contains("%20%28draft%29"));
        assert!(info.contains("DeletionDate="));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_names_get_suffixes() {
        let root = temp_root("dup");
        let work = root.join("work");
        fs::create_dir_all(&work).unwrap();
        let trash = root.join("Trash");

        for _ in 0..2 {
            let file = work.join("a.txt");
            fs::write(&file, b"x").unwrap();
            move_to_trash_in(&trash, &file).unwrap();
        }

        assert!(trash.join("files/a.txt").exists());
        assert!(trash.join("files/a.txt.1").exists());
        assert!(trash.join("info/a.txt.1.trashinfo").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn moves_directory() {
        let root = temp_root("dir");
        let dir = root.join("work/target");
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("nested/file.txt"), b"x").unwrap();

        let trash = root.join("Trash");
        let stored = move_to_trash_in(&trash, &dir).unwrap();
        assert!(!dir.exists());
        assert!(stored.join("nested/file.txt").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn timestamp_uses_rfc3339_shape() {
        // 2024-02-29T12:34:56Z
        assert_eq!(format_timestamp(1_709_210_096), "2024-02-29T12:34:56Z");
    }

    #[test]
    fn percent_encoding_keeps_slashes() {
        assert_eq!(percent_encode("/tmp/a b"), "/tmp/a%20b");
    }
}
