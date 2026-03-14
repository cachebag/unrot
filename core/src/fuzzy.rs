use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

use crate::scanner::BrokenSymlink;

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];

    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

fn dir_components(path: &Path) -> Vec<String> {
    path.parent()
        .unwrap_or(Path::new(""))
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str().map(String::from),
            _ => None,
        })
        .collect()
}

pub fn score_candidate(broken: &BrokenSymlink, candidate: &Path, search_root: &Path) -> f64 {
    let target_name = broken
        .target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let cand_name = candidate.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let edit_dist = levenshtein(target_name, cand_name);
    let max_len = target_name
        .chars()
        .count()
        .max(cand_name.chars().count())
        .max(1);
    let filename_score = edit_dist as f64 / max_len as f64;

    let target_dirs = dir_components(&broken.target);
    let cand_dirs = dir_components(candidate);
    let shared = target_dirs
        .iter()
        .filter(|d| cand_dirs.iter().any(|cd| cd == *d))
        .count();
    let max_dirs = target_dirs.len().max(cand_dirs.len()).max(1);
    let path_score = 1.0 - (shared as f64 / max_dirs as f64);

    let depth = candidate
        .strip_prefix(search_root)
        .map(|p| p.components().count())
        .unwrap_or(0);
    let depth_penalty = depth as f64 * 0.1;

    filename_score * 10.0 + path_score * 3.0 + depth_penalty
}

pub fn find_candidates(
    broken: &BrokenSymlink,
    search_root: &Path,
    ignore: &[String],
) -> Vec<ScoredCandidate> {
    let target_name = match broken.target.file_name() {
        Some(name) => name.to_os_string(),
        None => return vec![],
    };
    let target_str = target_name.to_str().unwrap_or("");

    let mut candidates: Vec<ScoredCandidate> = WalkDir::new(search_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_str().unwrap_or("");
                !ignore.iter().any(|pat| name == pat.as_str())
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            if e.path() == broken.link {
                return false;
            }
            let name = e.file_name();
            if name == target_name {
                return true;
            }
            if let (Some(n), Some(t)) = (name.to_str(), Some(target_str)) {
                if n.contains(t) || t.contains(n) {
                    return true;
                }
                let threshold = 3.max(t.chars().count() / 3);
                levenshtein(n, t) <= threshold
            } else {
                false
            }
        })
        .map(|e| {
            let path = e.into_path();
            let score = score_candidate(broken, &path, search_root);
            ScoredCandidate { path, score }
        })
        .collect();

    candidates.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates
}

#[derive(Debug)]
pub struct ScoredCandidate {
    pub path: PathBuf,
    pub score: f64,
}

