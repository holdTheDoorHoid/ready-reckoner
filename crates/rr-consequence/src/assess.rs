//! `assess`: household + hazard rates + county data + scenarios → bucket assessments, curves,
//! scenario summaries, cliff warnings and sentences.

use std::collections::BTreeMap;

use rr_types::math;
use rr_types::{
    BucketAssessment, BucketId, CitationId, Contribution, CookingFuel, Date, Evidence, HazardId,
    HouseholdEventRate, PlanInput, Relief, ReturnPeriod, ScenarioInfo, StressTest,
    TARGET_LADDER_DAYS, Target, TierId, Warning, WarningSeverity, WaterSource,
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

/// Hazards that can force a household out with minutes of warning: wildfire, flash flooding
/// (counted under floods from rivers or heavy rain), tsunami and chemical releases. Their short
/// warning enters the evacuation warning band whenever they contribute to it at all, however
/// small their share (model review M-08). Dam failure joins them when it becomes a hazard.
pub const FAST_WARNING_HAZARDS: [HazardId; 6] = [
    HazardId::Wildfire,
    HazardId::RiverineFlooding,
    HazardId::Tsunami,
    HazardId::HazmatRelease,
    HazardId::DamFailure,
    HazardId::Sinkhole,
];

/// A cause counts toward the short end of the evacuation warning when its own ten-year chance
/// is at least this (model review M-08: 1 in 1,000).
pub const WARNING_CAUSE_P10: f64 = 0.001;

/// Catalogue item ids that count as existing coverage (`PlanInput::existing`).
// awaiting: rr-content (the catalogue's id for "gas stove in the home").
pub const GAS_STOVE_ITEM_IDS: [&str; 1] = ["gas_stove"];

/// Everything `rr-consequence` produces for one household.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsequenceAssessment {
    /// Every bucket, in [`BucketId::ALL`] order. `covered` and `covered_today` hold only what the
    /// household's home already provides (a gas stove for boil-water notices, savings for
    /// income); the plan crate replaces them with the plan's coverage and today's. Readiness checklists (`done`/`of`) are left at 0 for the plan
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
    /// The clean-air need: its ten-year chance and the unhealthy-air days a year.
    pub clean_air: CleanAirDetail,
    /// For each event that sets a duration target, the other needs the same event brings at once
    /// (DESIGN §4.7's simultaneous-need check; the budget crate compares it with storage).
    pub simultaneous: Vec<SimultaneousNeed>,
    /// How easily the household's public water system breaks (model review M-03).
    pub fragility: crate::model::Fragility,
    /// Household coupling rules that fired (one explainable line each).
    pub couplings: Vec<CouplingApplied>,
    /// County records that replaced national defaults.
    pub overrides: Vec<OverrideApplied>,
    /// Plain notes (fallbacks used, data dropped).
    pub notes: Vec<String>,
    /// Yearly rate of the household's return period (1/N).
    pub dial_rate: f64,
    /// The horizon of the natural-frequency sentences, in years.
    pub horizon_years: f64,
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

    /// Events a year that outlast at least one duration target (model review M-04): each event
    /// class counts once, at the need it runs past most (the needs of one event are taken as
    /// co-monotone, the convention the well coupling uses), summed over classes. Each target
    /// alone is outlasted at about the dial's rate; together they are outlasted more often.
    pub fn joint_rate(&self) -> f64 {
        let mut by: BTreeMap<(HazardId, Option<usize>, &str), f64> = BTreeMap::new();
        for d in &self.details {
            let target = f64::from(d.ladder_days);
            if target <= 0.0 {
                continue;
            }
            for t in &d.curve.terms {
                let e = t.weight * t.sf(target);
                let slot = by
                    .entry((t.hazard, t.scenario, t.class.as_str()))
                    .or_insert(0.0);
                if e > *slot {
                    *slot = e;
                }
            }
        }
        by.values().sum()
    }

    /// The dial sentence, computed from the model (model review M-04, DESIGN-DELTA §3): "At this
    /// setting, about 1 in 10 households like yours will face a longer disruption of any one
    /// kind in 10 years; about 3 in 10 will face at least one kind that runs past its target."
    pub fn dial_sentence(&self) -> String {
        let years = self.horizon_years.max(1.0);
        let one = crate::curve::natural_frequency(self.dial_rate, years);
        let all = crate::curve::natural_frequency(self.joint_rate(), years).max(one);
        let in_10 = |n: f64| {
            let k = (n / 10.0 + 0.5).floor().clamp(1.0, 10.0) as i64;
            format!("{k} in 10")
        };
        format!(
            "At this setting, about {} households like yours will face a longer disruption of \
             any one kind in {}; about {} will face at least one kind that runs past its target.",
            in_10(one),
            words::horizon_phrase(years as u8),
            in_10(all)
        )
    }

    /// Yearly rate of power cuts longer than `days` (Λ_power), for the rare
    /// `multi_month_blackout` row, which the hazards crate shows from this number.
    pub fn power_beyond(&self, days: f64) -> f64 {
        self.curve(BucketId::Power).map_or(0.0, |c| c.lambda(days))
    }

    /// The power curve at two and three months (DESIGN-DELTA §3: `multi_month_blackout` is
    /// computed from the plan's own power curve).
    pub fn multi_month_blackout(&self) -> MultiMonthBlackout {
        let (l60, l90) = (self.power_beyond(60.0), self.power_beyond(90.0));
        let sources = self
            .buckets
            .iter()
            .find(|b| b.id == BucketId::Power)
            .map(|b| b.sources.clone())
            .unwrap_or_default();
        MultiMonthBlackout {
            rate_60_days: l60,
            rate_90_days: l90,
            p10_60_days: p10(l60),
            p10_90_days: p10(l90),
            sources,
        }
    }
}

