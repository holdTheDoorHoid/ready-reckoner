//! `docs/QUANTITY_RULES.md` is what `rr-content` validates catalogue items against, so its table
//! must list exactly the rules this crate implements (plus rows the content workstream has asked
//! for, which must be answered before merge).

use std::collections::BTreeSet;

const DOC: &str = include_str!("../../../docs/QUANTITY_RULES.md");

/// The body of the "Rules" section (up to the next heading).
fn rules_section() -> &'static str {
    let body = DOC.split("\n## Rules\n").nth(1).expect("a Rules section");
    body.split("\n## ").next().unwrap()
}

/// Rule ids from the Rules table: the first cell of each row, in backticks.
fn documented() -> Vec<(String, String)> {
    let section = rules_section();
    section
        .lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
            (
                cells[0].trim_matches('`').to_owned(),
                cells.last().copied().unwrap_or("").to_owned(),
            )
        })
        .collect()
}

#[test]
fn the_rules_table_matches_the_code() {
    let doc = documented();
    let doc_ids: BTreeSet<&str> = doc.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(doc_ids.len(), doc.len(), "a rule is listed twice");
    let code: BTreeSet<&str> = rr_supply::RULE_IDS.iter().copied().collect();
    let missing: Vec<_> = code.difference(&doc_ids).collect();
    assert!(
        missing.is_empty(),
        "rules not in docs/QUANTITY_RULES.md: {missing:?}"
    );
    for (id, status) in &doc {
        if !code.contains(id.as_str()) {
            panic!(
                "docs/QUANTITY_RULES.md lists `{id}` ({status}) but rr-supply does not implement it"
            );
        }
    }
}

#[test]
fn every_emitted_rule_has_the_documented_unit() {
    let units: std::collections::BTreeMap<String, String> = {
        rules_section()
            .lines()
            .filter(|l| l.starts_with("| `"))
            .map(|l| {
                let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
                (cells[0].trim_matches('`').to_owned(), cells[6].to_owned())
            })
            .collect()
    };
    for (name, input) in rr_types::fixtures::all() {
        let targets = rr_supply_test_targets(name);
        for l in rr_supply::requirements(&input, &targets) {
            let unit = units
                .get(&l.rule)
                .unwrap_or_else(|| panic!("{} undocumented", l.rule));
            assert!(
                unit.split(['(', ',', ' ']).any(|u| u == l.unit),
                "{name}: {} has unit {} but the table says {unit}",
                l.id,
                l.unit
            );
        }
    }
}

mod common;

fn rr_supply_test_targets(name: &str) -> Vec<rr_types::BucketAssessment> {
    common::targets_for(name)
}