pub const DEFAULT_IGNORE: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".hg",
    ".svn",
    "__pycache__",
    ".DS_Store",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein("hello", "hello"), 0);
    }

    #[test]
    fn levenshtein_empty() {
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("", ""), 0);
    }

    #[test]
    fn levenshtein_single_edit() {
        assert_eq!(levenshtein("cat", "bat"), 1);
        assert_eq!(levenshtein("cats", "cat"), 1);
        assert_eq!(levenshtein("cat", "at"), 1);
    }

    #[test]
    fn levenshtein_multiple_edits() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("saturday", "sunday"), 3);
    }

    #[test]
    fn exact_match_scores_lowest() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("app.yml"), b"exact").unwrap();
        fs::write(temp.path().join("app.yml.bak"), b"partial").unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: "app.yml".into(),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        assert!(found.len() >= 2);
        assert!(
            found[0].path.ends_with("app.yml"),
            "exact match should be first (lowest score)"
        );
        assert!(found[0].score < found[1].score);
    }

    #[test]
    fn exact_match_found() {
        use std::fs;
        use std::os::unix::fs::symlink;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let real_file = temp.path().join("target.txt");
        fs::write(&real_file, b"hello").unwrap();
        let link_path = temp.path().join("my_link");
        symlink("target.txt", &link_path).unwrap();

        fs::remove_file(&real_file).unwrap();

        let broken = BrokenSymlink {
            link: link_path.clone(),
            target: "target.txt".into(),
        };

        let candidate = temp.path().join("target.txt");
        fs::write(&candidate, b"new").unwrap();

        let found = find_candidates(&broken, temp.path(), &[]);

        assert!(
            found.iter().any(|sc| sc.path.ends_with("target.txt")),
            "should find the new candidate with the same name"
        );
    }

    #[test]
    fn no_candidates_when_nothing_matches() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("unrelated.txt"), b"data").unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: "gone.txt".into(),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        assert!(found.is_empty());
    }

    #[test]
    fn partial_match_found() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("config.yml.bak"), b"data").unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: "config.yml".into(),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        assert!(
            found.iter().any(|sc| sc.path.ends_with("config.yml.bak")),
            "should find partial match"
        );
    }

    #[test]
    fn exact_before_partial() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let sub = temp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("data.json"), b"exact").unwrap();
        fs::write(temp.path().join("data.json.bak"), b"partial").unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: "data.json".into(),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        assert!(found.len() >= 2, "should find both exact and partial");

        let exact_pos = found
            .iter()
            .position(|sc| sc.path.ends_with("data.json"))
            .unwrap();
        let partial_pos = found
            .iter()
            .position(|sc| sc.path.ends_with("data.json.bak"))
            .unwrap();
        assert!(
            exact_pos < partial_pos,
            "exact matches should come before partial"
        );
    }

    #[test]
    fn skips_the_broken_link_itself() {
        use std::fs;
        use std::os::unix::fs::symlink;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let link_path = temp.path().join("config.yml");
        symlink("/nonexistent/config.yml", &link_path).unwrap();
        fs::write(temp.path().join("sub_config.yml"), b"other").unwrap();

        let broken = BrokenSymlink {
            link: link_path.clone(),
            target: "config.yml".into(),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        assert!(
            !found.iter().any(|sc| sc.path == link_path),
            "should not suggest the broken link itself"
        );
    }

    #[test]
    fn no_filename_returns_empty() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: PathBuf::new(),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        assert!(found.is_empty());
    }

    #[test]
    fn multiple_exact_matches() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        fs::write(a.join("notes.md"), b"one").unwrap();
        fs::write(b.join("notes.md"), b"two").unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: "notes.md".into(),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        let exact_count = found
            .iter()
            .filter(|sc| sc.path.ends_with("notes.md"))
            .count();
        assert_eq!(exact_count, 2, "should find both exact matches");
    }

    #[test]
    fn ignore_patterns_skip_directories() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let git = temp.path().join(".git");
        let nm = temp.path().join("node_modules");
        let src = temp.path().join("src");
        fs::create_dir(&git).unwrap();
        fs::create_dir(&nm).unwrap();
        fs::create_dir(&src).unwrap();
        fs::write(git.join("config.yml"), b"git").unwrap();
        fs::write(nm.join("config.yml"), b"nm").unwrap();
        fs::write(src.join("config.yml"), b"src").unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: "config.yml".into(),
        };

        let ignore = vec![".git".to_string(), "node_modules".to_string()];
        let found = find_candidates(&broken, temp.path(), &ignore);

        assert!(
            found.iter().any(|sc| sc.path.ends_with("src/config.yml")),
            "should find candidate in src"
        );
        assert!(
            !found
                .iter()
                .any(|sc| sc.path.to_str().unwrap().contains(".git")),
            "should skip .git"
        );
        assert!(
            !found
                .iter()
                .any(|sc| sc.path.to_str().unwrap().contains("node_modules")),
            "should skip node_modules"
        );
    }

    #[test]
    fn near_match_via_levenshtein() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("config.yaml"), b"data").unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: "config.yml".into(),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        assert!(
            found.iter().any(|sc| sc.path.ends_with("config.yaml")),
            "should find near-match via Levenshtein"
        );
    }

    #[test]
    fn deeper_candidate_scores_higher() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("app.yml"), b"shallow").unwrap();
        let deep = temp.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("app.yml"), b"deep").unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: "app.yml".into(),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        assert!(found.len() >= 2);

        let shallow = found
            .iter()
            .find(|sc| sc.path.parent().unwrap() == temp.path())
            .unwrap();
        let deep_cand = found
            .iter()
            .find(|sc| sc.path.components().count() > shallow.path.components().count())
            .unwrap();
        assert!(
            shallow.score < deep_cand.score,
            "shallower candidate should have lower (better) score"
        );
    }

    #[test]
    fn path_similarity_boosts_score() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let configs = temp.path().join("configs");
        let other = temp.path().join("other");
        fs::create_dir(&configs).unwrap();
        fs::create_dir(&other).unwrap();
        fs::write(configs.join("app.yml"), b"match").unwrap();
        fs::write(other.join("app.yml"), b"no match").unwrap();

        let broken = BrokenSymlink {
            link: temp.path().join("my_link"),
            target: PathBuf::from("configs/app.yml"),
        };

        let found = find_candidates(&broken, temp.path(), &[]);
        let in_configs = found
            .iter()
            .find(|sc| sc.path.to_str().unwrap().contains("configs"))
            .unwrap();
        let in_other = found
            .iter()
            .find(|sc| sc.path.to_str().unwrap().contains("other"))
            .unwrap();
        assert!(
            in_configs.score < in_other.score,
            "candidate sharing path components with target should score better"
        );
    }
}
