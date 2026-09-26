//! The frozen backtest (`docs/VALIDATION.md`): 22 real events, the household the round-2 model
//! review built for each (`fixtures/backtest/`), what happened (median and about-nine-in-ten
//! durations from the sources), the scoring rule, and the verdicts recorded in the document. It
//! fails when a verdict changes, so a change that fixes or breaks a real-world case is seen.
//!
//! Inputs are frozen so the verdicts do not move with every data refresh:
//!
//! - `tests/data/backtest/counties.json`: the county records, resolved locations and national
//!   base rates of the data pack the verdicts were recorded on (regenerate with
//!   `RR_WRITE_BACKTEST=1 cargo test -p rr-consequence --test backtest -- --ignored`);
//! - `tests/data/backtest/regional.json`: the data pack v2 tables that are not merged yet (the
//!   regional outage model, restoration curves, drinking-water violations, smoke days), copied
//!   from the data workstreams' branches (provenance inside).
//!
//! Hazard rates come from `rr-hazards` at test time, so a change there shows here too.
//!
//! Four runs per household: the county-only model (what the engine does on today's pack and
//! today's hazard rates), the model with the data pack v2 tables, the same with the answers a
//! household there would have given to the v2 questions before the event (a basement bedroom in
//! Queens, SNAP in Philadelphia, the water system's record in Jackson, Asheville and Puerto Rico)
//! and stand-in rates for the new ranked hazards, and that run again with the county's
//! drinking-water record as it stood before the event where the pack's five-year window contains
//! the event itself (Buncombe). `target/backtest.md` gets the table.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use rr_consequence::{
    ConsequenceAssessment, CountyData, OutageModel, RestorationCurve, assess_with_parts,
};
use rr_types::{
    BaseRate, Benefit, BucketId, CitationId, CountyRecord, Evidence, HazardId, HouseholdEventRate,
    LocationResolved, PlanInput, Target, WaterSystemRecord,
};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/backtest");
const HOUSEHOLDS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/backtest");
const PACK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");

#[derive(serde::Deserialize, serde::Serialize)]
struct FrozenCounty {
    county: CountyRecord,
    location: LocationResolved,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Frozen {
    provenance: Vec<String>,
    pack_version: String,
    counties: BTreeMap<String, FrozenCounty>,
    base_rates: Vec<BaseRate>,
}

#[derive(serde::Deserialize)]
struct Regional {
    #[allow(dead_code)]
    provenance: Vec<String>,
    outage_models: BTreeMap<String, OutageModel>,
    curves: Vec<RestorationCurve>,
    sdwis_violation_share: BTreeMap<String, f64>,
    smoke_days_35: BTreeMap<String, f64>,
}

/// The scoring rule (model review Part 2, defined before the events were scored).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Verdict {
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
    fn word(self) -> &'static str {
        match self {
            Verdict::Short => "short",
            Verdict::Partial => "partial",
            Verdict::Covered => "covered",
            Verdict::Over => "over (covered)",
            Verdict::NotModelled => "not modelled",
        }
    }
    /// Over counts as covered for the event's verdict.
    fn rank(self) -> u8 {
        match self {
            Verdict::NotModelled => 0,
            Verdict::Short => 1,
            Verdict::Partial => 2,
            Verdict::Covered | Verdict::Over => 3,
        }
    }
}

