//! `rr citations`: the source registry (`content/citations.toml`), and with `--missing` every
//! citation id that something refers to but the registry does not define.
//!
//! "Something" is every place an id can come from without running a plan: the catalogue items,
//! the guidance blocks (front matter and `[^id]` footnotes), the glossary, the ids the hazards,
//! consequence, supply, budget and plan crates can emit, and the base rates of the loaded data.
//! Running a plan is avoided on purpose: in a debug build the engine panics on an unknown id, which
//! is exactly when this list is needed. An id listed in `rr_plan::provenance::AWAITING_CONTENT`
//! (requested in `docs/CITATION_IDS.md`, waiting for its entry) is reported but does not fail.

use std::collections::BTreeMap;

use rr_types::CitationId;

use crate::Output;
use crate::args::{CitationsArgs, DataArgs};
use crate::error::{CliError, Exit};
use crate::format::Table;
use crate::source;

/// Runs `rr citations`.
///
/// # Errors
///
/// If the content or the data pack cannot be read (exit 1). Missing ids are a result: exit 1
/// unless each is awaited.
pub fn run(data: &DataArgs, args: &CitationsArgs) -> Result<Output, CliError> {
    let content = rr_content::try_content()
        .map_err(|e| CliError::failure(format!("The built-in content could not be read: {e}")))?;
    if !args.missing {
        let prior = content.citations.iter().filter(|c| c.prior).count();
        let mut s = format!(
            "Sources: {} in content/citations.toml, {prior} of them expert estimates (content \
             {})\n\n",
            content.citations.len(),
            rr_content::CONTENT_VERSION
        );
        let mut t = Table::new(["Id", "Year", "Publisher", "Title"]);
        let mut sorted: Vec<&rr_types::Citation> = content.citations.iter().collect();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        for c in sorted {
            let title = if c.prior {
                format!("{} [expert estimate]", c.title)
            } else {
                c.title.clone()
            };
            t.row([
                c.id.as_str().to_owned(),
                c.year.map(|y| y.to_string()).unwrap_or_default(),
                c.publisher.clone(),
                title,
            ]);
        }
        s.push_str(&t.render(1));
        return Ok(Output::text(s));
    }

    let opened = source::open(data)?;
    for n in &opened.notes {
        eprintln!("{n}");
    }
    let refs = references(&opened.engine);
    let missing: Vec<(&String, &Vec<String>)> = refs
        .iter()
        .filter(|(id, _)| content.citation(id).is_none())
        .collect();
    let awaited = |id: &str| rr_plan::provenance::AWAITING_CONTENT.contains(&id);
    let mut s = format!(
        "Checked {} citation ids referred to by the content, the engine crates and the data ({}).\n",
        refs.len(),
        opened.engine.store().describe()
    );
    if missing.is_empty() {
        s.push_str("Every one is defined in content/citations.toml.\n");
        return Ok(Output::text(s));
    }
    s.push_str(&format!(
        "{} {} not defined in content/citations.toml:\n\n",
        missing.len(),
        if missing.len() == 1 { "is" } else { "are" }
    ));
    let mut t = Table::new(["Id", "Status", "Referred to by"]);
    for (id, by) in &missing {
        t.row([
            (*id).clone(),
            if awaited(id) {
                "awaited (requested in docs/CITATION_IDS.md)".to_owned()
            } else {
                "MISSING".to_owned()
            },
            by.join(", "),
        ]);
    }
    s.push_str(&t.render(1));
    let failing = missing.iter().filter(|(id, _)| !awaited(id)).count();
    let mut out = Output::text(s);
    if failing > 0 {
        out.exit = Exit::Failure;
        out.notes.push(format!(
            "{failing} citation id(s) are neither in content/citations.toml nor awaited: add them \
             to the registry, or request them in docs/CITATION_IDS.md and list them in \
             rr_plan::provenance::AWAITING_CONTENT."
        ));
    }
    Ok(out)
}

/// Every citation id referred to, with where from, sorted by id.
pub fn references(
    engine: &rr_plan::Engine<crate::source::Source>,
) -> BTreeMap<String, Vec<String>> {
    use rr_plan::CountySource;
    let content = engine.content();
    let mut refs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut add = |id: &str, by: &str| {
        let v = refs.entry(id.to_owned()).or_default();
        if !v.iter().any(|x| x == by) {
            v.push(by.to_owned());
        }
    };
    for it in &content.items {
        for c in &it.citations {
            add(c.as_str(), &format!("item {}", it.id));
        }
    }
    for g in &content.guidance {
        for c in &g.meta.citations {
            add(c.as_str(), &format!("guidance {}", g.meta.id));
        }
        for c in g.footnote_references() {
            add(&c, &format!("guidance {}", g.meta.id));
        }
    }
    for term in &content.glossary {
        for c in &term.citations {
            add(c.as_str(), &format!("glossary \"{}\"", term.term));
        }
    }
    for c in rr_hazards::CITATION_IDS {
        add(c, "rr-hazards");
    }
    for c in rr_consequence::citation_ids() {
        add(c.as_str(), "rr-consequence");
    }
    for c in rr_supply::citations_used() {
        add(c.as_str(), "rr-supply");
    }
    add(rr_budget::weights::HARM_WEIGHT_CITATION, "rr-budget");
    for c in rr_plan::coverage::COVERAGE_CITATIONS {
        add(c, "rr-plan");
    }
    for r in engine.store().base_rates() {
        let c: &CitationId = &r.source;
        add(c.as_str(), &format!("base rate {}", r.id));
    }
    refs
}
