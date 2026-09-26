//! `rr doctor`: runs every fixture household through the engine on the chosen source and reports
//! how long `assess` takes (the target is under 50 ms in a native release build), whether the
//! output is byte-for-byte the same on every run, the guardrail warnings, and anything that
//! reaches the household without a source: catalogue items or plan steps with no citation,
//! requirement lines, hazards or targets with no sources, and placeholder sources.
//!
//! Fails (exit 1) on a fixture that cannot be planned, output that changes between runs, content
//! that fails validation, or anything uncited. Timing is reported, never failed: CI machines and
//! debug builds are slower than the target describes.

use std::time::{Duration, Instant};

use rr_plan::Engine;
use rr_types::{PlanItemKind, PlanOutput, Target};

use crate::Output;
use crate::args::DoctorArgs;
use crate::error::{CliError, Exit};
use crate::format::{Table, wrap};
use crate::source::Source;

/// The `assess` time budget in a native release build (`docs/ENGINE-API.md`).
pub const TARGET_MS: f64 = 50.0;

/// What one fixture's check found.
struct Report {
    name: &'static str,
    place: String,
    median: Duration,
    max: Duration,
    deterministic: bool,
    warnings: Vec<rr_types::Warning>,
    uncited: Vec<String>,
    placeholders: Vec<String>,
}

/// Runs `rr doctor`.
///
/// # Errors
///
/// Never for a fixture problem (that is the report); only if the content cannot be read.
pub fn run(engine: &Engine<Source>, args: &DoctorArgs) -> Result<Output, CliError> {
    let content = engine.content();
    let build = if cfg!(debug_assertions) {
        "debug build"
    } else {
        "release build"
    };
    let fixtures = rr_types::fixtures::all();
    let mut s = format!(
        "rr doctor: {} fixture households on {}\nEngine {} (API {}), content {}, {build}, {} timed \
         run{} each\n\n",
        fixtures.len(),
        engine.store().describe(),
        rr_plan::ENGINE_VERSION,
        rr_types::ENGINE_API_VERSION,
        rr_content::CONTENT_VERSION,
        args.runs,
        if args.runs == 1 { "" } else { "s" }
    );
    let mut problems: Vec<String> = Vec::new();

    // Content: the validator the content tests run, and every catalogue item cites a real source.
    let validation = rr_content::validate_embedded();
    let errors: Vec<String> = validation.errors().map(|f| f.to_string()).collect();
    let warnings = validation.warnings().count();
    s.push_str(&format!(
        "  {}  content validation: {} error{}, {warnings} warning{}\n",
        if errors.is_empty() { "ok" } else { "!!" },
        errors.len(),
        if errors.len() == 1 { "" } else { "s" },
        if warnings == 1 { "" } else { "s" }
    ));
    problems.extend(errors.into_iter().map(|e| format!("content: {e}")));
    let mut uncited_items = 0usize;
    for it in &content.items {
        if it.citations.is_empty() {
            uncited_items += 1;
            problems.push(format!("catalogue item {} has no citation", it.id));
        }
        for c in &it.citations {
            if content.citation(c.as_str()).is_none()
                && !rr_plan::provenance::AWAITING_CONTENT.contains(&c.as_str())
            {
                uncited_items += 1;
                problems.push(format!(
                    "catalogue item {} cites {c}, which is not in content/citations.toml",
                    it.id
                ));
            }
        }
    }
    s.push_str(&format!(
        "  {}  catalogue: {} items, {} without a resolvable citation\n\n",
        if uncited_items == 0 { "ok" } else { "!!" },
        content.items.len(),
        uncited_items
    ));

    let mut reports: Vec<Report> = Vec::new();
    for (name, input) in &fixtures {
        // The first run parses nothing new (content is parsed once per process) but warms caches;
        // its bytes are the reference for the determinism check.
        let first = match engine.assess(input) {
            Ok(o) => o,
            Err(e) => {
                problems.push(format!("{name}: the engine could not plan it: {e}"));
                continue;
            }
        };
        let reference = rr_plan::to_json(&first);
        let mut times: Vec<Duration> = Vec::with_capacity(args.runs as usize);
        let mut deterministic = true;
        for _ in 0..args.runs {
            let t0 = Instant::now();
            let out = engine.assess(input);
            times.push(t0.elapsed());
            match out {
                Ok(o) => deterministic &= rr_plan::to_json(&o) == reference,
                Err(_) => deterministic = false,
            }
        }
        times.sort();
        let (uncited, placeholders) = uncited(&first, content);
        if !deterministic {
            problems.push(format!("{name}: the output changed between runs"));
        }
        problems.extend(uncited.iter().map(|u| format!("{name}: {u}")));
        for p in &placeholders {
            if !rr_plan::provenance::AWAITING_CONTENT.contains(&p.as_str()) {
                problems.push(format!("{name}: placeholder source for {p}"));
            }
        }
        reports.push(Report {
            name,
            place: format!(
                "{}, {}",
                first.location.county_name, first.location.state_abbr
            ),
            median: times[times.len() / 2],
            max: times[times.len() - 1],
            deterministic,
            warnings: first.warnings.clone(),
            uncited,
            placeholders,
        });
    }

    let mut t = Table::new([
        "Fixture",
        "County",
        "Median ms",
        "Max ms",
        "< 50 ms",
        "Same bytes",
        "Warnings",
        "Uncited",
    ])
    .right(2)
    .right(3)
    .right(6)
    .right(7);
    let mut slowest = Duration::ZERO;
    for r in &reports {
        slowest = slowest.max(r.median);
        t.row([
            r.name.to_owned(),
            r.place.clone(),
            ms(r.median),
            ms(r.max),
            if millis(r.median) < TARGET_MS {
                "yes"
            } else {
                "no"
            }
            .to_owned(),
            if r.deterministic { "yes" } else { "NO" }.to_owned(),
            r.warnings.len().to_string(),
            r.uncited.len().to_string(),
        ]);
    }
    s.push_str(&t.render(1));
    s.push_str(&format!(
        "\n Slowest median: {} ms (target: under {TARGET_MS:.0} ms in a native release build).\n",
        ms(slowest)
    ));
    if cfg!(debug_assertions) {
        s.push_str(
            " This is a debug build, several times slower than release; check the target with \
             `cargo run --release -p rr-cli -- doctor`.\n",
        );
    }

    s.push_str("\nWarnings the households see (warn, never block)\n\n");
    for r in &reports {
        if r.warnings.is_empty() {
            s.push_str(&format!("  {}: none\n", r.name));
            continue;
        }
        s.push_str(&format!("  {}:\n", r.name));
        for w in &r.warnings {
            let line = format!("{:<4}  {}: {}", w.severity.as_str(), w.id, w.message);
            s.push_str(&format!("    {}\n", wrap(&line, 96, 10)));
        }
    }
    let placeholders: Vec<String> = reports
        .iter()
        .flat_map(|r| {
            r.placeholders
                .iter()
                .map(move |p| format!("{} ({})", p, r.name))
        })
        .collect();
    if !placeholders.is_empty() {
        s.push_str(&format!(
            "\n  Placeholder sources (ids awaited in docs/CITATION_IDS.md): {}\n",
            placeholders.join(", ")
        ));
    }

    let mut out = Output::default();
    if problems.is_empty() {
        s.push_str(
            "\nResult: OK. Every fixture plans, repeats byte for byte, and cites every number.\n",
        );
    } else {
        s.push_str(&format!("\nResult: {} problem(s)\n", problems.len()));
        for p in &problems {
            s.push_str(&format!("  - {}\n", wrap(p, 92, 4)));
        }
        out.exit = Exit::Failure;
    }
    out.stdout = s;
    Ok(out)
}

