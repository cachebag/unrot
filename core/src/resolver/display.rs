use std::fmt;

use owo_colors::{OwoColorize, Stream};

use super::{
    model::RepairCase,
    safety::{format_warnings, relink_warnings},
};

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
    writeln!(
        w,
        "{} -> {}",
        link.display()
            .if_supports_color(Stream::Stdout, |v| v.red()),
        original_target
            .display()
            .if_supports_color(Stream::Stdout, |v| v.red())
    )
}

pub fn format_candidates(w: &mut impl fmt::Write, case: &RepairCase) -> fmt::Result {
    let RepairCase {
        ref candidates,
        ref original_target,
        ..
    } = *case;
    if candidates.is_empty() {
        writeln!(
            w,
            "  {}",
            "no candidates found".if_supports_color(Stream::Stdout, |v| v.yellow())
        )
    } else {
        let target_basename = original_target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        for (i, candidate) in candidates.iter().enumerate() {
            let basename_note = if candidate.basename_count <= 1 {
                "only match".to_string()
            } else {
                format!(
                    "{} files named {target_basename} found",
                    candidate.basename_count
                )
            };
            writeln!(
                w,
                "  [{}] {} (score: {:.2}, {} shared dirs, {})",
                (i + 1)
                    .to_string()
                    .if_supports_color(Stream::Stdout, |v| v.cyan()),
                candidate
                    .path
                    .display()
                    .if_supports_color(Stream::Stdout, |v| v.green()),
                candidate.score,
                candidate.shared_dirs,
                basename_note
            )?;
            let warnings = relink_warnings(&case.link, original_target, &candidate.path);
            if !warnings.is_empty() {
                write!(
                    w,
                    "{}",
                    format_warnings(&warnings).if_supports_color(Stream::Stdout, |v| v.yellow())
                )?;
            }
        }
        Ok(())
    }
}

pub fn format_actions(w: &mut impl fmt::Write, case: &RepairCase) -> fmt::Result {
    let RepairCase { ref candidates, .. } = *case;
    let actions = if candidates.is_empty() {
        "[c] custom path  [s] skip  [r] remove".to_string()
    } else {
        let n = candidates.len();
        if n == 1 {
            "[1] select  [c] custom path  [s] skip  [r] remove".to_string()
        } else {
            format!("[1-{n}] select  [c] custom path  [s] skip  [r] remove")
        }
    };
    writeln!(
        w,
        "  {}",
        actions.if_supports_color(Stream::Stdout, |v| v.cyan())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzy::ScoredCandidate;

    fn disable_colors() {
        owo_colors::set_override(false);
    }

    fn case_with_candidates() -> RepairCase {
        RepairCase::new(
            "/home/user/link".into(),
            "/old/target.txt".into(),
            vec![
                ScoredCandidate {
                    path: "/home/user/target.txt".into(),
                    score: 3.20,
                    shared_dirs: 0,
                    basename_count: 2,
                },
                ScoredCandidate {
                    path: "/archive/target.txt".into(),
                    score: 4.50,
                    shared_dirs: 0,
                    basename_count: 2,
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
                shared_dirs: 0,
                basename_count: 1,
            }],
        )
    }

    #[test]
    fn header_shows_link_and_target() {
        disable_colors();
        let mut out = String::new();
        format_header(&mut out, &case_with_candidates()).unwrap();
        assert_eq!(out, "/home/user/link -> /old/target.txt\n");
    }

    #[test]
    fn candidates_listed_with_scores() {
        disable_colors();
        let mut out = String::new();
        format_candidates(&mut out, &case_with_candidates()).unwrap();
        assert!(out.contains("[1] /home/user/target.txt (score: 3.20"));
        assert!(out.contains("[2] /archive/target.txt (score: 4.50"));
        assert!(out.contains("shared dirs"));
        assert!(out.contains("files named target.txt found"));
    }

    #[test]
    fn no_candidates_message() {
        disable_colors();
        let mut out = String::new();
        format_candidates(&mut out, &case_without_candidates()).unwrap();
        assert_eq!(out, "  no candidates found\n");
    }

    #[test]
    fn actions_with_multiple_candidates() {
        disable_colors();
        let mut out = String::new();
        format_actions(&mut out, &case_with_candidates()).unwrap();
        assert!(out.contains("[1-2] select"));
        assert!(out.contains("[c] custom path"));
        assert!(out.contains("[s] skip"));
        assert!(out.contains("[r] remove"));
    }

    #[test]
    fn actions_with_single_candidate() {
        disable_colors();
        let mut out = String::new();
        format_actions(&mut out, &case_single_candidate()).unwrap();
        assert!(out.contains("[1] select"));
        assert!(!out.contains("[1-1]"));
    }

    #[test]
    fn actions_without_candidates() {
        disable_colors();
        let mut out = String::new();
        format_actions(&mut out, &case_without_candidates()).unwrap();
        assert!(!out.contains("select"));
        assert!(out.contains("[c] custom path"));
        assert!(out.contains("[s] skip"));
        assert!(out.contains("[r] remove"));
    }

    #[test]
    fn present_combines_all_sections() {
        disable_colors();
        let mut out = String::new();
        present(&mut out, &case_with_candidates()).unwrap();

        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].contains("->"));
        assert!(lines[1].contains("[1]"));
        assert!(lines[2].contains("[2]"));
        assert!(lines[3].contains("select"));
    }
}
