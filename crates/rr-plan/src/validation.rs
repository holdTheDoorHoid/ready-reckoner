//! The frozen backtest, bundled with the engine (`docs/VALIDATION.md`, model review Part 3.1):
//! 22 real events, the household used for each, what happened, and the verdicts recorded for
//! four runs of the model. `fixtures/backtest/events.json` holds the set; it is embedded here so
//! `EngineInfo::validation` (the public `#/validation` page's tally) and the packet's "How well
//! do these numbers hold up?" line need no file, and `rr validate` re-scores every run against
//! the recorded verdicts.
//!
//! The scoring rule, defined in the round-2 model review before the events were scored: a
//! target is **covered** when it is at least the duration of about 9 in 10 affected households,
//! **partial** when it covers the median household but not the tail, **short** below the median,
//! **over** above three times the event (counted as covered); an event with several needs takes
//! the worst of them; an evacuation is covered when the household is told to keep a go-bag (a
//! ten-year chance of 2 in 100 or more), the warning it plans for from the event's own hazard is
//! no longer than the warning people had, and the time away it plans for reaches the median
//! evacuee's, partial when that time away is within a factor of three, short otherwise.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use rr_consequence::{
    ConsequenceAssessment, CountyData, OutageModel, RestorationCurve, TemperatureProfile,
};
use rr_types::{
    BaseRate, Benefit, BucketId, CitationId, CountyRecord, Evidence, HazardId, HouseholdEventRate,
    LocationResolved, PlanInput, Target, ValidationSummary, WaterSystemRecord,
};

/// Where the full table is published.
pub const DOC_URL: &str =
    "https://github.com/holdTheDoorHoid/ready-reckoner/blob/main/docs/VALIDATION.md";

/// The frozen set, as committed.
pub const EVENTS_JSON: &str = include_str!("../../../fixtures/backtest/events.json");

/// A verdict under the scoring rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Below the median affected household.
    Short,
    /// Covers the median affected household, not the tail.
    Partial,
    /// At least the duration of about nine in ten affected households.
    Covered,
    /// More than three times the event (counted as covered).
    Over,
    /// The model has no consequence for it.
    NotModelled,
}

impl Verdict {
    /// The word the table prints.
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Short => "short",
            Verdict::Partial => "partial",
            Verdict::Covered => "covered",
            Verdict::Over => "over (covered)",
            Verdict::NotModelled => "not modelled",
        }
    }

    /// How good it is, with over counted as covered: not modelled 0, short 1, partial 2,
    /// covered 3.
    pub fn rank(self) -> u8 {
        match self {
            Verdict::NotModelled => 0,
            Verdict::Short => 1,
            Verdict::Partial => 2,
            Verdict::Covered | Verdict::Over => 3,
        }
    }
}

/// One need that mattered in an event, with what happened.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scored {
    /// The duration bucket.
    pub bucket: BucketId,
    /// Days until about half of the affected households had service back.
    pub median: f64,
    /// Days until about nine in ten had it back.
    pub p90: f64,
}

/// An evacuation: the hazard that forced people out, the warning they had and how long they
/// were away.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Leave {
    /// The hazard.
    pub hazard: HazardId,
    /// Hours of warning people had.
    pub warning_hours: f64,
    /// Days the median evacuee was away.
    pub days_away: f64,
}

/// The answers to the contract v2 questions a household there would have given before the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answers {
    /// None.
    None,
    /// Someone sleeps below street level.
    Basement,
    /// The household relies on SNAP or WIC.
    Snap,
    /// The water system has frequent problems.
    Frequent,
}

impl Answers {
    /// Gives the household these answers.
    pub fn apply(self, input: &mut PlanInput) {
        match self {
            Answers::None => {}
            Answers::Basement => input.housing.below_grade_bedroom = true,
            Answers::Snap => input.finances.benefits = vec![Benefit::SnapWic],
            Answers::Frequent => {
                input.housing.water_system_record = Some(WaterSystemRecord::FrequentProblems);
            }
        }
    }
}

