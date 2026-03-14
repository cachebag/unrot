use std::fmt;

pub fn parse_choice(input: &str, num_candidates: usize) -> Result<ParsedInput, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "s" | "skip" => return Ok(ParsedInput::Skip),
        "r" | "remove" => return Ok(ParsedInput::Remove),
        "c" | "custom" => return Ok(ParsedInput::CustomPath),
        _ => {}
    }

    if let Ok(n) = trimmed.parse::<usize>() {
        if (1..=num_candidates).contains(&n) {
            return Ok(ParsedInput::SelectCandidate(n - 1));
        }
        return Err(ParseError::InvalidSelection {
            given: n,
            max: num_candidates,
        });
    }

    Err(ParseError::Unrecognized(trimmed.to_string()))
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "no input provided"),
            Self::InvalidSelection { given, max } => {
                write!(f, "selection {given} is out of range (1-{max})")
            }
            Self::Unrecognized(s) => write!(f, "unrecognized input: {s}"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParsedInput {
    SelectCandidate(usize),
    CustomPath,
    Skip,
    Remove,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    InvalidSelection { given: usize, max: usize },
    Unrecognized(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_variants() {
        assert_eq!(parse_choice("s", 0), Ok(ParsedInput::Skip));
        assert_eq!(parse_choice("skip", 0), Ok(ParsedInput::Skip));
        assert_eq!(parse_choice("S", 0), Ok(ParsedInput::Skip));
        assert_eq!(parse_choice("SKIP", 0), Ok(ParsedInput::Skip));
    }

    #[test]
    fn remove_variants() {
        assert_eq!(parse_choice("r", 0), Ok(ParsedInput::Remove));
        assert_eq!(parse_choice("remove", 0), Ok(ParsedInput::Remove));
        assert_eq!(parse_choice("R", 0), Ok(ParsedInput::Remove));
    }

    #[test]
    fn custom_path_variants() {
        assert_eq!(parse_choice("c", 0), Ok(ParsedInput::CustomPath));
        assert_eq!(parse_choice("custom", 0), Ok(ParsedInput::CustomPath));
        assert_eq!(parse_choice("C", 0), Ok(ParsedInput::CustomPath));
    }

    #[test]
    fn valid_candidate_selection() {
        assert_eq!(parse_choice("1", 3), Ok(ParsedInput::SelectCandidate(0)));
        assert_eq!(parse_choice("3", 3), Ok(ParsedInput::SelectCandidate(2)));
    }

    #[test]
    fn selection_out_of_range() {
        assert_eq!(
            parse_choice("4", 3),
            Err(ParseError::InvalidSelection { given: 4, max: 3 })
        );
        assert_eq!(
            parse_choice("0", 3),
            Err(ParseError::InvalidSelection { given: 0, max: 3 })
        );
    }

    #[test]
    fn selection_with_no_candidates() {
        assert_eq!(
            parse_choice("1", 0),
            Err(ParseError::InvalidSelection { given: 1, max: 0 })
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(parse_choice("", 3), Err(ParseError::Empty));
        assert_eq!(parse_choice("   ", 3), Err(ParseError::Empty));
    }

    #[test]
    fn unrecognized_input() {
        assert_eq!(
            parse_choice("xyz", 3),
            Err(ParseError::Unrecognized("xyz".to_string()))
        );
    }

    #[test]
    fn whitespace_trimmed() {
        assert_eq!(
            parse_choice("  2  ", 3),
            Ok(ParsedInput::SelectCandidate(1))
        );
        assert_eq!(parse_choice(" s ", 0), Ok(ParsedInput::Skip));
    }

    #[test]
    fn error_display_messages() {
        assert_eq!(ParseError::Empty.to_string(), "no input provided");
        assert_eq!(
            ParseError::InvalidSelection { given: 5, max: 3 }.to_string(),
            "selection 5 is out of range (1-3)"
        );
        assert_eq!(
            ParseError::Unrecognized("foo".into()).to_string(),
            "unrecognized input: foo"
        );
    }
}
