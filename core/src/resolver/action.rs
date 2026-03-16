use std::path::PathBuf;

use super::{
    input::ParsedInput,
    model::{Action, RepairCase},
};

pub fn resolve(parsed: ParsedInput, case: &RepairCase) -> Resolved {
    let RepairCase { ref candidates, .. } = *case;
    match parsed {
        ParsedInput::SelectCandidate(idx) => {
            Resolved::Action(Action::Relink(candidates[idx].path.clone()))
        }
        ParsedInput::Skip => Resolved::Action(Action::Skip),
        ParsedInput::Remove => Resolved::Action(Action::Remove),
        ParsedInput::CustomPath => Resolved::NeedsCustomPath,
    }
}

pub fn resolve_custom(path: PathBuf) -> Action {
    Action::Relink(path)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Resolved {
    Action(Action),
    NeedsCustomPath,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzy::ScoredCandidate;

    fn sample_case() -> RepairCase {
        RepairCase::new(
            "/home/user/link".into(),
            "/old/target.txt".into(),
            vec![
                ScoredCandidate {
                    path: "/first/candidate.txt".into(),
                    score: 1.0,
                    shared_dirs: 0,
                    basename_count: 2,
                },
                ScoredCandidate {
                    path: "/second/candidate.txt".into(),
                    score: 2.0,
                    shared_dirs: 0,
                    basename_count: 2,
                },
            ],
        )
    }

    #[test]
    fn select_candidate_resolves_to_relink() {
        let result = resolve(ParsedInput::SelectCandidate(0), &sample_case());
        assert_eq!(
            result,
            Resolved::Action(Action::Relink("/first/candidate.txt".into()))
        );
    }

    #[test]
    fn select_second_candidate() {
        let result = resolve(ParsedInput::SelectCandidate(1), &sample_case());
        assert_eq!(
            result,
            Resolved::Action(Action::Relink("/second/candidate.txt".into()))
        );
    }

    #[test]
    fn skip_resolves_directly() {
        let result = resolve(ParsedInput::Skip, &sample_case());
        assert_eq!(result, Resolved::Action(Action::Skip));
    }

    #[test]
    fn remove_resolves_directly() {
        let result = resolve(ParsedInput::Remove, &sample_case());
        assert_eq!(result, Resolved::Action(Action::Remove));
    }

    #[test]
    fn custom_path_signals_need() {
        let result = resolve(ParsedInput::CustomPath, &sample_case());
        assert_eq!(result, Resolved::NeedsCustomPath);
    }

    #[test]
    fn resolve_custom_creates_relink() {
        let action = resolve_custom("/my/custom/path.txt".into());
        assert_eq!(action, Action::Relink("/my/custom/path.txt".into()));
    }
}