/// One event of the frozen set.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// Stable id.
    pub id: String,
    /// The event, in words.
    pub name: String,
    /// The household file in `fixtures/backtest/`.
    pub household: String,
    /// The needs scored.
    #[serde(default)]
    pub scored: Vec<Scored>,
    /// The evacuation scored, if any.
    #[serde(default)]
    pub leave: Option<Leave>,
    /// The model has no consequence for it.
    pub not_modelled: bool,
    /// The v2 answers for the third and fourth runs.
    pub answers: Answers,
    /// Why the event is inside the records the model learned from, if it is.
    pub in_sample: String,
    /// The recorded verdict of each run.
    pub expected: [Verdict; 4],
}

/// The whole file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frozen {
    /// What the file is.
    pub about: String,
    /// The four runs, in words.
    pub runs: [String; 4],
    /// The run whose verdicts are this version's headline tally.
    pub headline_run: usize,
    /// The data pack the verdicts were recorded on.
    pub data_pack: String,
    /// Where the app shows the table.
    pub url_anchor: String,
    /// The county's share of public-water customers on a system with a health-based violation
    /// as it stood before the event, by county, where the pack's five-year window contains the
    /// event and the record before it was clean (the fourth run).
    pub pre_event_water_record: BTreeMap<String, f64>,
    /// The events.
    pub events: Vec<Event>,
}

/// The frozen set, parsed (`None` only if the embedded file is broken, which a test prevents).
pub fn frozen() -> Option<Frozen> {
    serde_json::from_str(EVENTS_JSON).ok()
}

/// The tally of the headline run (`EngineInfo::validation`): over counts as covered.
pub fn summary() -> ValidationSummary {
    let Some(f) = frozen() else {
        return ValidationSummary::default();
    };
    let run = f.headline_run.min(3);
    let mut s = ValidationSummary {
        events_tested: f.events.len() as u16,
        data_pack: f.data_pack.clone(),
        url_anchor: f.url_anchor.clone(),
        ..ValidationSummary::default()
    };
    for e in &f.events {
        match e.expected[run] {
            Verdict::Short => s.short += 1,
            Verdict::Partial => s.partial += 1,
            Verdict::Covered | Verdict::Over => s.covered += 1,
            Verdict::NotModelled => s.not_modelled += 1,
        }
    }
    s
}

/// The rule for one need: the target against the median and nine-in-ten durations.
pub fn score_days(target: f64, median: f64, p90: f64) -> Verdict {
    if target > 3.0 * p90 {
        Verdict::Over
    } else if target >= p90 {
        Verdict::Covered
    } else if target >= median {
        Verdict::Partial
    } else {
        Verdict::Short
    }
}

/// Where the frozen county data lives in the repository: the county records, resolved locations
/// and national base rates of the data pack the verdicts were recorded on, and the data pack v2
/// tables for the same counties. The consequence crate's backtest test reads the same files.
pub const BACKTEST_DATA_DIR: &str = "crates/rr-consequence/tests/data/backtest";

/// Where the backtest households live in the repository.
pub const BACKTEST_HOUSEHOLDS_DIR: &str = "fixtures/backtest";

/// One frozen county: its record and resolved location.
#[derive(Debug, Clone, Deserialize)]
pub struct BacktestCounty {
    /// The county record.
    pub county: CountyRecord,
    /// The resolved location.
    pub location: LocationResolved,
}

/// `counties.json`: the frozen county records and base rates.
#[derive(Debug, Clone, Deserialize)]
pub struct BacktestCounties {
    /// How the file was made.
    pub provenance: Vec<String>,
    /// The data pack it was extracted from.
    pub pack_version: String,
    /// The counties by FIPS code.
    pub counties: BTreeMap<String, BacktestCounty>,
    /// The national base rates.
    pub base_rates: Vec<BaseRate>,
}

