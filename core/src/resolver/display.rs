use std::fmt;

use super::model::RepairCase;

pub fn present(w: &mut impl fmt::Write, case: &RepairCase) -> fmt::Result {
    format_header(w, case)?;
    format_candidates(w, case)?;
    format_actions(w, case)
}

pub fn format_header(w: &mut impl fmt::Write, case: &RepairCase) -> fmt::Result {
    let RepairCase {
        ref link,
        ref original_target,
        ..
    } = *case;
    writeln!(w, "{} -> {}", link.display(), original_target.display())
}

pub fn format_candidates(w: &mut impl fmt::Write, case: &RepairCase) -> fmt::Result {
    let RepairCase { ref candidates, .. } = *case;
    if candidates.is_empty() {
        writeln!(w, "  no candidates found")
    } else {
        for (i, candidate) in candidates.iter().enumerate() {
            writeln!(
                w,
                "  [{}] {} (score: {:.2})",
                i + 1,
                candidate.path.display(),
                candidate.score
            )?;
        }
        Ok(())
    }
}

pub fn format_actions(w: &mut impl fmt::Write, case: &RepairCase) -> fmt::Result {
    let RepairCase { ref candidates, .. } = *case;
    if candidates.is_empty() {
        writeln!(w, "  [c] custom path  [s] skip  [r] remove")
    } else {
        let n = candidates.len();
        if n == 1 {
            writeln!(w, "  [1] select  [c] custom path  [s] skip  [r] remove")
        } else {
            writeln!(w, "  [1-{n}] select  [c] custom path  [s] skip  [r] remove")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzy::ScoredCandidate;

    fn case_with_candidates() -> RepairCase {
        RepairCase::new(
            "/home/user/link".into(),
            "/old/target.txt".into(),
            vec![
                ScoredCandidate {
                    path: "/home/user/target.txt".into(),
                    score: 3.20,
                },
                ScoredCandidate {
                    path: "/archive/target.txt".into(),
                    score: 4.50,
                },
            ],
        )
    }

    fn case_without_candidates() -> RepairCase {
        RepairCase::new("/home/user/link".into(), "/old/gone.txt".into(), vec![])
    }

    fn case_single_candidate() -> RepairCase {
        RepairCase::new(
            "/home/user/link".into(),
            "/old/target.txt".into(),
            vec![ScoredCandidate {
                path: "/home/user/target.txt".into(),
                score: 3.20,
            }],
        )
    }

    #[test]
    fn header_shows_link_and_target() {
        let mut out = String::new();
        format_header(&mut out, &case_with_candidates()).unwrap();
        assert_eq!(out, "/home/user/link -> /old/target.txt\n");
    }

    #[test]
    fn candidates_listed_with_scores() {
        let mut out = String::new();
        format_candidates(&mut out, &case_with_candidates()).unwrap();
        assert!(out.contains("[1] /home/user/target.txt (score: 3.20)"));
        assert!(out.contains("[2] /archive/target.txt (score: 4.50)"));
    }

    #[test]
    fn no_candidates_message() {
        let mut out = String::new();
        format_candidates(&mut out, &case_without_candidates()).unwrap();
        assert_eq!(out, "  no candidates found\n");
    }

    #[test]
    fn actions_with_multiple_candidates() {
        let mut out = String::new();
        format_actions(&mut out, &case_with_candidates()).unwrap();
        assert!(out.contains("[1-2] select"));
        assert!(out.contains("[c] custom path"));
        assert!(out.contains("[s] skip"));
        assert!(out.contains("[r] remove"));
    }

    #[test]
    fn actions_with_single_candidate() {
        let mut out = String::new();
        format_actions(&mut out, &case_single_candidate()).unwrap();
        assert!(out.contains("[1] select"));
        assert!(!out.contains("[1-1]"));
    }

    #[test]
    fn actions_without_candidates() {
        let mut out = String::new();
        format_actions(&mut out, &case_without_candidates()).unwrap();
        assert!(!out.contains("select"));
        assert!(out.contains("[c] custom path"));
        assert!(out.contains("[s] skip"));
        assert!(out.contains("[r] remove"));
    }

    #[test]
    fn present_combines_all_sections() {
        let mut out = String::new();
        present(&mut out, &case_with_candidates()).unwrap();

        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].contains("->"));
        assert!(lines[1].contains("[1]"));
        assert!(lines[2].contains("[2]"));
        assert!(lines[3].contains("select"));
    }
}
