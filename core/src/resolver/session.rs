use std::io;

use super::{
    action::{Resolved, resolve, resolve_custom},
    confirm::{format_confirmation, needs_confirmation, parse_confirmation},
    display::present,
    fs_ops::execute,
    input::parse_choice,
    io::ResolverIO,
    model::{Action, RepairCase, Summary},
};

pub fn run(cases: &[RepairCase], io: &mut impl ResolverIO, dry_run: bool) -> io::Result<Summary> {
    let mut summary = Summary::default();

    for case in cases {
        let mut buf = String::new();
        present(&mut buf, case).unwrap();
        io.write_str(&buf)?;

        let action = prompt_until_resolved(case, io)?;

        match execute(&case.link, &action, dry_run) {
            Ok(()) => {
                format_outcome(io, &action, dry_run)?;
                summary.record(&action);
            }
            Err(e) => {
                io.write_str(&format!("  error: {e}\n"))?;
                summary.record(&Action::Skip);
            }
        }

        io.write_str("\n")?;
    }

    Ok(summary)
}

fn prompt_until_resolved(case: &RepairCase, io: &mut impl ResolverIO) -> io::Result<Action> {
    loop {
        io.write_str("> ")?;
        let input = io.read_line()?;

        let parsed = match parse_choice(&input, case.candidates.len()) {
            Ok(p) => p,
            Err(e) => {
                io.write_str(&format!("  {e}\n"))?;
                continue;
            }
        };

        let action = match resolve(parsed, case) {
            Resolved::Action(a) => a,
            Resolved::NeedsCustomPath => {
                io.write_str("  enter path: ")?;
                let path_input = io.read_line()?;
                let trimmed = path_input.trim();
                if trimmed.is_empty() {
                    continue;
                }
                resolve_custom(trimmed.into())
            }
        };

        if needs_confirmation(&action) {
            let mut confirm_buf = String::new();
            format_confirmation(&mut confirm_buf, &case.link, &action).unwrap();
            io.write_str(&confirm_buf)?;
            let confirm_input = io.read_line()?;
            if !parse_confirmation(&confirm_input) {
                continue;
            }
        }

        return Ok(action);
    }
}

fn format_outcome(io: &mut impl ResolverIO, action: &Action, dry_run: bool) -> io::Result<()> {
    match (action, dry_run) {
        (Action::Relink(target), true) => io.write_str(&format!(
            "  [dry run] would relink -> {}\n",
            target.display()
        )),
        (Action::Remove, true) => io.write_str("  [dry run] would remove\n"),
        (Action::Relink(target), false) => {
            io.write_str(&format!("  relinked -> {}\n", target.display()))
        }
        (Action::Remove, false) => io.write_str("  removed\n"),
        (Action::Skip, _) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fuzzy::ScoredCandidate, resolver::io::MockIO};
    use std::{fs, os::unix::fs::symlink, path::PathBuf};
    use tempfile::TempDir;

    fn setup_case(temp: &TempDir) -> RepairCase {
        let target = temp.path().join("candidate.txt");
        fs::write(&target, b"hello").unwrap();
        let link = temp.path().join("my_link");
        symlink("/nonexistent", &link).unwrap();

        RepairCase::new(
            link,
            "/nonexistent".into(),
            vec![ScoredCandidate {
                path: target,
                score: 3.20,
            }],
        )
    }

    #[test]
    fn select_candidate_relinks() {
        let temp = TempDir::new().unwrap();
        let case = setup_case(&temp);
        let expected_target = case.candidates[0].path.clone();
        let mut io = MockIO::new(vec!["1"]);

        let summary = run(&[case], &mut io, false).unwrap();

        assert_eq!(summary.relinked, 1);
        let resolved = fs::read_link(temp.path().join("my_link")).unwrap();
        assert_eq!(resolved, expected_target);
        assert!(io.output().contains("relinked"));
    }

    #[test]
    fn skip_leaves_link_intact() {
        let temp = TempDir::new().unwrap();
        let case = setup_case(&temp);
        let mut io = MockIO::new(vec!["s"]);

        let summary = run(&[case], &mut io, false).unwrap();

        assert_eq!(summary.skipped, 1);
        assert!(temp.path().join("my_link").symlink_metadata().is_ok());
    }

    #[test]
    fn remove_with_confirmation() {
        let temp = TempDir::new().unwrap();
        let case = setup_case(&temp);
        let mut io = MockIO::new(vec!["r", "y"]);

        let summary = run(&[case], &mut io, false).unwrap();

        assert_eq!(summary.removed, 1);
        assert!(temp.path().join("my_link").symlink_metadata().is_err());
        assert!(io.output().contains("removed"));
    }

    #[test]
    fn remove_declined_then_skip() {
        let temp = TempDir::new().unwrap();
        let case = setup_case(&temp);
        let mut io = MockIO::new(vec!["r", "n", "s"]);

        let summary = run(&[case], &mut io, false).unwrap();

        assert_eq!(summary.skipped, 1);
        assert!(temp.path().join("my_link").symlink_metadata().is_ok());
    }

    #[test]
    fn invalid_input_retries() {
        let temp = TempDir::new().unwrap();
        let case = setup_case(&temp);
        let mut io = MockIO::new(vec!["xyz", "s"]);

        let summary = run(&[case], &mut io, false).unwrap();

        assert_eq!(summary.skipped, 1);
        assert!(io.output().contains("unrecognized input"));
    }

    #[test]
    fn custom_path_relinks() {
        let temp = TempDir::new().unwrap();
        let custom_target = temp.path().join("custom.txt");
        fs::write(&custom_target, b"custom").unwrap();
        let case = setup_case(&temp);
        let mut io = MockIO::new(vec!["c", custom_target.to_str().unwrap()]);

        let summary = run(&[case], &mut io, false).unwrap();

        assert_eq!(summary.relinked, 1);
        let resolved = fs::read_link(temp.path().join("my_link")).unwrap();
        assert_eq!(resolved, custom_target);
    }

    #[test]
    fn empty_custom_path_retries() {
        let temp = TempDir::new().unwrap();
        let case = setup_case(&temp);
        let mut io = MockIO::new(vec!["c", "", "s"]);

        let summary = run(&[case], &mut io, false).unwrap();

        assert_eq!(summary.skipped, 1);
    }

    #[test]
    fn dry_run_no_changes() {
        let temp = TempDir::new().unwrap();
        let case = setup_case(&temp);
        let mut io = MockIO::new(vec!["1"]);

        let summary = run(&[case], &mut io, true).unwrap();

        assert_eq!(summary.relinked, 1);
        let still_broken = fs::read_link(temp.path().join("my_link")).unwrap();
        assert_eq!(still_broken, PathBuf::from("/nonexistent"));
        assert!(io.output().contains("[dry run]"));
    }

    #[test]
    fn multiple_cases_tally_summary() {
        let temp = TempDir::new().unwrap();

        let t1 = temp.path().join("t1.txt");
        fs::write(&t1, b"one").unwrap();
        let l1 = temp.path().join("link1");
        symlink("/nonexistent", &l1).unwrap();
        let case1 = RepairCase::new(
            l1,
            "/nonexistent".into(),
            vec![ScoredCandidate {
                path: t1,
                score: 1.0,
            }],
        );

        let l2 = temp.path().join("link2");
        symlink("/nonexistent", &l2).unwrap();
        let case2 = RepairCase::new(l2, "/nonexistent".into(), vec![]);

        let mut io = MockIO::new(vec!["1", "s"]);

        let summary = run(&[case1, case2], &mut io, false).unwrap();

        assert_eq!(summary.relinked, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.total(), 2);
    }
}
