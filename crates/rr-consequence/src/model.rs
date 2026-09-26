//! Assembling the household's event model: which effect rows apply, at what rates, with which
//! county overrides and coupling rules.
//!
//! The result is a flat list of [`Term`]s. A term is one way a bucket can be disrupted:
//! `rate × share × S(max(d, threshold))` contributes to Λ_b(d), the yearly rate of disruptions
//! longer than `d` days. Uncertain inputs are named [`UParam`]s that the range calculation draws.

use std::collections::BTreeMap;

use rr_types::math::Z_90;
use rr_types::{
    Benefit, BucketId, CitationId, Cooling, CountyRecord, EventRate, Evidence, HazardId,
    HouseholdEventRate, HousingKind, OutageStats, PlanInput, WaterSource, WaterSystemRecord,
};
use serde::Serialize;

use crate::effects::{EffectRow, EffectsTable, Param, ReliefSpec, Requirement};
use crate::pack::{self, OutageModel, PoolBasis, RestorationCurve, TemperatureProfile};
use crate::survival::{EmpiricalCurve, Survival};
use crate::words;

/// County data the consequence model can use. Everything is optional: with none of the data
/// pack v2 fields, the model is the county-only model of v0.1 (and says so in the power
/// bucket's sentences).
#[derive(Debug, Clone, Copy, Default)]
pub struct CountyData<'a> {
    /// Power outage statistics (EAGLE-I): replace the county-scale storm outage class when the
    /// regional outage model is missing, and give the short-outage shape when it is present.
    pub outages: Option<&'a OutageStats>,
    /// Event records by type: their median/p90 days replace matching durations, and their rates
    /// drive the county-scale rows (`county_rate_keys`).
    pub events: Option<&'a BTreeMap<String, EventRate>>,
    /// The county touches the coast.
    pub coastal: bool,
    /// The county has a tsunami hazard zone.
    pub tsunami_zone: bool,
    /// Two-letter state abbreviation ("" when unknown): territories have their own grids.
    pub state_abbr: &'a str,
    /// Fifth National Climate Assessment region id ("" when unknown): the outage-pooling region.
    pub nca_region: &'a str,
    /// The regional outage model: pooled tails with credibility weights, causes and the region's
    /// worst event (data pack v2; model review M-01, M-02). awaiting: data-model
    pub outage_model: Option<&'a OutageModel>,
    /// Heat and cold shares during recorded outages (data pack v2; model review M-11).
    /// awaiting: data-model
    pub temperature: Option<&'a TemperatureProfile>,
    /// Pooled restoration curves by region and cause (data pack v2; model review M-10), from
    /// `rr_data::DataStore::restoration_curves()`. awaiting: data-model, plan
    pub curves: &'a [RestorationCurve],
    /// Share of the county's public-water customers served by a system with a health-based
    /// violation in the last five years (EPA SDWIS, `CountyExposure::sdwis_violation_pop_share`).
    pub sdwis_violation_share: Option<f64>,
    /// Days a year with wildfire smoke and PM2.5 of 35.5 µg/m³ or more
    /// (`CountyExposure::smoke_days_35`).
    pub smoke_days: Option<f64>,
}

impl<'a> CountyData<'a> {
    /// Everything the consequence model reads from a county record.
    pub fn from_record(record: &'a CountyRecord) -> CountyData<'a> {
        CountyData {
            outages: record.outages.as_ref(),
            events: Some(&record.events),
            coastal: record.coastal,
            tsunami_zone: record.tsunami_zone,
            state_abbr: &record.state_abbr,
            nca_region: &record.nca_region,
            // awaiting: data-model — `outage_model: record.outage_model.as_ref(),` and
            // `temperature: record.temperature.as_ref(),` once `CountyRecord` carries them.
            outage_model: None,
            temperature: None,
            curves: &[],
            sdwis_violation_share: record
                .exposure
                .sdwis_violation_pop_share
                .map(f64::from)
                .filter(|x| x.is_finite() && (0.0..=1.0).contains(x)),
            smoke_days: record
                .exposure
                .smoke_days_35
                .map(f64::from)
                .filter(|x| x.is_finite() && *x >= 0.0),
        }
    }

    /// The same county with the pack's pooled restoration curves (they are not per county:
    /// `rr_data::DataStore::restoration_curves()`). awaiting: plan — `rr-plan` passes them.
    pub fn with_curves(mut self, curves: &'a [RestorationCurve]) -> CountyData<'a> {
        self.curves = curves;
        self
    }

    /// The outage-pooling region: the NCA5 region, or `puerto_rico` / `virgin_islands`.
    pub fn outage_region(&self) -> String {
        pack::outage_region(self.state_abbr, self.nca_region)
    }

    /// The regional outage model, when it has a usable pooled tail.
    pub fn pooled(&self) -> Option<&'a OutageModel> {
        self.outage_model
            .filter(|m| m.lam_ge.iter().all(|x| x.is_finite() && *x >= 0.0) && m.lam_ge[0] > 0.0)
    }
}

/// States on the ERCOT, SPP and MISO-South grids, where the grid has failed in extreme cold: the
/// cold-emergency class applies there when the regional outage records are not loaded (model
/// review M-11).
pub const COLD_GRID_STATES: [&str; 8] = ["TX", "OK", "KS", "NE", "LA", "AR", "MS", "NM"];

/// How easily the household's public water system breaks, as a multiplier on the share of the
/// fragile rows (model review M-03): the county's record (EPA SDWIS) times the household's own
/// answer, bounded.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Fragility {
    /// The combined multiplier.
    pub multiplier: f64,
    /// From the county's drinking-water violations (1 when unknown).
    pub county_factor: f64,
    /// From the household's answer (1 when not asked or unknown).
    pub household_factor: f64,
    /// The county's violation share, when the pack has it.
    pub violation_share: Option<f64>,
    /// The household's answer, when given.
    pub record: Option<WaterSystemRecord>,
}

