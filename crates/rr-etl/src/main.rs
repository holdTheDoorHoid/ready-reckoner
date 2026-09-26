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
            keep_intermediate,
        } => {
            let http = Http::new(out.join("raw"), keep_raw)?;
            let ctx = Ctx {
                data: out,
                http,
                keep_intermediate,
            };
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
        Command::Rehash { data } => {
            let mut manifest = rr_etl::manifest::Manifest::load(&data)?;
            let before = manifest.clone();
            let changed = manifest.rehash(&data)?;
            if manifest == before {
                println!(
                    "manifest: every entry matches the files on disk (pack version {})",
                    manifest.pack_version
                );
                return Ok(0);
            }
            manifest.save(&data)?;
            let short = |s: &str| s.get(..12).unwrap_or(s).to_string();
            for c in &changed {
                println!(
                    "  {}: {} -> {} rows, sha256 {} -> {}",
                    c.path,
                    c.old_rows,
                    c.rows,
                    short(&c.old_sha256),
                    short(&c.sha256)
                );
            }
            if !changed.is_empty() || manifest.pack_version != before.pack_version {
                rr_etl::changes::append_rehash(&data, &manifest, &changed)?;
            }
            println!(
                "manifest: {} file entries recomputed; pack version {} -> {}",
                changed.len(),
                before.pack_version,
                manifest.pack_version
            );
            Ok(0)
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