/// `regional.json`: the data pack v2 tables for the backtest counties.
#[derive(Debug, Clone, Deserialize)]
pub struct BacktestRegional {
    /// How the file was made.
    pub provenance: Vec<String>,
    /// The data pack the tables were extracted from.
    #[serde(default)]
    pub pack_version: String,
    /// The regional outage models by FIPS code.
    pub outage_models: BTreeMap<String, OutageModel>,
    /// Heat and cold shares of each county's outage hours.
    #[serde(default)]
    pub temperature: BTreeMap<String, TemperatureProfile>,
    /// The pooled restoration curves.
    pub curves: Vec<RestorationCurve>,
    /// Each county's share of public-water customers on a system with a health-based violation.
    pub sdwis_violation_share: BTreeMap<String, f64>,
    /// Each county's days a year of unhealthy wildfire smoke.
    pub smoke_days_35: BTreeMap<String, f64>,
}

/// Everything the backtest reads, loaded from a repository checkout.
#[derive(Debug, Clone)]
pub struct Backtest {
    /// The events and their recorded verdicts (`fixtures/backtest/events.json`).
    pub frozen: Frozen,
    /// The frozen county records.
    pub counties: BacktestCounties,
    /// The frozen v2 tables.
    pub regional: BacktestRegional,
    /// Each event's household, by file stem.
    pub households: BTreeMap<String, PlanInput>,
}

impl Backtest {
    /// Loads the frozen set from a repository checkout.
    ///
    /// # Errors
    ///
    /// A file that cannot be read or parsed, with its path.
    pub fn load(repo: &Path) -> Result<Self, String> {
        let read = |rel: &str| -> Result<String, String> {
            let path = repo.join(rel);
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
        };
        let frozen: Frozen =
            serde_json::from_str(EVENTS_JSON).map_err(|e| format!("events.json: {e}"))?;
        let counties: BacktestCounties =
            serde_json::from_str(&read(&format!("{BACKTEST_DATA_DIR}/counties.json"))?)
                .map_err(|e| format!("{BACKTEST_DATA_DIR}/counties.json: {e}"))?;
        let regional: BacktestRegional =
            serde_json::from_str(&read(&format!("{BACKTEST_DATA_DIR}/regional.json"))?)
                .map_err(|e| format!("{BACKTEST_DATA_DIR}/regional.json: {e}"))?;
        let mut households = BTreeMap::new();
        for e in &frozen.events {
            if households.contains_key(&e.household) {
                continue;
            }
            let rel = format!("{BACKTEST_HOUSEHOLDS_DIR}/{}.json", e.household);
            let input = PlanInput::from_json(&read(&rel)?).map_err(|e| format!("{rel}: {e}"))?;
            households.insert(e.household.clone(), input);
        }
        Ok(Self {
            frozen,
            counties,
            regional,
            households,
        })
    }

    /// Runs every event four times and scores each run against what happened.
    ///
    /// # Errors
    ///
    /// An event whose household or county is missing from the frozen set.
    pub fn run(&self) -> Result<Report, String> {
        let mut rows = Vec::new();
        let mut tally = [[0u16; 5]; 4];
        let mut changed = Vec::new();
        for e in &self.frozen.events {
            let base = self
                .households
                .get(&e.household)
                .ok_or_else(|| format!("{}: no household {}", e.id, e.household))?;
            let fips = base.location.county_fips.clone().unwrap_or_default();
            let county = self
                .counties
                .counties
                .get(&fips)
                .ok_or_else(|| format!("{}: no frozen county {fips}", e.id))?;
            let mut runs = Vec::new();
            for (k, cell) in tally.iter_mut().enumerate() {
                let mut input = base.clone();
                if k >= 2 {
                    e.answers.apply(&mut input);
                }
                let a = self.assess(&input, county, k);
                let scored = score(e, &a);
                cell[column(scored.verdict)] += 1;
                if scored.verdict.rank() != e.expected[k].rank() {
                    changed.push(Change {
                        event: e.id.clone(),
                        run: k,
                        recorded: e.expected[k],
                        now: scored.verdict,
                        detail: scored.cells.join("; "),
                    });
                }
                runs.push(scored);
            }
            rows.push(EventRow {
                id: e.id.clone(),
                name: e.name.clone(),
                in_sample: e.in_sample.clone(),
                runs,
            });
        }
        Ok(Report {
            rows,
            tally,
            changed,
        })
    }

