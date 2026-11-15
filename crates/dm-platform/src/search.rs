//! 文件搜索。
//!
//! 目前的后端是**受限本地遍历**：常见目录 + 深度限制 + 时间预算，够用且不依赖
//! 任何外部索引。Deepin 的 `deepin-anything` 查询库（`deepin-anything-dev`）
//! 在本机还没安装，装好后应在 `search_with` 里优先接入，遍历只作为兜底。

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub roots: Vec<PathBuf>,
    pub max_depth: usize,
    pub max_results: usize,
    pub time_budget: Duration,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            roots: default_roots(),
            max_depth: 6,
            max_results: 120,
            time_budget: Duration::from_millis(350),
        }
    }
}

pub fn default_roots() -> Vec<PathBuf> {
    let home = crate::paths::home_dir();
    let mut roots = Vec::new();
    for name in [
        "Desktop",
        "Documents",
        "Downloads",
        "Pictures",
        "Music",
        "Videos",
    ] {
        let path = home.join(name);
        if path.is_dir() {
            roots.push(path);
        }
    }
    if roots.is_empty() {
        roots.push(home);
    }
    roots
}

pub fn search(query: &str, limit: usize) -> Vec<SearchHit> {
    let config = SearchConfig {
        max_results: limit.max(1),
        ..SearchConfig::default()
    };
    search_with(&config, query)
}

pub fn search_with(config: &SearchConfig, query: &str) -> Vec<SearchHit> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }

    let started = Instant::now();
    let mut hits: Vec<SearchHit> = Vec::new();
    let mut budget_hit = false;

    for root in &config.roots {
        if hits.len() >= config.max_results {
            break;
        }
        walk(
            root,
            0,
            config,
            &needle,
            &started,
            &mut hits,
            &mut budget_hit,
        );
        if budget_hit {
            break;
        }
    }

    rank(&mut hits, &needle);
    hits.truncate(config.max_results);
    hits
}

fn walk(
    dir: &Path,
    depth: usize,
    config: &SearchConfig,
    needle: &str,
    started: &Instant,
    hits: &mut Vec<SearchHit>,
    budget_hit: &mut bool,
) {
    if depth > config.max_depth || hits.len() >= config.max_results {
        return;
    }
    if started.elapsed() > config.time_budget {
        *budget_hit = true;
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if *budget_hit {
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        // 跳过隐藏项与明显的构建产物，避免一次搜索扫穿整个工程目录。
        if name.starts_with('.') || is_noise_dir(&name) {
            continue;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let is_dir = file_type.is_dir();

        if name.to_lowercase().contains(needle) {
            hits.push(SearchHit {
                path: path.clone(),
                name: name.clone(),
                is_dir,
            });
            if hits.len() >= config.max_results {
                return;
            }
        }

        if is_dir && !file_type.is_symlink() {
            walk(&path, depth + 1, config, needle, started, hits, budget_hit);
        }
    }
}

fn is_noise_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | "target" | "build" | "dist" | ".git" | "__pycache__"
    )
}

/// 前缀命中优先，其次名字更短的，最后按路径稳定排序。
fn rank(hits: &mut [SearchHit], needle: &str) {
    hits.sort_by(|a, b| {
        let a_prefix = a.name.to_lowercase().starts_with(needle);
        let b_prefix = b.name.to_lowercase().starts_with(needle);
        b_prefix
            .cmp(&a_prefix)
            .then_with(|| a.name.len().cmp(&b.name.len()))
            .then_with(|| a.path.cmp(&b.path))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempTree(PathBuf);

    impl TempTree {
        fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "deskmason-search-{tag}-{}-{}",
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

    fn config(root: &Path) -> SearchConfig {
        SearchConfig {
            roots: vec![root.to_path_buf()],
            max_depth: 4,
            max_results: 50,
            time_budget: Duration::from_secs(5),
        }
    }

    #[test]
    fn finds_matching_names_case_insensitively() {
        let tree = TempTree::new("basic");
        fs::create_dir_all(tree.path().join("docs")).unwrap();
        fs::write(tree.path().join("docs/Report.md"), b"x").unwrap();
        fs::write(tree.path().join("other.txt"), b"x").unwrap();

        let hits = search_with(&config(tree.path()), "report");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Report.md");
        assert!(!hits[0].is_dir);
    }

    #[test]
    fn skips_hidden_and_noise_directories() {
        let tree = TempTree::new("noise");
        fs::create_dir_all(tree.path().join(".git")).unwrap();
        fs::write(tree.path().join(".git/secret_report"), b"x").unwrap();
        fs::create_dir_all(tree.path().join("target")).unwrap();
        fs::write(tree.path().join("target/report.bin"), b"x").unwrap();

        let hits = search_with(&config(tree.path()), "report");
        assert!(hits.is_empty(), "不应搜到隐藏/构建目录: {hits:?}");
    }

    #[test]
    fn respects_depth_limit() {
        let tree = TempTree::new("depth");
        let mut deep = tree.path().to_path_buf();
        for i in 0..8 {
            deep = deep.join(format!("level{i}"));
        }
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("needle.txt"), b"x").unwrap();

        let mut cfg = config(tree.path());
        cfg.max_depth = 3;
        let hits = search_with(&cfg, "needle");
        assert!(hits.is_empty(), "超过深度的结果不应返回");
    }

    #[test]
    fn ranks_prefix_matches_first() {
        let tree = TempTree::new("rank");
        fs::write(tree.path().join("my_report.txt"), b"x").unwrap();
        fs::write(tree.path().join("report.txt"), b"x").unwrap();

        let hits = search_with(&config(tree.path()), "report");
        assert_eq!(hits[0].name, "report.txt");
    }

    #[test]
    fn finds_chinese_names() {
        let tree = TempTree::new("zh");
        fs::write(tree.path().join("会议记录.txt"), b"x").unwrap();
        let hits = search_with(&config(tree.path()), "会议");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "会议记录.txt");
    }

    #[test]
    fn empty_query_returns_nothing() {
        let tree = TempTree::new("empty");
        assert!(search_with(&config(tree.path()), "   ").is_empty());
    }

    #[test]
    fn result_count_is_capped() {
        let tree = TempTree::new("cap");
        for i in 0..20 {
            fs::write(tree.path().join(format!("hit{i:02}.txt")), b"x").unwrap();
        }
        let mut cfg = config(tree.path());
        cfg.max_results = 5;
        assert_eq!(search_with(&cfg, "hit").len(), 5);
    }
}
