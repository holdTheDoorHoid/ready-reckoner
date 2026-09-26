//! Keeps docs/RISK_MODEL.md ("Consequences and targets") in step with the code: the effects table
//! there is generated from `effects.toml`, and every citation id the crate emits is listed for
//! `content/citations.toml`.
//!
//! After changing `effects.toml`, rewrite the table with
//! `RR_WRITE_DOCS=1 cargo test -p rr-consequence --test docs`.

use std::fmt::Write as _;

use rr_consequence::{EffectRow, table};
use rr_types::{BucketKind, DurationDist, Evidence};

const DOC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/RISK_MODEL.md");
const START: &str =
    "<!-- effects-table:start (generated from crates/rr-consequence/src/effects.toml) -->";
const END: &str = "<!-- effects-table:end -->";

fn days(d: f64) -> String {
    if d < 1.0 {
        format!("{:.0} h", d * 24.0)
    } else if d < 10.0 {
        let r = (d * 10.0 + 0.5).floor() / 10.0;
        if r.fract() == 0.0 {
            format!("{r:.0} d")
        } else {
            format!("{r:.1} d")
        }
    } else {
        format!("{d:.0} d")
    }
}

fn row_line(r: &EffectRow) -> String {
    let owner = match (&r.scenario, &r.variant) {
        (Some(s), Some(v)) => format!("*{s}* ({v})"),
        (Some(s), None) => format!("*{s}*"),
        (None, _) => r.hazard.as_str().to_owned(),
    };
    let (median, p90) = match r.duration {
        Some(DurationDist::LogNormal {
            median_days,
            p90_days,
        }) => (days(median_days), days(p90_days)),
        Some(DurationDist::Fixed { days: d }) => (days(d), days(d)),
        None => ("—".to_owned(), "—".to_owned()),
    };
    let evidence = match (r.evidence, r.duration_basis()) {
        (Evidence::Empirical, _) => "data",
        (Evidence::Prior, Evidence::Empirical) => "duration data, share prior",
        (Evidence::Prior, Evidence::Prior) => "prior",
    };
    let mut extra = Vec::new();
    if let Some(part) = &r.part {
        let label = table()
            .part(r.hazard, part)
            .map_or(part.as_str(), |p| p.label.as_str());
        extra.push(format!("share of the {label} (part `{part}`)"));
    }
    if let Some(req) = r.requires {
        extra.push(req.describe().to_owned());
    }
    if let Some(req) = r.also_requires {
        extra.push(req.describe().to_owned());
    }
    if r.fragile {
        extra.push("scaled by how easily the water system breaks".to_owned());
    }
    if !r.county_rate_keys.is_empty() {
        extra.push(format!(
            "county-scale: share of the county's damaging {} episodes",
            r.county_rate_keys.join(" and ")
        ));
    }
    if let Some(f) = &r.fixed_rate {
        extra.push(format!("rate from `{f}`"));
    }
    if let Some(c) = &r.curve_class {
        extra.push(format!("regional restoration: {c}"));
    }
    if r.island_curve {
        extra.push("island grids: historic curve".to_owned());
    }
    if r.pool {
        extra.push("county outage records replace the county-wide part".to_owned());
    }
    if !r.event_keys.is_empty() {
        extra.push(format!("county events: {}", r.event_keys.join(", ")));
    }
    if r.heat_share > 0.0 {
        extra.push(format!("in heat {}", r.heat_share));
    }
    if r.cold_share > 0.0 {
        extra.push(format!("in cold {}", r.cold_share));
    }
    if let Some(s) = &r.replaced_by {
        extra.push(format!("dropped when *{s}* is offered"));
    }
    if let Some([lo, hi]) = r.notice_hours {
        extra.push(format!("warning {lo}–{hi} h"));
    }
    if let Some(rel) = &r.relief {
        extra.push(format!(
            "relief: help {} d, mostly restored {} d",
            rel.help_arrives_days, rel.mostly_restored_days
        ));
    }
    let sources: Vec<&str> = r.sources.iter().map(|s| s.as_str()).collect();
    let dur_cols = if r.bucket.kind() == BucketKind::Duration || r.duration.is_some() {
        format!("{median} | {p90}")
    } else {
        "— | —".to_owned()
    };
    format!(
        "| {owner} | {} | {} | {} | {} | {dur_cols} | {evidence} | {} | {} |",
        r.bucket,
        r.class,
        r.label,
        r.p_given_event,
        extra.join("; "),
        sources.join(", ")
    )
}