impl Fragility {
    pub(crate) fn of(input: &PlanInput, county: &CountyData<'_>, table: &EffectsTable) -> Self {
        let p = &table.params;
        let share = county
            .sdwis_violation_share
            .filter(|s| s.is_finite() && (0.0..=1.0).contains(s));
        let county_factor = share.map_or(1.0, |s| {
            let lo = p.fragility_county_clean.value;
            lo + (p.fragility_county_flagged.value - lo) * s
        });
        let record = input
            .housing
            .water_system_record
            .filter(|r| *r != WaterSystemRecord::Unknown);
        let household_factor = match record {
            Some(WaterSystemRecord::Fine) => p.fragility_record_fine.value,
            Some(WaterSystemRecord::OccasionalNotices) => p.fragility_record_occasional.value,
            Some(WaterSystemRecord::FrequentProblems) => p.fragility_record_frequent.value,
            Some(WaterSystemRecord::Unknown) | None => 1.0,
        };
        let multiplier = (county_factor * household_factor)
            .clamp(p.fragility_floor.value, p.fragility_cap.value);
        Fragility {
            multiplier,
            county_factor,
            household_factor,
            violation_share: share,
            record,
        }
    }

    /// Nothing is known about the system: no county record and no answer.
    pub fn unknown(&self) -> bool {
        self.violation_share.is_none() && self.record.is_none()
    }
}

/// A named scenario offered for this location, as `rr-hazards` detects it (its `on` already
/// reflects the user's override; `dials.scenario_overrides` is applied again here, harmlessly).
pub use rr_hazards::ScenarioCandidate;

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
    /// The shared water-system fragility draw (fragile rows and what derives from them): one
    /// latent factor for every row it scales, so their draws move together (model review M-14).
    pub frag_param: Option<usize>,
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
    pub below_grade: bool,
    pub food_benefit: bool,
    pub income_benefit: bool,
    pub cold_grid_region: bool,
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
            below_grade: h.below_grade_bedroom,
            food_benefit: input.finances.benefits.contains(&Benefit::SnapWic),
            income_benefit: input.finances.benefits.iter().any(|b| {
                matches!(
                    b,
                    Benefit::FederalPay | Benefit::SsiSsdi | Benefit::Va | Benefit::Unemployment
                )
            }),
            cold_grid_region: match county.outage_model {
                Some(m) => {
                    m.causes.get("cold_grid").is_some_and(|x| *x > 0.0)
                        || m.causes_ge_1d.get("cold_grid").is_some_and(|x| *x > 0.0)
                }
                None => COLD_GRID_STATES.contains(&county.state_abbr),
            },
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
            Some(Requirement::BelowGrade) => self.below_grade,
            Some(Requirement::FoodBenefit) => self.food_benefit,
            Some(Requirement::IncomeBenefit) => self.income_benefit,
            Some(Requirement::ColdGridRegion) => self.cold_grid_region,
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
    /// How easily the household's public water system breaks (model review M-03).
    pub fragility: Fragility,
    /// Sentences a bucket's "why we think this" carries about the data behind it: the regional
    /// outage records, or the fallback used where they are missing.
    pub bucket_notes: Vec<(BucketId, String)>,
    /// Buckets whose target rests on a fallback (a structural gap): their range is widened one
    /// ladder step at the high end (model review M-14).
    pub gaps: Vec<BucketId>,
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
    /// Parts of hazard rates kept as separate event classes (rr-hazards' `RatePart`); rows with
    /// a `part` fall back to the table's `fallback_share` of the whole rate when their hazard has
    /// none here.
    pub parts: &'a [rr_hazards::RatePart],
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

/// A share of 0 in a county's outage record bounds the curve there: no customer outage that long
/// was recorded, so the curve may not pass above this share at that length (half a percent of
/// customer outages, about the resolution of a small county's record: Eddy County, North Dakota,
/// has 21 events). Without it the zero is only "not seen", and the curve through the shorter
/// points ran on to a year (verification V-01).
pub const ZERO_SHARE_BOUND: f64 = 0.005;