/// Power out for two or three months or more, from the household's own power curve.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MultiMonthBlackout {
    /// Power cuts a year lasting longer than 60 days.
    pub rate_60_days: f64,
    /// Power cuts a year lasting longer than 90 days.
    pub rate_90_days: f64,
    /// Chance of at least one longer than 60 days in ten years.
    pub p10_60_days: f64,
    /// Chance of at least one longer than 90 days in ten years.
    pub p10_90_days: f64,
    /// Where the power curve comes from.
    pub sources: Vec<CitationId>,
}

/// The needs one design event brings at the same time (DESIGN §4.7).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimultaneousNeed {
    /// The event, in words ("major hurricanes (category 3 or a direct hit)").
    pub event: String,
    /// Its hazard (a scenario's family).
    pub hazard: HazardId,
    /// The named scenario, if it is one.
    pub scenario: Option<String>,
    /// The buckets whose target this event sets.
    pub sets_target_of: Vec<BucketId>,
    /// Every duration need the event brings, with the days at the design event's severity (on
    /// the ladder) and the chance the event brings it at all.
    pub needs: Vec<(BucketId, f32, f64)>,
}

/// The clean-air need (contract v2 `clean_air`, DESIGN §4.3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CleanAirDetail {
    /// Chance of needing clean air at home at least once in ten years.
    pub p_need_10yr: f64,
    /// Days a year with unhealthy air from smoke, dust or fumes.
    pub days_per_year: f64,
    /// The smoke part of `days_per_year` is the county's record (smoke days with PM2.5 of 35 or
    /// more), not the model's episode count.
    pub smoke_days_from_record: bool,
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
    /// The design event as (hazard, scenario index, class): the key the simultaneous-need check
    /// groups buckets by.
    pub design: Option<(HazardId, Option<usize>, String)>,
    /// The target rests on a fallback (a structural gap), so its range was widened.
    pub structural_gap: bool,
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
    /// The cause behind the short end of the warning band (model review M-08).
    pub fastest_cause: Option<HazardId>,
    /// The fastest cause that is not a fire at home, with its least warning in hours, when a fire
    /// at home sets the short end.
    pub fastest_outside: Option<(HazardId, f64)>,
    /// The cause with the longest time away (90th percentile, days) among those with a ten-year
    /// chance of 1 in 100 or more.
    pub longest_away: Option<(HazardId, f64)>,
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
    /// If damage forced the household out, the months by which nine in ten households like it
    /// would be home again (the 90th percentile of the displacement durations, weighted by how
    /// often each cause happens; model review M-08). 0 when there is no displacement risk.
    pub months_away_p90: f64,
    /// Months away at the household's dial: the displacement a 1-in-N year brings (0 when being
    /// displaced at all is rarer than the dial).
    pub months_away_at_dial: f64,
    /// Living elsewhere for `months_away_p90`, at the housing share of the household's monthly
    /// expenses (`None` without monthly expenses): what loss-of-use insurance pays for.
    pub displacement_cost_usd: Option<f64>,
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

/// Everything the consequence model needs for one household, from whole hazard rates. See the
/// crate docs. Rows that take a part of a hazard's rate (wildfire warnings to leave and safety
/// shutoffs) use the effects table's fixed fallback split; the engine passes the hazard crate's
/// own split with [`assess_with_parts`].
pub fn assess(
    input: &PlanInput,
    rates: &[HouseholdEventRate],
    county: CountyData<'_>,
    scenarios: &[ScenarioCandidate],
) -> ConsequenceAssessment {
    assess_full(input, rates, &[], county, scenarios, DRAWS)
}

/// [`assess`] with the parts of hazard rates that `rr-hazards` keeps apart
/// (`HazardAssessment::parts`), so each part is its own event class end to end. This is what
/// the engine runs.
pub fn assess_with_parts(
    input: &PlanInput,
    rates: &[HouseholdEventRate],
    parts: &[rr_hazards::RatePart],
    county: CountyData<'_>,
    scenarios: &[ScenarioCandidate],
) -> ConsequenceAssessment {
    assess_full(input, rates, parts, county, scenarios, DRAWS)
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
    assess_full(input, rates, &[], county, scenarios, draws)
}

