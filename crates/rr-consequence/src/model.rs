//! Assembling the household's event model: which effect rows apply, at what rates, with which
//! county overrides and coupling rules.
//!
//! The result is a flat list of [`Term`]s. A term is one way a bucket can be disrupted:
//! `rate × share × S(max(d, threshold))` contributes to Λ_b(d), the yearly rate of disruptions
//! longer than `d` days. Uncertain inputs are named [`UParam`]s that the range calculation draws.

use std::collections::BTreeMap;

use rr_types::math::Z_90;
use rr_types::{
    BucketId, CitationId, Cooling, CountyRecord, EventRate, Evidence, HazardId, HouseholdEventRate,
    HousingKind, OutageStats, PlanInput, WaterSource,
};
use serde::{Deserialize, Serialize};

use crate::effects::{EffectRow, EffectsTable, Param, ReliefSpec, Requirement};
use crate::survival::{EmpiricalCurve, Survival};
use crate::words;

/// County data the consequence model can use. Everything is optional.
#[derive(Debug, Clone, Copy, Default)]
pub struct CountyData<'a> {
    /// Power outage statistics (EAGLE-I): replace the county-scale storm outage class.
    pub outages: Option<&'a OutageStats>,
    /// Event records by type: their median/p90 days replace matching durations.
    pub events: Option<&'a BTreeMap<String, EventRate>>,
    /// The county touches the coast.
    pub coastal: bool,
    /// The county has a tsunami hazard zone.
    pub tsunami_zone: bool,
}

impl<'a> CountyData<'a> {
    /// Everything the consequence model reads from a county record.
    pub fn from_record(record: &'a CountyRecord) -> CountyData<'a> {
        CountyData {
            outages: record.outages.as_ref(),
            events: Some(&record.events),
            coastal: record.coastal,
            tsunami_zone: record.tsunami_zone,
        }
    }
}

/// A named scenario offered for this location, as `rr-hazards` detects it.
///
/// `rr-hazards` owns scenario detection; this is the shape the consequence model needs. The plan
/// crate converts `rr-hazards`' candidate into this one. // awaiting: rr-hazards
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioCandidate {
    /// Stable id, for example `cascadia_m9`.
    pub id: String,
    /// Plain name, for example "A magnitude 9 Cascadia earthquake".
    pub name: String,
    /// The hazard family the scenario is counted under in contributions (`earthquake`).
    pub hazard: HazardId,
    /// Events per year.
    pub rate_per_year: f64,
    /// Low end of the plausible rate (10th percentile).
    pub low: f64,
    /// High end of the plausible rate (90th percentile).
    pub high: f64,
    /// Included in the plan (the engine's default or the user's override).
    pub on: bool,
    /// Why it applies here, in plain language.
    pub applies_because: String,
    /// Variant of the consequences (`coast` or `valley` for Cascadia); when absent, `coast` for a
    /// coastal county and `valley` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// Where the rate comes from.
    pub sources: Vec<CitationId>,
}

/// What kind of quantity an uncertain parameter scales (picks the default uncertainty factor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamWhat {
    Share,
    Duration,
}

/// An uncertain input: its draws multiply the central value by `exp(z·σ)`, with σ below the
/// median for z < 0 and above it for z > 0 (the 10th and 90th percentiles sit at z = ∓1.28).
#[derive(Debug, Clone)]
pub(crate) struct UParam {
    pub key: String,
    pub sigma_lo: f64,
    pub sigma_hi: f64,
    pub label: String,
    pub evidence: Evidence,
}

impl UParam {
    /// The multiplier's logarithm for standard-normal score `z`.
    #[inline]
    pub fn ln_mult(&self, z: f64) -> f64 {
        if z < 0.0 {
            z * self.sigma_lo
        } else {
            z * self.sigma_hi
        }
    }
}

/// How a term came to be.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Origin {
    /// A row of the effects table.
    Table,
    /// The county-scale storm outage class (from county outage statistics or the fallback).
    Pool,
    /// Derived by a coupling rule.
    Coupling(&'static str),
}

/// A floor under a term's duration: the consequence lasts at least as long as this other
/// disruption (a well cannot pump while the power is out). Durations are taken as co-monotone, so
/// P(max(D₁, D₂) > d) = max(S₁(d), S₂(d)).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MaxWith {
    pub survival: Survival,
    pub dur_param: Option<usize>,
    pub threshold: f64,
}

/// One way a bucket can be disrupted.
#[derive(Debug, Clone)]
pub(crate) struct Term {
    pub bucket: BucketId,
    pub hazard: HazardId,
    /// Index into the scenario candidates, for scenario events.
    pub scenario: Option<usize>,
    pub class: String,
    pub label: String,
    /// Events per year reaching the household (r_h, or the scenario's rate).
    pub rate: f64,
    /// Chance an event causes this consequence (≤ 1).
    pub q: f64,
    pub survival: Survival,
    /// The consequence exists only when the underlying outage outlasts this many days.
    pub threshold: f64,
    /// The consequence lasts at least as long as this (see [`MaxWith`]).
    pub max_with: Option<MaxWith>,
    pub rate_param: Option<usize>,
    pub q_param: Option<usize>,
    pub dur_param: Option<usize>,
    pub evidence: Evidence,
    /// The duration rests on measured records (restoration curves, county outage records).
    pub duration_from_data: bool,
    pub sources: Vec<CitationId>,
    pub relief: Option<ReliefSpec>,
    pub notice_hours: Option<[f64; 2]>,
    pub heat_share: f64,
    pub cold_share: f64,
    pub origin: Origin,
}

