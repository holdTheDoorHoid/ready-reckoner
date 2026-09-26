//! `assess`: household + hazard rates + county data + scenarios → bucket assessments, curves,
//! scenario summaries, cliff warnings and sentences.

use std::collections::BTreeMap;

use rr_types::math;
use rr_types::{
    BucketAssessment, BucketId, CitationId, Contribution, Evidence, HazardId, HouseholdEventRate,
    PlanInput, Relief, ReturnPeriod, ScenarioInfo, Target, TierId, Warning, WarningSeverity,
};
use serde::Serialize;

use crate::curve::{DialPoint, Eval, ExceedanceCurve, dial_rate, owner_shares};
use crate::effects::{self, EffectsTable};
use crate::income::{GapRule, IncomeEval};
use crate::model::{
    self, BuildInput, CountyData, CouplingApplied, Model, OverrideApplied, Owner,
    ScenarioCandidate, Term,
};
use crate::ranges::{self, DRAWS, Draws};
use crate::words;

/// Duration buckets, in [`BucketId::ALL`] order.
pub const DURATION_BUCKETS: [BucketId; 7] = [
    BucketId::Power,
    BucketId::WaterBoil,
    BucketId::WaterOut,
    BucketId::Supplies,
    BucketId::Thermal,
    BucketId::Medication,
    BucketId::Comms,
];

/// Catalogue item ids that count as existing coverage (`PlanInput::existing`).
// awaiting: rr-content (the catalogue's id for "gas stove in the home").
pub const GAS_STOVE_ITEM_IDS: [&str; 1] = ["gas_stove"];

/// Everything `rr-consequence` produces for one household.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsequenceAssessment {
    /// Every bucket, in [`BucketId::ALL`] order. `covered` holds only what the household's home
    /// already provides (a gas stove for boil-water notices, savings for income); the plan crate
    /// adds the plan's coverage. Readiness checklists (`done`/`of`) are left at 0 for the plan
    /// crate to fill from `rr-supply`'s requirement lines. // awaiting: rr-plan
    pub buckets: Vec<BucketAssessment>,
    /// Named scenarios offered for this location, on or off, with what they change.
    pub scenarios: Vec<ScenarioInfo>,
    /// Cliff warnings ("your answer depends mostly on one event").
    pub warnings: Vec<Warning>,
    /// The self-sufficiency statement (research §3.7), one sentence per entry.
    pub statement: Vec<String>,
    /// The numbers behind each duration bucket, including its exceedance curve.
    pub details: Vec<BucketDetail>,
    /// The numbers behind the income target.
    pub income: IncomeDetail,
    /// The numbers behind the evacuation readiness target.
    pub evacuate: EvacuateDetail,
    /// Walking times home for each commuter.
    pub get_home: GetHomeDetail,
    /// Displacement probability and how long it tends to last.
    pub home_loss: HomeLossDetail,
    /// Household coupling rules that fired (one explainable line each).
    pub couplings: Vec<CouplingApplied>,
    /// County records that replaced national defaults.
    pub overrides: Vec<OverrideApplied>,
    /// Plain notes (fallbacks used, data dropped).
    pub notes: Vec<String>,
    /// Yearly rate of the household's return period (1/N).
    pub dial_rate: f64,
    /// Monte Carlo draws behind the ranges.
    pub draws: usize,
}

impl ConsequenceAssessment {
    /// The exceedance curve of a duration bucket.
    pub fn curve(&self, bucket: BucketId) -> Option<&ExceedanceCurve> {
        self.details
            .iter()
            .find(|d| d.bucket == bucket)
            .map(|d| &d.curve)
    }

    /// U(x) = ∫ₓ^∞ Λ_b(t) dt: expected days per year of this disruption beyond day `x` (0 for
    /// buckets without a curve).
    pub fn unmet_days(&self, bucket: BucketId, x: f64) -> f64 {
        self.curve(bucket).map_or(0.0, |c| c.unmet_days(x))
    }

    /// ∫_{x0}^{x1} Λ_b(t) dt: expected disruption-days per year covered by holding days `x0` to
    /// `x1` of supplies (0 for buckets without a curve).
    pub fn value_between(&self, bucket: BucketId, x0: f64, x1: f64) -> f64 {
        self.curve(bucket).map_or(0.0, |c| c.value_between(x0, x1))
    }

    /// The assessment of one bucket.
    pub fn bucket(&self, bucket: BucketId) -> &BucketAssessment {
        &self.buckets[BucketId::ALL.iter().position(|b| *b == bucket).unwrap_or(0)]
    }
}

/// One way a bucket can be disrupted, as the expert view shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TermSummary {
    /// The hazard.
    pub hazard: HazardId,
    /// The named scenario, if any.
    pub scenario: Option<String>,
    /// The event class, in words.
    pub label: String,
    /// Events per year that cause this disruption (rate × share).
    pub events_per_year: f64,
    /// Median duration in days.
    pub median_days: f64,
    /// 90th-percentile duration in days.
    pub p90_days: f64,
    /// Counted only when the underlying event outlasts this many days.
    pub threshold_days: f64,
    /// What it rests on.
    pub evidence: Evidence,
    /// Its share of Λ at the design duration.
    pub share_at_target: f64,
    /// Where it came from: the effects table, county outage records, or a coupling rule.
    pub origin: String,
}

/// The numbers behind a duration bucket's target.
#[derive(Debug, Clone, PartialEq)]
pub struct BucketDetail {
    /// Which bucket.
    pub bucket: BucketId,
    /// Λ_b(d) at central values.
    pub curve: ExceedanceCurve,
    /// Design duration at the household's return period, before rounding (days).
    pub target_days: f64,
    /// On the day ladder.
    pub ladder_days: f32,
    /// 10th percentile on the ladder.
    pub low_days: f32,
    /// 90th percentile on the ladder.
    pub high_days: f32,
    /// Targets at every return period.
    pub dial_table: Vec<DialPoint>,
    /// Days per year of this disruption on average (Σ r·q·E[D]).
    pub consumption_days_per_year: f64,
    /// The inputs that move the target most, in words.
    pub drivers: Vec<String>,
    /// The event class that dominates the design event.
    pub design_event: Option<String>,
    /// Every term, largest share first.
    pub terms: Vec<TermSummary>,
}

