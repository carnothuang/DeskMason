//! 文件移动原语：跨设备安全、冲突自动改名、URL 解析。
//!
//! 拖放与"导入到瓦片"都走这里，保证两处行为一致。

use crate::error::{PlatformError, PlatformResult};
use std::fs;
use std::path::{Path, PathBuf};

/// 把 `source` 移进 `dest_dir`，返回落点。
///
/// - 已经在目标目录里：原样返回，不算失败。
/// - 同名冲突：`a.txt` → `a (1).txt`。
/// - 跨设备：复制后删除源文件；删除失败时保留源、返回错误。
pub fn move_into(source: &Path, dest_dir: &Path) -> PlatformResult<PathBuf> {
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| PlatformError::InvalidPath(source.display().to_string()))?;

    if source.parent() == Some(dest_dir) {
        return Ok(source.to_path_buf());
    }

    fs::create_dir_all(dest_dir).map_err(PlatformError::Io)?;
    let target = unique_target(dest_dir, &name);
    let metadata = fs::symlink_metadata(source).map_err(PlatformError::Io)?;
    move_path(source, &target, metadata.is_dir())?;
    Ok(target)
}

/// 把 `source` 复制进 `dest_dir`，返回落点。同名冲突自动改名。
pub fn copy_into(source: &Path, dest_dir: &Path) -> PlatformResult<PathBuf> {
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| PlatformError::InvalidPath(source.display().to_string()))?;
    fs::create_dir_all(dest_dir).map_err(PlatformError::Io)?;
    let target = unique_target(dest_dir, &name);
    let metadata = fs::symlink_metadata(source).map_err(PlatformError::Io)?;
    if metadata.is_dir() {
        copy_dir_all(source, &target)?;
    } else {
        fs::copy(source, &target).map_err(PlatformError::Io)?;
    }
    Ok(target)
}

/// 同目录下的唯一文件名。
pub fn unique_target(dest_dir: &Path, name: &str) -> PathBuf {
    let candidate = dest_dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = split_name(name);
    for counter in 1..10_000u32 {
        let next = if ext.is_empty() {
            dest_dir.join(format!("{stem} ({counter})"))
        } else {
            dest_dir.join(format!("{stem} ({counter}).{ext}"))
        };
        if !next.exists() {
            return next;
        }
    }
    // 极端情况下（上万个同名）退回带时间戳的名字，保证总能落盘。
    dest_dir.join(format!(
        "{stem}-{}{}",
        std::process::id(),
        if ext.is_empty() {
            String::new()
        } else {
            format!(".{ext}")
        }
    ))
}

fn split_name(name: &str) -> (String, String) {
    match name.rsplit_once('.') {
        // `.bashrc` 这种以点开头的名字不当作有扩展名。
        Some((stem, ext)) if !stem.is_empty() => (stem.to_string(), ext.to_string()),
        _ => (name.to_string(), String::new()),
    }
}

/// 跨设备时 fs::rename 会返回 EXDEV，需要复制 + 删除。
pub fn move_path(source: &Path, target: &Path, is_dir: bool) -> PlatformResult<()> {
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(EXDEV) => {
            if is_dir {
                copy_dir_all(source, target)?;
            } else {
                fs::copy(source, target).map_err(PlatformError::Io)?;
            }
            if is_dir {
                fs::remove_dir_all(source).map_err(PlatformError::Io)?;
            } else {
                fs::remove_file(source).map_err(PlatformError::Io)?;
            }
            Ok(())
        }
        Err(error) => Err(PlatformError::Io(error)),
    }
}

fn copy_dir_all(source: &Path, target: &Path) -> PlatformResult<()> {
    fs::create_dir_all(target).map_err(PlatformError::Io)?;
    for entry in fs::read_dir(source).map_err(PlatformError::Io)? {
        let entry = entry.map_err(PlatformError::Io)?;
        let from = entry.path();
        let to = target.join(entry.file_name());
        if entry.file_type().map_err(PlatformError::Io)?.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(PlatformError::Io)?;
        }
    }
    Ok(())
}

/// Linux 上 EXDEV = 18。
const EXDEV: i32 = 18;