impl Term {
    /// Whose event this is, for grouping (hazard or scenario).
    pub fn owner(&self) -> Owner {
        match self.scenario {
            Some(i) => Owner::Scenario(i),
            None => Owner::Hazard(self.hazard),
        }
    }
}

/// Who an event belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Owner {
    Hazard(HazardId),
    Scenario(usize),
}

/// One income stream for this household.
#[derive(Debug, Clone)]
pub(crate) struct IncomeTerm {
    pub hazard: HazardId,
    pub label: String,
    /// Spells per year across the household.
    pub rate: f64,
    pub rate_param: Option<usize>,
    /// Spell length in weeks.
    pub spell: Survival,
    pub dur_param: Option<usize>,
    pub unemployment_insurance: bool,
    pub sources: Vec<CitationId>,
}

/// A coupling rule that fired for this household.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CouplingApplied {
    /// Stable id of the rule, for example `well_pump`.
    pub id: String,
    /// What it means for this household, in plain language.
    pub plain: String,
    /// The buckets it changes.
    pub buckets: Vec<BucketId>,
    /// Which crate applies it (`rr-consequence`, or a note for `rr-supply`, `rr-budget`,
    /// `rr-hazards`).
    pub applied_in: String,
    /// Where it comes from.
    pub sources: Vec<CitationId>,
}

/// A county record that replaced a default.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OverrideApplied {
    /// The bucket whose durations changed.
    pub bucket: BucketId,
    /// The hazards affected.
    pub hazards: Vec<HazardId>,
    /// What changed, in plain language.
    pub plain: String,
    /// Where the county numbers come from.
    pub sources: Vec<CitationId>,
}

/// The household facts the coupling rules and row requirements look at.
#[derive(Debug, Clone)]
pub(crate) struct Household {
    pub well: bool,
    pub high_rise_pumped: bool,
    pub heating_needs_power: bool,
    pub wood_heat: bool,
    pub no_heating: bool,
    pub air_conditioned: bool,
    pub refrigerated_rx: bool,
    pub commuter: bool,
    pub mobile_home: bool,
    pub coastal: bool,
    pub earners: u8,
}

impl Household {
    pub fn from_input(input: &PlanInput, county: &CountyData<'_>, table: &EffectsTable) -> Self {
        use rr_types::Heating;
        let h = &input.housing;
        let booster = table.params.booster_pump_floor.value;
        Household {
            well: h.water == WaterSource::Well,
            high_rise_pumped: h.kind == HousingKind::ApartmentHighRise
                && f64::from(h.floor) >= booster
                && h.water == WaterSource::Municipal,
            heating_needs_power: matches!(
                h.heating,
                Heating::Gas
                    | Heating::ElectricResistance
                    | Heating::HeatPump
                    | Heating::Oil
                    | Heating::Propane
                    | Heating::District
            ),
            wood_heat: h.heating == Heating::Wood,
            no_heating: h.heating == Heating::None,
            air_conditioned: matches!(h.cooling, Cooling::Central | Cooling::Window),
            refrigerated_rx: input.people.iter().any(|p| p.medical.refrigerated_rx),
            commuter: input
                .people
                .iter()
                .any(|p| p.commute.as_ref().is_some_and(|c| c.distance_km > 0.0)),
            mobile_home: h.kind == HousingKind::MobileHome,
            coastal: county.coastal,
            earners: input.finances.income.earners,
        }
    }

    fn meets(&self, req: Option<Requirement>) -> bool {
        match req {
            None => true,
            Some(Requirement::Municipal) => !self.well,
            Some(Requirement::Well) => self.well,
            Some(Requirement::Commuter) => self.commuter,
            Some(Requirement::NoCooling) => !self.air_conditioned,
            Some(Requirement::NoHeating) => self.no_heating,
            Some(Requirement::MobileHome) => self.mobile_home,
            Some(Requirement::Coastal) => self.coastal,
        }
    }
}

/// Everything the curves need, for one on/off state of the scenarios.
#[derive(Debug, Clone)]
pub(crate) struct Model {
    pub terms: Vec<Term>,
    pub params: Vec<UParam>,
    pub income: Vec<IncomeTerm>,
    pub couplings: Vec<CouplingApplied>,
    pub overrides: Vec<OverrideApplied>,
    /// Sources of each hazard's rate.
    pub rate_sources: BTreeMap<HazardId, Vec<CitationId>>,
    /// Plain notes (for example a fallback that was used).
    pub notes: Vec<String>,
    /// The job-loss rate came from the national base rate, not `rr-hazards`.
    pub job_loss_fallback: bool,
}

impl Model {
    /// Terms of one bucket.
    pub fn bucket_terms(&self, bucket: BucketId) -> impl Iterator<Item = &Term> {
        self.terms.iter().filter(move |t| t.bucket == bucket)
    }
}

/// Collects uncertain parameters, deduplicated by key.
#[derive(Default)]
struct ParamSet {
    params: Vec<UParam>,
    index: BTreeMap<String, usize>,
}

impl ParamSet {
    fn add(
        &mut self,
        key: String,
        sigma_lo: f64,
        sigma_hi: f64,
        label: String,
        evidence: Evidence,
    ) -> Option<usize> {
        if let Some(&i) = self.index.get(&key) {
            return Some(i);
        }
        if sigma_lo.is_nan() || sigma_hi.is_nan() || (sigma_lo <= 0.0 && sigma_hi <= 0.0) {
            return None;
        }
        let i = self.params.len();
        self.index.insert(key.clone(), i);
        self.params.push(UParam {
            key,
            sigma_lo: sigma_lo.max(0.0),
            sigma_hi: sigma_hi.max(0.0),
            label,
            evidence,
        });
        Some(i)
    }