/// The numbers behind the income target.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IncomeDetail {
    /// Months of income gap at the return period, before rounding.
    pub target_months: f64,
    /// Rounded up to the months ladder.
    pub ladder_months: f32,
    /// 10th percentile (ladder).
    pub low_months: f32,
    /// 90th percentile (ladder).
    pub high_months: f32,
    /// Targets at every return period: (years, months, ladder months).
    pub dial_table: Vec<(u16, f64, f32)>,
    /// Out of 100 households like this one, how many face a gap of more than 3 months over the
    /// horizon.
    pub gap_over_3_months_per_100: f64,
    /// Each earner's share of household income.
    pub earner_share: f64,
    /// Streams: (hazard, label, spells per year).
    pub streams: Vec<(HazardId, String, f64)>,
    /// The inputs that move the target most.
    pub drivers: Vec<String>,
    /// The national job-loss rate was used because no rate was supplied.
    pub base_rate_used: bool,
    /// The income curve at central values (for any other rate).
    #[serde(skip)]
    pub curve: crate::income::IncomeCurve,
}

/// The numbers behind the evacuation target.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvacuateDetail {
    /// Chance of having to leave at least once in ten years.
    pub p_need_10yr: f64,
    /// Events per year that force the household out.
    pub rate_per_year: f64,
    /// Least and most warning to expect, hours.
    pub notice_hours: [f64; 2],
    /// Typical days away (median of the time-away mixture, on the day ladder).
    pub days_away: f32,
    /// (hazard, label, events per year, notice band).
    pub causes: Vec<(HazardId, String, f64, [f64; 2])>,
}

/// One commuter's walk home.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CommuteWalk {
    /// Index of the person in `PlanInput::people`.
    pub person: usize,
    /// One-way distance, km.
    pub distance_km: f64,
    /// Hours on foot at the walking pace in `[params.walk_speed_mph]`.
    pub walk_hours: f64,
    /// Litres of water for the walk in hot weather.
    pub water_litres_in_heat: f64,
}

/// The get-home readiness numbers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GetHomeDetail {
    /// Chance someone is stranded away from home at least once in ten years.
    pub p_need_10yr: f64,
    /// Commuters and their walks home.
    pub commuters: Vec<CommuteWalk>,
}

/// The home-loss (displacement) numbers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HomeLossDetail {
    /// Chance of having to leave home for a while because of damage, in ten years.
    pub p_displaced_10yr: f64,
    /// Events per year.
    pub rate_per_year: f64,
    /// Share of displaced households back within a week.
    pub share_back_within_week: f64,
    /// Share out for more than six months (not counting those who never return).
    pub share_over_six_months: f64,
    /// Share of households like this one (renter or owner) that never return.
    pub share_never_return: f64,
}

/// Scenario on/off: the candidate's default, then the user's override.
fn scenario_states(input: &PlanInput, scenarios: &[ScenarioCandidate]) -> Vec<bool> {
    scenarios
        .iter()
        .map(|c| {
            input
                .dials
                .scenario_overrides
                .iter()
                .find(|t| t.id == c.id)
                .map_or(c.on, |t| t.on)
        })
        .collect()
}

/// Days for a tier ("the tier that covers the target").
fn tier_for_days(days: f32) -> TierId {
    let d = f64::from(days);
    TierId::ALL
        .iter()
        .copied()
        .find(|t| f64::from(t.days()) >= d)
        .unwrap_or(TierId::Y1)
}

/// Everything the consequence model needs for one household. See the crate docs.
pub fn assess(
    input: &PlanInput,
    rates: &[HouseholdEventRate],
    county: CountyData<'_>,
    scenarios: &[ScenarioCandidate],
) -> ConsequenceAssessment {
    assess_with_draws(input, rates, county, scenarios, DRAWS)
}

/// [`assess`] with an explicit number of Monte Carlo draws (for timing and tests; `assess` uses
/// [`DRAWS`]).
pub fn assess_with_draws(
    input: &PlanInput,
    rates: &[HouseholdEventRate],
    county: CountyData<'_>,
    scenarios: &[ScenarioCandidate],
    draws: usize,
) -> ConsequenceAssessment {
    let table = effects::table();
    let states = scenario_states(input, scenarios);
    let binp = BuildInput {
        plan: input,
        rates,
        county,
        scenarios,
        scenario_on: &states,
        table,
    };
    let model = model::build(&binp);
    let rate = dial_rate(input.dials.return_period);
    let years = f64::from(input.dials.horizon_years.max(1));
    let draws = Draws::new(&model.params, draws.max(10));
    let ctx = Ctx {
        input,
        table,
        model: &model,
        scenarios,
        rate,
        years,
        draws: &draws,
    };

    let mut details = Vec::new();
    let mut by_bucket: BTreeMap<BucketId, BucketAssessment> = BTreeMap::new();
    for b in DURATION_BUCKETS {
        let (assessment, detail) = duration_bucket(&ctx, b);
        by_bucket.insert(b, assessment);
        details.push(detail);
    }
    let (evac_assessment, evacuate) = evacuate_bucket(&ctx);
    by_bucket.insert(BucketId::Evacuate, evac_assessment);
    let (gh_assessment, get_home) = get_home_bucket(&ctx);
    by_bucket.insert(BucketId::GetHome, gh_assessment);
    for b in [
        BucketId::MedicalEmergency,
        BucketId::Fire,
        BucketId::Security,
    ] {
        by_bucket.insert(b, readiness_bucket(&ctx, b));
    }
    let (income_assessment, income) = income_bucket(&ctx);
    by_bucket.insert(BucketId::Income, income_assessment);
    let (hl_assessment, home_loss) = home_loss_bucket(&ctx);
    by_bucket.insert(BucketId::HomeLoss, hl_assessment);

    let scenario_infos = scenario_summaries(&binp, &model, &states, rate);
    let warnings = cliff_warnings(&ctx, &details);
    let statement = statement(
        &ctx, &by_bucket, &details, &income, &evacuate, &get_home, &warnings,
    );

    let buckets = BucketId::ALL
        .iter()
        .map(|b| by_bucket.remove(b).expect("every bucket is assessed"))
        .collect();
    ConsequenceAssessment {
        buckets,
        scenarios: scenario_infos,
        warnings,
        statement,
        details,
        income,
        evacuate,
        get_home,
        home_loss,
        couplings: model.couplings.clone(),
        overrides: model.overrides.clone(),
        notes: model.notes.clone(),
        dial_rate: rate,
        draws: draws.n(),
    }
}