/// The full entry point: whole rates, their parts and the number of draws.
pub fn assess_full(
    input: &PlanInput,
    rates: &[HouseholdEventRate],
    parts: &[rr_hazards::RatePart],
    county: CountyData<'_>,
    scenarios: &[ScenarioCandidate],
    draws: usize,
) -> ConsequenceAssessment {
    let table = effects::table();
    let states = scenario_states(input, scenarios);
    let binp = BuildInput {
        plan: input,
        rates,
        parts,
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
        county,
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
    let (ca_assessment, clean_air) = clean_air_bucket(&ctx);
    by_bucket.insert(BucketId::CleanAir, ca_assessment);
    let (income_assessment, income) = income_bucket(&ctx);
    by_bucket.insert(BucketId::Income, income_assessment);
    let (hl_assessment, home_loss) = home_loss_bucket(&ctx);
    by_bucket.insert(BucketId::HomeLoss, hl_assessment);

    let scenario_infos = scenario_summaries(&binp, &model, &states, rate);
    let warnings = cliff_warnings(&ctx, &details);
    let simultaneous = simultaneous_needs(&ctx, &details);
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
        clean_air,
        simultaneous,
        fragility: model.fragility,
        couplings: model.couplings.clone(),
        overrides: model.overrides.clone(),
        notes: model.notes.clone(),
        dial_rate: rate,
        horizon_years: years,
        draws: draws.n(),
    }
}

/// Shared context for building the buckets.
struct Ctx<'a> {
    input: &'a PlanInput,
    table: &'a EffectsTable,
    model: &'a Model,
    scenarios: &'a [ScenarioCandidate],
    county: CountyData<'a>,
    rate: f64,
    years: f64,
    draws: &'a Draws,
}

