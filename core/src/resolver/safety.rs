use std::{fs, path::Path};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub fn would_create_loop(link: &Path, new_target: &Path) -> bool {
    let link = link.to_path_buf();
    let mut current = new_target.to_path_buf();
    let mut visited = std::collections::HashSet::new();
    for _ in 0..64 {
        if current == link {
            return true;
        }
        if visited.contains(&current) {
            return true;
        }
        visited.insert(current.clone());
        let meta = match fs::symlink_metadata(&current) {
            Ok(m) => m,
            Err(_) => return false,
        };
        if !meta.file_type().is_symlink() {
            return false;
        }
        current = match fs::read_link(&current) {
            Ok(t) => {
                if t.is_absolute() {
                    t
                } else {
                    current.parent().unwrap_or(Path::new(".")).join(t)
                }
            }
            Err(_) => return false,
        };
    }
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelinkWarning {
    CrossFilesystem { link_dev: u64, target_dev: u64 },
    RelativeToAbsolute,
    AbsoluteToRelative,
}

pub fn relink_warnings(
    link: &Path,
    original_target: &Path,
    new_target: &Path,
) -> Vec<RelinkWarning> {
    let mut warnings = Vec::new();

    if original_target.is_relative() && new_target.is_absolute() {
        warnings.push(RelinkWarning::RelativeToAbsolute);
    }
    if original_target.is_absolute() && new_target.is_relative() {
        warnings.push(RelinkWarning::AbsoluteToRelative);
    }

    #[cfg(unix)]
    {
        let link_dev = fs::metadata(link).ok().map(|m| m.dev()).unwrap_or(0);
        let target_canon = new_target
            .canonicalize()
            .unwrap_or_else(|_| new_target.to_path_buf());
        let target_dev = fs::metadata(&target_canon)
            .ok()
            .map(|m| m.dev())
            .unwrap_or(0);
        if link_dev != 0 && target_dev != 0 && link_dev != target_dev {
            warnings.push(RelinkWarning::CrossFilesystem {
                link_dev,
                target_dev,
            });
        }
    }

    warnings
}

pub fn format_warnings(warnings: &[RelinkWarning]) -> String {
    let mut out = String::new();
    for w in warnings {
        match w {
            RelinkWarning::CrossFilesystem { .. } => {
                out.push_str("      ⚠ cross-filesystem (link and target on different mounts)\n");
            }
            RelinkWarning::RelativeToAbsolute => {
                out.push_str("      ⚠ was relative, would become absolute\n");
            }
            RelinkWarning::AbsoluteToRelative => {
                out.push_str("      ⚠ was absolute, would become relative\n");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[test]
    fn loop_a_to_b_to_a() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        symlink(&b, &a).unwrap();
        symlink(&a, &b).unwrap();

        assert!(would_create_loop(&a, &b));
        assert!(would_create_loop(&b, &a));
    }

    #[test]
    fn no_loop_simple_relink() {
        let temp = TempDir::new().unwrap();
        let link = temp.path().join("link");
        let target = temp.path().join("target");
        std::fs::write(&target, b"x").unwrap();
        symlink("/nonexistent", &link).unwrap();

        assert!(!would_create_loop(&link, &target));
    }

    #[test]
    fn relative_to_absolute_warning() {
        let temp = TempDir::new().unwrap();
        let link = temp.path().join("link");
        let target = temp.path().join("target");
        std::fs::write(&target, b"x").unwrap();

        let w = relink_warnings(&link, Path::new("../foo"), &target);
        assert!(
            w.iter()
                .any(|x| matches!(x, RelinkWarning::RelativeToAbsolute))
        );
    }
}
