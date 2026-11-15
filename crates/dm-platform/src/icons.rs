//! XDG 图标主题查找。
//!
//! 只要"给定图标名，返回一个能直接给 Qt 用的绝对路径"。
//!
//! Deepin 的 bloom 主题用的是 `mimetypes/128/name.svg`，而 freedesktop 规范里
//! 常见的是 `128x128/mimetypes/name.svg`，两种都要支持（这是实测踩过的坑）。

use crate::paths;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// 主题优先级。DDE 默认 bloom（含深色/经典变体），其余按常见发行版兜底。
const THEME_PRIORITY: &[&str] = &[
    "bloom-dark",
    "bloom",
    "bloom-classic-dark",
    "bloom-classic",
    "hicolor",
    "Adwaita",
    "Adwaita-dark",
    "breeze",
];

/// 位图尺寸优先级：越大越先。
const SIZES: &[&str] = &["512", "256", "128", "96", "64", "48", "32", "24", "16"];

const EXTENSIONS: &[&str] = &["svg", "png", "xpm"];

/// 找不到专用图标时的通用回退链。
const FALLBACKS: &[&str] = &["text-x-generic", "application-x-generic", "unknown"];

pub fn resolve_icon(name: &str) -> Option<PathBuf> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<PathBuf>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(map) = cache.lock() {
        if let Some(hit) = map.get(name) {
            return hit.clone();
        }
    }

    let root_dirs = paths::icon_roots();
    let theme_dirs = collect_theme_dirs(&root_dirs);
    let found = resolve_icon_in(&theme_dirs, name);

    if let Ok(mut map) = cache.lock() {
        map.insert(name.to_string(), found.clone());
    }
    found
}

/// 找不到就逐级回退，保证界面上至少有个占位图标。
pub fn resolve_icon_with_fallback(name: &str) -> Option<PathBuf> {
    if let Some(found) = resolve_icon(name) {
        return Some(found);
    }
    for candidate in FALLBACKS {
        if let Some(found) = resolve_icon(candidate) {
            return Some(found);
        }
    }
    None
}

/// 在一组"主题根目录"里找图标。抽出来是为了能用临时目录做测试。
pub fn resolve_icon_in(theme_dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    for theme in theme_dirs {
        // 1) 矢量目录
        if let Some(found) = search_dir(&theme.join("scalable"), name, 2) {
            return Some(found);
        }
        // 2) 位图尺寸目录：同时支持 `128x128/` 与 `128/` 两种布局
        for size in SIZES {
            if let Some(found) = search_dir(&theme.join(size), name, 2) {
                return Some(found);
            }
            if let Some(found) = search_dir(&theme.join(format!("{size}x{size}")), name, 2) {
                return Some(found);
            }
        }
        // 3) 主题根下的任意布局（兜底，深度 3）
        if let Some(found) = search_dir(theme, name, 3) {
            return Some(found);
        }
    }
    None
}

/// 把搜索根展开成"主题目录"列表，并按优先级排序。
fn collect_theme_dirs(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        // root 本身可能就是一个主题目录（如 /usr/share/pixmaps）。
        if root.join("index.theme").is_file()
            || root.join("scalable").is_dir()
            || root.join("mimetypes").is_dir()
        {
            out.push(root.clone());
            continue;
        }
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        let mut themes: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        themes.sort_by_key(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            THEME_PRIORITY
                .iter()
                .position(|t| *t == name)
                .unwrap_or(THEME_PRIORITY.len())
        });
        out.extend(themes);
    }
    out
}

/// 在 `base` 下最多 `depth` 层子目录里找 `<name>.<ext>`。
fn search_dir(base: &Path, name: &str, depth: usize) -> Option<PathBuf> {
    for ext in EXTENSIONS {
        let candidate = base.join(format!("{name}.{ext}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(base).ok()?;
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        if let Some(found) = search_dir(&dir, name, depth - 1) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_theme(tag: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "deskmason-icon-{tag}-{}-{}",
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
    fn finds_scalable_svg_first() {
        let theme = temp_theme("scalable");
        let dir = theme.join("scalable/places");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("folder.svg"), b"<svg/>").unwrap();

        let found = resolve_icon_in(std::slice::from_ref(&theme), "folder").unwrap();
        assert!(found.ends_with("scalable/places/folder.svg"));
        let _ = fs::remove_dir_all(&theme);
    }

    #[test]
    fn falls_back_to_png_when_no_svg() {
        let theme = temp_theme("png");
        let dir = theme.join("48x48/mimetypes");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("text-x-generic.png"), b"png").unwrap();

        let found = resolve_icon_in(std::slice::from_ref(&theme), "text-x-generic").unwrap();
        assert!(found.ends_with("48x48/mimetypes/text-x-generic.png"));
        let _ = fs::remove_dir_all(&theme);
    }

    #[test]
    fn supports_bare_size_directories_like_bloom() {
        // bloom 的实际布局：mimetypes/128/application-pdf.svg
        let theme = temp_theme("bloom");
        let dir = theme.join("mimetypes/128");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("application-pdf.svg"), b"<svg/>").unwrap();

        let found = resolve_icon_in(std::slice::from_ref(&theme), "application-pdf").unwrap();
        assert!(found.ends_with("mimetypes/128/application-pdf.svg"));
        let _ = fs::remove_dir_all(&theme);
    }

    #[test]
    fn returns_none_for_unknown_icon() {
        let theme = temp_theme("missing");
        assert!(resolve_icon_in(std::slice::from_ref(&theme), "no-such-icon-name").is_none());
        let _ = fs::remove_dir_all(&theme);
    }

    #[test]
    fn empty_name_is_rejected() {
        assert!(resolve_icon_in(&[PathBuf::from("/usr/share/icons")], "").is_none());
    }

    #[test]
    fn fallback_finds_a_real_icon_on_this_system() {
        // 只有装了主题才成立；在 CI 容器里可能没有图标，跳过。
        if let Some(found) = resolve_icon_with_fallback("definitely-not-a-real-icon-xyz") {
            assert!(found.is_file());
        }
    }
}

#[cfg(test)]
mod host_probe {
    #[test]
    fn resolves_folder_from_any_configured_root() {
        let found = super::resolve_icon_with_fallback("folder");
        println!("folder icon = {:?}", found);
        let generic = super::resolve_icon_with_fallback("application-pdf");
        println!("application-pdf icon = {:?}", generic);
        // 有 bloom 的环境必须能解析；没有的环境回退 unknown/text-x-generic。
        assert!(generic.is_some() || found.is_some());
    }
}