impl Ctx<'_> {
    fn owner_name(&self, owner: Owner) -> String {
        match owner {
            Owner::Hazard(h) => words::hazard_one(h).to_owned(),
            Owner::Scenario(i) => words::scenario_one(&self.scenarios[i].name),
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

/// A gas range boils water while the gas flows: the household said so (contract v2
/// `Housing::cooking`), or it lists a gas stove among what it has.
fn has_gas_stove(input: &PlanInput) -> bool {
    input.housing.cooking == Some(CookingFuel::Gas)
        || input
            .existing
            .iter()
            .any(|o| GAS_STOVE_ITEM_IDS.contains(&o.item_id.as_str()) && o.qty >= 1.0)
}

/// The next ladder step above `days` (365 stays 365): the structural widening of a range whose
/// target rests on a fallback (model review M-14).
fn next_step(days: f32) -> f32 {
    TARGET_LADDER_DAYS
        .iter()
        .copied()
        .find(|&l| l > days)
        .unwrap_or(TARGET_LADDER_DAYS[TARGET_LADDER_DAYS.len() - 1])
}

/// Share still out after `days` on a restoration curve given at a few day marks (share of the
/// peak), interpolated on log days; 1 before the first mark's day 0, and the last mark's share
/// (or nothing) after it.
fn share_out_after(points: &[(f32, f32)], days: f64) -> f64 {
    let mut pts: Vec<(f64, f64)> = points
        .iter()
        .map(|(d, s)| (f64::from(*d), f64::from(*s).clamp(0.0, 1.0)))
        .filter(|(d, _)| d.is_finite() && *d > 0.0)
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let Some(&(d0, s0)) = pts.first() else {
        return 0.0;
    };
    if days <= d0 {
        // From everyone out at the peak down to the first mark.
        return 1.0 - (1.0 - s0) * (days / d0).clamp(0.0, 1.0);
    }
    for w in pts.windows(2) {
        let ((a, sa), (b, sb)) = (w[0], w[1]);
        if days <= b {
            let t = (math::ln(days) - math::ln(a)) / (math::ln(b) - math::ln(a));
            return sa + (sb - sa) * t;
        }
    }
    pts.last().map_or(0.0, |p| p.1)
}

/// The worst event on record for a duration bucket (model review Part 3.3): the region's worst
/// power cut from the regional outage table, and for homes on public water the worst documented
/// water failure in the state. `covered_by_target` holds when nine in ten of the households it
/// reached had service back by the target (the backtest's "covered" rule).
fn stress_test(ctx: &Ctx<'_>, bucket: BucketId, target: f32) -> Option<StressTest> {
    match bucket {
        BucketId::Power => {
            let e = ctx.county.outage_model.and_then(|m| m.stress.as_ref())?;
            let date = Date::parse(&e.date).ok()?;
            if e.share_out_at_days.is_empty() {
                return None;
            }
            let peak = f64::from(e.peak_share).clamp(0.0, 1.0);
            let historic = e.source.starts_with("historic:");
            let region = if historic {
                "the territory's records".to_owned()
            } else {
                match e.distance_km {
                    Some(km) if km < 1.0 => "your county's records".to_owned(),
                    Some(km) => format!(
                        "your region's records (where it was worst, about {} away)",
                        words::distance_phrase(f64::from(km))
                    ),
                    None => "your region's records".to_owned(),
                }
            };
            let covered = share_out_after(&e.share_out_at_days, f64::from(target)) <= 0.1;
            Some(StressTest {
                event: e.event.clone(),
                date,
                region,
                share_out_at_days: e
                    .share_out_at_days
                    .iter()
                    .map(|(d, s)| (*d, (f64::from(*s) * peak) as f32))
                    .collect(),
                covered_by_target: covered,
                sources: vec![CitationId::from(if historic {
                    words::historic_source(&e.source)
                } else {
                    "ornl_eagle_i_outages"
                })],
            })
        }
        BucketId::WaterOut | BucketId::WaterBoil => {
            if ctx.input.housing.water == WaterSource::Well || ctx.county.state_abbr.is_empty() {
                return None;
            }
            let e = ctx
                .table
                .water_events
                .iter()
                .filter(|e| {
                    e.bucket == bucket && e.states.iter().any(|s| s == ctx.county.state_abbr)
                })
                .max_by(|a, b| a.p90_days.total_cmp(&b.p90_days).then(b.id.cmp(&a.id)))?;
            let date = Date::parse(&e.date).ok()?;
            Some(StressTest {
                event: e.event.clone(),
                date,
                region: e.place.clone(),
                share_out_at_days: if e.p90_days > e.median_days {
                    vec![(e.median_days as f32, 0.5), (e.p90_days as f32, 0.1)]
                } else {
                    vec![(e.median_days as f32, 1.0)]
                },
                covered_by_target: f64::from(target) >= e.p90_days,
                sources: e.sources.clone(),
            })
        }
        _ => None,
    }
}

/// The stress line in words, for the bucket's sentences.
fn stress_sentence(bucket: BucketId, st: &StressTest, target: f32) -> String {
    let year = &st.date.to_string()[..4];
    let what = match bucket {
        BucketId::Power => "homes that lost power",
        BucketId::WaterOut => "homes",
        _ => "homes",
    };
    let at = share_out_after(&st.share_out_at_days, f64::from(target));
    match bucket {
        BucketId::Power => {
            format!(
                "The worst power cut in {} was {} ({year}); being ready for {} would have {}.",
                st.region,
                st.event,
                words::ladder_phrase(target),
                if st.covered_by_target {
                    "outlasted it for at least 9 in 10 of the homes that lost power".to_owned()
                } else {
                    format!(
                        "left some {what} still waiting (about {} in 100 of all customers there                          were still out)",
                        words::per_100(100.0 * at)
                    )
                }
            )
        }
        _ => {
            let (lo, hi) = match st.share_out_at_days.as_slice() {
                [(m, _), (p, _)] => (*m, *p),
                [(m, _)] => (*m, *m),
                _ => (0.0, 0.0),
            };
            let long = if (hi - lo).abs() < 0.5 {
                words::days_phrase(f64::from(hi))
            } else {
                format!(
                    "{} for most, up to {}",
                    words::days_phrase(f64::from(lo)),
                    words::days_phrase(f64::from(hi))
                )
            };
            format!(
                "The worst {} on record in your state was in {} after {} ({year}): {long}. Being \
                 ready for {} {} that.",
                if bucket == BucketId::WaterOut {
                    "loss of tap water"
                } else {
                    "boil-water notice"
                },
                st.region,
                st.event,
                words::ladder_phrase(target),
                if st.covered_by_target {
                    "would have covered"
                } else {
                    "would not have covered all of"
                }
            )
        }
    }
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
    let mut high = range.high.max(ladder);
    // A target that rests on a fallback (no regional outage records, nothing known about the
    // water system) could be too short in ways the parameter draws cannot show: one ladder step
    // more at the high end (model review M-14).
    let gap = ladder > 0.0 && ctx.model.gaps.contains(&bucket);
    if gap {
        high = next_step(high);
    }
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

    let mut dial_table: Vec<DialPoint> = ReturnPeriod::ALL
        .iter()
        .map(|rp| {
            let r = dial_rate(*rp);
            DialPoint {
                return_period_years: rp.years(),
                target_days: eval.target(r, 26),
                ladder_days: eval.ladder_target(r),
                cliff: false,
            }
        })
        .collect();
    mark_cliffs(&eval, &mut dial_table);
    let curve = ExceedanceCurve::from_eval(bucket, &eval, target_c, ladder);

    // Relief for the design event.
    let design = order
        .first()
        .filter(|(_, s)| *s > 0.0 && target_c > 0.0)
        .map(|(i, _)| eval.terms()[*i]);
    let relief = design.and_then(|t| relief_for(ctx, t, ladder));

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
    // What the target rests on: the regional outage records or the fallback, the water system.
    sentences.extend(
        ctx.model
            .bucket_notes
            .iter()
            .filter(|(b, _)| *b == bucket)
            .map(|(_, s)| s.clone()),
    );
    // Rounding up to the ladder: say so when the raw number is just past a step (model review
    // M-15: a 6 % change can move the target a whole step).
    if let Some(note) = rounding_note(target_c, ladder) {
        sentences.push(note);
    }
    let gas_stove = bucket == BucketId::WaterBoil && has_gas_stove(ctx.input);
    if gas_stove {
        sentences.push(
            "Your gas stove can boil water during a boil-water notice, as long as the gas stays on."
                .to_owned(),
        );
    }
    let stress = if ladder > 0.0 {
        stress_test(ctx, bucket, ladder)
    } else {
        None
    };
    if let Some(st) = &stress {
        sentences.push(stress_sentence(bucket, st, ladder));
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
        covered_today: Target::Days {
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
            if let Some(st) = &stress {
                s.extend(st.sources.iter().cloned());
            }
            sorted_sources(s)
        },
        relief,
        stress_test: stress,
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
        design: design.map(|t| (t.hazard, t.scenario, t.class.clone())),
        structural_gap: gap,
        terms,
    };
    (assessment, detail)
}

/// "This rounds up to 5 days: before rounding it is about 3.2 days, just past 3." when the raw
/// target sits less than 15 % past the ladder step below the target (model review M-15).
fn rounding_note(raw: f64, ladder: f32) -> Option<String> {
    if raw <= 0.0 || ladder <= 0.0 {
        return None;
    }
    let below = TARGET_LADDER_DAYS.iter().copied().rfind(|&l| l < ladder)?;
    let b = f64::from(below);
    if raw > b * 1.15 || raw <= b {
        return None;
    }
    let raw_words = if raw < 10.0 {
        format!("{:.1} days", (raw * 10.0 + 0.5).floor() / 10.0)
    } else {
        words::days_phrase(raw)
    };
    Some(format!(
        "This rounds up to {}: before rounding it is about {raw_words}, just past {}.",
        words::ladder_phrase(ladder),
        words::ladder_phrase(below)
    ))
}

/// Marks the dial settings where the target jumps from the setting before it by the engine's
/// cliff rule (elasticity of at least 2 and 3 days or more), for the sweep (model review M-15:
/// only true cliffs, not every log-normal tail).
fn mark_cliffs(eval: &Eval<'_>, table: &mut [DialPoint]) {
    let rates: Vec<f64> = ReturnPeriod::ALL.iter().map(|rp| dial_rate(*rp)).collect();
    for j in 1..table.len() {
        let (a, b) = (table[j - 1].target_days, table[j].target_days);
        if a.is_nan() || b.is_nan() || b <= 0.0 {
            continue;
        }
        let step = math::ln(rates[j - 1] / rates[j]).abs();
        let e = math::ln((b + 0.05) / (a + 0.05)).abs() / step;
        if e < CLIFF_ELASTICITY || (b - a).abs() < CLIFF_MIN_DAYS || a.min(b) < CLIFF_FROM_DAYS {
            continue;
        }
        // One event dominates the jump's upper end, with a rate near that setting.
        let owners = owner_shares(eval, b);
        let Some(&(owner, share)) = owners.first() else {
            continue;
        };
        let owner_rate: f64 = eval
            .terms()
            .iter()
            .filter(|t| t.owner() == owner)
            .map(|t| t.rate * t.q)
            .sum();
        if share >= 0.5 && owner_rate >= rates[j] / 3.0 && owner_rate <= rates[j] * 3.0 {
            table[j].cliff = true;
        }
    }
}

/// Outages shorter than this do not count toward "mostly restored" for the design event: the
/// relief rating describes the disruptions a supplies target is for (model review M-13).
pub const RELIEF_FROM_DAYS: f64 = 1.0;

/// Relief for a design event: the row's own rating (Oregon Resilience Plan for Cascadia), or,
/// when its duration comes from a measured restoration curve, help within the 72-hour standard
/// (or sooner, if service is back sooner) and "mostly restored" when 90 % of the class's
/// outages that last at least [`RELIEF_FROM_DAYS`] are over.
///
/// A rating whose "mostly restored" comes out shorter than a third of the target describes some
/// other, smaller event than the one behind the target, so it is left out ("not known"): no
/// screen or packet may print a relief time that short without naming the event it belongs to.
fn relief_for(ctx: &Ctx<'_>, t: &Term, target_days: f32) -> Option<Relief> {
    relief_of(ctx, t).filter(|r| 3.0 * r.mostly_restored_days >= target_days)
}

fn relief_of(ctx: &Ctx<'_>, t: &Term) -> Option<Relief> {
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
    // "Mostly restored" for the design event: the time by which 90 % of this class's outages
    // that last at least a day are over. Over all outages (most of them an hour or two) the 90th
    // percentile describes ordinary outages, not the design event: Asheville read "mostly back
    // in half a day" beside a two-week target (model review M-13).
    let from = t.survival.sf(RELIEF_FROM_DAYS, 0.0);
    let restored = if from >= 1.0 {
        t.survival.p90_days()
    } else if from > 0.0 {
        t.survival.quantile_days(1.0 - 0.1 * from)
    } else {
        return None;
    };
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

/// The clean-air need (contract v2): its ten-year chance from the smoke, dust, ash and chemical
/// rows, and the unhealthy-air days a year for the sentence: the county's recorded smoke days
/// where the pack has them, plus the model's dust, ash and fume days; otherwise the model's
/// episodes times their typical length.
fn clean_air_bucket(ctx: &Ctx<'_>) -> (BucketAssessment, CleanAirDetail) {
    let bucket = BucketId::CleanAir;
    let (r, lo, hi, contributions, mut sources) = readiness_parts(ctx, bucket);
    let p = p10(r);
    let eval = Eval::central(ctx.model, bucket);
    let days_of = |smoke: bool| -> f64 {
        eval.terms()
            .iter()
            .filter(|t| (t.hazard == HazardId::WildfireSmoke) == smoke)
            .map(|t| t.rate * t.q * t.survival.mean_days())
            .sum()
    };
    let (smoke, from_record) = match ctx.county.smoke_days.filter(|d| d.is_finite() && *d >= 0.0) {
        Some(d) => {
            sources.push(CitationId::from("hms_aqs_smoke_days"));
            (d, true)
        }
        None => (days_of(true), false),
    };
    let days = smoke + days_of(false);
    let mut sentences = vec![readiness_sentence(ctx, bucket, r, lo, hi)];
    if days >= 0.5 {
        let n = words::round_nice(days);
        sentences.push(format!(
            "The air outside is unhealthy from smoke, dust or fumes about {n} day{} a year here{}.",
            if n == "1" { "" } else { "s" },
            if from_record {
                " (smoke days with fine particles at unhealthy levels, 2016-2023)"
            } else {
                ""
            }
        ));
    }
    if p > 0.0 {
        sentences.push(
            "What helps: a room you can seal, an air cleaner or a box-fan filter, and \
             well-fitting respirators for going outside."
                .to_owned(),
        );
    }
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
            covered_today: target,
            tier_enough: readiness_tier(ctx, p),
            contributions,
            frequency_sentences: sentences,
            sources: sorted_sources(sources),
            relief: None,
            stress_test: None,
        },
        CleanAirDetail {
            p_need_10yr: p,
            days_per_year: days,
            smoke_days_from_record: from_record,
        },
    )
}

/// DESIGN §4.7's simultaneous-need check: for each event class that sets a duration target, the
/// other duration needs the same event brings, at the same severity (the co-monotone convention
/// of the couplings): the design duration's survival level in the setting bucket, read off each
/// other bucket's duration for that event. The budget crate compares the stored water, food and
/// fuel with it (a warning, never a block).
fn simultaneous_needs(ctx: &Ctx<'_>, details: &[BucketDetail]) -> Vec<SimultaneousNeed> {
    let mut out: Vec<SimultaneousNeed> = Vec::new();
    for d in details {
        let Some((hazard, scenario, class)) = &d.design else {
            continue;
        };
        if d.ladder_days <= 0.0 {
            continue;
        }
        if let Some(n) = out.iter_mut().find(|n| {
            n.hazard == ctx.owner_hazard(owner_of(*hazard, *scenario))
                && n.scenario == scenario.map(|i| ctx.scenarios[i].id.clone())
                && n.event
                    == ctx
                        .model
                        .terms
                        .iter()
                        .find(|t| {
                            t.hazard == *hazard && t.scenario == *scenario && &t.class == class
                        })
                        .map_or(String::new(), |t| t.label.clone())
        }) {
            if !n.sets_target_of.contains(&d.bucket) {
                n.sets_target_of.push(d.bucket);
            }
            continue;
        }
        let same = |t: &&Term| t.hazard == *hazard && t.scenario == *scenario && &t.class == class;
        let Some(anchor) = ctx
            .model
            .terms
            .iter()
            .filter(same)
            .find(|t| t.bucket == d.bucket)
        else {
            continue;
        };
        let u = anchor
            .survival
            .sf(d.target_days, 0.0)
            .clamp(1e-6, 1.0 - 1e-6);
        let mut needs = Vec::new();
        for b in DURATION_BUCKETS {
            let Some(t) = ctx
                .model
                .terms
                .iter()
                .filter(same)
                .filter(|t| t.bucket == b)
                .max_by(|a, c| a.q.total_cmp(&c.q))
            else {
                continue;
            };
            let days = if b == d.bucket {
                f64::from(d.ladder_days)
            } else {
                t.survival.quantile_days(1.0 - u).max(t.threshold)
            };
            let chance = (t.q / anchor.q.max(1e-12)).min(1.0);
            needs.push((b, crate::curve::round_up_to_ladder(days), chance));
        }
        out.push(SimultaneousNeed {
            event: anchor.label.clone(),
            hazard: ctx.owner_hazard(anchor.owner()),
            scenario: scenario.map(|i| ctx.scenarios[i].id.clone()),
            sets_target_of: vec![d.bucket],
            needs,
        });
    }
    out
}

fn owner_of(hazard: HazardId, scenario: Option<usize>) -> Owner {
    match scenario {
        Some(i) => Owner::Scenario(i),
        None => Owner::Hazard(hazard),
    }
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
        covered_today: target,
        tier_enough: readiness_tier(ctx, p),
        contributions,
        frequency_sentences: sentences,
        sources,
        relief: None,
        stress_test: None,
    }
}

