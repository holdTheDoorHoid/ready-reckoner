//! The Prepare sheet (DESIGN-DELTA-v3 §4: `PlanOutput::prepare_markdown`): the v2 packet's
//! preparation content. Its sections, sources and life-safety rules are checked in
//! `tests/binder.rs` and `tests/round2.rs`; this file holds the checks of what its lists say.

mod common;

use common::outputs;
use rr_types::{PlanInput, PlanOutput};

/// The part of the Prepare sheet from one `##` heading to the next.
fn section<'a>(p: &'a str, heading: &str) -> Option<&'a str> {
    let start = p.find(&format!("\n{heading}\n"))? + 1;
    let rest = &p[start..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |e| e + 3);
    Some(&rest[..end])
}

fn fixture(name: &str) -> &'static (&'static str, PlanInput, PlanOutput) {
    outputs()
        .iter()
        .find(|(n, _, _)| *n == name)
        .unwrap_or_else(|| panic!("no fixture {name}"))
}

/// The get-home bag line prints once, for each person, when every commuter's line is the same
/// (Sugar Land: both keep theirs in the car), and per person when they differ (Philadelphia: one
/// in the car, one at work); each commuter keeps their own trip and walk supplies.
#[test]
fn a_shared_get_home_bag_line_prints_once() {
    let sugar = section(
        &fixture("sugar-land-ev-household-3").2.prepare_markdown,
        "## Checklists",
    )
    .unwrap();
    assert_eq!(
        sugar.matches("get-home bag in the car").count(),
        1,
        "{sugar}"
    );
    assert!(sugar.contains("- [ ] For each person: keep a get-home bag in the car"));
    assert_eq!(sugar.matches("- [ ] For the walk, from home: ").count(), 2);
    let phl = section(
        &fixture("philadelphia-renters-4").2.prepare_markdown,
        "## Checklists",
    )
    .unwrap();
    assert!(phl.contains("- [ ] Keep a get-home bag in the car"));
    assert!(phl.contains("- [ ] Keep a get-home bag at work or in your daily bag"));
    assert!(!phl.contains("For each person: keep a get-home bag"));
}

/// The decisions and the savings goal stay on the Prepare sheet (the four-part kit, the cash and
/// the cost of a damaged home are the binder's Documents and money page).
#[test]
fn decisions_and_savings_stay_on_the_prepare_sheet() {
    for (name, input, out) in outputs() {
        let s = section(
            &out.prepare_markdown,
            "## Documents and money: decisions and savings",
        )
        .unwrap_or_else(|| panic!("{name}: no decisions and savings"));
        assert!(s.contains("### Savings"), "{name}");
        assert!(!s.contains("Emergency Financial First Aid Kit"), "{name}");
        if input.finances.income.earners == 0 {
            assert!(s.contains("No one in the household earns wages"), "{name}");
        }
    }
}