    /// One run of one household, as `crates/rr-consequence/tests/backtest.rs` makes it: the
    /// county-only model (0), with the data pack v2 tables (1), with the tables and the v2 answers
    /// plus the stand-in rates (2), and that with the pre-event water record (3). Hazard rates
    /// come from `rr-hazards` as it is now.
    fn assess(&self, input: &PlanInput, fc: &BacktestCounty, run: usize) -> ConsequenceAssessment {
        let reg = &self.regional;
        let mut hz = rr_hazards::assess(input, &fc.county, &self.counties.base_rates, &fc.location);
        let fips = fc.county.fips.as_str();
        if run >= 2 {
            let extra = stand_in_rates(input, &hz.rates, reg.smoke_days_35.get(fips).copied());
            hz.rates.extend(extra);
        }
        let mut county = CountyData::from_record(&fc.county);
        if run >= 1 {
            county.outage_model = reg.outage_models.get(fips);
            county.temperature = reg.temperature.get(fips);
            county = county.with_curves(&reg.curves);
            county.sdwis_violation_share = reg.sdwis_violation_share.get(fips).copied();
            county.smoke_days = reg.smoke_days_35.get(fips).copied();
        }
        if run == 3 {
            if let Some(s) = self.frozen.pre_event_water_record.get(fips) {
                county.sdwis_violation_share = Some(*s);
            }
        }
        rr_consequence::assess_with_parts(input, &hz.rates, &hz.parts, county, &hz.scenarios)
    }
}

/// Stand-in rates for the runs with the v2 answers: typical values for the contract v2 hazards
/// (the round-2 hazard candidates), each dropped when `rr-hazards` emits the hazard itself. Only
/// wildfire smoke is still stood in (the frozen county records predate the smoke-day column), as
/// in the consequence crate's backtest.
fn stand_in_rates(
    input: &PlanInput,
    have: &[HouseholdEventRate],
    smoke_days: Option<f64>,
) -> Vec<HouseholdEventRate> {
    let r = |h: HazardId, rate: f64| HouseholdEventRate {
        hazard: h,
        rate_per_year: rate,
        low: rate / 3.0,
        high: rate * 3.0,
        evidence: Evidence::Prior,
        sources: vec![CitationId::from("rr_risk_model_priors")],
    };
    let people = input.people.len() as f64;
    let daily_rx = input.people.iter().filter(|p| p.medical.daily_rx).count() as f64;
    let mut v = vec![
        r(HazardId::NetworkOutage, 0.3),
        r(HazardId::WaterDamage, 0.015),
        r(HazardId::ArrestOrDetention, 0.021 * people),
    ];
    if daily_rx > 0.0 {
        v.push(r(HazardId::DrugShortage, 0.05 * daily_rx));
    }
    if !input.finances.benefits.is_empty() {
        v.push(r(HazardId::BenefitInterruption, 0.077));
    }
    if input.housing.tenure == rr_types::Tenure::Rent {
        v.push(r(HazardId::Eviction, 0.023));
    }
    if let Some(d) = smoke_days.filter(|d| *d > 0.0) {
        v.push(r(HazardId::WildfireSmoke, d / 3.0));
    }
    v.retain(|x| !have.iter().any(|h| h.hazard == x.hazard));
    v
}

/// The tally column of a verdict: short, partial, covered, over, not modelled.
fn column(v: Verdict) -> usize {
    match v {
        Verdict::Short => 0,
        Verdict::Partial => 1,
        Verdict::Covered => 2,
        Verdict::Over => 3,
        Verdict::NotModelled => 4,
    }
}

/// A run's verdict and the numbers behind it ("power 3 d vs 2/4 d: partial").
#[derive(Debug, Clone, PartialEq)]
pub struct RunVerdict {
    /// The event's verdict for this run: the worst of its needs.
    pub verdict: Verdict,
    /// Each need's target against what happened, with its verdict.
    pub cells: Vec<String>,
}

/// One event's four runs.
#[derive(Debug, Clone, PartialEq)]
pub struct EventRow {
    /// Stable id.
    pub id: String,
    /// The event, in words.
    pub name: String,
    /// Why the event is inside the records the model learned from, if it is.
    pub in_sample: String,
    /// The four runs, in order.
    pub runs: Vec<RunVerdict>,
}

