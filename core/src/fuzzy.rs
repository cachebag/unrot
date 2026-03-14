use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::scanner::BrokenSymlink;

pub fn find_candidates(broken: &BrokenSymlink, search_root: &Path) -> Vec<PathBuf> {
    let target_name = match broken.target.file_name() {
        Some(name) => name.to_os_string(),
        None => return vec![],
    };

    let mut exact = Vec::new();
    let mut partial = Vec::new();

    for entry in WalkDir::new(search_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path() == broken.link {
            continue;
        }

        let name = entry.file_name();

        if name == target_name {
            exact.push(entry.into_path());
        } else if let (Some(n), Some(t)) = (name.to_str(), target_name.to_str())
            && (n.contains(t) || t.contains(n))
        {
            partial.push(entry.into_path());
        }
    }

    exact.extend(partial);
    exact
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let broken = crate::scanner::BrokenSymlink {
            link: link_path.clone(),
            target: "target.txt".into(),
        };

        let candidate = temp.path().join("target.txt");
        fs::write(&candidate, b"new").unwrap();

        let found = super::find_candidates(&broken, temp.path());

        assert!(
            found.iter().any(|p| p.ends_with("target.txt")),
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

        let found = find_candidates(&broken, temp.path());
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

        let found = find_candidates(&broken, temp.path());
        assert!(
            found.iter().any(|p| p.ends_with("config.yml.bak")),
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

        let found = find_candidates(&broken, temp.path());
        assert!(found.len() >= 2, "should find both exact and partial");

        let exact_pos = found.iter().position(|p| p.ends_with("data.json")).unwrap();
        let partial_pos = found
            .iter()
            .position(|p| p.ends_with("data.json.bak"))
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

        let found = find_candidates(&broken, temp.path());
        assert!(
            !found.contains(&link_path),
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

        let found = find_candidates(&broken, temp.path());
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

        let found = find_candidates(&broken, temp.path());
        let exact_count = found.iter().filter(|p| p.ends_with("notes.md")).count();
        assert_eq!(exact_count, 2, "should find both exact matches");
    }
}