/// What in a plan reaches the household without a source, and which sources are placeholders.
fn uncited(o: &PlanOutput, content: &rr_content::Content) -> (Vec<String>, Vec<String>) {
    let mut v = Vec::new();
    for m in &o.plan.months {
        for i in &m.items {
            if i.kind == PlanItemKind::Reserve {
                continue;
            }
            match content.item(i.item_id.as_str()) {
                Some(it) if it.citations.is_empty() => {
                    v.push(format!(
                        "plan step {} (month {}) has no citation",
                        i.item_id, m.index
                    ));
                }
                Some(_) => {}
                None => v.push(format!(
                    "plan step {} (month {}) is not in the catalogue",
                    i.item_id, m.index
                )),
            }
        }
    }
    for l in &o.requirements {
        if l.citations.is_empty() && l.quantity > 0.0 {
            v.push(format!("requirement line {} has no citation", l.id));
        }
    }
    for p in &o.register {
        if p.sources.is_empty() {
            v.push(format!("hazard {} has no source", p.id));
        }
    }
    for b in &o.buckets {
        let active = match b.target {
            Target::Days { value, .. } | Target::Months { value, .. } => value > 0.0,
            Target::Evacuate { p_need_10yr, .. } | Target::Readiness { p_need_10yr, .. } => {
                p_need_10yr > 0.0
            }
        };
        if active && b.sources.is_empty() {
            v.push(format!("the {} target has no source", b.id));
        }
    }
    let placeholders = o
        .provenance
        .iter()
        .filter(|c| content.citation(c.id.as_str()).is_none())
        .map(|c| c.id.as_str().to_owned())
        .collect();
    (v, placeholders)
}

fn millis(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn ms(d: Duration) -> String {
    let m = millis(d);
    if m < 10.0 {
        format!("{m:.1}")
    } else {
        format!("{m:.0}")
    }
}