    /// A rate given as central value with a 10th–90th percentile range.
    fn rate(
        &mut self,
        key: String,
        central: f64,
        low: f64,
        high: f64,
        label: String,
        evidence: Evidence,
    ) -> Option<usize> {
        if central.is_nan() || central <= 0.0 {
            return None;
        }
        // A low end of 0 cannot be a multiplicative 10th percentile; treat it as a tenth of the rate.
        let lo = if low > 0.0 { low } else { central / 10.0 };
        let sigma_lo = if lo < central {
            rr_types::math::ln(central / lo) / Z_90
        } else {
            0.0
        };
        let sigma_hi = if high > central {
            rr_types::math::ln(high / central) / Z_90
        } else {
            0.0
        };
        self.add(key, sigma_lo, sigma_hi, label, evidence)
    }

    /// A symmetric multiplicative factor k (the 90th percentile is ×k, the 10th ÷k).
    fn factor(&mut self, key: String, k: f64, label: String, evidence: Evidence) -> Option<usize> {
        if k.is_nan() || k <= 1.0 {
            return None;
        }
        let s = rr_types::math::ln(k) / Z_90;
        self.add(key, s, s, label, evidence)
    }
}

/// Inputs to [`build`], borrowed.
pub(crate) struct BuildInput<'a> {
    pub plan: &'a PlanInput,
    pub rates: &'a [HouseholdEventRate],
    pub county: CountyData<'a>,
    pub scenarios: &'a [ScenarioCandidate],
    /// On/off per scenario candidate (same order as `scenarios`).
    pub scenario_on: &'a [bool],
    pub table: &'a EffectsTable,
}

fn event_override<'e>(
    events: Option<&'e BTreeMap<String, EventRate>>,
    keys: &[String],
) -> Option<(&'e str, f64, Option<f64>)> {
    let events = events?;
    keys.iter().find_map(|k| {
        let (name, rec) = events.get_key_value(k)?;
        let median = f64::from(rec.median_days?);
        if !median.is_finite() || median <= 0.0 {
            return None;
        }
        let p90 = rec
            .p90_days
            .map(f64::from)
            .filter(|p| p.is_finite() && *p >= median);
        Some((name.as_str(), median, p90))
    })
}

/// The survival curve from county outage statistics, or `None` if they are unusable.
pub(crate) fn outage_curve(stats: &OutageStats) -> Option<(Survival, usize)> {
    let pts = [
        (f64::from(stats.median_hours) / 24.0, 0.5),
        (f64::from(stats.p90_hours) / 24.0, 0.1),
        (1.0, f64::from(stats.p_ge_1d)),
        (3.0, f64::from(stats.p_ge_3d)),
        (7.0, f64::from(stats.p_ge_7d)),
        (14.0, f64::from(stats.p_ge_14d)),
    ];
    EmpiricalCurve::from_points(&pts).map(|(c, dropped)| (Survival::Empirical(c), dropped))
}

fn default_factor(table: &EffectsTable, evidence: Evidence, what: ParamWhat) -> f64 {
    let p = &table.params;
    match (what, evidence) {
        (ParamWhat::Duration, Evidence::Empirical) => p.duration_factor_empirical.value,
        (ParamWhat::Duration, Evidence::Prior) => p.duration_factor_prior.value,
        (_, Evidence::Empirical) => p.p_factor_empirical.value,
        (_, Evidence::Prior) => p.p_factor_prior.value,
    }
}

fn scenario_variant(c: &ScenarioCandidate, county: &CountyData<'_>) -> String {
    c.variant.clone().unwrap_or_else(|| {
        if county.coastal {
            "coast".to_owned()
        } else {
            "valley".to_owned()
        }
    })
}

