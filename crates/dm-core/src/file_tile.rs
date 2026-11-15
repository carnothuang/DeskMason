//! 文件瓦片的领域规则：目录扫描、排序、图标名映射。
//!
//! 这里不碰 inotify、不碰窗口；纯函数 + 纯数据，便于测试。

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl FileEntry {
    pub fn icon_name(&self) -> &'static str {
        if self.is_dir {
            "folder"
        } else {
            icon_name_for_file(&self.name)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FileSortMode {
    /// 自然序（`file2` 在 `file10` 之前），目录在前。
    #[default]
    Name,
    Modified,
    Size,
    Kind,
}

/// 扫描一个目录。`include_hidden` 为 false 时跳过 `.` 开头的条目。
///
/// 目录永远排在文件前面，这是文件管理器的通行做法，也让"新建文件夹"落在显眼位置。
pub fn scan_dir(
    dir: &Path,
    sort: FileSortMode,
    include_hidden: bool,
) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    for item in fs::read_dir(dir)? {
        let item = match item {
            Ok(item) => item,
            // 单个条目读不到（权限、竞态删除）不应让整个瓦片失败。
            Err(_) => continue,
        };
        let name = item.file_name().to_string_lossy().to_string();
        if !include_hidden && name.starts_with('.') {
            continue;
        }
        let path = item.path();
        let metadata = match item.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        entries.push(FileEntry {
            name,
            path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified: metadata.modified().ok(),
        });
    }
    sort_entries(&mut entries, sort);
    Ok(entries)
}

pub fn sort_entries(entries: &mut [FileEntry], sort: FileSortMode) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| match sort {
                FileSortMode::Name | FileSortMode::Kind => natural_cmp(&a.name, &b.name),
                FileSortMode::Modified => b.modified.cmp(&a.modified),
                FileSortMode::Size => b.size.cmp(&a.size),
            })
            // 同键时用名字兜底，保证顺序稳定，避免每次刷新跳来跳去。
            .then_with(|| natural_cmp(&a.name, &b.name))
    });
}

/// 数字感知比较：`img2` < `img10`。
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ac), Some(bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let an = take_number(&mut ai);
                    let bn = take_number(&mut bi);
                    match an.cmp(&bn) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
                match ac.to_lowercase().cmp(bc.to_lowercase()) {
                    Ordering::Equal => {
                        ai.next();
                        bi.next();
                    }
                    other => return other,
                }
            }
        }
    }
}

fn take_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u128 {
    let mut value: u128 = 0;
    while let Some(c) = chars.peek().copied() {
        if let Some(d) = c.to_digit(10) {
            // 溢出时饱和即可：排序不需要精确值。
            value = value.saturating_mul(10).saturating_add(d as u128);
            chars.next();
        } else {
            break;
        }
    }
    value
}

/// 按扩展名给出图标名（XDG icon naming spec）。
pub fn icon_name_for_file(name: &str) -> &'static str {
    let ext = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "pdf" => "application-pdf",
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" | "heic" | "avif" => {
            "image-x-generic"
        }
        "mp3" | "flac" | "wav" | "ogg" | "m4a" | "opus" => "audio-x-generic",
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "video-x-generic",
        "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" | "deb" | "rpm" | "AppImage" => {
            "package-x-generic"
        }
        "desktop" => "application-x-desktop",
        "txt" | "md" | "log" | "json" | "toml" | "yaml" | "yml" | "rs" | "c" | "h" | "cpp"
        | "py" | "js" | "ts" | "sh" | "css" | "html" | "xml" | "ini" | "conf" => "text-x-generic",
        _ => "text-x-generic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "deskmason-test-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn natural_order_sorts_numbers_by_value() {
        assert_eq!(natural_cmp("img2", "img10"), Ordering::Less);
        assert_eq!(natural_cmp("img10", "img9"), Ordering::Greater);
        assert_eq!(natural_cmp("IMG2", "img2"), Ordering::Equal);
        assert_eq!(natural_cmp("a", "ab"), Ordering::Less);
    }

    #[test]
    fn scan_puts_directories_first_and_skips_hidden() {
        let temp = TempDir::new("scan");
        std::fs::create_dir(temp.path().join("zfolder")).unwrap();
        std::fs::write(temp.path().join("b.txt"), b"x").unwrap();
        std::fs::write(temp.path().join("a10.txt"), b"x").unwrap();
        std::fs::write(temp.path().join("a2.txt"), b"x").unwrap();
        std::fs::write(temp.path().join(".hidden"), b"x").unwrap();

        let entries = scan_dir(temp.path(), FileSortMode::Name, false).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["zfolder", "a2.txt", "a10.txt", "b.txt"]);
        assert!(entries[0].is_dir);
    }

    #[test]
    fn scan_can_include_hidden() {
        let temp = TempDir::new("hidden");
        std::fs::write(temp.path().join(".env"), b"x").unwrap();
        let entries = scan_dir(temp.path(), FileSortMode::Name, true).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn size_sort_is_descending_with_dirs_first() {
        let temp = TempDir::new("size");
        std::fs::create_dir(temp.path().join("dir")).unwrap();
        let mut small = std::fs::File::create(temp.path().join("small.bin")).unwrap();
        small.write_all(&[0; 10]).unwrap();
        let mut large = std::fs::File::create(temp.path().join("large.bin")).unwrap();
        large.write_all(&[0; 1000]).unwrap();

        let entries = scan_dir(temp.path(), FileSortMode::Size, false).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["dir", "large.bin", "small.bin"]);
    }

    #[test]
    fn icon_names_follow_extension() {
        assert_eq!(icon_name_for_file("a.pdf"), "application-pdf");
        assert_eq!(icon_name_for_file("a.PNG"), "image-x-generic");
        assert_eq!(icon_name_for_file("a.tar.gz"), "package-x-generic");
        assert_eq!(icon_name_for_file("noext"), "text-x-generic");
    }
}
