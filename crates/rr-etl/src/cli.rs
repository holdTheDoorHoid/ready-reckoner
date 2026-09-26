//! Command-line parsing (no dependency; the surface is tiny).

use crate::{Result, data_err};
use std::path::PathBuf;

/// Usage text.
pub const USAGE: &str = "\
rr-etl — build Ready Reckoner's data packs from public sources

USAGE:
    rr-etl refresh --out <DIR> [--only <JOB>[,<JOB>...]] [--keep-raw]
    rr-etl verify --data <DIR>
    rr-etl jobs

COMMANDS:
    refresh   Download the sources, rebuild the packs in <DIR>, update <DIR>/manifest.json
              and append a summary to <DIR>/CHANGES.md. Raw downloads are streamed or held in
              memory and never written to disk unless --keep-raw is given (then under
              <DIR>/raw/, which git ignores).
    verify    Recompute every pack file's checksum and row count against the manifest and check
              that every county joins across every pack.
    jobs      List the jobs in run order.

OPTIONS:
    --out, --data <DIR>   Data directory (normally `data`).
    --only <JOBS>         Run only these jobs (comma separated or repeated).
    --keep-raw            Keep raw downloads under <DIR>/raw/.
";

/// A parsed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `refresh`.
    Refresh {
        /// Data directory.
        out: PathBuf,
        /// Jobs to run (empty = all).
        only: Vec<String>,
        /// Keep raw downloads.
        keep_raw: bool,
    },
    /// `verify`.
    Verify {
        /// Data directory.
        data: PathBuf,
    },
    /// `jobs`.
    Jobs,
    /// `help`.
    Help,
}

/// Parse arguments (without the program name).
pub fn parse(args: &[String]) -> Result<Command> {
    let Some(cmd) = args.first() else { return Ok(Command::Help) };
    let mut dir: Option<PathBuf> = None;
    let mut only = Vec::new();
    let mut keep_raw = false;
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        let (flag, inline) = match a.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (a, None),
        };
        let mut value = || -> Result<String> {
            if let Some(v) = inline.clone() {
                return Ok(v);
            }
            i += 1;
            args.get(i).cloned().ok_or_else(|| data_err(format!("{flag} needs a value")))
        };
        match flag {
            "--out" | "--data" => dir = Some(PathBuf::from(value()?)),
            "--only" => only.extend(value()?.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty())),
            "--keep-raw" => keep_raw = true,
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(data_err(format!("unknown option {other}\n\n{USAGE}"))),
        }
        i += 1;
    }
    match cmd.as_str() {
        "refresh" => Ok(Command::Refresh {
            out: dir.ok_or_else(|| data_err("refresh needs --out <DIR>"))?,
            only,
            keep_raw,
        }),
        "verify" => Ok(Command::Verify { data: dir.ok_or_else(|| data_err("verify needs --data <DIR>"))? }),
        "jobs" => Ok(Command::Jobs),
        "help" | "-h" | "--help" => Ok(Command::Help),
        other => Err(data_err(format!("unknown command {other}\n\n{USAGE}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_refresh() {
        let c = parse(&s(&["refresh", "--out", "data", "--only", "nri,geography", "--keep-raw"])).unwrap();
        assert_eq!(
            c,
            Command::Refresh { out: "data".into(), only: vec!["nri".into(), "geography".into()], keep_raw: true }
        );
        let c = parse(&s(&["refresh", "--out=data", "--only=nri", "--only", "flood"])).unwrap();
        assert_eq!(c, Command::Refresh { out: "data".into(), only: vec!["nri".into(), "flood".into()], keep_raw: false });
    }

    #[test]
    fn parses_verify_and_errors() {
        assert_eq!(parse(&s(&["verify", "--data", "d"])).unwrap(), Command::Verify { data: "d".into() });
        assert!(parse(&s(&["verify"])).is_err());
        assert!(parse(&s(&["refresh", "--out", "d", "--bogus"])).is_err());
        assert_eq!(parse(&[]).unwrap(), Command::Help);
    }
}
