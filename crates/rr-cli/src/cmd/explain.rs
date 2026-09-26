//! `rr explain <kind> <id>`: why a number is what it is, from the engine's `explain` function:
//! plain sentences first, then the arithmetic, then the sources. An id that names nothing in the
//! plan lists the ids that would work.

use rr_plan::Engine;
use rr_types::{BucketId, ErrorCode, ExplainKind, Explanation, PlanOutput};

use crate::Output;
use crate::args::ExplainArgs;
use crate::error::CliError;
use crate::format::wrap;
use crate::household;
use crate::source::Source;

/// Width paragraphs wrap at.
const WRAP: usize = 96;

/// Most ids suggested after an unknown one.
const MAX_SUGGESTIONS: usize = 12;

/// Runs `rr explain`.
///
/// # Errors
///
/// Household problems, an unknown location or an id that names nothing in the plan (exit 2); an
/// engine error (exit 1).
pub fn run(engine: &Engine<Source>, args: &ExplainArgs) -> Result<Output, CliError> {
    let h = household::load(&args.household)?;
    let hint = engine.store().location_hint();
    match engine.explain(args.kind, &args.id, &h.input) {
        Ok(e) => Ok(Output::text(if args.json {
            rr_plan::to_json(&e)
        } else {
            render(&e)
        })),
        Err(e)
            if e.code == ErrorCode::BadInput
                && e.problems()
                    .is_some_and(|p| p.iter().any(|p| p.field == "id")) =>
        {
            let output = engine
                .assess(&h.input)
                .map_err(|e| CliError::engine(&e, hint.as_deref()))?;
            let ids = candidates(engine, args.kind, &output);
            let close = suggest(&args.id, &ids);
            let mut message = format!(
                "There is no {} with the id \"{}\" in this plan.",
                args.kind, args.id
            );
            if !close.is_empty() {
                message.push_str(&format!("\n  Did you mean: {}", close.join(", ")));
            }
            let shown: Vec<&str> = ids.iter().take(40).map(String::as_str).collect();
            message.push_str(&format!(
                "\n  {} ids you can explain for this household{}:\n    {}",
                args.kind,
                if ids.len() > shown.len() {
                    format!(
                        " (first {} of {}; see `rr catalogue`)",
                        shown.len(),
                        ids.len()
                    )
                } else {
                    String::new()
                },
                wrap(&shown.join(", "), WRAP, 4)
            ));
            Err(CliError::input(message))
        }
        Err(e) => Err(CliError::engine(&e, hint.as_deref())),
    }
}

/// The explanation as text.
pub fn render(e: &Explanation) -> String {
    let mut s = format!("{}\n\n", e.title);
    for p in &e.plain {
        s.push_str(&wrap(p, WRAP, 0));
        s.push_str("\n\n");
    }
    if let Some(math) = &e.math {
        s.push_str("The arithmetic\n");
        for m in math {
            s.push_str(&format!("  - {}\n", wrap(m, WRAP, 4)));
        }
        s.push('\n');
    }
    if !e.sources.is_empty() {
        s.push_str("Sources\n");
        for c in &e.sources {
            let year = c.year.map(|y| format!(", {y}")).unwrap_or_default();
            let prior = if c.prior { " [expert estimate]" } else { "" };
            s.push_str(&format!(
                "  {}  {}. {}{year}.{prior} {}\n",
                c.id, c.title, c.publisher, c.url
            ));
        }
    }
    s
}

/// Every id of this kind the household's plan can explain.
fn candidates(engine: &Engine<Source>, kind: ExplainKind, out: &PlanOutput) -> Vec<String> {
    match kind {
        ExplainKind::Hazard => out.register.iter().map(|p| p.id.to_string()).collect(),
        ExplainKind::Bucket => BucketId::ALL.iter().map(|b| b.to_string()).collect(),
        ExplainKind::Item => {
            // Items in the plan first, then the rest of the catalogue.
            let mut v: Vec<String> = Vec::new();
            for m in &out.plan.months {
                for i in &m.items {
                    let id = i.item_id.as_str().to_owned();
                    if !v.contains(&id) {
                        v.push(id);
                    }
                }
            }
            for it in &engine.content().items {
                let id = it.id.as_str().to_owned();
                if !v.contains(&id) {
                    v.push(id);
                }
            }
            v
        }
        ExplainKind::Requirement => out.requirements.iter().map(|l| l.id.clone()).collect(),
        ExplainKind::Warning => out.warnings.iter().map(|w| w.id.clone()).collect(),
    }
}

/// Ids that look like the one typed: sharing a word, containing it, or contained in it.
fn suggest(typed: &str, ids: &[String]) -> Vec<String> {
    let t = typed.to_lowercase();
    let words: Vec<&str> = t.split(['_', '.']).filter(|w| w.len() >= 3).collect();
    let mut scored: Vec<(usize, &String)> = ids
        .iter()
        .filter_map(|id| {
            let l = id.to_lowercase();
            let score = if l.contains(&t) || t.contains(&l) {
                3
            } else if words.iter().any(|w| l.contains(w)) {
                2
            } else if l.chars().take(4).eq(t.chars().take(4)) {
                1
            } else {
                return None;
            };
            Some((score, id))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    scored
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, id)| id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions_prefer_containment_then_shared_words() {
        let ids: Vec<String> = ["heat_wave", "hurricane", "cold_wave", "house_fire"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        assert_eq!(suggest("hurrican", &ids), ["hurricane"]);
        assert_eq!(suggest("wave", &ids), ["cold_wave", "heat_wave"]);
        assert!(suggest("zzz", &ids).is_empty());
    }
}
