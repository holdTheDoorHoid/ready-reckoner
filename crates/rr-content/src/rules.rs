//! The quantity-rule registry in `docs/QUANTITY_RULES.md`.
//!
//! The file is shared with the supply workstream: items name a rule, `rr-supply` implements it.
//! The table's first column holds the rule name in backticks; rows are recognised by that shape,
//! so prose, headings and the header row are ignored.

use std::collections::BTreeSet;

/// The rule name every item may use without a table row: one per household.
pub const BUILT_IN_ONCE: &str = "once";

/// Rule names in a Markdown table whose first column is `` `rule_name` ``. `once` is always
/// included.
pub fn parse_rule_table(markdown: &str) -> BTreeSet<String> {
    let mut rules = BTreeSet::new();
    rules.insert(BUILT_IN_ONCE.to_owned());
    for line in markdown.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('|') else {
            continue;
        };
        let Some(first) = rest.split('|').next() else {
            continue;
        };
        let cell = first.trim();
        if let Some(name) = cell.strip_prefix('`').and_then(|c| c.strip_suffix('`'))
            && rr_types::is_well_formed_id(name)
        {
            rules.insert(name.to_owned());
        }
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_backticked_first_cells_only() {
        let md = "# Rules\n\n| Rule | Inputs |\n|---|---|\n| `water_gallons` | days |\n\
                  | `per_person` | people |\n| not a rule | x |\n| `Bad Name` | y |\n";
        let r = parse_rule_table(md);
        assert!(r.contains("water_gallons"));
        assert!(r.contains("per_person"));
        assert!(r.contains("once"));
        assert!(!r.contains("Bad Name"));
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn the_embedded_registry_has_the_core_rules() {
        let r = parse_rule_table(crate::quantity_rules_markdown());
        for name in [
            "water_gallons",
            "food_kcal",
            "medication_days",
            "per_person",
        ] {
            assert!(r.contains(name), "missing {name}");
        }
    }
}