/// Shared context for building the buckets.
struct Ctx<'a> {
    input: &'a PlanInput,
    table: &'a EffectsTable,
    model: &'a Model,
    scenarios: &'a [ScenarioCandidate],
    rate: f64,
    years: f64,
    draws: &'a Draws,
}

impl Ctx<'_> {
    fn owner_name(&self, owner: Owner) -> String {
        match owner {
            Owner::Hazard(h) => words::hazard_one(h).to_owned(),
            Owner::Scenario(i) => words::lower_first(&self.scenarios[i].name),
        }
    }

    fn owner_hazard(&self, owner: Owner) -> HazardId {
        match owner {
            Owner::Hazard(h) => h,
            Owner::Scenario(i) => self.scenarios[i].hazard,
        }
    }

    fn term_rate_sources(&self, t: &Term) -> Vec<CitationId> {
        match t.scenario {
            Some(i) => self.scenarios[i].sources.clone(),
            None => self
                .model
                .rate_sources
                .get(&t.hazard)
                .cloned()
                .unwrap_or_default(),
        }
    }

    fn horizon(&self) -> String {
        words::horizon_phrase(self.input.dials.horizon_years.max(1))
    }
}

/// Contributions by hazard (scenarios counted under their hazard family), largest first, adding
/// up to 1.
fn contributions(ctx: &Ctx<'_>, owners: &[(Owner, f64)]) -> Vec<Contribution> {
    let mut by: BTreeMap<HazardId, f64> = BTreeMap::new();
    for (o, s) in owners {
        *by.entry(ctx.owner_hazard(*o)).or_insert(0.0) += s;
    }
    let mut v: Vec<(HazardId, f64)> = by.into_iter().filter(|(_, s)| *s >= 0.005).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let total: f64 = v.iter().map(|(_, s)| s).sum();
    if total <= 0.0 {
        return Vec::new();
    }
    v.into_iter()
        .map(|(hazard, s)| Contribution {
            hazard,
            share: (s / total) as f32,
        })
        .collect()
}

fn sorted_sources(mut v: Vec<CitationId>) -> Vec<CitationId> {
    v.sort();
    v.dedup();
    v
}

/// Sources for a bucket: the terms that matter, their rates, the couplings and overrides.
fn bucket_sources(
    ctx: &Ctx<'_>,
    bucket: BucketId,
    terms: &[&Term],
    shares: &[f64],
) -> Vec<CitationId> {
    let mut v: Vec<CitationId> = Vec::new();
    for (t, s) in terms.iter().zip(shares) {
        if *s >= 0.01 {
            v.extend(t.sources.iter().cloned());
            v.extend(ctx.term_rate_sources(t));
        }
    }
    for c in &ctx.model.couplings {
        if c.applied_in == "rr-consequence" && c.buckets.contains(&bucket) {
            v.extend(c.sources.iter().cloned());
        }
    }
    for o in &ctx.model.overrides {
        if o.bucket == bucket {
            v.extend(o.sources.iter().cloned());
        }
    }
    sorted_sources(v)
}

fn coupling_sentences(ctx: &Ctx<'_>, bucket: BucketId) -> Vec<String> {
    ctx.model
        .couplings
        .iter()
        .filter(|c| c.applied_in == "rr-consequence" && c.buckets.contains(&bucket))
        .map(|c| c.plain.clone())
        .collect()
}

fn has_gas_stove(input: &PlanInput) -> bool {
    input
        .existing
        .iter()
        .any(|o| GAS_STOVE_ITEM_IDS.contains(&o.item_id.as_str()) && o.qty >= 1.0)
}

/// The duration in the first frequency sentence: a day for utilities, three days for shopping
/// and medicine, where one-day gaps are near certain (as in the research register, §8.3).
fn headline_days(bucket: BucketId) -> f64 {
    match bucket {
        BucketId::Supplies | BucketId::Medication => 3.0,
        _ => 1.0,
    }
}

