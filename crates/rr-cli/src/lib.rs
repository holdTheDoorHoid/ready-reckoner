//! Ready Reckoner — `rr-cli`: the `rr` command line, the oracle for humans, agents and CI.
//!
//! Every command runs the same engine the web app runs (`rr_plan::Engine`), on the data packs in
//! `data/` (loaded into an `rr_data::DataStore`, every file checked against its sha256 in
//! `data/manifest.json`) or, when there is no pack or `--fixtures` is given, on the seven
//! hand-built fixture counties. See `docs/CLI.md` for every command with examples.
//!
//! | Command | What it prints |
//! | --- | --- |
//! | `plan` | the printable packet (Markdown) and/or the whole `PlanOutput` (JSON) |
//! | `risks` | the ranked risk register and the rare-but-severe box, with sources |
//! | `targets [--sweep]` | days to be ready for, per need, with ranges and relief; `--sweep` shows every dial setting |
//! | `explain <kind> <id>` | why a number is what it is: plain words, the arithmetic, the sources |
//! | `catalogue` | the item catalogue |
//! | `citations [--missing]` | the source registry; ids referenced but not defined |
//! | `county search/show` | find a county; its data record |
//! | `data verify/info` | check the packs; versions and attributions |
//! | `golden [--update]` | compare (or rewrite) `fixtures/golden/*` with rr-plan's own helper |
//! | `doctor` | every fixture household: timing, warnings, uncited items, determinism |
//!
//! Exit status: 0 success; 1 a check failed or the engine could not run; 2 the input needs
//! fixing (validation problems, an unknown or ambiguous location, a usage error).
//!
//! The CLI is native only (it reads files and the clock for timings); the engine it drives stays
//! deterministic and never sees either.
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

pub mod args;
pub mod cmd;
pub mod error;
pub mod format;
pub mod household;
pub mod source;

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use clap::Parser;

pub use error::{CliError, Exit};

/// What a command produced: text for standard output, notes for standard error, and the exit
/// status (a failed comparison is a result, not an error).
#[derive(Debug, Default)]
pub struct Output {
    /// Printed to standard output, as is.
    pub stdout: String,
    /// Printed to standard error, one per line, after the output.
    pub notes: Vec<String>,
    /// The exit status.
    pub exit: Exit,
}

impl Output {
    /// Output with this text on standard output and success.
    pub fn text(stdout: String) -> Self {
        Self {
            stdout,
            ..Self::default()
        }
    }
}

/// Parses the arguments, runs the command and prints its output. Returns the exit status.
pub fn main<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match args::Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            return ExitCode::from(u8::try_from(e.exit_code()).unwrap_or(2));
        }
    };
    match cmd::run(&cli) {
        Ok(out) => {
            write_stdout(&out.stdout);
            for n in &out.notes {
                eprintln!("{n}");
            }
            ExitCode::from(out.exit.code())
        }
        Err(e) => {
            eprintln!("rr: {}", e.message);
            ExitCode::from(e.exit.code())
        }
    }
}

/// Writes to standard output, quietly stopping if the reader has gone away (`rr plan | head`).
fn write_stdout(text: &str) {
    let mut out = std::io::stdout().lock();
    if out.write_all(text.as_bytes()).is_ok() {
        let _ = out.flush();
    }
}