/// Builds the model for one scenario state.
pub(crate) fn build(inp: &BuildInput<'_>) -> Model {
    let table = inp.table;
    let hh = Household::from_input(inp.plan, &inp.county, table);
    let mut ps = ParamSet::default();
    let mut terms: Vec<Term> = Vec::new();
    let mut overrides: Vec<OverrideApplied> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    // Hazard rates: the first valid entry per hazard wins.
    let mut rates: BTreeMap<HazardId, &HouseholdEventRate> = BTreeMap::new();
    for r in inp.rates {
        if r.validate().is_ok() && r.rate_per_year > 0.0 {
            rates.entry(r.hazard).or_insert(r);
        }
    }
    let rate_sources: BTreeMap<HazardId, Vec<CitationId>> =
        rates.iter().map(|(h, r)| (*h, r.sources.clone())).collect();
    let mut rate_param: BTreeMap<HazardId, Option<usize>> = BTreeMap::new();
    for (h, r) in &rates {
        let id = ps.rate(
            format!("rate:{h}"),
            r.rate_per_year,
            r.low,
            r.high,
            format!(
                "how often {} happen where you live",
                words::hazard_plural(*h)
            ),
            r.evidence,
        );
        rate_param.insert(*h, id);
    }
    let offered: Vec<&str> = inp.scenarios.iter().map(|c| c.id.as_str()).collect();
    let mut scenario_param: Vec<Option<usize>> = Vec::with_capacity(inp.scenarios.len());
    for c in inp.scenarios {
        scenario_param.push(ps.rate(
            format!("rate:scenario:{}", c.id),
            c.rate_per_year,
            c.low,
            c.high,
            format!("how likely {} is", words::lower_first(&c.name)),
            Evidence::Prior,
        ));
    }

    // (owner, class, bucket) -> total share of applicable table rows, for the coupling rules.
    let mut table_share: BTreeMap<(Owner, String, BucketId), f64> = BTreeMap::new();

    for row in &table.effects {
        if let Some(s) = &row.replaced_by {
            if offered.contains(&s.as_str()) {
                continue;
            }
        }
        if !hh.meets(row.requires) {
            continue;
        }
        let (rate, rparam, scenario) = match &row.scenario {
            Some(sid) => {
                let Some(i) = inp.scenarios.iter().position(|c| &c.id == sid) else {
                    continue;
                };
                let c = &inp.scenarios[i];
                if !inp.scenario_on.get(i).copied().unwrap_or(c.on)
                    || c.rate_per_year.is_nan()
                    || c.rate_per_year <= 0.0
                {
                    continue;
                }
                if let Some(v) = &row.variant {
                    if *v != scenario_variant(c, &inp.county) {
                        continue;
                    }
                }
                (c.rate_per_year, scenario_param[i], Some(i))
            }
            None => match rates.get(&row.hazard) {
                Some(r) => (r.rate_per_year, rate_param[&row.hazard], None),
                None => continue,
            },
        };
        let term = table_term(row, rate, rparam, scenario, inp, &mut ps, &mut overrides);
        *table_share
            .entry((term.owner(), row.class.clone(), row.bucket))
            .or_insert(0.0) += row.p_given_event;
        terms.push(term);
    }

    add_pool_terms(inp, &mut ps, &mut terms, &mut overrides, &mut notes);

    let mut couplings: Vec<CouplingApplied> = Vec::new();
    apply_couplings(&hh, inp, &mut ps, &mut terms, &table_share, &mut couplings);
    household_notes(inp.plan, &hh, &mut couplings);

    let (income, job_loss_fallback) =
        income_terms(inp, &hh, &rates, &rate_param, &scenario_param, &mut ps);
    if job_loss_fallback {
        notes.push(
            "No job-loss rate was supplied, so the national rate (8.3 in 100 workers a year, \
             adjusted for how steady the income is) was used."
                .to_owned(),
        );
    }

    terms.sort_by_key(|t| t.bucket);
    Model {
        terms,
        params: ps.params,
        income,
        couplings,
        overrides,
        rate_sources,
        notes,
        job_loss_fallback,
    }
}

fn table_term(
    row: &EffectRow,
    rate: f64,
    rparam: Option<usize>,
    scenario: Option<usize>,
    inp: &BuildInput<'_>,
    ps: &mut ParamSet,
    overrides: &mut Vec<OverrideApplied>,
) -> Term {
    let table = inp.table;
    let key = row.key();
    let mut sources = row.sources.clone();
    let mut duration_from_data = row.duration_basis() == Evidence::Empirical;
    let mut dur_evidence = row.duration_basis();
    let survival = match row.duration {
        None => Survival::Fixed { days: 0.0 },
        Some(dist) => match event_override(inp.county.events, &row.event_keys) {
            Some((name, median, p90)) => {
                let (m0, p0) = match dist {
                    rr_types::DurationDist::LogNormal {
                        median_days,
                        p90_days,
                    } => (median_days, p90_days),
                    rr_types::DurationDist::Fixed { days } => (days, days),
                };
                let p90 = p90.unwrap_or(median * p0 / m0);
                duration_from_data = true;
                dur_evidence = Evidence::Empirical;
                let src = CitationId::from(words::event_source(name));
                sources.push(src.clone());
                overrides.push(OverrideApplied {
                    bucket: row.bucket,
                    hazards: vec![row.hazard],
                    plain: format!(
                        "{}: this county's records ({}) give a median of {} and a bad case of {} \
                         instead of the national default.",
                        words::bucket_short(row.bucket),
                        name.replace('_', " "),
                        words::days_phrase(median),
                        words::days_phrase(p90)
                    ),
                    sources: vec![src],
                });
                Survival::log_normal(median, p90)
            }
            None => Survival::from_dist(&dist),
        },
    };
    let q_param = if row.p_given_event > 0.0 && row.p_given_event < 1.0 {
        let k = row
            .p_factor
            .unwrap_or_else(|| default_factor(table, row.evidence, ParamWhat::Share));
        ps.factor(
            format!("share:{key}"),
            k,
            format!(
                "how often {} {}",
                row.label,
                words::bucket_cause_verb(row.bucket)
            ),
            row.evidence,
        )
    } else {
        None
    };
    let dur_param = if row.duration.is_some() {
        let k = row
            .duration_factor
            .unwrap_or_else(|| default_factor(table, dur_evidence, ParamWhat::Duration));
        ps.factor(
            format!("dur:{key}"),
            k,
            format!(
                "how long {} from {} last",
                words::bucket_noun_plural(row.bucket),
                row.label
            ),
            dur_evidence,
        )
    } else {
        None
    };
    Term {
        bucket: row.bucket,
        hazard: row.hazard,
        scenario,
        class: row.class.clone(),
        label: row.label.clone(),
        rate,
        q: row.p_given_event,
        survival,
        threshold: 0.0,
        max_with: None,
        rate_param: rparam,
        q_param,
        dur_param,
        evidence: row.evidence,
        duration_from_data,
        sources,
        relief: row.relief.clone(),
        notice_hours: row.notice_hours,
        heat_share: row.heat_share,
        cold_share: row.cold_share,
        origin: Origin::Table,
    }
}