fn duration_bucket(ctx: &Ctx<'_>, bucket: BucketId) -> (BucketAssessment, BucketDetail) {
    let model = ctx.model;
    let mut eval = Eval::central(model, bucket);
    let target_c = eval.target(ctx.rate, 32);
    let ladder = eval.ladder_target(ctx.rate);
    let d1 = headline_days(bucket);
    let d2 = f64::from(ladder).max(1.0);
    let range =
        ranges::duration_range(&mut eval, ctx.draws, ctx.rate, ladder, &[d1, d2], ctx.years);
    let low = range.low.min(ladder);
    let high = range.high.max(ladder);
    let at = if target_c > 0.0 { target_c } else { 0.0 };
    let shares = eval.shares(at);
    let owners = owner_shares(&eval, at);

    // Drivers: inputs of the terms that feed the design event.
    let mut order: Vec<(usize, f64)> = shares.iter().copied().enumerate().collect();
    order.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut candidates: Vec<usize> = Vec::new();
    for (i, s) in &order {
        if *s < 0.02 || candidates.len() >= 6 {
            break;
        }
        let t = eval.terms()[*i];
        for p in [t.rate_param, t.q_param, t.dur_param].into_iter().flatten() {
            if !candidates.contains(&p) {
                candidates.push(p);
            }
        }
    }
    let drivers = if target_c > 0.0 {
        let params = &model.params;
        let rate = ctx.rate;
        let mut target_at = |z: &dyn Fn(usize) -> f64| {
            eval.set_draw(params, z);
            eval.target_near(rate, target_c / 30.0, target_c * 30.0, 12)
        };
        let d = ranges::drivers(&candidates, params, 2, &mut target_at);
        eval.set_draw(params, |_| 0.0);
        d
    } else {
        Vec::new()
    };

    let dial_table: Vec<DialPoint> = ReturnPeriod::ALL
        .iter()
        .map(|rp| {
            let r = dial_rate(*rp);
            DialPoint {
                return_period_years: rp.years(),
                target_days: eval.target(r, 26),
                ladder_days: eval.ladder_target(r),
            }
        })
        .collect();
    let curve = ExceedanceCurve::from_eval(bucket, &eval, target_c, ladder);

    // Relief for the design event.
    let design = order
        .first()
        .filter(|(_, s)| *s > 0.0 && target_c > 0.0)
        .map(|(i, _)| eval.terms()[*i]);
    let relief = design.and_then(|t| relief_for(ctx, t));

    let terms: Vec<TermSummary> = order
        .iter()
        .map(|(i, s)| {
            let t = eval.terms()[*i];
            TermSummary {
                hazard: t.hazard,
                scenario: t.scenario.map(|k| ctx.scenarios[k].id.clone()),
                label: t.label.clone(),
                events_per_year: t.rate * t.q,
                median_days: t.survival.median_days(),
                p90_days: t.survival.p90_days(),
                threshold_days: t.threshold,
                evidence: t.evidence,
                share_at_target: *s,
                origin: match &t.origin {
                    model::Origin::Table => "effects table".to_owned(),
                    model::Origin::Pool => "county-wide storm outages".to_owned(),
                    model::Origin::Coupling(r) => format!("coupling rule: {r}"),
                },
            }
        })
        .collect();

    // Sentences.
    let horizon = ctx.horizon();
    let (f1lo, f1hi) = range.freq[0];
    let f1 = eval.natural_frequency(d1, ctx.years);
    let mut sentences = vec![format!(
        "Of 100 households like yours, {} will {} for {} or more in {horizon}.",
        words::per_100_range(f1, f1lo, f1hi),
        words::bucket_verb(bucket),
        words::ladder_phrase(d1 as f32)
    )];
    if ladder > 0.0 {
        let (f2lo, f2hi) = range.freq[1];
        let f2 = eval.natural_frequency(d2, ctx.years);
        sentences.push(format!(
            "Being ready for {} leaves {} of 100 households like yours facing a longer {} in {horizon}.",
            words::ladder_phrase(ladder),
            words::per_100_range(f2, f2lo, f2hi),
            words::bucket_noun(bucket)
        ));
    } else {
        sentences.push(format!(
            "At the setting you chose (1 in {} years), this is rare enough that there is no target.",
            ctx.input.dials.return_period.years()
        ));
    }
    match drivers.as_slice() {
        [a] => sentences.push(format!("This number mostly depends on {}.", a.phrase())),
        [a, b, ..] => sentences.push(format!(
            "This number mostly depends on {} and {}.",
            a.phrase(),
            b.phrase()
        )),
        [] => {}
    }
    sentences.extend(coupling_sentences(ctx, bucket));
    let gas_stove = bucket == BucketId::WaterBoil && has_gas_stove(ctx.input);
    if gas_stove {
        sentences.push(
            "Your gas stove can boil water during a boil-water notice, as long as the gas stays on."
                .to_owned(),
        );
    }

    let target = Target::Days {
        value: ladder,
        low,
        high,
    };
    let covered_days = if gas_stove { ladder } else { 0.0 };
    let assessment = BucketAssessment {
        id: bucket,
        name: bucket.name().to_owned(),
        target,
        covered: Target::Days {
            value: covered_days,
            low: covered_days,
            high: covered_days,
        },
        tier_enough: tier_for_days(ladder),
        contributions: contributions(ctx, &owners),
        frequency_sentences: sentences,
        sources: {
            let mut s = bucket_sources(ctx, bucket, eval.terms(), &shares);
            if let Some(r) = &relief {
                s.extend(r.sources.iter().cloned());
            }
            sorted_sources(s)
        },
        relief,
    };
    let detail = BucketDetail {
        bucket,
        consumption_days_per_year: curve.consumption_days_per_year(),
        curve,
        target_days: target_c,
        ladder_days: ladder,
        low_days: low,
        high_days: high,
        dial_table,
        drivers: drivers.iter().map(|d| d.phrase()).collect(),
        design_event: design.map(|t| t.label.clone()),
        terms,
    };
    (assessment, detail)
}

/// Relief for a design event: the row's own rating (Oregon Resilience Plan for Cascadia), or,
/// when its duration comes from a measured restoration curve, help within the 72-hour standard
/// (or sooner, if service is back sooner) and "mostly restored" at the time to 90 % restored.
fn relief_for(ctx: &Ctx<'_>, t: &Term) -> Option<Relief> {
    if let Some(r) = &t.relief {
        return Some(Relief {
            help_arrives_days: r.help_arrives_days as f32,
            mostly_restored_days: r.mostly_restored_days as f32,
            sources: r.sources.clone(),
        });
    }
    if !t.duration_from_data {
        return None;
    }
    let restored = t.survival.p90_days();
    if !restored.is_finite() || restored <= 0.0 {
        return None;
    }
    let standard = &ctx.table.params.relief_standard_days;
    let mut sources = t.sources.clone();
    sources.extend(standard.sources.iter().cloned());
    Some(Relief {
        help_arrives_days: standard.value.min(restored) as f32,
        mostly_restored_days: restored as f32,
        sources: sorted_sources(sources),
    })
}

fn readiness_sentence(ctx: &Ctx<'_>, bucket: BucketId, r: f64, lo: f64, hi: f64) -> String {
    let n = crate::curve::natural_frequency(r, ctx.years);
    let nlo = crate::curve::natural_frequency(lo, ctx.years);
    let nhi = crate::curve::natural_frequency(hi, ctx.years);
    format!(
        "Of 100 households like yours, {} will {} at least once in {}.",
        words::per_100_range(n, nlo, nhi),
        words::bucket_verb(bucket),
        ctx.horizon()
    )
}

fn p10(r: f64) -> f64 {
    -math::exp_m1(-10.0 * r)
}

fn readiness_parts(
    ctx: &Ctx<'_>,
    bucket: BucketId,
) -> (f64, f64, f64, Vec<Contribution>, Vec<CitationId>) {
    let mut eval = Eval::central(ctx.model, bucket);
    let r = eval.lambda0();
    let (lo, hi) = ranges::rate_range(&mut eval, ctx.draws);
    let shares = eval.shares(0.0);
    let owners = owner_shares(&eval, 0.0);
    let sources = bucket_sources(ctx, bucket, eval.terms(), &shares);
    (r, lo, hi, contributions(ctx, &owners), sources)
}

fn readiness_tier(ctx: &Ctx<'_>, p_need: f64) -> TierId {
    if p_need >= ctx.table.params.readiness_threshold.value {
        TierId::H72
    } else {
        TierId::Now
    }
}