fn evacuate_bucket(ctx: &Ctx<'_>) -> (BucketAssessment, EvacuateDetail) {
    let bucket = BucketId::Evacuate;
    let (r, lo, hi, contributions, sources) = readiness_parts(ctx, bucket);
    let eval = Eval::central(ctx.model, bucket);
    let shares = eval.shares(0.0);
    let mut notice = [f64::INFINITY, 0.0f64];
    let mut causes = Vec::new();
    // Causes are the terms summed by hazard (a scenario counts under its family): the short end
    // of the warning is the least warning among causes whose own ten-year chance is at least 1 in
    // 1,000, however small their share (model review M-08: Lahaina's wildfire and local tsunami
    // each held under 5 % of the rate). The long end stays the most warning among terms with at
    // least 5 % of the rate.
    let mut by_cause: BTreeMap<HazardId, (f64, f64)> = BTreeMap::new();
    for (t, s) in eval.terms().iter().zip(&shares) {
        let band = t.notice_hours.unwrap_or([0.0, 0.0]);
        causes.push((t.hazard, t.label.clone(), t.rate * t.q, band));
        let h = ctx.owner_hazard(t.owner());
        let e = by_cause.entry(h).or_insert((0.0, f64::INFINITY));
        e.0 += t.rate * t.q;
        if t.rate * t.q > 0.0 {
            e.1 = e.1.min(band[0]);
        }
        if *s >= 0.05 {
            notice[1] = notice[1].max(band[1]);
        }
    }
    let mut fastest: Option<(HazardId, f64)> = None;
    let mut fastest_outside: Option<(HazardId, f64)> = None;
    for (h, (r, least)) in &by_cause {
        if p10(*r) < WARNING_CAUSE_P10 || !least.is_finite() {
            continue;
        }
        if fastest.is_none_or(|(_, n)| *least < n) {
            fastest = Some((*h, *least));
        }
        if *h != HazardId::HouseFire && fastest_outside.is_none_or(|(_, n)| *least < n) {
            fastest_outside = Some((*h, *least));
        }
    }
    match fastest {
        Some((_, n)) => notice[0] = n,
        None => {
            // No cause reaches 1 in 1,000: the least warning among the terms that matter.
            for (t, s) in eval.terms().iter().zip(&shares) {
                if *s >= 0.05 {
                    notice[0] = notice[0].min(t.notice_hours.unwrap_or([0.0, 0.0])[0]);
                }
            }
        }
    }
    if !notice[0].is_finite() {
        notice = [0.0, 0.0];
    }
    if notice[1] < notice[0] {
        notice[1] = notice[0];
    }
    let fastest_outside = fastest_outside
        .filter(|(h, _)| fastest.is_some_and(|(f, _)| f == HazardId::HouseFire && *h != f));
    // The longest time away among likely causes (90th percentile of their time away).
    let mut longest: Option<(HazardId, f64)> = None;
    for (h, (r, _)) in &by_cause {
        if p10(*r) < 0.01 {
            continue;
        }
        let p90 = eval
            .terms()
            .iter()
            .filter(|t| ctx.owner_hazard(t.owner()) == *h)
            .map(|t| t.survival.p90_days())
            .fold(0.0_f64, f64::max);
        if longest.is_none_or(|(_, d)| p90 > d) {
            longest = Some((*h, p90));
        }
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
        let cause = fastest.map_or(String::new(), |(h, _)| {
            format!(" ({})", words::hazard_one(h))
        });
        sentences.push(format!(
            "Warning can be as short as {}{cause} or as long as {}.",
            notice_words(notice[0]),
            notice_words(notice[1])
        ));
        if let Some((h, n)) = fastest_outside {
            sentences.push(format!(
                "For {}, plan for as little as {} of warning.",
                words::hazard_plural(h),
                notice_words(n)
            ));
        }
        sentences.push(format!(
            "Plan to be away for about {}.",
            words::ladder_phrase(days_away)
        ));
        if let Some((h, d)) = longest.filter(|(_, d)| *d >= 2.0 * f64::from(days_away).max(1.0)) {
            sentences.push(format!(
                "Most evacuations last a few days, but after {} you could be away for {} or more.",
                words::hazard_one(h),
                words::days_phrase(d)
            ));
        }
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
        covered_today: covered,
        tier_enough: readiness_tier(ctx, p),
        contributions,
        frequency_sentences: sentences,
        sources,
        relief: None,
        stress_test: None,
    };
    let detail = EvacuateDetail {
        p_need_10yr: p,
        rate_per_year: r,
        notice_hours: notice,
        days_away,
        causes,
        fastest_cause: fastest.map(|(h, _)| h),
        fastest_outside,
        longest_away: longest,
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
            covered_today: target,
            tier_enough: if commuters.is_empty() {
                TierId::Now
            } else {
                readiness_tier(ctx, p)
            },
            contributions,
            frequency_sentences: sentences,
            sources,
            relief: None,
            stress_test: None,
        },
        GetHomeDetail {
            p_need_10yr: p,
            commuters,
        },
    )
}

