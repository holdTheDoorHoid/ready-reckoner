//! `rr-etl` command-line entry point. See [`rr_etl::cli::USAGE`].
#![forbid(unsafe_code)]

use rr_etl::cli::{Command, USAGE, parse};
use rr_etl::http::Http;
use rr_etl::jobs::{Ctx, JOBS, refresh};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            2
        }
    };
    std::process::exit(code);
}

fn run(args: &[String]) -> rr_etl::Result<i32> {
    match parse(args)? {
        Command::Help => {
            print!("{USAGE}");
            Ok(0)
        }
        Command::Jobs => {
            for j in JOBS {
                let mark = if j.default { "" } else { " (optional)" };
                println!("{:<16} {}{mark}", j.id, j.title);
            }
            Ok(0)
        }
        Command::Refresh {
            out,
            only,
            optional,
            keep_raw,
        } => {
            let http = Http::new(out.join("raw"), keep_raw)?;
            let ctx = Ctx { data: out, http };
            let summary = refresh(&ctx, &only, optional)?;
            println!(
                "refresh finished: {} job(s) ok, {} failed",
                summary.ok.len(),
                summary.failed.len()
            );
            for (id, e) in &summary.failed {
                println!("  {id}: {e}");
            }
            Ok(if summary.failed.is_empty() { 0 } else { 1 })
        }
        Command::Verify { data } => {
            let report = rr_etl::verify::verify(&data)?;
            for line in &report.lines {
                println!("{line}");
            }
            if report.problems.is_empty() {
                println!(
                    "verify: OK ({} files, {} checks)",
                    report.files, report.checks
                );
                Ok(0)
            } else {
                for p in &report.problems {
                    println!("PROBLEM: {p}");
                }
                println!("verify: {} problem(s)", report.problems.len());
                Ok(1)
            }
        }
    }
}