fn readiness_bucket(ctx: &Ctx<'_>, bucket: BucketId) -> BucketAssessment {
    let (r, lo, hi, contributions, sources) = readiness_parts(ctx, bucket);
    let p = p10(r);
    let mut sentences = Vec::new();
    if bucket == BucketId::MedicalEmergency && r >= 0.75 {
        let n = words::round_nice(r);
        let visits = if n == "1" { "visit" } else { "visits" };
        sentences.push(format!(
            "Households like yours average about {n} emergency-room {visits} a year (national figures)."
        ));
    } else {
        sentences.push(readiness_sentence(ctx, bucket, r, lo, hi));
    }
    sentences.extend(coupling_sentences(ctx, bucket));
    let target = Target::Readiness {
        p_need_10yr: p,
        done: 0,
        of: 0,
    };
    BucketAssessment {
        id: bucket,
        name: bucket.name().to_owned(),
        target,
        covered: target,
        tier_enough: readiness_tier(ctx, p),
        contributions,
        frequency_sentences: sentences,
        sources,
        relief: None,
    }
}

fn evacuate_bucket(ctx: &Ctx<'_>) -> (BucketAssessment, EvacuateDetail) {
    let bucket = BucketId::Evacuate;
    let (r, lo, hi, contributions, sources) = readiness_parts(ctx, bucket);
    let eval = Eval::central(ctx.model, bucket);
    let shares = eval.shares(0.0);
    let mut notice = [f64::INFINITY, 0.0f64];
    let mut causes = Vec::new();
    for (t, s) in eval.terms().iter().zip(&shares) {
        let band = t.notice_hours.unwrap_or([0.0, 0.0]);
        causes.push((t.hazard, t.label.clone(), t.rate * t.q, band));
        if *s >= 0.05 {
            notice[0] = notice[0].min(band[0]);
            notice[1] = notice[1].max(band[1]);
        }
    }
    if !notice[0].is_finite() {
        notice = [0.0, 0.0];
    }
    causes.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
    // Typical time away: the median of the time-away mixture, on the ladder.
    let total = eval.lambda0();
    let days_away = if total > 0.0 {
        let median = eval.target(0.5 * total, 26);
        crate::curve::round_up_to_ladder(median)
    } else {
        0.0
    };
    let p = p10(r);
    let mut sentences = vec![readiness_sentence(ctx, bucket, r, lo, hi)];
    if p > 0.0 {
        sentences.push(format!(
            "Warning can be as short as {} or as long as {}.",
            notice_words(notice[0]),
            notice_words(notice[1])
        ));
        sentences.push(format!(
            "Plan to be away for about {}.",
            words::ladder_phrase(days_away)
        ));
    }
    sentences.extend(coupling_sentences(ctx, bucket));
    let target = Target::Evacuate {
        p_need_10yr: p,
        notice_hours_low: notice[0] as f32,
        notice_hours_high: notice[1] as f32,
        days_away,
    };
    let covered = Target::Evacuate {
        p_need_10yr: p,
        notice_hours_low: notice[0] as f32,
        notice_hours_high: notice[1] as f32,
        days_away: 0.0,
    };
    let assessment = BucketAssessment {
        id: bucket,
        name: bucket.name().to_owned(),
        target,
        covered,
        tier_enough: readiness_tier(ctx, p),
        contributions,
        frequency_sentences: sentences,
        sources,
        relief: None,
    };
    let detail = EvacuateDetail {
        p_need_10yr: p,
        rate_per_year: r,
        notice_hours: notice,
        days_away,
        causes,
    };
    (assessment, detail)
}

/// "no warning", "a few minutes", "15 minutes", "2 hours", "3 days".
fn notice_words(hours: f64) -> String {
    if hours <= 0.0 {
        "no warning at all".to_owned()
    } else if hours < 0.125 {
        "a few minutes".to_owned()
    } else if hours < 1.0 {
        format!("{} minutes", words::round_nice(hours * 60.0))
    } else if hours < 1.5 {
        "an hour".to_owned()
    } else if hours < 36.0 {
        format!("{} hours", words::round_nice(hours))
    } else {
        words::days_phrase(hours / 24.0)
    }
}

fn get_home_bucket(ctx: &Ctx<'_>) -> (BucketAssessment, GetHomeDetail) {
    let bucket = BucketId::GetHome;
    let (r, lo, hi, contributions, sources) = readiness_parts(ctx, bucket);
    let p = p10(r);
    let prm = &ctx.table.params;
    let kmh = prm.walk_speed_mph.value * 1.609_344;
    let commuters: Vec<CommuteWalk> = ctx
        .input
        .people
        .iter()
        .enumerate()
        .filter_map(|(i, person)| {
            let c = person.commute.as_ref()?;
            let km = f64::from(c.distance_km);
            (km > 0.0).then(|| {
                let hours = km / kmh;
                CommuteWalk {
                    person: i,
                    distance_km: km,
                    walk_hours: hours,
                    water_litres_in_heat: hours * prm.walk_water_l_per_hour.value,
                }
            })
        })
        .collect();
    let mut sentences = vec![readiness_sentence(ctx, bucket, r, lo, hi)];
    for c in &commuters {
        sentences.push(format!(
            "The {} trip is about {} on foot; carry about {} of water for it in hot weather.",
            words::distance_adjective(c.distance_km),
            hours_on_foot(c.walk_hours),
            litres_words(c.water_litres_in_heat)
        ));
    }
    sentences.extend(coupling_sentences(ctx, bucket));
    let target = Target::Readiness {
        p_need_10yr: p,
        done: 0,
        of: 0,
    };
    let mut sources = sources;
    if !commuters.is_empty() {
        sources.extend(prm.walk_speed_mph.sources.iter().cloned());
        sources.extend(prm.walk_water_l_per_hour.sources.iter().cloned());
        sources = sorted_sources(sources);
    }
    (
        BucketAssessment {
            id: bucket,
            name: bucket.name().to_owned(),
            target,
            covered: target,
            tier_enough: if commuters.is_empty() {
                TierId::Now
            } else {
                readiness_tier(ctx, p)
            },
            contributions,
            frequency_sentences: sentences,
            sources,
            relief: None,
        },
        GetHomeDetail {
            p_need_10yr: p,
            commuters,
        },
    )
}

fn hours_on_foot(h: f64) -> String {
    if h < 0.75 {
        return format!("{} minutes", words::round_nice(h * 60.0));
    }
    let halves = (h * 2.0 + 0.5).floor() as i64; // nearest half hour
    match halves {
        0..=2 => "an hour".to_owned(),
        3 => "an hour and a half".to_owned(),
        n if n % 2 == 0 => format!("{} hours", n / 2),
        n => format!("{} and a half hours", n / 2),
    }
}

