//! 调起外部程序。Linux 没有 ShellExecute，统一走桌面约定。

use crate::error::{PlatformError, PlatformResult};
use std::path::Path;
use std::process::{Command, Stdio};

/// 用默认应用打开文件或目录。
pub fn open_path(path: &Path) -> PlatformResult<()> {
    spawn("xdg-open", &[path.as_os_str()])
}

/// 在文件管理器里显示。Deepin 用 `dde-file-manager`，缺失时退回 `xdg-open` 父目录。
pub fn reveal_path(path: &Path) -> PlatformResult<()> {
    let parent = path.parent().unwrap_or(path);
    if which("dde-file-manager") {
        return spawn("dde-file-manager", &[parent.as_os_str()]);
    }
    spawn("xdg-open", &[parent.as_os_str()])
}

fn spawn(program: &str, args: &[&std::ffi::OsStr]) -> PlatformResult<()> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| PlatformError::Spawn(format!("{program}: {e}")))
}

fn which(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_finds_common_programs() {
        // 任何 Linux 桌面都应该有 sh；这条测试只验证 PATH 扫描逻辑。
        assert!(which("sh"));
        assert!(!which("definitely-not-a-real-program-xyz"));
    }

    #[test]
    fn revealing_a_missing_path_still_returns_a_result() {
        // 只验证不会 panic；外部程序是否真的存在由运行时决定。
        let _ = reveal_path(Path::new("/tmp/nonexistent-deskmason-probe"));
    }
}