fn score_days(target: f64, median: f64, p90: f64) -> Verdict {
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

/// A duration bucket that mattered in the event, with what happened.
struct Scored {
    bucket: BucketId,
    /// Days until about half of the affected households had service back.
    median: f64,
    /// Days until about nine in ten had it back.
    p90: f64,
}

/// An evacuation: the hazard that forced people out, the warning they had and how long they
/// were away.
struct Leave {
    hazard: HazardId,
    warning_hours: f64,
    days_away: f64,
}

struct Event {
    id: &'static str,
    name: &'static str,
    household: &'static str,
    scored: &'static [Scored],
    leave: Option<Leave>,
    not_modelled: bool,
    /// The answers to the v2 questions a household there would have given before the event.
    answers: fn(&mut PlanInput),
    /// Why the event is in the records the model learned from, if it is.
    in_sample: &'static str,
    /// Recorded verdicts: county-only, with the v2 tables, with the tables and the answers, and
    /// the last with the pre-event water record.
    expected: [Verdict; 4],
}

/// The county's share of public-water customers on a system with a health-based violation, as it
/// stood before the event, where the pack's five-year window (2021-2026) contains the event and
/// the record before it was clean: Buncombe had none before Helene broke its mains (data audit
/// section 6.1). Jackson's (EPA's 2020 emergency order), Austin's (the October 2018 flood boil
/// notice) and Puerto Rico's came first.
const PRE_EVENT_WATER_RECORD: &[(&str, f64)] = &[("37021", 0.0)];

/// Stand-in rates for the contract v2 ranked hazards that `rr-hazards` does not emit yet
/// (hazard-candidates.csv, typical values), for the runs with the v2 answers, so the new effects
/// rows show in the table. awaiting: hazards — its own rates replace these when it emits them.
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

fn none(_: &mut PlanInput) {}
fn basement(p: &mut PlanInput) {
    p.housing.below_grade_bedroom = true;
}
fn snap(p: &mut PlanInput) {
    p.finances.benefits = vec![Benefit::SnapWic];
}
fn frequent(p: &mut PlanInput) {
    p.housing.water_system_record = Some(WaterSystemRecord::FrequentProblems);
}

use BucketId as B;
use Verdict::*;

/// The frozen set (docs/VALIDATION.md, "The events").
const EVENTS: &[Event] = &[
    Event {
        id: "uri_austin",
        name: "Winter Storm Uri, Austin, Feb 2021",
        household: "uri-austin-3",
        scored: &[
            Scored {
                bucket: B::Power,
                median: 2.0,
                p90: 4.0,
            },
            Scored {
                bucket: B::Thermal,
                median: 2.0,
                p90: 4.0,
            },
            Scored {
                bucket: B::WaterBoil,
                median: 6.0,
                p90: 6.0,
            },
        ],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "the storm is in the 2014-2025 outage records",
        expected: [Covered, Partial, Partial, Partial],
    },
    Event {
        id: "uri_houston",
        name: "Winter Storm Uri, Houston, Feb 2021",
        household: "uri-houston-3",
        scored: &[
            Scored {
                bucket: B::Power,
                median: 2.0,
                p90: 4.0,
            },
            Scored {
                bucket: B::Thermal,
                median: 2.0,
                p90: 4.0,
            },
        ],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "the storm is in the 2014-2025 outage records",
        expected: [Covered, Covered, Covered, Covered],
    },
    Event {
        id: "helene_asheville",
        name: "Hurricane Helene, Asheville (city water), Sep 2024",
        household: "helene-asheville-3",
        scored: &[
            Scored {
                bucket: B::WaterOut,
                median: 18.0,
                p90: 21.0,
            },
            Scored {
                bucket: B::WaterBoil,
                median: 52.0,
                p90: 52.0,
            },
            Scored {
                bucket: B::Power,
                median: 7.0,
                p90: 14.0,
            },
        ],
        leave: None,
        not_modelled: false,
        answers: frequent,
        in_sample: "the storm is in the outage records; the county's five-year violation record includes violations after it",
        expected: [Short, Short, Short, Short],
    },
    Event {
        id: "helene_well",
        name: "Hurricane Helene, rural Buncombe (well), Sep 2024",
        household: "helene-buncombe-well-2",
        scored: &[Scored {
            bucket: B::WaterOut,
            median: 7.0,
            p90: 21.0,
        }],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "the storm is in the 2014-2025 outage records",
        expected: [Covered, Covered, Covered, Covered],
    },
    Event {
        id: "ida_jefferson",
        name: "Hurricane Ida, Jefferson Parish, Aug 2021",
        household: "ida-jefferson-3",
        scored: &[Scored {
            bucket: B::Power,
            median: 8.0,
            p90: 18.0,
        }],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "the storm is in the 2014-2025 outage records",
        expected: [Partial, Partial, Partial, Partial],
    },
    Event {
        id: "ida_queens",
        name: "Ida remnants, Queens basement flat, Sep 2021",
        household: "ida-queens-basement-2",
        scored: &[],
        leave: Some(Leave {
            hazard: HazardId::RiverineFlooding,
            warning_hours: 0.25,
            days_away: 7.0,
        }),
        not_modelled: false,
        answers: basement,
        in_sample: "",
        expected: [Short, Short, Partial, Partial],
    },
    Event {
        id: "camp_fire",
        name: "Camp Fire, Paradise, Nov 2018",
        household: "campfire-paradise-2",
        scored: &[],
        leave: Some(Leave {
            hazard: HazardId::Wildfire,
            warning_hours: 1.5,
            days_away: 180.0,
        }),
        not_modelled: false,
        answers: none,
        in_sample: "",
        expected: [Short, Short, Short, Short],
    },
    Event {
        id: "lahaina",
        name: "Lahaina fire, Maui, Aug 2023",
        household: "lahaina-maui-3",
        scored: &[],
        leave: Some(Leave {
            hazard: HazardId::Wildfire,
            warning_hours: 0.25,
            days_away: 180.0,
        }),
        not_modelled: false,
        answers: none,
        in_sample: "",
        expected: [Short, Short, Short, Short],
    },
    Event {
        id: "jackson",
        name: "Jackson water crisis, Aug-Sep 2022",
        household: "jackson-hinds-3",
        scored: &[
            Scored {
                bucket: B::WaterOut,
                median: 7.0,
                p90: 10.0,
            },
            Scored {
                bucket: B::WaterBoil,
                median: 48.0,
                p90: 48.0,
            },
        ],
        leave: None,
        not_modelled: false,
        answers: frequent,
        in_sample: "the county's five-year violation record includes violations during the crisis (and earlier ones)",
        expected: [Short, Short, Short, Short],
    },
    Event {
        id: "east_palestine",
        name: "East Palestine derailment, Feb 2023",
        household: "eastpalestine-3",
        scored: &[],
        leave: Some(Leave {
            hazard: HazardId::HazmatRelease,
            warning_hours: 1.0,
            days_away: 5.0,
        }),
        not_modelled: false,
        answers: none,
        in_sample: "",
        expected: [Partial, Partial, Partial, Partial],
    },
    Event {
        id: "colonial",
        name: "Colonial Pipeline, Atlanta and Charlotte, May 2021",
        household: "colonial-gwinnett-3",
        scored: &[],
        leave: None,
        not_modelled: true,
        answers: none,
        in_sample: "",
        expected: [NotModelled, NotModelled, NotModelled, NotModelled],
    },
    Event {
        id: "pharmacy_it",
        name: "Change Healthcare and CrowdStrike, Franklin County OH, 2024",
        household: "pharmacy-franklin-oh-2",
        scored: &[Scored {
            bucket: B::Medication,
            median: 7.0,
            p90: 15.0,
        }],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "",
        expected: [Partial, Partial, Covered, Covered],
    },
    Event {
        id: "maria_san_juan",
        name: "Hurricane Maria, San Juan, Sep 2017",
        household: "maria-sanjuan-3",
        scored: &[
            Scored {
                bucket: B::Power,
                median: 84.0,
                p90: 170.0,
            },
            Scored {
                bucket: B::WaterOut,
                median: 68.0,
                p90: 150.0,
            },
        ],
        leave: None,
        not_modelled: false,
        answers: frequent,
        in_sample: "the restoration curve behind the island's major-hurricane class is Maria's own",
        expected: [Short, Partial, Partial, Partial],
    },
    Event {
        id: "maria_utuado",
        name: "Hurricane Maria, Utuado, Sep 2017",
        household: "maria-utuado-3",
        scored: &[
            Scored {
                bucket: B::Power,
                median: 84.0,
                p90: 170.0,
            },
            Scored {
                bucket: B::WaterOut,
                median: 68.0,
                p90: 150.0,
            },
        ],
        leave: None,
        not_modelled: false,
        answers: frequent,
        in_sample: "the restoration curve behind the island's major-hurricane class is Maria's own",
        expected: [Short, Short, Short, Short],
    },
    Event {
        id: "snap_lapse",
        name: "SNAP lapse, Philadelphia, Nov 2025",
        household: "snap-philadelphia-3",
        scored: &[Scored {
            bucket: B::Supplies,
            median: 12.0,
            p90: 12.0,
        }],
        leave: None,
        not_modelled: false,
        answers: snap,
        in_sample: "",
        expected: [Short, Short, Covered, Covered],
    },
    Event {
        id: "sandy_staten_island",
        name: "Superstorm Sandy, Staten Island, Oct 2012",
        household: "sandy-statenisland-3",
        scored: &[Scored {
            bucket: B::Power,
            median: 5.0,
            p90: 13.0,
        }],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "",
        expected: [Partial, Partial, Partial, Partial],
    },
    Event {
        id: "sandy_long_beach",
        name: "Superstorm Sandy, Long Beach NY, Oct 2012",
        household: "sandy-longbeach-2",
        scored: &[
            Scored {
                bucket: B::Power,
                median: 10.0,
                p90: 13.0,
            },
            Scored {
                bucket: B::WaterOut,
                median: 13.0,
                p90: 13.0,
            },
        ],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "",
        expected: [Short, Short, Short, Short],
    },
    Event {
        id: "blackout_cleveland",
        name: "Northeast blackout, Cleveland, Aug 2003",
        household: "blackout-cleveland-3",
        scored: &[
            Scored {
                bucket: B::Power,
                median: 1.5,
                p90: 2.0,
            },
            Scored {
                bucket: B::WaterBoil,
                median: 3.0,
                p90: 3.0,
            },
        ],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "",
        expected: [Covered, Covered, Covered, Covered],
    },
    Event {
        id: "blackout_manhattan",
        name: "Northeast blackout, Manhattan 20th floor, Aug 2003",
        household: "blackout-manhattan-highrise-2",
        scored: &[
            Scored {
                bucket: B::Power,
                median: 1.2,
                p90: 2.0,
            },
            Scored {
                bucket: B::WaterOut,
                median: 1.2,
                p90: 2.0,
            },
        ],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "",
        expected: [Covered, Covered, Covered, Covered],
    },
    Event {
        id: "ice_okc",
        name: "Ice storm, Oklahoma City, Oct 2020",
        household: "icestorm-okc-3",
        scored: &[Scored {
            bucket: B::Power,
            median: 5.0,
            p90: 10.0,
        }],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "the storm is in the 2014-2025 outage records",
        expected: [Covered, Partial, Partial, Partial],
    },
    Event {
        id: "ice_austin",
        name: "Ice storm, Austin, Feb 2023",
        household: "uri-austin-3",
        scored: &[Scored {
            bucket: B::Power,
            median: 3.0,
            p90: 7.0,
        }],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "the storm is in the 2014-2025 outage records",
        expected: [Partial, Partial, Partial, Partial],
    },
    Event {
        id: "derecho_linn",
        name: "Derecho, Linn County IA, Aug 2020",
        household: "derecho-linn-3",
        scored: &[Scored {
            bucket: B::Power,
            median: 4.0,
            p90: 8.0,
        }],
        leave: None,
        not_modelled: false,
        answers: none,
        in_sample: "the storm is in the 2014-2025 outage records",
        expected: [Covered, Partial, Partial, Partial],
    },
];

fn frozen() -> Frozen {
    let text = std::fs::read_to_string(format!("{DATA}/counties.json")).expect(
        "tests/data/backtest/counties.json (RR_WRITE_BACKTEST=1 ... -- --ignored makes it)",
    );
    serde_json::from_str(&text).expect("counties.json")
}

fn regional() -> Regional {
    serde_json::from_str(
        &std::fs::read_to_string(format!("{DATA}/regional.json")).expect("regional.json"),
    )
    .expect("regional.json parses")
}

fn household(name: &str) -> PlanInput {
    let text = std::fs::read_to_string(format!("{HOUSEHOLDS}/{name}.json")).expect("household");
    PlanInput::from_json(&text).expect("a valid household")
}

/// The four runs of one household.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Run {
    CountyOnly,
    Regional,
    Answers,
    PreEvent,
}

