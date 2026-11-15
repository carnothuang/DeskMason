//! 目录监听（inotify）。

use crate::error::PlatformResult;
use notify::{RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};

/// 非递归监听一个目录。事件只用来置"脏"，具体怎么刷新由上层决定。
pub struct DirWatcher {
    _watcher: notify::RecommendedWatcher,
    rx: Receiver<()>,
}

impl DirWatcher {
    pub fn new(dir: &Path) -> PlatformResult<Self> {
        let (tx, rx) = channel();
        let mut watcher =
            notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
                if result.is_ok() {
                    // 接收端可能已经丢弃，忽略发送错误。
                    let _ = tx.send(());
                }
            })?;
        watcher.watch(dir, RecursiveMode::NonRecursive)?;
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// 取出并清空所有待处理事件；返回是否发生过变化。
    pub fn take_dirty(&self) -> bool {
        let mut dirty = false;
        while self.rx.try_recv().is_ok() {
            dirty = true;
        }
        dirty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn detects_file_creation() {
        let dir = std::env::temp_dir().join(format!(
            "deskmason-watch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let watcher = DirWatcher::new(&dir).unwrap();
        std::fs::write(dir.join("new.txt"), b"x").unwrap();

        // inotify 是异步的，给它一点时间；最多等 2 秒。
        let started = Instant::now();
        let mut seen = false;
        while started.elapsed() < Duration::from_secs(2) {
            if watcher.take_dirty() {
                seen = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        let _ = std::fs::remove_dir_all(&dir);
        assert!(seen, "创建文件后 2 秒内没有收到任何事件");
    }
}