fn litres_words(l: f64) -> String {
    if l < 0.75 {
        "half a litre".to_owned()
    } else if l < 1.25 {
        "a litre".to_owned()
    } else {
        format!("{} litres", words::round_nice(l))
    }
}

fn home_loss_bucket(ctx: &Ctx<'_>) -> (BucketAssessment, HomeLossDetail) {
    let bucket = BucketId::HomeLoss;
    let (r, lo, hi, contributions, mut sources) = readiness_parts(ctx, bucket);
    let p = p10(r);
    let prm = &ctx.table.params;
    let renter = ctx.input.housing.tenure == rr_types::Tenure::Rent;
    let never = if renter {
        prm.never_return_renter.value
    } else {
        prm.never_return_owner.value
    };
    let mut sentences = vec![readiness_sentence(ctx, bucket, r, lo, hi)];
    sentences.push(format!(
        "After a disaster, about 1 in 3 displaced households are back within a week, but about 1 in {} {} never return, so insurance and copies of documents matter more than supplies here.",
        words::round_nice(1.0 / never),
        if renter { "renters" } else { "homeowners" }
    ));
    sentences.extend(coupling_sentences(ctx, bucket));
    sources.extend(prm.never_return_renter.sources.iter().cloned());
    sources.extend(prm.displaced_back_within_week.sources.iter().cloned());
    let target = Target::Readiness {
        p_need_10yr: p,
        done: 0,
        of: 0,
    };
    (
        BucketAssessment {
            id: bucket,
            name: bucket.name().to_owned(),
            target,
            covered: target,
            tier_enough: TierId::Now,
            contributions,
            frequency_sentences: sentences,
            sources: sorted_sources(sources),
            relief: None,
        },
        HomeLossDetail {
            p_displaced_10yr: p,
            rate_per_year: r,
            share_back_within_week: prm.displaced_back_within_week.value,
            share_over_six_months: prm.displaced_over_six_months.value,
            share_never_return: never,
        },
    )
}

fn gap_rule(ctx: &Ctx<'_>) -> GapRule {
    let prm = &ctx.table.params;
    let earners = f64::from(ctx.input.finances.income.earners.max(1));
    GapRule {
        share: 1.0 / earners,
        replacement: prm.ui_replacement.value,
        ui_weeks: prm.ui_weeks.value,
        weeks_per_month: prm.weeks_per_month.value,
    }
}