/// County-scale storm outages. With county outage statistics, they happen at the county's
/// measured rate (`events_per_customer_year`) with its measured duration curve, split between the
/// storm hazards in proportion to their short-outage rates. Without them, a third of short storm
/// outages are taken to be county-scale (`storm_pool_fallback_ratio`).
fn add_pool_terms(
    inp: &BuildInput<'_>,
    ps: &mut ParamSet,
    terms: &mut Vec<Term>,
    overrides: &mut Vec<OverrideApplied>,
    notes: &mut Vec<String>,
) {
    let table = inp.table;
    let pool_rows: Vec<usize> = terms
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            t.bucket == BucketId::Power
                && t.origin == Origin::Table
                && t.scenario.is_none()
                && table
                    .effects
                    .iter()
                    .any(|r| r.pool && r.hazard == t.hazard && r.class == t.class)
        })
        .map(|(i, _)| i)
        .collect();
    let total: f64 = pool_rows.iter().map(|&i| terms[i].rate * terms[i].q).sum();
    let from_stats = inp
        .county
        .outages
        .filter(|s| s.events_per_customer_year.is_finite() && s.events_per_customer_year > 0.0)
        .and_then(|s| outage_curve(s).map(|(c, dropped)| (s, c, dropped)));
    let mut new_terms = Vec::new();
    match from_stats {
        Some((stats, curve, dropped)) => {
            let rate = f64::from(stats.events_per_customer_year);
            let rp = ps.factor(
                "rate:outage_stats".to_owned(),
                table.params.outage_stats_rate_factor.value,
                "how often county-wide storm outages happen here".to_owned(),
                Evidence::Empirical,
            );
            let dp = ps.factor(
                "dur:outage_stats".to_owned(),
                table.params.duration_factor_empirical.value,
                "how long county-wide storm outages last here".to_owned(),
                Evidence::Empirical,
            );
            let source = CitationId::from("eagle_i_outages");
            // Attribution: by short-outage rate; with no storm rates at all, to strong wind.
            let shares: Vec<(HazardId, f64, f64, f64)> = if total > 0.0 {
                pool_rows
                    .iter()
                    .map(|&i| {
                        let t = &terms[i];
                        (t.hazard, t.rate * t.q / total, t.heat_share, t.cold_share)
                    })
                    .collect()
            } else {
                vec![(HazardId::StrongWind, 1.0, 0.15, 0.4)]
            };
            let mut hazards = Vec::new();
            for (hazard, share, heat, cold) in shares {
                hazards.push(hazard);
                new_terms.push(Term {
                    bucket: BucketId::Power,
                    hazard,
                    scenario: None,
                    class: "county".to_owned(),
                    label: "county-wide storm outages".to_owned(),
                    rate: rate * share,
                    q: 1.0,
                    survival: curve.clone(),
                    threshold: 0.0,
                    max_with: None,
                    rate_param: rp,
                    q_param: None,
                    dur_param: dp,
                    evidence: Evidence::Empirical,
                    duration_from_data: true,
                    sources: vec![source.clone()],
                    relief: None,
                    notice_hours: None,
                    heat_share: heat,
                    cold_share: cold,
                    origin: Origin::Pool,
                });
            }
            hazards.sort();
            hazards.dedup();
            overrides.push(OverrideApplied {
                bucket: BucketId::Power,
                hazards,
                plain: format!(
                    "Power: county-wide storm outages use this county's outage records ({}): a home \
                     here is caught in one about {}; half are over within {}, 9 in 10 within {}.",
                    stats.years_covered,
                    words::rate_phrase(rate),
                    words::hours_phrase(f64::from(stats.median_hours)),
                    words::hours_phrase(f64::from(stats.p90_hours))
                ),
                sources: vec![source],
            });
            if dropped > 0 {
                notes.push(format!(
                    "{dropped} of the county's outage-duration figures were left out because they \
                     were missing or did not fit the others."
                ));
            }
        }
        None => {
            let p = &table.params.storm_pool_fallback_ratio;
            let dist = p.duration_or(rr_types::DurationDist::LogNormal {
                median_days: 2.0 / 24.0,
                p90_days: 20.0 / 24.0,
            });
            let qp = ps.rate(
                "share:storm_pool_fallback".to_owned(),
                p.value,
                p.low.unwrap_or(p.value),
                p.high.unwrap_or(p.value),
                "how many storm outages are county-wide".to_owned(),
                p.evidence,
            );
            let dp = ps.factor(
                "dur:storm_pool_fallback".to_owned(),
                table.params.duration_factor_prior.value,
                "how long county-wide storm outages last".to_owned(),
                Evidence::Prior,
            );
            for &i in &pool_rows {
                let t = &terms[i];
                new_terms.push(Term {
                    bucket: BucketId::Power,
                    hazard: t.hazard,
                    scenario: None,
                    class: "county".to_owned(),
                    label: "county-wide storm outages".to_owned(),
                    rate: t.rate,
                    q: (t.q * p.value).min(1.0),
                    survival: Survival::from_dist(&dist),
                    threshold: 0.0,
                    max_with: None,
                    rate_param: t.rate_param,
                    q_param: qp,
                    dur_param: dp,
                    evidence: Evidence::Prior,
                    duration_from_data: false,
                    sources: p.sources.clone(),
                    relief: None,
                    notice_hours: None,
                    heat_share: t.heat_share,
                    cold_share: t.cold_share,
                    origin: Origin::Pool,
                });
            }
        }
    }
    terms.extend(new_terms);
}

fn coupling_note(
    couplings: &mut Vec<CouplingApplied>,
    id: &str,
    plain: String,
    buckets: &[BucketId],
    applied_in: &str,
    sources: &[CitationId],
) {
    if couplings.iter().any(|c| c.id == id) {
        return;
    }
    couplings.push(CouplingApplied {
        id: id.to_owned(),
        plain,
        buckets: buckets.to_vec(),
        applied_in: applied_in.to_owned(),
        sources: sources.to_vec(),
    });
}

