//! `explain`: why a number is what it is, for a hazard, bucket, catalogue item, requirement line
//! or warning. Plain sentences first, then the arithmetic (the household event rate, Λ at the
//! target, the value integral, the quantity formula), then the sources.

use rr_content::Content;
use rr_types::{
    BucketId, BucketKind, CitationId, EngineError, ExplainKind, Explanation, HazardId, Problem,
    ProblemCode, Target,
};

use crate::coverage::Excluded;
use crate::packet::text;
use crate::pipeline::Assessment;
use crate::provenance;

fn not_found(kind: ExplainKind, id: &str) -> EngineError {
    EngineError::bad_input(vec![Problem::new(
        ProblemCode::Schema,
        "id",
        format!("There is no {kind} with the id \"{id}\" in this plan. Pick one from the plan."),
    )])
}

fn sources(ids: &[CitationId], content: &Content) -> Vec<rr_types::Citation> {
    let mut v: Vec<CitationId> = Vec::new();
    for id in ids {
        if !v.contains(id) {
            v.push(id.clone());
        }
    }
    provenance::resolve(&v, content).0
}

fn chance(rate: f64, years: f64) -> f64 {
    if rate <= 0.0 {
        0.0
    } else {
        -rr_types::math::exp_m1(-rate * years)
    }
}

/// Explains one thing in an assessed plan.
pub fn explain(
    a: &Assessment,
    content: &Content,
    kind: ExplainKind,
    id: &str,
) -> Result<Explanation, EngineError> {
    match kind {
        ExplainKind::Hazard => hazard(a, content, id),
        ExplainKind::Bucket => bucket(a, content, id),
        ExplainKind::Item => item(a, content, id),
        ExplainKind::Requirement => requirement(a, content, id),
        ExplainKind::Warning => warning(a, content, id),
    }
}

fn hazard(a: &Assessment, content: &Content, id: &str) -> Result<Explanation, EngineError> {
    let h: HazardId = id.parse().map_err(|_| not_found(ExplainKind::Hazard, id))?;
    let years = f64::from(a.input.dials.horizon_years.max(1));
    let Some(p) = a.hazards.profiles.iter().find(|p| p.id == h) else {
        return Ok(Explanation {
            title: h.name().to_owned(),
            plain: vec![format!(
                "{} is too rare where you live to list (under 1 in 100,000 a year), or does not \
                 apply to your household.",
                h.name()
            )],
            math: None,
            sources: Vec::new(),
        });
    };
    let mut plain = vec![
        p.frequency_sentence.clone(),
        rr_hazards::why_we_think_this(h).to_owned(),
    ];
    if let Some(s) = a.consequence.scenarios.iter().find(|s| {
        a.hazards
            .scenarios
            .iter()
            .any(|c| c.id == s.id && c.hazard == h)
    }) {
        plain.push(format!("{} {}", s.applies_because, s.effect_summary));
    }
    let mut math = vec![
        format!(
            "Household event rate r = λ · a · m = {:.4} a year (plausible range {:.4} to {:.4}).",
            p.rate_per_year, p.rate_range[0], p.rate_range[1]
        ),
        format!(
            "Chance of at least one in a year: 1 − e^(−r) = {:.4}.",
            p.annual_probability
        ),
        format!(
            "Of 100 households over {years} years: 100 · (1 − e^(−{years} · r)) = {:.1}.",
            100.0 * chance(p.rate_per_year, years)
        ),
        format!(
            "Severity on the fixed $50 to $500,000 scale: {:.2}.",
            p.severity
        ),
    ];
    if (p.climate_multiplier - 1.0).abs() > 1e-9 {
        math.push(format!(
            "Climate around 2050 multiplies the rate by {:.2}.",
            p.climate_multiplier
        ));
    }
    Ok(Explanation {
        title: p.name.clone(),
        plain,
        math: Some(math),
        sources: sources(&p.sources, content),
    })
}