/// `file:///home/a%20b.txt` → `/home/a b.txt`。非 file:// 的直接当作本地路径。
pub fn url_to_path(url: &str) -> Option<PathBuf> {
    let rest = match url.strip_prefix("file://") {
        Some(rest) => rest,
        None => return Some(PathBuf::from(url)),
    };
    // 跳过 host 部分（本机应为空）。
    let path = &rest[rest.find('/')?..];
    Some(PathBuf::from(percent_decode(path)))
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(value);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "deskmason-files-{tag}-{}-{}",
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

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn moves_file_into_directory() {
        let root = TempDir::new("move");
        let src_dir = root.path().join("src");
        let dst_dir = root.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        let file = src_dir.join("a.txt");
        fs::write(&file, b"data").unwrap();

        let moved = move_into(&file, &dst_dir).unwrap();
        assert_eq!(moved, dst_dir.join("a.txt"));
        assert!(!file.exists());
        assert_eq!(fs::read(&moved).unwrap(), b"data");
    }

    #[test]
    fn conflicts_get_numbered_suffix() {
        let root = TempDir::new("conflict");
        let dst = root.path().join("dst");
        fs::create_dir_all(&dst).unwrap();
        fs::write(dst.join("a.txt"), b"existing").unwrap();

        let src = root.path().join("a.txt");
        fs::write(&src, b"new").unwrap();
        let moved = move_into(&src, &dst).unwrap();

        assert_eq!(moved, dst.join("a (1).txt"));
        assert_eq!(fs::read(&moved).unwrap(), b"new");
        assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"existing");
    }

    #[test]
    fn already_in_destination_is_a_no_op() {
        let root = TempDir::new("noop");
        let file = root.path().join("a.txt");
        fs::write(&file, b"x").unwrap();
        let moved = move_into(&file, root.path()).unwrap();
        assert_eq!(moved, file);
        assert!(file.exists());
    }

    #[test]
    fn moves_directory_recursively() {
        let root = TempDir::new("dir");
        let src = root.path().join("src");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("nested/x.txt"), b"x").unwrap();
        let dst = root.path().join("dst");
        fs::create_dir_all(&dst).unwrap();

        let moved = move_into(&src, &dst).unwrap();
        assert!(moved.join("nested/x.txt").exists());
        assert!(!src.exists());
    }

    #[test]
    fn copies_file_and_keeps_source() {
        let root = TempDir::new("copy");
        let src_dir = root.path().join("src");
        let dst_dir = root.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        let file = src_dir.join("a.txt");
        fs::write(&file, b"data").unwrap();

        let copied = copy_into(&file, &dst_dir).unwrap();
        assert_eq!(copied, dst_dir.join("a.txt"));
        assert!(file.exists(), "复制不应删除源文件");
        assert_eq!(fs::read(&copied).unwrap(), b"data");
    }

    #[test]
    fn copies_directory_recursively() {
        let root = TempDir::new("copydir");
        let src = root.path().join("src");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("nested/x.txt"), b"x").unwrap();
        let dst = root.path().join("dst");
        fs::create_dir_all(&dst).unwrap();

        let copied = copy_into(&src, &dst).unwrap();
        assert!(copied.join("nested/x.txt").exists());
        assert!(src.exists());
    }

    #[test]
    fn copy_conflict_gets_suffix() {
        let root = TempDir::new("copyconflict");
        let dst = root.path().join("dst");
        fs::create_dir_all(&dst).unwrap();
        fs::write(dst.join("a.txt"), b"old").unwrap();
        let src = root.path().join("a.txt");
        fs::write(&src, b"new").unwrap();

        let copied = copy_into(&src, &dst).unwrap();
        assert_eq!(copied, dst.join("a (1).txt"));
        assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"old");
    }

    #[test]
    fn parses_file_urls() {
        assert_eq!(
            url_to_path("file:///home/a%20b.txt").unwrap(),
            PathBuf::from("/home/a b.txt")
        );
        assert_eq!(
            url_to_path("file:///tmp/%E4%B8%AD%E6%96%87.txt").unwrap(),
            PathBuf::from("/tmp/中文.txt")
        );
        // 不是 URL 的当作本地路径
        assert_eq!(
            url_to_path("/tmp/plain.txt").unwrap(),
            PathBuf::from("/tmp/plain.txt")
        );
    }

    #[test]
    fn dotfiles_keep_whole_name() {
        assert_eq!(
            split_name(".bashrc"),
            (".bashrc".to_string(), String::new())
        );
        assert_eq!(
            split_name("archive.tar.gz"),
            ("archive.tar".to_string(), "gz".to_string())
        );
    }
}