/// The survival curve from county outage statistics, or `None` if they are unusable.
pub(crate) fn outage_curve(stats: &OutageStats) -> Option<(Survival, usize)> {
    let shares = [
        (1.0, f64::from(stats.p_ge_1d)),
        (3.0, f64::from(stats.p_ge_3d)),
        (7.0, f64::from(stats.p_ge_7d)),
        (14.0, f64::from(stats.p_ge_14d)),
    ];
    let mut pts = vec![
        (f64::from(stats.median_hours) / 24.0, 0.5),
        (f64::from(stats.p90_hours) / 24.0, 0.1),
    ];
    pts.extend(shares);
    let (curve, dropped) = EmpiricalCurve::from_points(&pts)?;
    // The first length past every positive share at which the record has none.
    let last_positive = pts
        .iter()
        .filter(|(d, s)| d.is_finite() && *d > 0.0 && *s > 0.0 && *s < 1.0)
        .map(|(d, _)| *d)
        .fold(0.0_f64, f64::max);
    let first_zero = shares
        .iter()
        .filter(|(d, s)| *d > last_positive && *s == 0.0)
        .map(|(d, _)| *d)
        .fold(f64::INFINITY, f64::min);
    if first_zero.is_finite() && curve.sf(first_zero) > ZERO_SHARE_BOUND {
        for p in &mut pts {
            if p.0 == first_zero {
                p.1 = ZERO_SHARE_BOUND;
            }
        }
        return EmpiricalCurve::from_points(&pts)
            .map(|(c, dropped)| (Survival::Empirical(c), dropped));
    }
    Some((Survival::Empirical(curve), dropped))
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
    let fragility = Fragility::of(inp.plan, &inp.county, table);
    let mut ps = ParamSet::default();
    let mut terms: Vec<Term> = Vec::new();
    let mut overrides: Vec<OverrideApplied> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut bucket_notes: Vec<(BucketId, String)> = Vec::new();
    let mut gaps: Vec<BucketId> = Vec::new();

    // Hazard rates: the first valid entry per hazard wins. Rare families are shown in their own
    // box and never enter a bucket's curve (DESIGN §4.2, §4.7).
    let mut rates: BTreeMap<HazardId, &HouseholdEventRate> = BTreeMap::new();
    for r in inp.rates {
        if r.hazard.is_rare() || r.hazard.is_retired() {
            continue;
        }
        if r.validate().is_ok() && r.rate_per_year > 0.0 {
            rates.entry(r.hazard).or_insert(r);
        }
    }
    // One shared draw for how easily the public water system breaks, for every fragile row.
    let frag_param = if hh.well {
        None
    } else {
        ps.factor(
            "share:water_fragility".to_owned(),
            table.params.fragility_uncertainty.value,
            "how easily the local public water system breaks".to_owned(),
            Evidence::Prior,
        )
    };
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
    // Parts of hazard rates (wildfire warnings to leave and safety shutoffs): the first valid
    // entry per (hazard, part) wins. A hazard with any part here never falls back to the table's
    // fixed split, so a part with no events (no shutoffs outside the West) stays at zero.
    let mut parts: BTreeMap<(HazardId, &str), &rr_hazards::RatePart> = BTreeMap::new();
    for p in inp.parts {
        let ok = p.low.is_finite()
            && p.rate_per_year.is_finite()
            && p.high.is_finite()
            && 0.0 <= p.low
            && p.low <= p.rate_per_year
            && p.rate_per_year <= p.high
            && rates.contains_key(&p.hazard);
        if ok {
            parts.entry((p.hazard, p.part.as_str())).or_insert(p);
        }
    }
    let split: Vec<HazardId> = parts.keys().map(|(h, _)| *h).collect();
    let mut part_param: BTreeMap<(HazardId, &str), Option<usize>> = BTreeMap::new();
    for (key, p) in &parts {
        let label = table.part(key.0, key.1).map_or_else(
            || words::hazard_plural(key.0).to_owned(),
            |r| r.label.clone(),
        );
        let id = ps.rate(
            format!("rate:{}:{}", key.0, key.1),
            p.rate_per_year,
            p.low,
            p.high,
            format!("how often {label} happen where you live"),
            p.evidence,
        );
        part_param.insert(*key, id);
    }
    let offered: Vec<&str> = inp.scenarios.iter().map(|c| c.id.as_str()).collect();
    let mut scenario_param: Vec<Option<usize>> = Vec::with_capacity(inp.scenarios.len());
    for c in inp.scenarios {
        scenario_param.push(ps.rate(
            format!("rate:scenario:{}", c.id),
            c.rate_per_year,
            c.low,
            c.high,
            format!("how likely {} is", words::scenario_one(&c.name)),
            c.evidence,
        ));
    }
    let parent_factor = overlap_factors(inp, &rates, &mut notes);

    // (owner, class, bucket) -> total share of applicable table rows, for the coupling rules.
    let mut table_share: BTreeMap<(Owner, String, BucketId), f64> = BTreeMap::new();

    for row in &table.effects {
        if let Some(s) = &row.replaced_by {
            if offered.contains(&s.as_str()) {
                continue;
            }
        }
        if !hh.meets(row.requires) || !hh.meets(row.also_requires) {
            continue;
        }
        // County-scale rows (a flood that shuts the water plant) and fixed-rate classes (a grid
        // emergency in extreme cold) carry their own rate; they need their hazard in the register
        // so the contributions name a hazard the household sees.
        if !row.county_rate_keys.is_empty() || row.fixed_rate.is_some() {
            if !rates.contains_key(&row.hazard) {
                continue;
            }
            let (rate, rparam) = if let Some(name) = &row.fixed_rate {
                let Some(p) = table.params.get(name) else {
                    continue;
                };
                let id = ps.rate(
                    format!("rate:fixed:{name}"),
                    p.value,
                    p.low.unwrap_or(p.value),
                    p.high.unwrap_or(p.value),
                    format!("how often {} happen", row.label),
                    p.evidence,
                );
                (p.value, id)
            } else {
                let Some(r) = county_event_rate(inp.county.events, &row.county_rate_keys) else {
                    continue;
                };
                let id = ps.factor(
                    format!("rate:county_events:{}", row.county_rate_keys.join("+")),
                    table.params.outage_stats_rate_factor.value,
                    format!(
                        "how often {} are recorded here",
                        words::county_event_plural(&row.county_rate_keys)
                    ),
                    Evidence::Empirical,
                );
                (r, id)
            };
            let mut term = table_term(row, rate, rparam, None, inp, &mut ps, &mut overrides);
            if row.fragile {
                fragile(&mut term, &fragility, frag_param);
            }
            if row.bucket == BucketId::Power {
                restoration(&mut term, row, inp, &mut ps, &mut overrides);
            }
            *table_share
                .entry((term.owner(), row.class.clone(), row.bucket))
                .or_insert(0.0) += row.p_given_event;
            terms.push(term);
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
            None => match (rates.get(&row.hazard), row.part.as_deref()) {
                (Some(r), part) => {
                    let factor = parent_factor.get(&row.hazard).copied().unwrap_or(1.0);
                    match part {
                        None => (r.rate_per_year * factor, rate_param[&row.hazard], None),
                        // The hazard crate split this rate: the part's own rate, or nothing.
                        Some(name) if split.contains(&row.hazard) => {
                            match parts.get(&(row.hazard, name)) {
                                Some(p) if p.rate_per_year > 0.0 => (
                                    p.rate_per_year * factor,
                                    part_param[&(row.hazard, name)],
                                    None,
                                ),
                                _ => continue,
                            }
                        }
                        // Whole rates only: the table's fixed share of the whole.
                        Some(name) => {
                            let share = table
                                .part(row.hazard, name)
                                .map_or(0.0, |p| p.fallback_share);
                            if share <= 0.0 {
                                continue;
                            }
                            (
                                r.rate_per_year * factor * share,
                                rate_param[&row.hazard],
                                None,
                            )
                        }
                    }
                }
                (None, _) => continue,
            },
        };
        let mut term = table_term(row, rate, rparam, scenario, inp, &mut ps, &mut overrides);
        if row.fragile {
            fragile(&mut term, &fragility, frag_param);
        }
        if row.bucket == BucketId::Power {
            restoration(&mut term, row, inp, &mut ps, &mut overrides);
        }
        *table_share
            .entry((term.owner(), row.class.clone(), row.bucket))
            .or_insert(0.0) += row.p_given_event;
        terms.push(term);
    }

    add_pool_terms(
        inp,
        &mut ps,
        &mut terms,
        &mut overrides,
        &mut notes,
        &mut bucket_notes,
        &mut gaps,
    );

    let mut couplings: Vec<CouplingApplied> = Vec::new();
    apply_couplings(
        &hh,
        inp,
        &mut ps,
        &mut terms,
        &table_share,
        &mut couplings,
        (&fragility, frag_param),
    );
    household_notes(inp.plan, &hh, &mut couplings);
    water_notes(&hh, &fragility, &mut bucket_notes, &mut gaps);

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
        fragility,
        bucket_notes,
        gaps,
    }
}