fn bucket(a: &Assessment, content: &Content, id: &str) -> Result<Explanation, EngineError> {
    let b: BucketId = id.parse().map_err(|_| not_found(ExplainKind::Bucket, id))?;
    let ba = a.bucket(b);
    let mut plain = ba.frequency_sentences.clone();
    let mut math: Vec<String> = Vec::new();
    let rate = a.consequence.dial_rate;
    let weight = rr_budget::weights::harm_weight(b, &a.input).0;
    match ba.target {
        Target::Days { value, low, high } => {
            plain.insert(
                0,
                format!(
                    "Be ready for {} ({} is enough for this need).",
                    text::target_days(f64::from(value), f64::from(low), f64::from(high)),
                    text::lower_first(ba.tier_enough.name())
                ),
            );
            if let Some(d) = a.consequence.details.iter().find(|d| d.bucket == b) {
                let t = f64::from(value);
                math.push(format!(
                    "Λ(d) = Σ r_h · q_h,b · S_h,b(d): disruptions a year lasting longer than d \
                     days. Dial rate Λ* = {rate:.5} a year (about 1 in {}).",
                    a.input.dials.return_period.years()
                ));
                math.push(format!(
                    "Design duration: the smallest d with Λ(d) ≤ Λ* is {:.2} days; on the day \
                     ladder {} days (10th to 90th percentile {} to {}).",
                    d.target_days, value, low, high
                ));
                if t > 0.0 {
                    math.push(format!(
                        "Λ at the target: Λ({value}) = {:.5} a year.",
                        d.curve.lambda(t)
                    ));
                    let v = d.curve.value_between(0.0, t);
                    math.push(format!(
                        "Value of holding days 0 to {value}: ∫₀^{value} Λ(t) dt = {v:.4} \
                         disruption-days a year; over ten years at harm weight {weight}: 10 × \
                         {weight} × {v:.4} = {:.3}.",
                        10.0 * weight * v
                    ));
                }
                math.push(format!(
                    "Consumption: Σ r · q · E[D] = {:.3} days a year.",
                    d.consumption_days_per_year
                ));
                let table: Vec<String> = d
                    .dial_table
                    .iter()
                    .map(|p| format!("1 in {}: {} days", p.return_period_years, p.ladder_days))
                    .collect();
                math.push(format!("Targets at each setting: {}.", table.join("; ")));
            }
            let parts = a.offers.rule.parts_of(b);
            if !parts.is_empty() {
                let covered = crate::coverage::target_days(&ba.covered);
                let names: Vec<String> = parts.iter().map(|p| p.name.clone()).collect();
                math.push(format!(
                    "Covered by the plan: {covered} of {value} days, the weakest of its parts \
                     ({}).",
                    names.join(", ")
                ));
            }
        }
        Target::Months { value, low, high } => {
            plain.insert(
                0,
                format!(
                    "Aim for {} of expenses in savings.",
                    text::target_months(f64::from(value), f64::from(low), f64::from(high))
                ),
            );
            let d = &a.consequence.income;
            math.push(format!(
                "Months of income gap at the dial: {:.2}; on the months ladder {}.",
                d.target_months, d.ladder_months
            ));
            let table: Vec<String> = d
                .dial_table
                .iter()
                .map(|(y, _, l)| format!("1 in {y}: {l} months"))
                .collect();
            math.push(format!("Targets at each setting: {}.", table.join("; ")));
        }
        Target::Evacuate {
            p_need_10yr,
            notice_hours_low,
            notice_hours_high,
            days_away,
        } => {
            math.push(format!(
                "Ten-year chance of having to leave: P = 1 − e^(−10 · Σ r · q) = {p_need_10yr:.3} \
                 ({:.4} events a year).",
                a.consequence.evacuate.rate_per_year
            ));
            math.push(format!(
                "Warning: {} to {} hours; typical days away: {days_away}.",
                notice_hours_low, notice_hours_high
            ));
        }
        Target::Readiness {
            p_need_10yr,
            done,
            of,
        } => {
            math.push(format!(
                "Ten-year chance of needing it: P = 1 − e^(−10 · Σ r · q) = {p_need_10yr:.3}."
            ));
            if b.kind() == BucketKind::Readiness && of > 0 {
                plain.push(format!(
                    "The plan's checklist for it: {done} of {of} steps."
                ));
            }
        }
    }
    let mut ids = ba.sources.clone();
    if let Some(r) = &ba.relief {
        plain.push(format!(
            "For the kind of disruption behind this target, outside help plausibly arrives in \
             about {} and service is mostly back in about {}.",
            text::day_phrase(f64::from(r.help_arrives_days).round().max(0.5)),
            text::day_phrase(f64::from(r.mostly_restored_days).round().max(0.5))
        ));
        ids.extend(r.sources.iter().cloned());
    }
    if weight > 0.0 {
        ids.push(CitationId::from(rr_budget::weights::HARM_WEIGHT_CITATION));
    }
    Ok(Explanation {
        title: ba.name.clone(),
        plain,
        math: (!math.is_empty()).then_some(math),
        sources: sources(&ids, content),
    })
}