/// A verdict that differs from the one recorded.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// The event.
    pub event: String,
    /// The run (0 to 3).
    pub run: usize,
    /// What `events.json` records.
    pub recorded: Verdict,
    /// What the model gives now.
    pub now: Verdict,
    /// The numbers behind the new verdict.
    pub detail: String,
}

/// The whole backtest.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// Every event, in the frozen order.
    pub rows: Vec<EventRow>,
    /// Each run's count of short, partial, covered, over and not modelled.
    pub tally: [[u16; 5]; 4],
    /// Every verdict that differs from the recorded one.
    pub changed: Vec<Change>,
}

fn days(a: &ConsequenceAssessment, b: BucketId) -> f64 {
    match a.bucket(b).target {
        Target::Days { value, .. } => f64::from(value),
        _ => 0.0,
    }
}

/// An evacuation is covered when the household is told to keep a go-bag (a ten-year chance of 2
/// in 100 or more), the warning it is told to plan for from the event's own hazard is no longer
/// than the warning people had, and the time away it plans for reaches the median evacuee's;
/// partial when the time away is within a factor of three of it; short otherwise.
fn score_leave(a: &ConsequenceAssessment, l: &Leave) -> (Verdict, String) {
    let need = a.evacuate.p_need_10yr >= 0.02;
    let least = a
        .evacuate
        .causes
        .iter()
        .filter(|(h, _, r, _)| *h == l.hazard && *r > 0.0)
        .map(|(_, _, _, band)| band[0])
        .fold(f64::INFINITY, f64::min);
    let warned = least <= l.warning_hours;
    let away = f64::from(a.evacuate.days_away);
    let v = if need && warned && away >= l.days_away {
        Verdict::Covered
    } else if need && warned && 3.0 * away >= l.days_away {
        Verdict::Partial
    } else {
        Verdict::Short
    };
    let least_words = if least.is_finite() {
        format!("{least:.2} h")
    } else {
        "none".to_owned()
    };
    (
        v,
        format!(
            "leave {:.0} in 100, warning {least_words}, away {away} d",
            100.0 * a.evacuate.p_need_10yr
        ),
    )
}

/// Scores one run of an event: each need's target against what happened, the worst of them.
pub fn score(e: &Event, a: &ConsequenceAssessment) -> RunVerdict {
    if e.not_modelled {
        return RunVerdict {
            verdict: Verdict::NotModelled,
            cells: vec!["no fuel consequence".to_owned()],
        };
    }
    let mut worst = Verdict::Covered;
    let mut cells = Vec::new();
    for s in &e.scored {
        let t = days(a, s.bucket);
        let v = score_days(t, s.median, s.p90);
        if v.rank() < worst.rank() {
            worst = v;
        }
        cells.push(format!(
            "{} {t} d vs {}/{} d: {}",
            s.bucket,
            s.median,
            s.p90,
            v.word()
        ));
    }
    if let Some(l) = &e.leave {
        let (v, text) = score_leave(a, l);
        if v.rank() < worst.rank() {
            worst = v;
        }
        cells.push(format!("{text}: {}", v.word()));
    }
    RunVerdict {
        verdict: worst,
        cells,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_table_parses_and_tallies_as_the_document_says() {
        let f = frozen().expect("fixtures/backtest/events.json parses");
        assert_eq!(f.events.len(), 22);
        let s = summary();
        // docs/VALIDATION.md, "This version, with the tables and the v2 answers".
        assert_eq!(
            (
                s.events_tested,
                s.covered,
                s.partial,
                s.short,
                s.not_modelled
            ),
            (22, 6, 9, 6, 1)
        );
        assert_eq!(s.url_anchor, "#/validation");
        assert_eq!(score_days(10.0, 2.0, 3.0), Verdict::Over);
        assert_eq!(score_days(3.0, 2.0, 3.0), Verdict::Covered);
        assert_eq!(score_days(2.0, 2.0, 3.0), Verdict::Partial);
        assert_eq!(score_days(1.0, 2.0, 3.0), Verdict::Short);
    }
}