/// A fragile row: its share scaled by how easily the public water system breaks, with the shared
/// draw, and the county's violation record among its sources when the pack has it.
fn fragile(term: &mut Term, f: &Fragility, frag_param: Option<usize>) {
    term.q = (term.q * f.multiplier).min(1.0);
    term.frag_param = frag_param;
    if f.violation_share.is_some() {
        term.sources.push(CitationId::from("epa_echo_sdwa"));
    }
}

/// The county's recorded rate of damaging episodes of these NOAA Storm Events types (each type's
/// yearly rate times its damaging share; all of it when the share is not recorded), or `None`
/// where the county has no such record.
fn county_event_rate(events: Option<&BTreeMap<String, EventRate>>, keys: &[String]) -> Option<f64> {
    let events = events?;
    let mut total = 0.0;
    let mut any = false;
    for k in keys {
        if let Some(e) = events.get(k) {
            let r = f64::from(e.rate_per_year);
            let d = e.share_damaging.map_or(1.0, f64::from);
            if r.is_finite() && r > 0.0 && d.is_finite() && d > 0.0 {
                total += r * d.min(1.0);
                any = true;
            }
        }
    }
    (any && total > 0.0).then_some(total)
}

/// A pooled restoration curve as a survival curve: its share of the peak still out at each day
/// mark, plus the days to half and to nine in ten restored.
pub(crate) fn curve_survival(c: &RestorationCurve) -> Option<Survival> {
    let mut pts: Vec<(f64, f64)> = c
        .share_out_at_days
        .iter()
        .map(|(d, s)| (f64::from(*d), f64::from(*s)))
        .collect();
    pts.push((f64::from(c.t50_days), 0.5));
    pts.push((f64::from(c.t90_days), 0.1));
    EmpiricalCurve::from_points(&pts).map(|(curve, _)| Survival::Empirical(curve))
}