fn item(a: &Assessment, content: &Content, id: &str) -> Result<Explanation, EngineError> {
    let it = content
        .item(id)
        .ok_or_else(|| not_found(ExplainKind::Item, id))?;
    let mut plain = vec![it.spec.clone()];
    let mut math: Vec<String> = Vec::new();
    let mut ids = it.citations.clone();
    match a.offers.get(id) {
        Some(o) => {
            plain.push(format!(
                "Your household needs {}.",
                text::quantity(o.quantity, &it.unit).trim()
            ));
            if let Some((m, p)) = a
                .budget
                .plan
                .months
                .iter()
                .flat_map(|m| m.items.iter().map(move |i| (m.index, i)))
                .find(|(_, i)| i.item_id == id && i.kind != rr_types::PlanItemKind::Reserve)
            {
                plain.push(format!("In the plan in month {m}. {}", p.why));
                math.push(format!(
                    "Risk reduced (expected weighted disruption-days covered per decade): {:.3}.",
                    p.risk_reduction
                ));
            } else {
                plain.push(
                    "The plan does not buy it: other items meet the same need for less, or the \
                     need is already covered."
                        .to_owned(),
                );
            }
            for j in &o.joins {
                if let Some(l) = a.lines.iter().find(|l| l.line.id == j.line_id) {
                    if j.via_alternative && j.bucket.kind() != rr_types::BucketKind::Duration {
                        // A staging step: packed in the bag from household supplies.
                        math.push(format!(
                            "Counts toward {}: packed in it from household supplies, as part of \
                             that step rather than instead of it.",
                            l.line.id
                        ));
                    } else {
                        math.push(format!(
                            "Meets {}: needs {} {}; each {} counts as {:.3} {}.",
                            l.line.id,
                            text::number(l.quantity),
                            l.line.unit,
                            it.unit,
                            j.units_per_item,
                            l.line.unit
                        ));
                    }
                    math.extend(l.math.iter().cloned());
                    ids.extend(l.line.citations.iter().cloned());
                }
            }
        }
        None => {
            let why = a
                .offers
                .excluded
                .iter()
                .find(|(x, _)| x == id)
                .map(|(_, r)| *r);
            plain.push(
                match why {
                    Some(Excluded::NotNeeded) => {
                        "Your household does not need it (its sizing rule gives zero for you)."
                    }
                    Some(Excluded::OptionalOnly) => {
                        "It is an optional upgrade, not needed to meet your targets."
                    }
                    Some(Excluded::HazardTooRare) => {
                        "It helps with a hazard that is rare where you live, so the plan leaves \
                         it out."
                    }
                    Some(Excluded::UnknownRule) | None => "It is not part of your plan.",
                }
                .to_owned(),
            );
        }
    }
    plain.push(format!(
        "Typical price: {} per {}.",
        text::band(
            f64::from(it.price_band_usd.low),
            f64::from(it.price_band_usd.high)
        ),
        it.price_band_usd.per
    ));
    Ok(Explanation {
        title: it.name.clone(),
        plain,
        math: (!math.is_empty()).then_some(math),
        sources: sources(&ids, content),
    })
}

fn requirement(a: &Assessment, content: &Content, id: &str) -> Result<Explanation, EngineError> {
    let l = a
        .lines
        .iter()
        .find(|l| l.line.id == id)
        .ok_or_else(|| not_found(ExplainKind::Requirement, id))?;
    let mut plain = vec![l.line.plain.clone()];
    if l.prior {
        plain.push("Some amounts in it are expert estimates.".to_owned());
    }
    let mut math = l.math.clone();
    math.push(format!(
        "{} {} for the household ({}), {} step.",
        text::number(l.quantity),
        l.line.unit,
        l.line.per,
        text::lower_first(l.tier.name())
    ));
    Ok(Explanation {
        title: format!(
            "{}: {}",
            l.line.bucket.name(),
            l.line.rule.replace('_', " ")
        ),
        plain,
        math: Some(math),
        sources: sources(&l.line.citations, content),
    })
}

fn warning(a: &Assessment, content: &Content, id: &str) -> Result<Explanation, EngineError> {
    let w = a
        .warnings
        .iter()
        .find(|w| w.id == id)
        .ok_or_else(|| not_found(ExplainKind::Warning, id))?;
    let mut ids: Vec<CitationId> = Vec::new();
    for r in &w.related {
        if let Ok(b) = r.parse::<BucketId>() {
            ids.extend(a.bucket(b).sources.iter().cloned());
        }
        if let Some(s) = a.consequence.scenarios.iter().find(|s| &s.id == r) {
            ids.extend(s.sources.iter().cloned());
        }
    }
    Ok(Explanation {
        title: w.message.clone(),
        plain: vec![w.why.clone()],
        math: None,
        sources: sources(&ids, content),
    })
}