fn assess(
    input: &PlanInput,
    fc: &FrozenCounty,
    base: &[BaseRate],
    reg: &Regional,
    run: Run,
) -> ConsequenceAssessment {
    let mut hz = rr_hazards::assess(input, &fc.county, base, &fc.location);
    let fips = fc.county.fips.as_str();
    if matches!(run, Run::Answers | Run::PreEvent) {
        let extra = stand_in_rates(input, &hz.rates, reg.smoke_days_35.get(fips).copied());
        hz.rates.extend(extra);
    }
    let mut county = CountyData::from_record(&fc.county);
    if run != Run::CountyOnly {
        county.outage_model = reg.outage_models.get(fips);
        county = county.with_curves(&reg.curves);
        county.sdwis_violation_share = reg.sdwis_violation_share.get(fips).copied();
        county.smoke_days = reg.smoke_days_35.get(fips).copied();
    }
    if run == Run::PreEvent {
        if let Some((_, s)) = PRE_EVENT_WATER_RECORD.iter().find(|(f, _)| *f == fips) {
            county.sdwis_violation_share = Some(*s);
        }
    }
    assess_with_parts(input, &hz.rates, &hz.parts, county, &hz.scenarios)
}

fn days(a: &ConsequenceAssessment, b: BucketId) -> f64 {
    match a.bucket(b).target {
        Target::Days { value, .. } => f64::from(value),
        _ => 0.0,
    }
}

