//! `rr validate`: the frozen backtest (`docs/VALIDATION.md`). Twenty-two real events, each with
//! the household the round-2 model review built for it, are planned four ways (the county-only
//! model, with the data pack v2 tables, with the tables and the answers a household there would
//! have given, and with the county's drinking-water record as it stood before the event), and each
//! plan's targets are scored against what happened. The table marks the events that sit inside
//! the records the model learned from (in-sample), and the tally of the third run is the
//! headline the app's validation page shows.
//!
//! Exits 1 when any verdict differs from the one recorded in `fixtures/backtest/events.json`:
//! worse is a regression; better has to be recorded (with `docs/VALIDATION.md`) before the
//! published tally can claim it. The inputs are frozen files in the repository, so the verdicts
//! move only with the model, never with a data refresh.

use std::path::{Path, PathBuf};

use rr_plan::validation::{self, Backtest};

use crate::Output;
use crate::args::ValidateArgs;
use crate::error::{CliError, Exit};
use crate::format::{Table, wrap};

/// The repository the CLI was built from: the frozen backtest files live there.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The run names as table headings.
const RUN_HEADINGS: [&str; 4] = [
    "County only",
    "With v2 tables",
    "With v2 answers",
    "Pre-event water record",
];

/// Runs `rr validate`.
///
/// # Errors
///
/// A frozen file that cannot be read or parsed, or an event whose household or county is missing
/// (exit 1). A changed verdict is a result (exit 1 with the report), not an error.
pub fn run(args: &ValidateArgs) -> Result<Output, CliError> {
    let bt = Backtest::load(&repo()).map_err(|e| {
        CliError::failure(format!(
            "The frozen backtest could not be read: {e}\n  `rr validate` runs from a checkout of \
             the repository it was built from."
        ))
    })?;
    let report = bt.run().map_err(CliError::failure)?;
    let f = &bt.frozen;
    let mut s = format!(
        "rr validate: the frozen backtest, {} real events (docs/VALIDATION.md)\n",
        f.events.len()
    );
    s.push_str(&format!(
        "Targets at the 1-in-100 setting from frozen inputs (county records from data pack {}, \
         the v2 tables from {}), scored against what happened.\n\n",
        bt.counties.pack_version, bt.regional.pack_version
    ));

    let mut t = Table::new(std::iter::once("Event").chain(RUN_HEADINGS.iter().copied()));
    for r in &report.rows {
        let star = if r.in_sample.is_empty() { "" } else { " *" };
        let mut cells = vec![format!("{}{star}", r.name)];
        cells.extend(r.runs.iter().map(|run| run.verdict.word().to_owned()));
        t.row(cells);
        if args.details {
            for (k, run) in r.runs.iter().enumerate() {
                t.note(format!("{}: {}", RUN_HEADINGS[k], run.cells.join("; ")));
            }
            if !r.in_sample.is_empty() {
                t.note(format!("* In-sample: {}.", r.in_sample));
            }
        }
    }
    s.push_str(&t.render(1));
    s.push_str(
        "\n * In-sample: the event is inside the records the model learned from, so it is a \
         weaker test (docs/VALIDATION.md).\n",
    );
    if !args.details {
        s.push_str(" --details shows each target against what happened.\n");
    }
    s.push('\n');

    let mut tally = Table::new(["Run", "Short", "Partial", "Covered", "Over", "Not modeled"])
        .right(1)
        .right(2)
        .right(3)
        .right(4)
        .right(5);
    for (k, name) in RUN_HEADINGS.iter().enumerate() {
        let c = report.tally[k];
        tally.row([
            (*name).to_owned(),
            c[0].to_string(),
            c[1].to_string(),
            c[2].to_string(),
            c[3].to_string(),
            c[4].to_string(),
        ]);
    }
    s.push_str(&tally.render(1));

    let h = f.headline_run.min(3);
    let c = report.tally[h];
    s.push_str(&format!(
        "\n Headline ({}): {} covered, {} partial, {} short, {} not modeled (over counts as \
         covered).\n",
        f.runs[h],
        c[2] + c[3],
        c[1],
        c[0],
        c[4]
    ));
    let published = validation::summary();
    if (
        published.covered,
        published.partial,
        published.short,
        published.not_modelled,
    ) != (c[2] + c[3], c[1], c[0], c[4])
    {
        s.push_str(&format!(
            " The app publishes {} covered, {} partial, {} short, {} not modeled from the \
             recorded verdicts.\n",
            published.covered, published.partial, published.short, published.not_modelled
        ));
    }

    let total = report.rows.len() * RUN_HEADINGS.len();
    if report.changed.is_empty() {
        s.push_str(&format!(" All {total} recorded verdicts reproduced.\n"));
        return Ok(Output::text(s));
    }
    let worse = report
        .changed
        .iter()
        .filter(|c| c.now.rank() < c.recorded.rank())
        .count();
    s.push_str(&format!(
        "\n {} of {total} verdicts differ from the recorded ones ({worse} worse, {} better):\n",
        report.changed.len(),
        report.changed.len() - worse
    ));
    for c in &report.changed {
        let direction = if c.now.rank() < c.recorded.rank() {
            "WORSE"
        } else {
            "better"
        };
        let line = format!(
            "{direction}  {} ({}): recorded {}, now {} ({})",
            c.event,
            RUN_HEADINGS[c.run].to_lowercase(),
            c.recorded.word(),
            c.now.word(),
            c.detail
        );
        s.push_str(&format!("   {}\n", wrap(&line, 96, 11)));
    }
    s.push_str(
        "\n A model or data change moved these. If the change is intended, record the new \
         verdicts in fixtures/backtest/events.json, crates/rr-consequence/tests/backtest.rs and \
         docs/VALIDATION.md together.\n",
    );
    Ok(Output {
        stdout: s,
        notes: vec![format!(
            "rr validate: {} verdict{} changed",
            report.changed.len(),
            if report.changed.len() == 1 { "" } else { "s" }
        )],
        exit: Exit::Failure,
    })
}
