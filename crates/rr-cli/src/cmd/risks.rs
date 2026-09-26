//! `rr risks`: the risk register as the app and the packet show it. Ranked hazards first (most
//! important first), each with its natural-frequency sentence and sources; then the rare but
//! severe ones in their own box, where how likely and how bad are separate columns and nothing is
//! ranked by expected loss (DESIGN §4.2); then named scenarios, the notes behind the numbers, and
//! every source.

use rr_plan::Engine;
use rr_types::{CitationId, HazardDisplay, HazardProfile};

use super::{header, source_line, sources_legend};
use crate::Output;
use crate::args::HouseholdArgs;
use crate::error::CliError;
use crate::format::{self, Table, households, sig2, wrap};
use crate::household;
use crate::source::Source;

/// Width the sentences under each row wrap at.
const WRAP: usize = 96;

/// Runs `rr risks`.
///
/// # Errors
///
/// Household problems or an unknown location (exit 2); an engine error (exit 1).
pub fn run(engine: &Engine<Source>, args: &HouseholdArgs) -> Result<Output, CliError> {
    let h = household::load(args)?;
    let a = engine
        .run(&h.input)
        .map_err(|e| CliError::engine(&e, engine.store().location_hint().as_deref()))?;
    let years = f64::from(h.input.dials.horizon_years.max(1));
    let mut s = header(
        &format!("Risks for {}", super::place(&a.location)),
        &h,
        &a.location,
        engine.store(),
    );
    let mut cited: Vec<CitationId> = Vec::new();

    let ranked: Vec<&HazardProfile> = a
        .hazards
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::Ranked)
        .collect();
    let rare: Vec<&HazardProfile> = a
        .hazards
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::RareCatastrophic)
        .collect();

    s.push_str(&format!(
        "\nRanked, most important first ({} hazards)\n\n",
        ranked.len()
    ));
    let mut t = Table::new([
        "#",
        "Hazard",
        &format!("Next {years} years"),
        "Per year (range)",
        "How bad",
        "How sure",
    ])
    .right(0);
    for (i, p) in ranked.iter().enumerate() {
        t.row([
            (i + 1).to_string(),
            p.name.clone(),
            households(format::chance(p.rate_per_year, years)),
            rate_cell(p),
            format::severity(p.severity).to_owned(),
            format::confidence(p.confidence).to_owned(),
        ]);
        t.note(p.frequency_sentence.clone());
        t.note(source_line(&p.sources));
        cited.extend(p.sources.iter().cloned());
    }
    s.push_str(&t.render(1));

    if !rare.is_empty() {
        s.push_str(
            "\nRare but severe (shown apart: how likely and how bad are separate columns, never \
             ranked by expected loss,\nand the plan never lets them take over the budget)\n\n",
        );
        let mut t = Table::new(["Hazard", "How likely (per year)", "How bad", "How sure"]);
        for p in &rare {
            t.row([
                p.name.clone(),
                rate_cell(p),
                format::severity(p.severity).to_owned(),
                format::confidence(p.confidence).to_owned(),
            ]);
            t.note(p.frequency_sentence.clone());
            t.note(source_line(&p.sources));
            cited.extend(p.sources.iter().cloned());
        }
        s.push_str(&t.render(1));
    }

    if !a.consequence.scenarios.is_empty() {
        s.push_str("\nNamed scenarios (turn one off with --scenario <id>=off)\n\n");
        for sc in &a.consequence.scenarios {
            let state = if sc.on { "in the plan" } else { "left out" };
            s.push_str(&format!("  {} ({}): {state}\n", sc.name, sc.id));
            s.push_str(&format!(
                "    {}\n",
                wrap(
                    &format!("{} {}", sc.applies_because, sc.effect_summary),
                    WRAP,
                    4
                )
            ));
            s.push_str(&format!("    {}\n", source_line(&sc.sources)));
            cited.extend(sc.sources.iter().cloned());
        }
    }

    if !a.hazards.notes.is_empty() {
        s.push_str("\nNotes on these numbers\n\n");
        for n in &a.hazards.notes {
            s.push_str(&format!("  - {}\n", wrap(n, WRAP, 4)));
        }
    }

    s.push('\n');
    s.push_str(&sources_legend(&cited, engine.content()));
    let mut out = Output::text(s);
    out.notes = household::scenario_notes(&h, &a.consequence.scenarios, &super::place(&a.location));
    Ok(out)
}

/// "0.083 (0.050–0.12)".
fn rate_cell(p: &HazardProfile) -> String {
    format!(
        "{} ({}–{})",
        sig2(p.rate_per_year),
        sig2(p.rate_range[0]),
        sig2(p.rate_range[1])
    )
}