/// A derived term: the same event (same rate, share and duration draws) feeding another bucket.
fn derive(source: &Term, bucket: BucketId, q: f64, threshold: f64, rule: &'static str) -> Term {
    let mut t = source.clone();
    t.bucket = bucket;
    t.q = q.clamp(0.0, 1.0);
    t.threshold = threshold;
    t.max_with = None;
    t.origin = Origin::Coupling(rule);
    t.notice_hours = None;
    t.heat_share = 0.0;
    t.cold_share = 0.0;
    t.relief = if bucket == BucketId::WaterOut {
        source.relief.clone()
    } else {
        None
    };
    t.sources.push(CitationId::from("prior_rr_coupling"));
    t
}

fn apply_couplings(
    hh: &Household,
    inp: &BuildInput<'_>,
    ps: &mut ParamSet,
    terms: &mut Vec<Term>,
    table_share: &BTreeMap<(Owner, String, BucketId), f64>,
    couplings: &mut Vec<CouplingApplied>,
) {
    let table = inp.table;
    let prm = &table.params;
    let coupling_src = vec![CitationId::from("prior_rr_coupling")];
    let already = |t: &Term, b: BucketId| -> f64 {
        table_share
            .get(&(t.owner(), t.class.clone(), b))
            .copied()
            .unwrap_or(0.0)
    };
    let power: Vec<Term> = terms
        .iter()
        .filter(|t| t.bucket == BucketId::Power)
        .cloned()
        .collect();
    let mut derived: Vec<Term> = Vec::new();
    let mut floors: Vec<(usize, MaxWith, Option<usize>)> = Vec::new();
    let mut fired: Vec<&'static str> = Vec::new();
    let water_rule = if hh.well {
        "well_pump"
    } else {
        "high_rise_pumps"
    };
    for t in &power {
        // Water (well or high-rise pumps) and refrigerated medicine last at least as long as the
        // power cut. Table rows of the same event class get the power cut as a floor; the rest of
        // the power cuts become new terms.
        for (bucket, active, threshold, rule) in [
            (
                BucketId::WaterOut,
                hh.well || hh.high_rise_pumped,
                0.0,
                water_rule,
            ),
            (
                BucketId::Medication,
                hh.refrigerated_rx,
                prm.cold_chain_days.value,
                "refrigerated_medicine",
            ),
        ] {
            if !active {
                continue;
            }
            let overlap: Vec<usize> = terms
                .iter()
                .enumerate()
                .filter(|(_, w)| {
                    w.bucket == bucket
                        && w.origin == Origin::Table
                        && w.owner() == t.owner()
                        && w.class == t.class
                })
                .map(|(i, _)| i)
                .collect();
            let q_table: f64 = overlap.iter().map(|&i| terms[i].q).sum();
            for &i in &overlap {
                floors.push((
                    i,
                    MaxWith {
                        survival: t.survival.clone(),
                        dur_param: t.dur_param,
                        threshold,
                    },
                    t.q_param,
                ));
            }
            let q = t.q - q_table;
            if q > 0.0 {
                derived.push(derive(t, bucket, q, threshold, rule));
            }
            if (q > 0.0 || !overlap.is_empty()) && !fired.contains(&rule) {
                fired.push(rule);
            }
        }
        if hh.heating_needs_power && t.cold_share > 0.0 {
            let q = t.q * t.cold_share - already(t, BucketId::Thermal);
            if q > 0.0 {
                derived.push(derive(
                    t,
                    BucketId::Thermal,
                    q,
                    prm.cold_onset_days.value,
                    "heating_needs_power",
                ));
                if !fired.contains(&"heating_needs_power") {
                    fired.push("heating_needs_power");
                }
            }
        }
        if hh.air_conditioned && t.heat_share > 0.0 {
            let q = t.q * t.heat_share - already(t, BucketId::Thermal);
            if q > 0.0 {
                derived.push(derive(t, BucketId::Thermal, q, 0.0, "cooling_needs_power"));
                if !fired.contains(&"cooling_needs_power") {
                    fired.push("cooling_needs_power");
                }
            }
        }
    }
    for (i, floor, q_param) in floors {
        let w = &mut terms[i];
        w.max_with = Some(floor);
        // The shared part moves with the power cut's share, so the coupled bucket never falls
        // below power in any draw.
        if q_param.is_some() {
            w.q_param = q_param;
        }
        w.sources.push(CitationId::from("prior_rr_coupling"));
    }
    let (f_high_rise, f_heating, f_cooling, f_rx) = (
        fired.contains(&"high_rise_pumps"),
        fired.contains(&"heating_needs_power"),
        fired.contains(&"cooling_needs_power"),
        fired.contains(&"refrigerated_medicine"),
    );
    let activated = |class: &str| terms.iter().any(|t| t.class == class);
    let (f_no_cooling, f_no_heating) = (activated("no_cooling"), activated("no_heating"));
    terms.extend(derived);

    if hh.well {
        let p = &prm.well_pump_failure;
        terms.push(fixed_rate_term(
            ps,
            "well_pump_failure",
            p,
            HazardId::LocalUtilityOutage,
            BucketId::WaterOut,
            "well pump failures",
            "how often a well pump fails",
        ));
        coupling_note(
            couplings,
            "well_pump",
            "Your well pump runs on electricity, so every power cut is also a water cut. The pump \
             itself also fails now and then (about once in 14 years)."
                .to_owned(),
            &[BucketId::WaterOut],
            "rr-consequence",
            &coupling_src,
        );
    }
    if hh.high_rise_pumped && f_high_rise {
        coupling_note(
            couplings,
            "high_rise_pumps",
            format!(
                "Above about floor {}, water and elevators rely on electric pumps, so a power cut \
                 can also stop your water and trap people who cannot use stairs.",
                prm.booster_pump_floor.value as i64 - 1
            ),
            &[BucketId::WaterOut, BucketId::Evacuate],
            "rr-consequence",
            &prm.booster_pump_floor.sources,
        );
    }
    if hh.heating_needs_power && f_heating {
        coupling_note(
            couplings,
            "heating_needs_power",
            "Your heating needs electricity to run (a gas furnace needs it for the igniter and \
             fan), so a long winter power cut also cuts your heat."
                .to_owned(),
            &[BucketId::Thermal],
            "rr-consequence",
            &prm.cold_onset_days.sources,
        );
    }
    if hh.wood_heat {
        let p = &prm.wood_heat_shortfall;
        terms.push(fixed_rate_term(
            ps,
            "wood_heat_shortfall",
            p,
            HazardId::ColdWave,
            BucketId::Thermal,
            "a long winter outage that outruns the wood stove",
            "how often a winter outage outruns a wood stove",
        ));
        coupling_note(
            couplings,
            "wood_heat",
            "Your wood stove keeps the house warm without power, so winter power cuts are not a \
             heating emergency unless the wood runs out."
                .to_owned(),
            &[BucketId::Thermal],
            "rr-consequence",
            &p.sources,
        );
    }
    if hh.no_heating && f_no_heating {
        coupling_note(
            couplings,
            "no_heating",
            "With no heating system, a cold wave itself makes the house dangerously cold."
                .to_owned(),
            &[BucketId::Thermal],
            "rr-consequence",
            &coupling_src,
        );
    }
    if hh.air_conditioned && f_cooling {
        coupling_note(
            couplings,
            "cooling_needs_power",
            "A power cut during a heat wave also stops your air conditioning.".to_owned(),
            &[BucketId::Thermal],
            "rr-consequence",
            &coupling_src,
        );
    } else if !hh.air_conditioned && f_no_cooling {
        coupling_note(
            couplings,
            "no_cooling",
            "Without air conditioning, a heat wave itself can make the house dangerously hot."
                .to_owned(),
            &[BucketId::Thermal],
            "rr-consequence",
            &coupling_src,
        );
    }
    if hh.refrigerated_rx && f_rx {
        coupling_note(
            couplings,
            "refrigerated_medicine",
            "Medicine that must stay cold is at risk in any power cut longer than about a day, \
             unless you have a cooler plan."
                .to_owned(),
            &[BucketId::Medication],
            "rr-consequence",
            &prm.cold_chain_days.sources,
        );
    }
    if hh.mobile_home {
        coupling_note(
            couplings,
            "mobile_home",
            "Mobile homes are told to leave for hurricane winds and to go to a sturdy shelter \
             for tornado warnings that most houses ride out."
                .to_owned(),
            &[BucketId::Evacuate],
            "rr-consequence",
            &coupling_src,
        );
    }
}