/// Regional restoration for a power row (model review M-10): on an island grid with a
/// hand-copied historic curve (Puerto Rico after Maria), that curve is the duration of the rows
/// marked `island_curve`; otherwise, where the region's pooled curve for the row's cause rests on
/// enough major events, the row's duration is stretched by the region's time to 90 % restored
/// over the mainland's (bounded 0.5 to 5).
fn restoration(
    term: &mut Term,
    row: &EffectRow,
    inp: &BuildInput<'_>,
    ps: &mut ParamSet,
    overrides: &mut Vec<OverrideApplied>,
) {
    let Some(class) = row.curve_class.as_deref() else {
        return;
    };
    let curves = inp.county.curves;
    if curves.is_empty() {
        return;
    }
    let region = inp.county.outage_region();
    let eagle = CitationId::from("ornl_eagle_i_outages");
    if row.island_curve {
        if let Some(h) = pack::historic_curve(curves, &region) {
            if let Some(surv) = curve_survival(h) {
                let src = CitationId::from(words::historic_source(&h.class));
                term.survival = surv;
                term.duration_from_data = true;
                term.dur_param = ps.factor(
                    format!("dur:{}", h.class),
                    inp.table.params.duration_factor_empirical.value,
                    "how long the power stays out after a storm like the historic one here"
                        .to_owned(),
                    Evidence::Empirical,
                );
                term.sources.push(src.clone());
                let name = words::historic_event(&h.class);
                let plain = format!(
                    "Power: {} use the restoration record of {name} here (half the customers back \
                     after {}, nine in ten after {}), because this grid takes far longer to \
                     restore than mainland grids.",
                    row.label,
                    words::days_phrase(f64::from(h.t50_days)),
                    words::days_phrase(f64::from(h.t90_days)),
                );
                push_override(overrides, BucketId::Power, row.hazard, plain, vec![src]);
                return;
            }
        }
    }
    let Some(c) = pack::curve(curves, &region, class) else {
        return;
    };
    if f64::from(c.events) < inp.table.params.curve_min_events.value {
        return;
    }
    let Some(f) = c
        .factor
        .map(f64::from)
        .filter(|f| f.is_finite() && *f > 0.0)
    else {
        return;
    };
    let f = f.clamp(0.5, 5.0);
    if (f - 1.0).abs() < 0.05 {
        return;
    }
    term.survival = term.survival.scaled(f);
    term.sources.push(eagle.clone());
    let plain = format!(
        "Power: {} take {} to restore here than on the mainland (the region's records since \
         2014: nine in ten customers back after {}), so their durations are scaled by {}.",
        row.label,
        if f > 1.0 { "longer" } else { "less time" },
        words::days_phrase(f64::from(c.t90_days)),
        words::factor_phrase(f),
    );
    push_override(overrides, BucketId::Power, row.hazard, plain, vec![eagle]);
}

fn push_override(
    overrides: &mut Vec<OverrideApplied>,
    bucket: BucketId,
    hazard: HazardId,
    plain: String,
    sources: Vec<CitationId>,
) {
    if overrides.iter().any(|o| o.plain == plain) {
        return;
    }
    overrides.push(OverrideApplied {
        bucket,
        hazards: vec![hazard],
        plain,
        sources,
    });
}

/// What the water buckets say about the system behind them, and the structural gap where
/// nothing is known about it (model review M-03, Part 3.4).
fn water_notes(
    hh: &Household,
    f: &Fragility,
    bucket_notes: &mut Vec<(BucketId, String)>,
    gaps: &mut Vec<BucketId>,
) {
    if hh.well {
        return;
    }
    let mut parts = Vec::new();
    if let Some(s) = f.violation_share {
        let who = if s <= 0.0 {
            "no public-water customer in your county is".to_owned()
        } else if s < 0.01 {
            "fewer than 1 of 100 public-water customers in your county are".to_owned()
        } else {
            format!(
                "about {} of 100 public-water customers in your county are",
                words::per_100(100.0 * s)
            )
        };
        parts.push(format!(
            "{who} served by a system with a health-based violation in the last five years (EPA)"
        ));
    }
    if let Some(r) = f.record {
        parts.push(format!(
            "you said your water system {}",
            match r {
                WaterSystemRecord::Fine => "has had no problems you remember",
                WaterSystemRecord::OccasionalNotices => "has a notice or a problem now and then",
                WaterSystemRecord::FrequentProblems => "has frequent notices or outages",
                WaterSystemRecord::Unknown => "record is unknown",
            }
        ));
    }
    let sentence = if parts.is_empty() {
        for b in [BucketId::WaterOut, BucketId::WaterBoil] {
            if !gaps.contains(&b) {
                gaps.push(b);
            }
        }
        "We have no record of how your public water system has held up, so this is an estimate \
         that could be too short."
            .to_owned()
    } else {
        format!(
            "Water-system failures are counted {} as often as the national average here (an \
             expert estimate), because {}.",
            words::factor_phrase(f.multiplier),
            parts.join(" and ")
        )
    };
    for b in [BucketId::WaterOut, BucketId::WaterBoil] {
        bucket_notes.push((b, sentence.clone()));
    }
}