/// An evacuation is covered when the household is told to keep a go-bag (ten-year chance of 2 in
/// 100 or more), the warning it is told to plan for from the event's own hazard is no longer than
/// the warning people had, and the time away it plans for reaches the median evacuee's; partial
/// when the time away is within a factor of three of it (a 2-day plan for a 5-day evacuation);
/// short otherwise.
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
        Covered
    } else if need && warned && 3.0 * away >= l.days_away {
        Partial
    } else {
        Short
    };
    let least_words = if least.is_finite() {
        format!("{:.2} h", least)
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

struct Row {
    verdict: Verdict,
    cells: Vec<String>,
}

fn score(e: &Event, a: &ConsequenceAssessment) -> Row {
    if e.not_modelled {
        return Row {
            verdict: NotModelled,
            cells: vec!["no fuel consequence".to_owned()],
        };
    }
    let mut worst = Covered;
    let mut cells = Vec::new();
    for s in e.scored {
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
    Row {
        verdict: worst,
        cells,
    }
}

#[test]
fn the_frozen_backtest_keeps_its_recorded_verdicts() {
    let fz = frozen();
    let reg = regional();
    let mut md = String::from(
        "# Backtest: 22 real events (docs/VALIDATION.md)\n\nGenerated by `cargo test -p \
         rr-consequence --test backtest`. Targets at the 1-in-100 setting; actual durations are \
         the median and about nine-in-ten affected households from the sources in \
         docs/VALIDATION.md. Runs: county-only model (today's pack), with the data pack v2 tables \
         (regional outage records, restoration curves, drinking-water violations), and the same \
         with the v2 answers, and with the answers and the pre-event water record.\n\n| Event | County only | With v2 tables | With v2 answers | Pre-event water record |\n|---|---|---|---|---|\n",
    );
    let mut tally = [[0u16; 5]; 4];
    let mut changed = Vec::new();
    for e in EVENTS {
        let base = household(e.household);
        let fc = fz
            .counties
            .get(&base.location.county_fips.clone().unwrap_or_default())
            .unwrap_or_else(|| panic!("{}: no frozen county", e.id));
        let mut rows = Vec::new();
        for (k, run) in [Run::CountyOnly, Run::Regional, Run::Answers, Run::PreEvent]
            .into_iter()
            .enumerate()
        {
            let mut input = base.clone();
            if matches!(run, Run::Answers | Run::PreEvent) {
                (e.answers)(&mut input);
            }
            let a = assess(&input, fc, &fz.base_rates, &reg, run);
            let row = score(e, &a);
            let i = match row.verdict {
                Short => 0,
                Partial => 1,
                Covered => 2,
                Over => 3,
                NotModelled => 4,
            };
            tally[k][i] += 1;
            if row.verdict.rank() != e.expected[k].rank() {
                changed.push(format!(
                    "{} ({}): recorded {}, now {} ({})",
                    e.id,
                    [
                        "county only",
                        "with v2 tables",
                        "with v2 answers",
                        "pre-event water record"
                    ][k],
                    e.expected[k].word(),
                    row.verdict.word(),
                    row.cells.join("; ")
                ));
            }
            rows.push(row);
        }
        let star = if e.in_sample.is_empty() { "" } else { "\\*" };
        let _ = writeln!(
            md,
            "| {}{star} | {} | {} | {} | {} |",
            e.name,
            cell(&rows[0]),
            cell(&rows[1]),
            cell(&rows[2]),
            cell(&rows[3]),
        );
    }
    let _ = writeln!(
        md,
        "\n\\* in-sample (see docs/VALIDATION.md).\n\n| Run | Short | Partial | Covered | Over | Not modelled |\n|---|---|---|---|---|---|"
    );
    for (k, name) in [
        "County only",
        "With v2 tables",
        "With v2 answers",
        "Pre-event water record",
    ]
    .iter()
    .enumerate()
    {
        let t = tally[k];
        let _ = writeln!(
            md,
            "| {name} | {} | {} | {} | {} | {} |",
            t[0], t[1], t[2], t[3], t[4]
        );
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/backtest.md");
    std::fs::write(path, &md).expect("write target/backtest.md");
    assert!(
        changed.is_empty(),
        "backtest verdicts changed (update docs/VALIDATION.md and EVENTS if intended):\n{}",
        changed.join("\n")
    );
}

fn cell(r: &Row) -> String {
    format!("**{}**: {}", r.verdict.word(), r.cells.join("; "))
}

/// Regenerates `tests/data/backtest/counties.json` from the data pack in `data/`.
#[test]
#[ignore = "writes the frozen inputs; run with RR_WRITE_BACKTEST=1 when the frozen pack changes"]
fn write_frozen_inputs() {
    if std::env::var("RR_WRITE_BACKTEST").is_err() {
        return;
    }
    let manifest_bytes =
        std::fs::read(format!("{PACK}/manifest.json")).expect("data/manifest.json");
    let manifest: rr_data::Manifest = serde_json::from_slice(&manifest_bytes).expect("manifest");
    let mut files: Vec<(String, Vec<u8>)> = vec![("manifest.json".to_owned(), manifest_bytes)];
    for pack in manifest.packs.values() {
        for f in &pack.files {
            files.push((
                f.path.clone(),
                std::fs::read(format!("{PACK}/{}", f.path)).expect("pack file"),
            ));
        }
    }
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let mut store = rr_data::DataStore::new();
    store.load_many(&refs).expect("the data pack loads");
    let mut counties = BTreeMap::new();
    for e in EVENTS {
        let fips = household(e.household)
            .location
            .county_fips
            .expect("backtest households name their county");
        let county = store.county(&fips).expect("county in the pack").clone();
        let location = store.location(&fips, None).expect("location");
        counties.insert(fips, FrozenCounty { county, location });
    }
    // The other Colonial Pipeline county is frozen too, for rr validate.
    let fips = "37119";
    let county = store.county(fips).expect("county").clone();
    let location = store.location(fips, None).expect("location");
    counties.insert(fips.to_owned(), FrozenCounty { county, location });
    let out = Frozen {
        provenance: vec![
            "The county records, resolved locations and national base rates of the data pack the \
             backtest verdicts were recorded on, extracted by tests/backtest.rs \
             (RR_WRITE_BACKTEST=1). Not the data pack: a frozen copy so the verdicts do not move \
             with each refresh (docs/VALIDATION.md)."
                .to_owned(),
        ],
        pack_version: store.pack_version().unwrap_or_default().to_owned(),
        counties,
        base_rates: store.base_rates().to_vec(),
    };
    std::fs::write(
        format!("{DATA}/counties.json"),
        serde_json::to_string_pretty(&out).expect("json") + "\n",
    )
    .expect("write counties.json");
}