fn fixed_rate_term(
    ps: &mut ParamSet,
    key: &str,
    p: &Param,
    hazard: HazardId,
    bucket: BucketId,
    label: &str,
    param_label: &str,
) -> Term {
    let rp = ps.rate(
        format!("rate:{key}"),
        p.value,
        p.low.unwrap_or(p.value),
        p.high.unwrap_or(p.value),
        param_label.to_owned(),
        p.evidence,
    );
    let dist = p.duration_or(rr_types::DurationDist::Fixed { days: 1.0 });
    Term {
        bucket,
        hazard,
        scenario: None,
        class: key.to_owned(),
        label: label.to_owned(),
        rate: p.value,
        q: 1.0,
        survival: Survival::from_dist(&dist),
        threshold: 0.0,
        max_with: None,
        rate_param: rp,
        q_param: None,
        dur_param: None,
        evidence: p.evidence,
        duration_from_data: false,
        sources: p.sources.clone(),
        relief: None,
        notice_hours: None,
        heat_share: 0.0,
        cold_share: 0.0,
        origin: Origin::Coupling("fixed_rate"),
    }
}

/// Coupling rules applied by other crates, recorded so the household sees one explainable table.
fn household_notes(input: &PlanInput, hh: &Household, couplings: &mut Vec<CouplingApplied>) {
    use rr_types::{AgeBand, PoweredDevice, Setting, Tenure};
    let prior = [CitationId::from("prior_rr_coupling")];
    if input.people.iter().any(|p| p.age_band == AgeBand::Infant) {
        coupling_note(
            couplings,
            "infant",
            "An infant raises daily water needs by about half and adds formula to the plan."
                .to_owned(),
            &[BucketId::WaterOut],
            "rr-supply",
            &prior,
        );
    }
    if input
        .people
        .iter()
        .any(|p| p.medical.powered_device != PoweredDevice::None)
    {
        coupling_note(
            couplings,
            "powered_device",
            "A powered medical device makes backup power a life-safety item and triples the \
             weight of power cuts in the budget."
                .to_owned(),
            &[BucketId::Power],
            "rr-budget",
            &prior,
        );
    }
    if input.people.iter().any(|p| p.age_band == AgeBand::Senior) {
        coupling_note(
            couplings,
            "senior",
            "Someone aged 65 or over raises the weight of dangerous heat or cold in the budget."
                .to_owned(),
            &[BucketId::Thermal],
            "rr-budget",
            &prior,
        );
    }
    if input.housing.tenure == Tenure::Rent {
        coupling_note(
            couplings,
            "renter",
            "Renters insurance that pays for somewhere to stay is the main protection if your \
             home becomes unlivable."
                .to_owned(),
            &[BucketId::HomeLoss],
            "rr-consequence",
            &[CitationId::from("census_pulse_displacement")],
        );
        coupling_note(
            couplings,
            "renter_no_generator",
            "Renters usually cannot install a generator or transfer switch, so backup power means \
             batteries or a portable power station."
                .to_owned(),
            &[BucketId::Power],
            "rr-budget",
            &prior,
        );
    }
    if matches!(
        input.housing.kind,
        HousingKind::Rowhouse | HousingKind::ApartmentLowRise | HousingKind::ApartmentHighRise
    ) {
        coupling_note(
            couplings,
            "attached_housing",
            "Shared walls mean a neighbour's fire can reach you; the fire rate is doubled."
                .to_owned(),
            &[BucketId::Fire, BucketId::Evacuate],
            "rr-hazards",
            &prior,
        );
    }
    if input.mobility.vehicles.is_empty() {
        coupling_note(
            couplings,
            "no_vehicle",
            "With no car, plan to leave early when told to evacuate: arrange a ride or know the \
             transit route."
                .to_owned(),
            &[BucketId::Evacuate],
            "rr-consequence",
            &prior,
        );
    }
    if hh.commuter {
        coupling_note(
            couplings,
            "commute",
            "The get-home bag is sized by the walk home at about 3 miles an hour, with about half \
             a litre of water per hour in hot weather."
                .to_owned(),
            &[BucketId::GetHome],
            "rr-consequence",
            &prior,
        );
    }
    if input.location.setting == Setting::Rural {
        coupling_note(
            couplings,
            "rural_ems",
            "Ambulances take longer to reach rural homes, so first-aid skills and supplies are \
             worth more."
                .to_owned(),
            &[BucketId::MedicalEmergency],
            "rr-consequence",
            &[CitationId::from("mell_2017_ems")],
        );
    }
    if input.pets.dogs + input.pets.cats + input.pets.small + input.pets.large_animals > 0 {
        coupling_note(
            couplings,
            "pets",
            "Pets add water and food per day and need carriers for evacuation.".to_owned(),
            &[BucketId::WaterOut, BucketId::Evacuate],
            "rr-supply",
            &prior,
        );
    }
}