/// How much of each parent hazard's rate is left for its typical rows once the offered named
/// scenarios take out their long-run shares (see `[[overlap]]` in effects.toml): 1 − Σ share / r,
/// never below a quarter.
fn overlap_factors(
    inp: &BuildInput<'_>,
    rates: &BTreeMap<HazardId, &HouseholdEventRate>,
    notes: &mut Vec<String>,
) -> BTreeMap<HazardId, f64> {
    let mut shares: BTreeMap<HazardId, f64> = BTreeMap::new();
    for o in &inp.table.overlaps {
        let Some(c) = inp.scenarios.iter().find(|c| c.id == o.scenario) else {
            continue;
        };
        let Some(r) = rates.get(&o.hazard) else {
            continue;
        };
        let share = c
            .alternatives
            .iter()
            .map(|a| a.rate_per_year)
            .filter(|x| x.is_finite() && *x > 0.0)
            .fold(c.rate_per_year, f64::min);
        if share.is_finite() && share > 0.0 && r.rate_per_year > 0.0 {
            *shares.entry(o.hazard).or_insert(0.0) += share;
            let then = if c.on {
                "so it is counted once, as the scenario"
            } else {
                "and the scenario is off, so the plan leaves it out"
            };
            notes.push(format!(
                "{}: {}'s own long-run share (about {}) is taken out of ordinary {}, {then}.",
                o.hazard.name(),
                words::scenario_one(&c.name),
                words::rate_phrase(share),
                words::hazard_plural(o.hazard),
            ));
        }
    }
    shares
        .into_iter()
        .map(|(h, share)| {
            let r = rates[&h].rate_per_year;
            (h, (1.0 - share / r).max(0.25))
        })
        .collect()
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
        frag_param: None,
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

/// Pooled-tail causes and the hazard each is shown under (the regional outage model leaves out
/// hurricanes, wildfires, floods, cold-driven grid emergencies and other grid failures, which
/// have rows of their own).
const POOL_CAUSES: [(&str, HazardId); 4] = [
    ("wind", HazardId::StrongWind),
    ("ice", HazardId::IceStorm),
    ("winter", HazardId::WinterWeather),
    ("heat", HazardId::HeatWave),
];

/// The pooled regional tail as one survival curve with its rate: the blended rates of outages
/// lasting 1, 3, 7, 14 and 30 days over the pool's rate of outages of any length, with the
/// county's own median and 90th-percentile hours for the short part where they fit below a day.
/// `None` when fewer than two usable points remain.
pub fn pooled_curve(m: &OutageModel, stats: Option<&OutageStats>) -> Option<(Survival, f64)> {
    let lam: Vec<f64> = m.lam_ge.iter().map(|x| f64::from(*x)).collect();
    let mut rate = f64::from(m.rate);
    // The pool's rate is the county's own; where its record is thin (no outages of its own, as in
    // Manhattan's underground grid) the blended tail can exceed it. At most half of the outages
    // then last a day or more.
    let raised = !rate.is_finite() || rate < 2.0 * lam[0];
    if raised {
        rate = 2.0 * lam[0];
    }
    if rate <= 0.0 {
        return None;
    }
    let mut pts: Vec<(f64, f64)> = pack::OUTAGE_MARKS_DAYS
        .iter()
        .zip(&lam)
        .filter(|(_, l)| **l > 0.0)
        .map(|(d, l)| (f64::from(*d), l / rate))
        .collect();
    let s1 = lam[0] / rate;
    if let (false, Some(st)) = (raised, stats) {
        let med = f64::from(st.median_hours) / 24.0;
        let p90 = f64::from(st.p90_hours) / 24.0;
        if med.is_finite() && med > 0.0 && med < 1.0 && s1 < 0.5 {
            pts.push((med, 0.5));
        }
        if p90.is_finite() && p90 > med && p90 < 1.0 && s1 < 0.1 {
            pts.push((p90, 0.1));
        }
    }
    let (curve, _) = EmpiricalCurve::from_points(&pts)?;
    Some((Survival::Empirical(curve), rate))
}

/// County-scale storm outages.
///
/// - **With the regional outage model** (data pack v2; model review M-01, M-02): they happen at
///   the pool's rate with the blended regional tail ([`pooled_curve`]), shown under the storm
///   hazards by the county's recorded causes (M-18), with the heat share of the county's outage
///   hours when the temperature record has it (M-11). Hurricanes, wildfire shutoffs, floods,
///   cold-driven grid emergencies and other grid failures are outside the pool: their own rows
///   carry them, so nothing is counted twice (M-10).
/// - **Without it** (the fallback, said in the power bucket's sentences): the county's own outage
///   statistics (or its state's series), split between the storm hazards in proportion to their
///   short-outage rates; without any record, a third of short storm outages are taken to be
///   county-scale (`storm_pool_fallback_ratio`).
fn add_pool_terms(
    inp: &BuildInput<'_>,
    ps: &mut ParamSet,
    terms: &mut Vec<Term>,
    overrides: &mut Vec<OverrideApplied>,
    notes: &mut Vec<String>,
    bucket_notes: &mut Vec<(BucketId, String)>,
    gaps: &mut Vec<BucketId>,
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
    // Attribution by short-outage rate; with no storm rates at all, to strong wind.
    let by_rows: Vec<(HazardId, f64, f64, f64)> = if total > 0.0 {
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
    let source = CitationId::from("ornl_eagle_i_outages");
    let hot_share = inp.county.temperature.and_then(|t| {
        t.region_outage_hot_share
            .or(t.outage_hot_share)
            .map(f64::from)
            .filter(|x| x.is_finite() && (0.0..=1.0).contains(x))
    });

    if let Some((m, (curve, rate))) = inp
        .county
        .pooled()
        .and_then(|m| pooled_curve(m, inp.county.outages).map(|c| (m, c)))
    {
        let rp = ps.factor(
            "rate:outage_pooled".to_owned(),
            table.params.outage_stats_rate_factor.value,
            "how often county-wide storm outages happen here and nearby".to_owned(),
            Evidence::Empirical,
        );
        let dp = ps.factor(
            "dur:outage_pooled".to_owned(),
            table.params.duration_factor_empirical.value,
            "how long county-wide storm outages last here and nearby".to_owned(),
            Evidence::Empirical,
        );
        // Shares by recorded cause; unattributed outages and causes whose hazard is not in the
        // table's pool rows follow the short-outage split.
        let mut shares: BTreeMap<HazardId, (f64, f64, f64)> = BTreeMap::new();
        let known: f64 = POOL_CAUSES
            .iter()
            .map(|(c, _)| m.causes.get(*c).map_or(0.0, |x| f64::from(*x)))
            .sum::<f64>()
            + m.causes.get("unattributed").map_or(0.0, |x| f64::from(*x));
        let mut rest = 1.0;
        if known > 0.0 {
            rest = 0.0;
            for (cause, hazard) in POOL_CAUSES {
                let x = m.causes.get(cause).map_or(0.0, |x| f64::from(*x)) / known;
                match by_rows.iter().find(|(h, ..)| *h == hazard) {
                    Some(&(_, _, heat, cold)) if x > 0.0 => {
                        let e = shares.entry(hazard).or_insert((0.0, heat, cold));
                        e.0 += x;
                    }
                    _ => rest += x,
                }
            }
            rest += m.causes.get("unattributed").map_or(0.0, |x| f64::from(*x)) / known;
        }
        for &(hazard, share, heat, cold) in &by_rows {
            let e = shares.entry(hazard).or_insert((0.0, heat, cold));
            e.0 += rest * share;
        }
        let mut hazards = Vec::new();
        for (hazard, (share, heat, cold)) in shares {
            if share <= 0.0 {
                continue;
            }
            hazards.push(hazard);
            terms.push(Term {
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
                frag_param: None,
                dur_param: dp,
                evidence: Evidence::Empirical,
                duration_from_data: true,
                sources: vec![source.clone()],
                relief: None,
                notice_hours: None,
                heat_share: hot_share.unwrap_or(heat),
                cold_share: cold,
                origin: Origin::Pool,
            });
        }
        let region_words = match m.basis {
            PoolBasis::Blend => format!(
                "this county's outage records blended with about {} nearby counties'",
                words::round_nice(f64::from(m.region_counties.max(1)))
            ),
            PoolBasis::RegionOnly => format!(
                "about {} nearby counties' outage records, because this county has too few of its \
                 own",
                words::round_nice(f64::from(m.region_counties.max(1)))
            ),
            PoolBasis::OwnOnly => {
                "this county's own outage records (no nearby county has any)".to_owned()
            }
        };
        let ge3 = f64::from(m.lam_ge[1]);
        overrides.push(OverrideApplied {
            bucket: BucketId::Power,
            hazards,
            plain: format!(
                "Power: county-wide storm outages use {region_words} (2014-2025): a home here is \
                 caught in one about {}, and in one lasting 3 days or more about {}. Hurricanes, \
                 wildfire shutoffs, floods and grid emergencies have rows of their own.",
                words::rate_phrase(rate),
                words::rate_phrase(ge3),
            ),
            sources: vec![source],
        });
        let sentence = match m.basis {
            PoolBasis::Blend => format!(
                "Power cuts from storms here use your county's outage records since 2014, blended \
                 with about {} nearby counties', because about eleven years is too short to see \
                 the rare storms.",
                words::round_nice(f64::from(m.region_counties.max(1)))
            ),
            PoolBasis::RegionOnly => format!(
                "Power cuts from storms here use about {} nearby counties' outage records since \
                 2014, because your county has too few of its own.",
                words::round_nice(f64::from(m.region_counties.max(1)))
            ),
            PoolBasis::OwnOnly => {
                gaps.push(BucketId::Power);
                "Power cuts from storms here use only your county's own outage records since \
                 2014 (no nearby county has any), so one big storm can push this up or down."
                    .to_owned()
            }
        };
        bucket_notes.push((BucketId::Power, sentence));
        return;
    }

    let from_stats = inp
        .county
        .outages
        .filter(|s| s.events_per_customer_year.is_finite() && s.events_per_customer_year > 0.0)
        .and_then(|s| outage_curve(s).map(|(c, dropped)| (s, c, dropped)));
    let mut new_terms = Vec::new();
    match from_stats {
        Some((stats, curve, dropped)) => {
            let rate = f64::from(stats.events_per_customer_year);
            // A county with no records of its own uses its state's pooled series (rr-data).
            let place = match &stats.state_series {
                Some(state) => format!("across {state}"),
                None => "here".to_owned(),
            };
            let rp = ps.factor(
                "rate:outage_stats".to_owned(),
                table.params.outage_stats_rate_factor.value,
                format!("how often county-wide storm outages happen {place}"),
                Evidence::Empirical,
            );
            let dp = ps.factor(
                "dur:outage_stats".to_owned(),
                table.params.duration_factor_empirical.value,
                format!("how long county-wide storm outages last {place}"),
                Evidence::Empirical,
            );
            let mut hazards = Vec::new();
            for &(hazard, share, heat, cold) in &by_rows {
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
                    frag_param: None,
                    dur_param: dp,
                    evidence: Evidence::Empirical,
                    duration_from_data: true,
                    sources: vec![source.clone()],
                    relief: None,
                    notice_hours: None,
                    heat_share: hot_share.unwrap_or(heat),
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
                    "Power: county-wide storm outages use {} ({}): a home here is caught in one \
                     about {}; half are over within {}, 9 in 10 within {}.",
                    match &stats.state_series {
                        Some(state) => format!(
                            "{state}'s outage records, because this county has none of its own"
                        ),
                        None => "this county's outage records".to_owned(),
                    },
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
            // The regional records are not loaded: say so, and widen the range (M-14).
            gaps.push(BucketId::Power);
            bucket_notes.push((
                BucketId::Power,
                match &stats.state_series {
                    Some(state) => format!(
                        "Power cuts from storms here use {state}'s outage records, not nearby \
                         counties' blended with yours, so this is less certain than usual."
                    ),
                    None => "Power cuts from storms here use only your county's own outage \
                             records: nearby counties' records are not blended in, so one big \
                             storm can push this up or down."
                        .to_owned(),
                },
            ));
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
                    frag_param: None,
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
            if !pool_rows.is_empty() {
                gaps.push(BucketId::Power);
                bucket_notes.push((
                    BucketId::Power,
                    "There are no outage records for this place, so power cuts from storms are an \
                     estimate that could be too short."
                        .to_owned(),
                ));
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
    t.sources.push(CitationId::from("rr_risk_model_priors"));
    t
}

fn apply_couplings(
    hh: &Household,
    inp: &BuildInput<'_>,
    ps: &mut ParamSet,
    terms: &mut Vec<Term>,
    table_share: &BTreeMap<(Owner, String, BucketId), f64>,
    couplings: &mut Vec<CouplingApplied>,
    (fragility, frag_param): (&Fragility, Option<usize>),
) {
    let table = inp.table;
    let prm = &table.params;
    let coupling_src = vec![CitationId::from("rr_risk_model_priors")];
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
    let mut water_floors: Vec<(usize, MaxWith)> = Vec::new();
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
        // Public water keeps flowing on backup power for a few days; in a longer outage some
        // systems lose pressure (Maria, Harvey, Laura). The event's own no-water rows then last
        // at least as long as the power cut past the backup's days; other long power cuts stop
        // the water for a share of homes, scaled by how easily the system breaks (M-03).
        if !hh.well && !hh.high_rise_pumped {
            let thr = prm.public_water_power_days.value;
            let overlap: Vec<usize> = terms
                .iter()
                .enumerate()
                .filter(|(_, w)| {
                    w.bucket == BucketId::WaterOut
                        && w.origin == Origin::Table
                        && w.owner() == t.owner()
                        && w.class == t.class
                        && w.max_with.is_none()
                })
                .map(|(i, _)| i)
                .collect();
            if overlap.is_empty() {
                let q = t.q * prm.public_water_power_share.value * fragility.multiplier;
                if q > 0.0 && t.survival.sf(thr, 0.0) > 1e-9 {
                    let mut d = derive(t, BucketId::WaterOut, q, thr, "public_water_power");
                    d.frag_param = frag_param.or(t.frag_param);
                    d.relief = None;
                    derived.push(d);
                    if !fired.contains(&"public_water_power") {
                        fired.push("public_water_power");
                    }
                }
            } else {
                for i in overlap {
                    water_floors.push((
                        i,
                        MaxWith {
                            survival: t.survival.clone(),
                            dur_param: t.dur_param,
                            threshold: thr,
                        },
                    ));
                }
            }
        }
    }
    for (i, floor) in water_floors {
        let w = &mut terms[i];
        if w.max_with.is_none() {
            w.max_with = Some(floor);
            w.sources.push(CitationId::from("rr_risk_model_priors"));
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
        w.sources.push(CitationId::from("rr_risk_model_priors"));
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
    if fired.contains(&"public_water_power") {
        coupling_note(
            couplings,
            "public_water_power",
            format!(
                "Public water systems keep pumping on backup power for about {} days; in longer \
                 power cuts some lose pressure, more often where the system has a record of \
                 problems.",
                words::round_nice(prm.public_water_power_days.value)
            ),
            &[BucketId::WaterOut],
            "rr-consequence",
            &prm.public_water_power_share.sources,
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
        frag_param: None,
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
    let prior = [CitationId::from("rr_risk_model_priors")];
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
            &[CitationId::from("mell_2017_ems_response")],
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
        if !hh.meets(row.requires) {
            continue;
        }
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
                        // awaiting: rr-consequence / content — `effects.toml` has no calibration
                        // `Param` for `very_stable` yet (that file is owned by the
                        // consequence/content workstream). This branch only runs when rr-hazards
                        // reports no `JobLoss` rate for a household with earners (rr-hazards
                        // always emits one once `earners > 0`, so in practice this is a
                        // calibration/test fallback, not the production path); falling back to
                        // `stable`'s Param here keeps the match exhaustive without guessing a
                        // number that crate does not own. rr-hazards' own `income_stability`
                        // (crates/rr-hazards/src/params.rs) already applies the real ×0.5 factor
                        // whenever a rate is supplied.
                        rr_types::IncomeStability::VeryStable => &p.stability_stable,
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
                    if row.household_rate {
                        r.rate_per_year * row.per_earner
                    } else {
                        r.rate_per_year * row.per_earner * earners
                    },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(p: [f32; 4], median_hours: f32, p90_hours: f32) -> OutageStats {
        OutageStats {
            events_per_customer_year: 1.9,
            p_ge_1d: p[0],
            p_ge_3d: p[1],
            p_ge_7d: p[2],
            p_ge_14d: p[3],
            median_hours,
            p90_hours,
            years_covered: "2020-2025".to_owned(),
            event_definition: String::new(),
            state_series: None,
        }
    }

    #[test]
    fn a_zero_share_bounds_the_curve_where_the_record_ends() {
        // Eddy County, North Dakota (21 events): nothing recorded at 3 days or more. The curve
        // may not sit above half a percent there, so the 1-in-100 outage stays near the record
        // (it ran on to 365 days when the zeros were only "not seen").
        let (curve, _) = outage_curve(&stats([0.1375, 0.0, 0.0, 0.0], 1.0, 61.5)).unwrap();
        assert!(curve.sf(3.0, 0.0) <= ZERO_SHARE_BOUND + 1e-12);
        assert!(curve.sf(7.0, 0.0) < 1e-4, "{}", curve.sf(7.0, 0.0));
        // Philadelphia: 0.18 % at 3 days is already below the bound, so its zeros change
        // nothing (the curve is the one the positive points give).
        let phl = stats([0.09841, 0.001794, 0.0, 0.0], 5.0, 23.75);
        let (with, _) = outage_curve(&phl).unwrap();
        let pts = [
            (5.0 / 24.0, 0.5),
            (23.75 / 24.0, 0.1),
            (1.0, f64::from(0.09841_f32)),
            (3.0, f64::from(0.001794_f32)),
        ];
        let (plain, _) = EmpiricalCurve::from_points(&pts).unwrap();
        let plain = Survival::Empirical(plain);
        for d in [0.5, 1.0, 3.0, 7.0, 14.0] {
            assert!((with.sf(d, 0.0) - plain.sf(d, 0.0)).abs() < 1e-12, "{d}");
        }
    }
}
