//! Calibration against the research worked examples (docs/research/risk-model.md §3.8, §8, §9).
//! Writes `target/calibration.md` (the number check the owner reads) and fails if a headline
//! number drifts, or if a research-table number drifts that is not a known, explained deviation.

mod support {
    pub mod report;
    pub mod research;
}

/// Research-table numbers that are expected to differ by more than 25 %, with the reason. Every
/// other number must match.
const KNOWN_DEVIATIONS: &[(&str, &str)] = &[
    (
        "Coos Bay Income gap at 1 in 50",
        "the research gave the 55- and 58-year-old earners longer spells; PlanInput has age bands, not ages",
    ),
    ("Coos Bay Income gap at 1 in 95", "as above"),
    ("Coos Bay Income gap at 1 in 500", "as above"),
    (
        "Philadelphia Heat or cold at 1 in 50",
        "more coupled classes than the prototype: county-wide winter outages and the ice storm of record also stop the furnace",
    ),
    ("Philadelphia Heat or cold at 1 in 500", "as above"),
    (
        "Philadelphia Boil-water notice at 1 in 500",
        "hurricane boil-water notices (Texas records) added to the prototype's single local class",
    ),
    (
        "Coos Bay Communications at 1 in 10",
        "the storm rows carry the Philadelphia duration for storm-caused phone outages (0.4 d / 1.75 d) where the prototype used 0.3 / 1.5 for Coos",
    ),
];

#[test]
fn calibration_matches_the_research() {
    let (md, checks) = support::report::calibration_report();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/calibration.md");
    std::fs::write(path, &md).expect("write target/calibration.md");

    let mut unexplained = Vec::new();
    for c in &checks {
        let known = KNOWN_DEVIATIONS.iter().any(|(label, _)| *label == c.label);
        if !c.ok && !known {
            unexplained.push(format!(
                "{}: research {:.3}, ours {:.3} (±{:.3})",
                c.label, c.research, c.ours, c.tolerance
            ));
        }
    }
    assert!(
        unexplained.is_empty(),
        "drifted from the research:\n{}",
        unexplained.join("\n")
    );
    // Known deviations must still be deviations; if one starts matching, take it off the list.
    for (label, _) in KNOWN_DEVIATIONS {
        let c = checks
            .iter()
            .find(|c| c.label == *label)
            .expect("known check exists");
        assert!(
            !c.ok,
            "{label} now matches the research; remove it from KNOWN_DEVIATIONS"
        );
    }
}
