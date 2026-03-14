use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Recursively scans the given path for broken symlinks and returns a list of them.
/// This function does not follow symlinks, so it will only report links that are directly broken,
/// not those that are broken due to a parent directory being a broken link.
pub fn find_broken_symlinks(path: &Path, ignore: &[String]) -> Vec<BrokenSymlink> {
    let mut broken_symlinks = Vec::new();

    for entry in WalkDir::new(path)
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
    {
        match entry {
            Ok(dir_entry) => {
                let file_type = dir_entry.file_type();
                if file_type.is_symlink() {
                    let link_path = dir_entry.path().to_path_buf();
                    match fs::read_link(&link_path) {
                        Ok(target_path) => {
                            // This resolves relative targets against the symlinks parent
                            let resolved: PathBuf = if target_path.is_relative() {
                                link_path
                                    .parent()
                                    .unwrap_or(Path::new("."))
                                    .join(&target_path)
                            } else {
                                target_path.clone()
                            };
                            if !resolved.exists() {
                                broken_symlinks.push(BrokenSymlink {
                                    link: link_path,
                                    target: target_path,
                                });
                            }
                        }
                        Err(_) => {
                            // An unreadable link isn't really actionable
                            // So we can just continue
                            continue;
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }

    broken_symlinks
}

pub struct BrokenSymlink {
    pub link: PathBuf,
    pub target: PathBuf,
}

impl fmt::Display for BrokenSymlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.link.display(), self.target.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    /// Temp dir with a valid symlink pointing to a real file
    fn setup_valid_symlink() -> TempDir {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("real_file.txt");
        fs::write(&target, "hello").unwrap();
        let link = dir.path().join("good_link");
        symlink(&target, &link).unwrap();
        dir
    }

    /// Create a temp dir with a broken symlink
    fn setup_broken_symlink() -> TempDir {
        let dir = TempDir::new().unwrap();
        let link = dir.path().join("broken_link");
        symlink("/nonexistent/path/target.txt", &link).unwrap();
        dir
    }

    #[test]
    fn no_symlinks_returns_empty() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file.txt"), "data").unwrap();

        let result = find_broken_symlinks(dir.path(), &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn valid_symlink_is_not_reported() {
        let dir = setup_valid_symlink();

        let result = find_broken_symlinks(dir.path(), &[]);
        assert!(result.is_empty(), "valid symlinks should not be reported");
    }

    #[test]
    fn broken_symlink_is_detected() {
        let dir = setup_broken_symlink();

        let result = find_broken_symlinks(dir.path(), &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].link.ends_with("broken_link"));
        assert_eq!(
            result[0].target,
            PathBuf::from("/nonexistent/path/target.txt")
        );
    }

    #[test]
    fn mixed_bag_symlinks() {
        let dir = TempDir::new().unwrap();

        let real = dir.path().join("real.txt");
        fs::write(&real, "content").unwrap();
        let good = dir.path().join("good_link");
        symlink(&real, &good).unwrap();

        let bad = dir.path().join("bad_link");
        symlink("/no/such/file", &bad).unwrap();

        let result = find_broken_symlinks(dir.path(), &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].link.ends_with("bad_link"));
    }

    #[test]
    fn broken_relative_symlink_is_detected() {
        let dir = TempDir::new().unwrap();
        let link = dir.path().join("relative_broken");
        symlink("nonexistent_sibling.txt", &link).unwrap();

        let result = find_broken_symlinks(dir.path(), &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].link.ends_with("relative_broken"));
        assert_eq!(result[0].target, PathBuf::from("nonexistent_sibling.txt"));
    }

    #[test]
    fn valid_relative_symlink_is_not_reported() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("sibling.txt");
        fs::write(&target, "hi").unwrap();
        let link = dir.path().join("relative_good");
        symlink("sibling.txt", &link).unwrap();

        let result = find_broken_symlinks(dir.path(), &[]);
        assert!(
            result.is_empty(),
            "valid relative symlinks should not be reported"
        );
    }

    #[test]
    fn nested_broken_symlink_is_found() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        let link = sub.join("deep_broken");
        symlink("/gone/forever", &link).unwrap();

        let result = find_broken_symlinks(dir.path(), &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].link.ends_with("deep_broken"));
    }

    #[test]
    fn empty_directory_returns_empty() {
        let dir = TempDir::new().unwrap();

        let result = find_broken_symlinks(dir.path(), &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn multiple_broken_symlinks_all_detected() {
        let dir = TempDir::new().unwrap();

        for i in 0..5 {
            let link = dir.path().join(format!("broken_{i}"));
            symlink(format!("/missing/target_{i}"), &link).unwrap();
        }

        let result = find_broken_symlinks(dir.path(), &[]);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn symlink_becomes_broken_after_target_removed() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("temp_target.txt");
        fs::write(&target, "exists").unwrap();
        let link = dir.path().join("will_break");
        symlink(&target, &link).unwrap();

        assert!(find_broken_symlinks(dir.path(), &[]).is_empty());

        fs::remove_file(&target).unwrap();
        let result = find_broken_symlinks(dir.path(), &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].link.ends_with("will_break"));
    }

    #[test]
    fn ignore_patterns_skip_directories() {
        let dir = TempDir::new().unwrap();
        let git = dir.path().join(".git");
        fs::create_dir(&git).unwrap();
        let link = git.join("broken");
        symlink("/nonexistent", &link).unwrap();

        let visible_link = dir.path().join("also_broken");
        symlink("/also_nonexistent", &visible_link).unwrap();

        let ignore = vec![".git".to_string()];
        let result = find_broken_symlinks(dir.path(), &ignore);
        assert_eq!(result.len(), 1);
        assert!(result[0].link.ends_with("also_broken"));
    }

    #[test]
    fn display_format_is_correct() {
        let bs = BrokenSymlink {
            link: PathBuf::from("/home/user/link"),
            target: PathBuf::from("/home/user/missing"),
        };
        assert_eq!(bs.to_string(), "/home/user/link -> /home/user/missing");
    }
}
