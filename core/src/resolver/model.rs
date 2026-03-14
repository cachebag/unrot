use std::{fmt, path::PathBuf};

use crate::fuzzy::ScoredCandidate;

impl RepairCase {
    pub fn new(link: PathBuf, original_target: PathBuf, candidates: Vec<ScoredCandidate>) -> Self {
        Self {
            link,
            original_target,
            candidates,
        }
    }

    pub fn has_candidates(&self) -> bool {
        !self.candidates.is_empty()
    }
}

impl Summary {
    pub fn record(&mut self, action: &Action) {
        match action {
            Action::Relink(_) => self.relinked += 1,
            Action::Remove => self.removed += 1,
            Action::Skip => self.skipped += 1,
        }
    }

    pub fn total(&self) -> usize {
        self.relinked + self.removed + self.skipped
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} fixed, {} skipped, {} removed",
            self.relinked, self.skipped, self.removed
        )
    }
}

pub struct RepairCase {
    pub link: PathBuf,
    pub original_target: PathBuf,
    pub candidates: Vec<ScoredCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Relink(PathBuf),
    Remove,
    Skip,
}

pub struct Resolution {
    pub link: PathBuf,
    pub action: Action,
}

#[derive(Debug, Default)]
pub struct Summary {
    pub relinked: usize,
    pub removed: usize,
    pub skipped: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_starts_at_zero() {
        let Summary {
            relinked,
            removed,
            skipped,
        } = Summary::default();
        assert_eq!(relinked, 0);
        assert_eq!(removed, 0);
        assert_eq!(skipped, 0);
    }

    #[test]
    fn summary_records_all_action_types() {
        let mut summary = Summary::default();
        summary.record(&Action::Relink("/new/target".into()));
        summary.record(&Action::Relink("/another".into()));
        summary.record(&Action::Skip);
        summary.record(&Action::Remove);

        assert_eq!(summary.relinked, 2);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.removed, 1);
        assert_eq!(summary.total(), 4);
    }

    #[test]
    fn summary_display_format() {
        let mut summary = Summary::default();
        summary.record(&Action::Relink("/target".into()));
        summary.record(&Action::Skip);
        summary.record(&Action::Remove);

        assert_eq!(summary.to_string(), "1 fixed, 1 skipped, 1 removed");
    }

    #[test]
    fn repair_case_without_candidates() {
        let RepairCase { ref candidates, .. } =
            RepairCase::new("/link".into(), "/target".into(), vec![]);
        assert!(candidates.is_empty());
    }

    #[test]
    fn repair_case_with_candidates() {
        let case = RepairCase::new(
            "/link".into(),
            "/target".into(),
            vec![ScoredCandidate {
                path: "/candidate".into(),
                score: 1.0,
            }],
        );
        assert!(case.has_candidates());
    }

    #[test]
    fn resolution_pairs_link_with_action() {
        let Resolution { link, action } = Resolution {
            link: "/some/link".into(),
            action: Action::Remove,
        };
        assert_eq!(link, PathBuf::from("/some/link"));
        assert_eq!(action, Action::Remove);
    }

    #[test]
    fn action_equality() {
        assert_eq!(Action::Relink("/a".into()), Action::Relink("/a".into()));
        assert_ne!(Action::Relink("/a".into()), Action::Relink("/b".into()));
        assert_ne!(Action::Skip, Action::Remove);
    }
}