fn income_bucket(ctx: &Ctx<'_>) -> (BucketAssessment, IncomeDetail) {
    let bucket = BucketId::Income;
    let model = ctx.model;
    let rule = gap_rule(ctx);
    let mut eval = IncomeEval::central(&model.income, rule);
    let target_c = eval.target(ctx.rate, 60);
    let ladder = eval.ladder_target(ctx.rate);
    let (lo_t, hi_t, lam_lo, lam_hi) = ranges::income_range(&mut eval, ctx.draws, ctx.rate, 3.0);
    let low = lo_t.min(ladder);
    let high = hi_t.max(ladder);
    let shares = eval.shares(target_c);
    let mut candidates: Vec<usize> = Vec::new();
    for (t, s) in eval.terms().iter().zip(&shares) {
        if *s >= 0.02 {
            for p in [t.rate_param, t.dur_param].into_iter().flatten() {
                if !candidates.contains(&p) {
                    candidates.push(p);
                }
            }
        }
    }
    let drivers = if target_c > 0.0 {
        let params = &model.params;
        let rate = ctx.rate;
        let mut target_at = |z: &dyn Fn(usize) -> f64| {
            eval.set_draw(params, z);
            eval.target(rate, 40)
        };
        let d = ranges::drivers(&candidates, params, 2, &mut target_at);
        eval.set_draw(params, |_| 0.0);
        d
    } else {
        Vec::new()
    };
    let dial_table = ReturnPeriod::ALL
        .iter()
        .map(|rp| {
            let r = dial_rate(*rp);
            (rp.years(), eval.target(r, 60), eval.ladder_target(r))
        })
        .collect();
    let lam3 = eval.lambda(3.0);
    let n3 = crate::curve::natural_frequency(lam3, ctx.years);
    let mut by: BTreeMap<HazardId, f64> = BTreeMap::new();
    for (t, s) in eval.terms().iter().zip(&shares) {
        *by.entry(t.hazard).or_insert(0.0) += s;
    }
    let mut contrib: Vec<(HazardId, f64)> = by.into_iter().filter(|(_, s)| *s >= 0.005).collect();
    contrib.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let total: f64 = contrib.iter().map(|(_, s)| s).sum();
    let contributions = if total > 0.0 {
        contrib
            .into_iter()
            .map(|(hazard, s)| Contribution {
                hazard,
                share: (s / total) as f32,
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut sources: Vec<CitationId> = eval
        .terms()
        .iter()
        .flat_map(|t| t.sources.iter().cloned())
        .collect();
    let prm = &ctx.table.params;
    sources.extend(prm.ui_replacement.sources.iter().cloned());
    sources.extend(prm.ui_weeks.sources.iter().cloned());
    let sources = sorted_sources(sources);

    let earners = ctx.input.finances.income.earners;
    let mut sentences = Vec::new();
    if earners == 0 {
        sentences.push(
            "No one in the household earns wages, so there is no income gap to save for."
                .to_owned(),
        );
    } else {
        sentences.push(format!(
            "Of 100 households like yours, {} will have an income gap of more than 3 months in {}.",
            words::per_100_range(
                n3,
                crate::curve::natural_frequency(lam_lo, ctx.years),
                crate::curve::natural_frequency(lam_hi, ctx.years)
            ),
            ctx.horizon()
        ));
        if ladder > 0.0 {
            sentences.push(format!(
                "Aim for about {} of income in savings over time, a goal separate from the supplies budget.",
                months_words(ladder)
            ));
        }
        match drivers.as_slice() {
            [a] => sentences.push(format!("This number mostly depends on {}.", a.phrase())),
            [a, b, ..] => sentences.push(format!(
                "This number mostly depends on {} and {}.",
                a.phrase(),
                b.phrase()
            )),
            [] => {}
        }
        sentences.push(
            "The death or long-term disability of an earner is a question for life and disability insurance, not savings."
                .to_owned(),
        );
    }
    let covered_months = ctx.input.finances.emergency_fund_months.max(0.0);
    let assessment = BucketAssessment {
        id: bucket,
        name: bucket.name().to_owned(),
        target: Target::Months {
            value: ladder,
            low,
            high,
        },
        covered: Target::Months {
            value: covered_months,
            low: covered_months,
            high: covered_months,
        },
        tier_enough: if ladder > 0.0 {
            TierId::M3
        } else {
            TierId::Now
        },
        contributions,
        frequency_sentences: sentences,
        sources,
        relief: None,
    };
    let detail = IncomeDetail {
        target_months: target_c,
        ladder_months: ladder,
        low_months: low,
        high_months: high,
        dial_table,
        gap_over_3_months_per_100: n3,
        earner_share: rule.share,
        streams: eval
            .terms()
            .iter()
            .map(|t| (t.hazard, t.label.clone(), t.rate))
            .collect(),
        drivers: drivers.iter().map(|d| d.phrase()).collect(),
        base_rate_used: model.job_loss_fallback,
        curve: crate::income::IncomeCurve::from_terms(&model.income, rule),
    };
    (assessment, detail)
}

fn months_words(m: f32) -> String {
    if m == 1.0 {
        "1 month".to_owned()
    } else if m < 1.0 {
        "half a month".to_owned()
    } else if m.fract() == 0.0 {
        format!("{} months", m as i64)
    } else {
        format!("{m} months")
    }
}

/// Targets under one scenario state (ladder values), for the with/without summary.
fn ladder_targets(model: &Model, rate: f64, rule: GapRule) -> BTreeMap<BucketId, f32> {
    let mut out = BTreeMap::new();
    for b in DURATION_BUCKETS {
        out.insert(b, Eval::central(model, b).ladder_target(rate));
    }
    out.insert(
        BucketId::Income,
        IncomeEval::central(&model.income, rule).ladder_target(rate),
    );
    out
}

fn target_words(bucket: BucketId, v: f32) -> String {
    if bucket == BucketId::Income {
        if v == 0.0 {
            "none".to_owned()
        } else {
            months_words(v)
        }
    } else if v == 0.0 {
        "none".to_owned()
    } else if v < 1.0 {
        "half a day".to_owned()
    } else if v == 1.0 {
        "1 day".to_owned()
    } else {
        format!("{} days", v as i64)
    }
}

/// With/without summaries: the current model is one side of every comparison, so only the other
/// side is rebuilt.
fn scenario_summaries(
    binp: &BuildInput<'_>,
    current: &Model,
    states: &[bool],
    rate: f64,
) -> Vec<ScenarioInfo> {
    let earners = f64::from(binp.plan.finances.income.earners.max(1));
    let prm = &binp.table.params;
    let rule = GapRule {
        share: 1.0 / earners,
        replacement: prm.ui_replacement.value,
        ui_weeks: prm.ui_weeks.value,
        weeks_per_month: prm.weeks_per_month.value,
    };
    binp.scenarios
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut flipped = states.to_vec();
            flipped[i] = !states[i];
            let other = model::build(&BuildInput {
                plan: binp.plan,
                rates: binp.rates,
                county: binp.county,
                scenarios: binp.scenarios,
                scenario_on: &flipped,
                table: binp.table,
            });
            let (m_off, m_on) = if states[i] {
                (&other, current)
            } else {
                (current, &other)
            };
            let without = ladder_targets(m_off, rate, rule);
            let with = ladder_targets(m_on, rate, rule);
            let need = |m: &Model| {
                let r = Eval::central(m, BucketId::Evacuate).lambda0();
                (100.0 * p10(r) + 0.5).floor()
            };
            let (need_off, need_on) = (need(m_off), need(m_on));
            let mut changes = Vec::new();
            for b in DURATION_BUCKETS.iter().chain([BucketId::Income].iter()) {
                let (a, z) = (without[b], with[b]);
                if a != z {
                    changes.push(format!(
                        "{}: {} → {}",
                        words::bucket_short(*b),
                        target_words(*b, a),
                        target_words(*b, z)
                    ));
                }
            }
            if need_on != need_off {
                changes.push(format!(
                    "households like yours that have to leave home quickly within 10 years: {} → {} in 100",
                    need_off as i64, need_on as i64
                ));
            }
            let effect_summary = if changes.is_empty() {
                "Does not change your targets at this setting.".to_owned()
            } else {
                format!("Planning for it changes: {}.", changes.join("; "))
            };
            let mut sources = c.sources.clone();
            sources.extend(
                binp.table
                    .effects
                    .iter()
                    .filter(|r| r.scenario.as_deref() == Some(c.id.as_str()))
                    .flat_map(|r| r.sources.iter().cloned()),
            );
            ScenarioInfo {
                id: c.id.clone(),
                name: c.name.clone(),
                applies_because: c.applies_because.clone(),
                on: states[i],
                effect_summary,
                sources: sorted_sources(sources),
            }
        })
        .collect()
}

/// The target must grow at least this fast with the return period between adjacent dial settings
/// (elasticity ln(t₂/t₁) / ln(N₂/N₁); a log-normal tail gives about 1)…
const CLIFF_ELASTICITY: f64 = 2.0;
/// …and change by at least this many days, for the cliff warning.
const CLIFF_MIN_DAYS: f64 = 3.0;

/// The cliff rule (DESIGN §4.4, research §3.3): one hazard or scenario dominates a bucket's design
/// event (half or more of Λ at the target), its rate for that bucket is within a factor of 3 of
/// the dial rate, and the target jumps between adjacent dial settings (it grows at least as fast
/// as the square of the return period, and by 3 days or more). One warning per bucket, id
/// `cliff_<bucket>`, naming the event (rr-plan drops rr-budget's duplicate).
fn cliff_warnings(ctx: &Ctx<'_>, details: &[BucketDetail]) -> Vec<Warning> {
    let here = ReturnPeriod::ALL
        .iter()
        .position(|r| *r == ctx.input.dials.return_period)
        .unwrap_or(2);
    let mut out = Vec::new();
    for d in details {
        if d.target_days.is_nan() || d.target_days <= 0.0 {
            continue;
        }
        let eval = Eval::central(ctx.model, d.bucket);
        let owners = owner_shares(&eval, d.target_days);
        let Some(&(owner, share)) = owners.first() else {
            continue;
        };
        if share < 0.5 {
            continue;
        }
        let owner_rate: f64 = eval
            .terms()
            .iter()
            .filter(|t| t.owner() == owner)
            .map(|t| t.rate * t.q)
            .sum();
        if owner_rate < ctx.rate / 3.0 || owner_rate > ctx.rate * 3.0 {
            continue;
        }
        // Does the target jump between this dial setting and a neighbouring one?
        let t0 = d.target_days;
        let jumps = [here.checked_sub(1), Some(here + 1)]
            .into_iter()
            .flatten()
            .filter_map(|j| d.dial_table.get(j).map(|p| (j, p)))
            .any(|(j, p)| {
                let dial_step = math::ln(dial_rate(ReturnPeriod::ALL[j]) / ctx.rate).abs();
                let e = math::ln((p.target_days + 0.05) / (t0 + 0.05)).abs() / dial_step;
                e >= CLIFF_ELASTICITY && (p.target_days - t0).abs() >= CLIFF_MIN_DAYS
            });
        if !jumps {
            continue;
        }
        let name = ctx.owner_name(owner);
        let (owner_id, event_rate) = match owner {
            Owner::Hazard(h) => (
                h.as_str().to_owned(),
                eval.terms()
                    .iter()
                    .find(|t| t.owner() == owner)
                    .map_or(0.0, |t| t.rate),
            ),
            Owner::Scenario(i) => (ctx.scenarios[i].id.clone(), ctx.scenarios[i].rate_per_year),
        };
        let cells: Vec<String> = d
            .dial_table
            .iter()
            .filter(|p| p.return_period_years >= 50)
            .map(|p| {
                format!(
                    "{} at 1 in {}",
                    target_words(d.bucket, p.ladder_days),
                    p.return_period_years
                )
            })
            .collect();
        let toggle = match owner {
            Owner::Scenario(_) => {
                " You can turn it off to see the plan without it; long outages are better met with ways to make water safe and stay warm than with bigger stockpiles."
            }
            Owner::Hazard(_) => "",
        };
        out.push(Warning {
            id: format!("cliff_{}", d.bucket),
            severity: WarningSeverity::Warn,
            message: format!("Your answer depends mostly on one event: {name}."),
            why: format!(
                "It happens about {} here, close to the level you chose (about once in {} years), so small changes in either move your {} target a lot: {}.{toggle}",
                words::rate_phrase(event_rate),
                ctx.input.dials.return_period.years(),
                words::bucket_short(d.bucket).to_lowercase(),
                cells.join(", ")
            ),
            related: vec![owner_id, d.bucket.as_str().to_owned()],
        });
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn statement(
    ctx: &Ctx<'_>,
    buckets: &BTreeMap<BucketId, BucketAssessment>,
    details: &[BucketDetail],
    income: &IncomeDetail,
    evacuate: &EvacuateDetail,
    get_home: &GetHomeDetail,
    warnings: &[Warning],
) -> Vec<String> {
    let days = |b: BucketId| -> f32 {
        match buckets[&b].target {
            Target::Days { value, .. } => value,
            _ => 0.0,
        }
    };
    let input = ctx.input;
    let mut out = Vec::new();
    let (power, water) = (days(BucketId::Power), days(BucketId::WaterOut));
    if power > 0.0 && power == water {
        out.push(format!(
            "Be ready to manage about {} at home with no power or tap water.",
            words::ladder_phrase(power)
        ));
    } else {
        if power > 0.0 {
            out.push(format!(
                "Be ready to manage about {} at home with no power.",
                words::ladder_phrase(power)
            ));
        }
        if water > 0.0 {
            out.push(format!(
                "Plan for about {} with no usable tap water.",
                words::ladder_phrase(water)
            ));
        }
    }
    if input.housing.water == rr_types::WaterSource::Well {
        out.push(
            "Your well pump runs on electricity, so every power cut is a water cut.".to_owned(),
        );
    }
    if water >= 30.0 {
        out.push(
            "Meet the long tail with a way to make rain or surface water safe to drink, not only with stored water."
                .to_owned(),
        );
    }
    if has_gas_stove(input) && days(BucketId::WaterBoil) > 0.0 {
        out.push("Your gas stove handles boil-water notices.".to_owned());
    }
    let food = days(BucketId::Supplies);
    if food > 0.0 {
        out.push(format!(
            "Keep about {} of food you normally eat.",
            words::ladder_phrase(food)
        ));
    }
    let meds = days(BucketId::Medication);
    let daily = input.people.iter().any(|p| p.medical.daily_rx);
    let cold = input.people.iter().any(|p| p.medical.refrigerated_rx);
    if meds > 0.0 && (daily || cold) {
        out.push(format!(
            "Keep {} of everyday medicine on hand at all times{}.",
            words::ladder_phrase(meds),
            if cold {
                ", with a way to keep cold medicine cold in a power cut"
            } else {
                ""
            }
        ));
    }
    if days(BucketId::Thermal) > 0.0 {
        out.push("Have a plan for dangerous heat or cold at home with no power.".to_owned());
    }
    if let Some(longest) = get_home
        .commuters
        .iter()
        .max_by(|a, b| a.distance_km.total_cmp(&b.distance_km))
    {
        out.push(format!(
            "Keep a get-home plan for the {} trip.",
            words::distance_adjective(longest.distance_km)
        ));
    }
    if evacuate.p_need_10yr >= ctx.table.params.readiness_threshold.value {
        out.push(format!(
            "Keep a go-bag: about {} of 100 households like yours have to leave home quickly within 10 years.",
            words::per_100(100.0 * evacuate.p_need_10yr)
        ));
    }
    let mut said: Vec<&str> = Vec::new();
    for w in warnings {
        if said.contains(&w.message.as_str()) {
            continue;
        }
        said.push(&w.message);
        out.push(w.message.clone());
        if let Some(d) = details
            .iter()
            .filter(|d| w.related.iter().any(|r| r == d.bucket.as_str()))
            .find_map(|d| buckets[&d.bucket].relief.clone())
        {
            out.push(format!(
                "After it, expect about {} before outside help arrives.",
                words::days_phrase(f64::from(d.help_arrives_days))
            ));
        }
    }
    if input.finances.income.earners > 0 && income.ladder_months > 0.0 {
        let longest_days = details.iter().map(|d| d.ladder_days).fold(0.0f32, f32::max);
        if f64::from(income.ladder_months) * 30.44 >= f64::from(longest_days) {
            out.push("Your biggest long disruption is losing a paycheck.".to_owned());
        }
        out.push(format!(
            "Aim for about {} of income gap in savings over time, separate from the supplies budget.",
            months_words(income.ladder_months)
        ));
    }
    out
}