fn generated_table() -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{START}");
    let _ = writeln!(
        out,
        "| Hazard or *scenario* | Bucket | Class | Event class in words | Share | Median | Bad case (p90) | Evidence | Applies / notes | Sources |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|---|");
    for r in &table().effects {
        let _ = writeln!(out, "{}", row_line(r));
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Income stream | Per earner | Spell median | Spell bad case | Unemployment insurance | Evidence | Sources |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|");
    for s in &table().income {
        let (m, p) = match s.spell_weeks {
            DurationDist::LogNormal {
                median_days,
                p90_days,
            } => (median_days, p90_days),
            DurationDist::Fixed { days } => (days, days),
        };
        let owner = s
            .scenario
            .as_ref()
            .map_or_else(|| s.hazard.as_str().to_owned(), |x| format!("*{x}*"));
        let sources: Vec<&str> = s.sources.iter().map(|c| c.as_str()).collect();
        let _ = writeln!(
            out,
            "| {owner}: {} | {} | {m} wk | {p} wk | {} | {} | {} |",
            s.label,
            s.per_earner,
            if s.unemployment_insurance {
                "yes"
            } else {
                "no"
            },
            s.evidence,
            sources.join(", ")
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Hazard | Part | Events it counts | Share when no split is passed | Why |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    for p in &table().parts {
        let _ = writeln!(
            out,
            "| {} | `{}` | {} | {} | {} |",
            p.hazard, p.part, p.label, p.fallback_share, p.note
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Scenario | Parent hazard whose ordinary rows give up the scenario's long-run share | Why |"
    );
    let _ = writeln!(out, "|---|---|---|");
    for o in &table().overlaps {
        let _ = writeln!(out, "| *{}* | {} | {} |", o.scenario, o.hazard, o.note);
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Water failure on record (stress line) | Bucket | States | Began | Median | 9 in 10 back | Sources |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|");
    for e in &table().water_events {
        let sources: Vec<&str> = e.sources.iter().map(|c| c.as_str()).collect();
        let _ = writeln!(
            out,
            "| {}, {} | {} | {} | {} | {} | {} | {} |",
            e.event,
            e.place,
            e.bucket,
            e.states.join(", "),
            e.date,
            days(e.median_days),
            days(e.p90_days),
            sources.join(", ")
        );
    }
    let _ = write!(out, "{END}");
    out
}

#[test]
fn risk_model_doc_carries_the_generated_effects_table() {
    let doc = std::fs::read_to_string(DOC).expect("docs/RISK_MODEL.md exists");
    let want = generated_table();
    let (Some(a), Some(b)) = (doc.find(START), doc.find(END)) else {
        panic!("docs/RISK_MODEL.md lacks the effects-table markers");
    };
    let have = &doc[a..b + END.len()];
    if have != want {
        if std::env::var("RR_WRITE_DOCS").is_ok() {
            let updated = format!("{}{}{}", &doc[..a], want, &doc[b + END.len()..]);
            std::fs::write(DOC, updated).expect("write docs/RISK_MODEL.md");
            return;
        }
        panic!(
            "the effects table in docs/RISK_MODEL.md is out of date; run \
             `RR_WRITE_DOCS=1 cargo test -p rr-consequence --test docs`"
        );
    }
}

#[test]
fn every_citation_id_is_registered_or_requested() {
    let registry = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/citations.toml"
    ))
    .unwrap_or_default();
    let ids_doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/CITATION_IDS.md"
    ))
    .expect("docs/CITATION_IDS.md exists");
    let section = ids_doc
        .split("## Used by consequence")
        .nth(1)
        .expect("a 'Used by consequence' section");
    for id in rr_consequence::citation_ids() {
        assert!(id.is_well_formed(), "{id}");
        let in_registry = registry.contains(&format!("id = \"{id}\""));
        let listed = section.contains(&format!("`{id}`"));
        assert!(
            in_registry || listed,
            "{id} is neither in content/citations.toml nor listed under 'Used by consequence' in docs/CITATION_IDS.md"
        );
    }
}