fn hours_on_foot(h: f64) -> String {
    if h < 0.75 {
        let m = words::round_nice(h * 60.0);
        return format!("{m} minute{}", if m == "1" { "" } else { "s" });
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
        "After a disaster, most displaced households are back within a month, but about 1 in {} {} never return, so insurance and copies of documents matter more than supplies here.",
        words::round_nice(1.0 / never),
        if renter { "renters" } else { "homeowners" }
    ));
    // Displacement cost (model review M-08): how long a household is out when damage forces it
    // out, from the displacement durations on the home-loss rows weighted by how often each cause
    // happens, and what living elsewhere that long costs.
    let eval = Eval::central(ctx.model, bucket);
    let months_p90 = if r > 0.0 {
        eval.target(0.1 * r, 30) / 30.44
    } else {
        0.0
    };
    let months_at_dial = eval.target(ctx.rate, 30) / 30.44;
    let housing = ctx
        .input
        .finances
        .monthly_expenses_usd
        .map(f64::from)
        .filter(|x| x.is_finite() && *x > 0.0)
        .map(|x| x * prm.housing_share_of_expenses.value);
    let cost = housing.filter(|_| months_p90 > 0.0).map(|h| h * months_p90);
    if months_p90 >= 0.5 && p > 0.0 {
        let months = words::round_nice(months_p90.max(1.0));
        let unit = if months == "1" { "month" } else { "months" };
        sentences.push(match cost {
            Some(c) => format!(
                "If damage forced you out, 9 in 10 households like yours would be home again \
                 within about {months} {unit}; living elsewhere that long costs about ${} at \
                 {} % of your monthly spending, which loss-of-use insurance pays for.",
                words::round_nice(c),
                words::round_nice(100.0 * prm.housing_share_of_expenses.value)
            ),
            None => format!(
                "If damage forced you out, 9 in 10 households like yours would be home again \
                 within about {months} {unit}."
            ),
        });
        sources.extend(prm.housing_share_of_expenses.sources.iter().cloned());
    }
    sentences.extend(coupling_sentences(ctx, bucket));
    // "Most back within a month" and "never return" are both from the Household Pulse report
    // (`census_pulse_displacement`: 56 % of renters and 71 % of owners back in under a month).
    sources.extend(prm.never_return_renter.sources.iter().cloned());
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
            covered_today: target,
            tier_enough: TierId::Now,
            contributions,
            frequency_sentences: sentences,
            sources: sorted_sources(sources),
            relief: None,
            stress_test: None,
        },
        HomeLossDetail {
            p_displaced_10yr: p,
            rate_per_year: r,
            share_back_within_week: prm.displaced_back_within_week.value,
            share_over_six_months: prm.displaced_over_six_months.value,
            share_never_return: never,
            months_away_p90: months_p90,
            months_away_at_dial: months_at_dial,
            displacement_cost_usd: cost,
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
        covered_today: Target::Months {
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
        stress_test: None,
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
                parts: binp.parts,
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
/// …and change by at least this many days, for the cliff warning…
const CLIFF_MIN_DAYS: f64 = 3.0;
/// …from a target of at least this many days on both sides: a target that appears from nothing
/// at the next setting (none at 1 in 10, 5 days at 1 in 50) is the dial doing its job, not one
/// event near the dial making the answer jump (model review M-15: mark only true cliffs).
const CLIFF_FROM_DAYS: f64 = 0.5;

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
                e >= CLIFF_ELASTICITY
                    && (p.target_days - t0).abs() >= CLIFF_MIN_DAYS
                    && p.target_days.min(t0) >= CLIFF_FROM_DAYS
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
        // The chosen setting and its neighbours (three settings), so the jump shows.
        let first = here
            .saturating_sub(1)
            .min(d.dial_table.len().saturating_sub(3));
        let cells: Vec<String> = d
            .dial_table
            .iter()
            .skip(first)
            .take(3)
            .map(|p| {
                format!(
                    "{} at 1 in {}",
                    target_words(d.bucket, p.ladder_days),
                    p.return_period_years
                )
            })
            .collect();
        let toggle = match owner {
            Owner::Scenario(_) => format!(
                " You can turn it off to see the plan without it; {}",
                capability_advice(d.bucket)
            ),
            Owner::Hazard(_) => String::new(),
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

/// The long-tail advice of a scenario cliff (DESIGN §4.4: capabilities over stockpiles), in the
/// words of the bucket it is about, so a phone or medicine warning never talks about water.
fn capability_advice(bucket: BucketId) -> &'static str {
    match bucket {
        BucketId::WaterOut | BucketId::WaterBoil => {
            "a long outage is better met with a way to make water safe, such as a filter and a nearby water source, than with ever more stored water."
        }
        BucketId::Power => {
            "a long outage is better met with ways to stay warm or cool, keep medicine cold and charge phones than with ever more fuel."
        }
        BucketId::Thermal => {
            "a long spell is better met with one room you can keep warm or cool, warm layers and a place to go than with stored fuel."
        }
        BucketId::Supplies => {
            "a long stretch is better met with staples you already eat and rotate, and with neighbours who share, than with ever bigger stockpiles."
        }
        BucketId::Medication => {
            "a long gap is better met by asking the prescriber about an emergency supply and early refills than by stockpiling alone."
        }
        BucketId::Comms => {
            "a long outage is better met with a battery or hand-crank radio, a way to charge phones and meeting places agreed in advance than with more gear."
        }
        _ => "a long disruption is better met with skills and plans than with bigger stockpiles.",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cliff_advice_speaks_about_its_own_bucket() {
        for b in [
            BucketId::Power,
            BucketId::Thermal,
            BucketId::Supplies,
            BucketId::Medication,
            BucketId::Comms,
        ] {
            let a = capability_advice(b);
            assert!(!a.contains("water"), "{b}: {a}");
        }
        for b in [BucketId::WaterOut, BucketId::WaterBoil] {
            assert!(capability_advice(b).contains("make water safe"));
        }
        assert!(capability_advice(BucketId::Comms).contains("radio"));
        assert!(capability_advice(BucketId::Medication).contains("prescriber"));
    }

    #[test]
    fn walking_times_are_singular_when_they_should_be() {
        assert_eq!(hours_on_foot(1.0 / 60.0), "1 minute");
        assert_eq!(hours_on_foot(0.5), "30 minutes");
        assert_eq!(hours_on_foot(1.0), "an hour");
    }
}
