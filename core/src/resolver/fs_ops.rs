use std::{
    fmt,
    path::{Path, PathBuf},
};

use super::{model::Action, safety::would_create_loop};

pub fn execute(link: &Path, action: &Action, dry_run: bool) -> Result<(), FsError> {
    match action {
        Action::Skip => Ok(()),
        Action::Remove => {
            if dry_run {
                return Ok(());
            }
            std::fs::remove_file(link).map_err(|source| FsError::RemoveFailed {
                path: link.to_path_buf(),
                source,
            })
        }
        Action::Relink(target) => {
            if would_create_loop(link, target) {
                return Err(FsError::WouldCreateLoop {
                    link: link.to_path_buf(),
                    target: target.clone(),
                });
            }
            if dry_run {
                return Ok(());
            }
            std::fs::remove_file(link).map_err(|source| FsError::RemoveFailed {
                path: link.to_path_buf(),
                source,
            })?;
            std::os::unix::fs::symlink(target, link).map_err(|source| FsError::SymlinkFailed {
                link: link.to_path_buf(),
                target: target.clone(),
                source,
            })
        }
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RemoveFailed { path, source } => {
                write!(f, "failed to remove {}: {source}", path.display())
            }
            Self::SymlinkFailed {
                link,
                target,
                source,
            } => {
                write!(
                    f,
                    "failed to symlink {} -> {}: {source}",
                    link.display(),
                    target.display()
                )
            }
            Self::WouldCreateLoop { link, target } => {
                write!(
                    f,
                    "refused: relinking {} -> {} would create a symlink loop",
                    link.display(),
                    target.display()
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum FsError {
    RemoveFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    SymlinkFailed {
        link: PathBuf,
        target: PathBuf,
        source: std::io::Error,
    },
    WouldCreateLoop {
        link: PathBuf,
        target: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::symlink};
    use tempfile::TempDir;

    #[test]
    fn relink_replaces_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("new_target.txt");
        fs::write(&target, b"hello").unwrap();
        let link = temp.path().join("my_link");
        symlink("/nonexistent", &link).unwrap();

        execute(&link, &Action::Relink(target.clone()), false).unwrap();

        let resolved = fs::read_link(&link).unwrap();
        assert_eq!(resolved, target);
    }

    #[test]
    fn remove_deletes_symlink() {
        let temp = TempDir::new().unwrap();
        let link = temp.path().join("my_link");
        symlink("/nonexistent", &link).unwrap();
        assert!(link.symlink_metadata().is_ok());

        execute(&link, &Action::Remove, false).unwrap();

        assert!(link.symlink_metadata().is_err());
    }

    #[test]
    fn skip_is_noop() {
        let temp = TempDir::new().unwrap();
        let link = temp.path().join("my_link");
        symlink("/nonexistent", &link).unwrap();

        execute(&link, &Action::Skip, false).unwrap();

        assert!(link.symlink_metadata().is_ok());
    }

    #[test]
    fn dry_run_relink_does_not_modify() {
        let temp = TempDir::new().unwrap();
        let link = temp.path().join("my_link");
        symlink("/nonexistent", &link).unwrap();
        let target = temp.path().join("new_target.txt");
        fs::write(&target, b"hello").unwrap();

        execute(&link, &Action::Relink(target), true).unwrap();

        let still_broken = fs::read_link(&link).unwrap();
        assert_eq!(still_broken, PathBuf::from("/nonexistent"));
    }

    #[test]
    fn dry_run_remove_does_not_modify() {
        let temp = TempDir::new().unwrap();
        let link = temp.path().join("my_link");
        symlink("/nonexistent", &link).unwrap();

        execute(&link, &Action::Remove, true).unwrap();

        assert!(link.symlink_metadata().is_ok());
    }

    #[test]
    fn remove_nonexistent_fails() {
        let temp = TempDir::new().unwrap();
        let link = temp.path().join("no_such_link");

        let result = execute(&link, &Action::Remove, false);
        assert!(result.is_err());
    }

    #[test]
    fn error_display_includes_path() {
        let err = FsError::RemoveFailed {
            path: "/some/link".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = err.to_string();
        assert!(msg.contains("/some/link"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn would_create_loop_refused() {
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        symlink(&b, &a).unwrap();
        symlink(&a, &b).unwrap();

        let result = execute(&a, &Action::Relink(b), false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("loop"));
    }

    #[test]
    fn error_display_symlink_includes_both_paths() {
        let err = FsError::SymlinkFailed {
            link: "/some/link".into(),
            target: "/some/target".into(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        let msg = err.to_string();
        assert!(msg.contains("/some/link"));
        assert!(msg.contains("/some/target"));
        assert!(msg.contains("denied"));
    }
}
