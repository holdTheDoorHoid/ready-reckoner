//! Errors and exit status. Engine errors are printed the way the app would show them: the
//! plain-language problems with the field each one is about, or the counties to choose from.

use rr_types::{EngineError, ErrorCode, LocationResolved};

/// How the process ends.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Exit {
    /// Everything worked (exit 0).
    #[default]
    Ok,
    /// A check failed, or the engine or a file could not be read (exit 1).
    Failure,
    /// The input needs fixing: validation problems, an unknown or ambiguous location, a usage
    /// error (exit 2).
    Input,
}

impl Exit {
    /// The process exit code.
    pub const fn code(self) -> u8 {
        match self {
            Exit::Ok => 0,
            Exit::Failure => 1,
            Exit::Input => 2,
        }
    }
}

/// A command that could not do its job, with the message to print and the exit status.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct CliError {
    /// Exit status.
    pub exit: Exit,
    /// What went wrong, in plain words (may span several lines).
    pub message: String,
}

impl CliError {
    /// The input needs fixing (exit 2).
    pub fn input(message: impl Into<String>) -> Self {
        Self {
            exit: Exit::Input,
            message: message.into(),
        }
    }

    /// Something failed (exit 1).
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            exit: Exit::Failure,
            message: message.into(),
        }
    }

    /// An engine error. Problems with the household or its location exit 2; a missing or corrupt
    /// pack or an engine bug exits 1. `hint` is added under the message (for example which
    /// counties the fixture data knows).
    pub fn engine(e: &EngineError, hint: Option<&str>) -> Self {
        let exit = match e.code {
            ErrorCode::BadInput
            | ErrorCode::UnknownZip
            | ErrorCode::UnknownCounty
            | ErrorCode::AmbiguousZip => Exit::Input,
            ErrorCode::PackMissing | ErrorCode::PackCorrupt | ErrorCode::Internal => Exit::Failure,
        };
        let mut message = describe(e);
        if let Some(h) = hint.filter(|h| !h.is_empty()) {
            message.push_str("\n  ");
            message.push_str(h);
        }
        Self { exit, message }
    }
}

/// The error in plain words: every validation problem on its own line with its field, or the
/// suggested counties with their FIPS codes.
pub fn describe(e: &EngineError) -> String {
    match e.code {
        ErrorCode::BadInput => match e.problems().filter(|p| !p.is_empty()) {
            Some(problems) => {
                let mut s = if problems.len() == 1 {
                    "The household has 1 problem to fix:".to_owned()
                } else {
                    format!("The household has {} problems to fix:", problems.len())
                };
                for p in &problems {
                    let field = if p.field.is_empty() {
                        "(the whole file)"
                    } else {
                        p.field.as_str()
                    };
                    s.push_str(&format!("\n  - {field}: {} [{}]", p.message, p.code));
                }
                s
            }
            None => e.message.clone(),
        },
        ErrorCode::UnknownZip | ErrorCode::UnknownCounty | ErrorCode::AmbiguousZip => {
            let mut s = e.message.clone();
            let suggestions = e.suggestions().unwrap_or_default();
            if !suggestions.is_empty() {
                s.push_str(if e.code == ErrorCode::AmbiguousZip {
                    "\n  The counties it covers, largest share first:"
                } else {
                    "\n  Did you mean:"
                });
                for l in &suggestions {
                    s.push_str(&format!("\n    {}", suggestion_line(l)));
                }
                s.push_str("\n  Run again with --county <FIPS> to choose one.");
            }
            s
        }
        _ => e.message.clone(),
    }
}

fn suggestion_line(l: &LocationResolved) -> String {
    let share = l
        .zip_county_share
        .map(|s| format!("  ({:.0}% of the ZIP code)", 100.0 * s))
        .unwrap_or_default();
    format!(
        "{}  {}, {}{share}",
        l.county_fips, l.county_name, l.state_abbr
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::{Problem, ProblemCode};

    #[test]
    fn problems_list_every_field() {
        let e = EngineError::bad_input(vec![
            Problem::new(
                ProblemCode::NoPeople,
                "people",
                "Add at least one person to your household.",
            ),
            Problem::new(ProblemCode::Schema, "", "This is not a plan."),
        ]);
        let c = CliError::engine(&e, None);
        assert_eq!(c.exit, Exit::Input);
        assert!(
            c.message
                .starts_with("The household has 2 problems to fix:")
        );
        assert!(
            c.message
                .contains("  - people: Add at least one person to your household. [no_people]")
        );
        assert!(
            c.message
                .contains("(the whole file): This is not a plan. [schema]")
        );
    }

    #[test]
    fn engine_failures_exit_1() {
        let e = EngineError::new(ErrorCode::PackCorrupt, "bad checksum");
        let c = CliError::engine(&e, Some("hint"));
        assert_eq!(
            (c.exit, c.message.as_str()),
            (Exit::Failure, "bad checksum\n  hint")
        );
        assert_eq!(Exit::Input.code(), 2);
    }
}