/// Income streams: job loss from `rr-hazards` (or the national base rate), plus pandemic and
/// scenario-driven job losses.
fn income_terms(
    inp: &BuildInput<'_>,
    hh: &Household,
    rates: &BTreeMap<HazardId, &HouseholdEventRate>,
    rate_param: &BTreeMap<HazardId, Option<usize>>,
    scenario_param: &[Option<usize>],
    ps: &mut ParamSet,
) -> (Vec<IncomeTerm>, bool) {
    let table = inp.table;
    let mut out = Vec::new();
    let mut fallback = false;
    if hh.earners == 0 {
        return (out, false);
    }
    let earners = f64::from(hh.earners);
    for row in &table.income {
        let spell = Survival::from_dist(&row.spell_weeks);
        let dur_param = ps.factor(
            format!(
                "dur:income:{}:{}",
                row.hazard,
                row.scenario.as_deref().unwrap_or("-")
            ),
            default_factor(table, row.evidence, ParamWhat::Duration).min(1.5),
            format!("how long income gaps from {} last", row.label),
            row.evidence,
        );
        let (rate, rparam, sources) = match &row.scenario {
            Some(sid) => {
                let Some(i) = inp.scenarios.iter().position(|c| &c.id == sid) else {
                    continue;
                };
                let c = &inp.scenarios[i];
                if !inp.scenario_on.get(i).copied().unwrap_or(c.on) {
                    continue;
                }
                let per = if scenario_variant(c, &inp.county) == "valley" {
                    row.per_earner * 2.0 / 3.0
                } else {
                    row.per_earner
                };
                (
                    c.rate_per_year * per * earners,
                    scenario_param[i],
                    c.sources.clone(),
                )
            }
            None if row.hazard == HazardId::JobLoss => match rates.get(&HazardId::JobLoss) {
                Some(r) => (
                    r.rate_per_year * row.per_earner,
                    rate_param[&HazardId::JobLoss],
                    r.sources.clone(),
                ),
                None => {
                    fallback = true;
                    let p = &table.params;
                    let factor = match inp.plan.finances.income.stability {
                        rr_types::IncomeStability::Stable => &p.stability_stable,
                        rr_types::IncomeStability::Variable => &p.stability_variable,
                        rr_types::IncomeStability::Seasonal => &p.stability_seasonal,
                        rr_types::IncomeStability::Gig => &p.stability_gig,
                    };
                    let base = &p.job_loss_base_rate;
                    let rate = base.value * factor.value * earners;
                    let lo = base.low.unwrap_or(base.value)
                        * factor.low.unwrap_or(factor.value)
                        * earners;
                    let hi = base.high.unwrap_or(base.value)
                        * factor.high.unwrap_or(factor.value)
                        * earners;
                    let id = ps.rate(
                        "rate:job_loss_base".to_owned(),
                        rate,
                        lo,
                        hi,
                        "how often earners lose a job".to_owned(),
                        Evidence::Empirical,
                    );
                    let mut src = base.sources.clone();
                    src.extend(factor.sources.iter().cloned());
                    (rate, id, src)
                }
            },
            None => match rates.get(&row.hazard) {
                Some(r) => (
                    r.rate_per_year * row.per_earner * earners,
                    rate_param[&row.hazard],
                    r.sources.clone(),
                ),
                None => continue,
            },
        };
        if rate.is_nan() || rate <= 0.0 {
            continue;
        }
        let mut all_sources = row.sources.clone();
        all_sources.extend(sources);
        out.push(IncomeTerm {
            hazard: row.hazard,
            label: row.label.clone(),
            rate,
            rate_param: rparam,
            spell,
            dur_param,
            unemployment_insurance: row.unemployment_insurance,
            sources: all_sources,
        });
    }
    (out, fallback)
}
