//! The nine rare families (REVIEW §2.3–§2.4; hazard-expansion Deliverable B): the rare-but-severe
//! box.
//!
//! Every family is a range, never a point: its factors are published forecasts and expert
//! judgement stacked on each other (`prior`), multiplied low by low and high by high
//! ([`Estimate::times_span`]). The middle value is the product of the middles, used only to sort
//! the box ("sorted by how likely here, never by expected loss") and never shown. Rare rows are
//! display-only: [`crate::assess`] never hands them to `rr-consequence`, so no rare row enters a
//! bucket's Λ and none can drive a target or the budget.
//!
//! Location terms: the nuclear family by the county's strategic-exposure class, the solar storm by
//! geomagnetic latitude, war by nearness to military sites and infrastructure, chemical,
//! biological or radiological attack by the metro area's FEMA UASI share. The others are
//! national or worldwide and say so.

use rr_types::{
    CitationId, DataConfidence, HazardDisplay, HazardId, LocationFactor, StrategicClass, SubCause,
};

use crate::cite;
use crate::ctx::{Ctx, Notes};
use crate::estimate::Estimate;
use crate::exposure::Uasi;
use crate::params::*;
use crate::rate::HazardRate;
use crate::sentence;
use crate::strategic;

fn span(t: Triple, sources: &[&str]) -> Estimate {
    Estimate::prior(t.0, t.1, t.2, sources)
}

fn ids(list: &[&str]) -> Vec<CitationId> {
    list.iter().map(|s| CitationId::from(*s)).collect()
}

/// A sub-cause with its own yearly rate range.
fn sub(id: &str, name: &str, note: &str, range: Option<[f64; 2]>, sources: &[&str]) -> SubCause {
    SubCause {
        id: id.to_owned(),
        name: name.to_owned(),
        note: note.to_owned(),
        rate_range: range,
        sources: ids(sources),
    }
}

/// One family row before it becomes a [`HazardRate`].
struct Row {
    hazard: HazardId,
    est: Estimate,
    /// Completes "households like yours would …".
    verb: &'static str,
    severity: f64,
    confidence: DataConfidence,
    sub_causes: Vec<SubCause>,
    location_factor: Option<LocationFactor>,
    if_it_reaches_you: String,
    what_it_changes: String,
}

impl Row {
    fn into_rate(self, years: u8) -> HazardRate {
        let sentence = sentence::range_sentence(self.verb, self.est.low, self.est.high, years);
        let mut r = HazardRate::new(self.hazard, self.est, "", 0.0);
        r.display = HazardDisplay::RareCatastrophic;
        r.range_sentence = Some(sentence);
        r.fixed_severity = Some(self.severity);
        r.fixed_confidence = Some(self.confidence);
        r.sub_causes = self.sub_causes;
        r.location_factor = self.location_factor;
        r.range_only = true;
        r.if_it_reaches_you = Some(self.if_it_reaches_you);
        r.what_it_changes = Some(self.what_it_changes);
        r
    }
}

/// The metro weight w_m for the attack rows, and the words for "Why here".
pub(crate) struct MetroWeight {
    /// The weight as an estimate (exact when the area is known).
    pub w: Option<f64>,
    /// Class for [`LocationFactor::class`].
    pub class: &'static str,
    /// "Why here" in words.
    pub label: String,
}

/// The metro weight of this county: its FEMA urban area's share of the UASI money.
pub(crate) fn metro_weight(ctx: &Ctx<'_>) -> MetroWeight {
    match ctx.exposure().uasi() {
        Uasi::Funded { area, metro_share } => {
            let class = if metro_share >= 0.05 {
                "uasi_top"
            } else if metro_share >= 0.02 {
                "uasi_large"
            } else {
                "uasi"
            };
            let name = match area {
                Some(a) => format!("The {a} urban area"),
                None => format!("The urban area around {}", ctx.county_label()),
            };
            let money = if metro_share * 100.0 >= 0.95 {
                format!("about {} in 100", sentence::sig2(metro_share * 100.0))
            } else {
                format!("about {} in 1,000", sentence::sig2(metro_share * 1000.0))
            };
            MetroWeight {
                w: Some(metro_share),
                class,
                label: format!(
                    "{name} is one of the 44 urban areas FEMA funds for terrorism preparedness, \
                     chosen by the risk the Department of Homeland Security assigns them. It gets \
                     {money} of the money, which is how this estimate weighs your area."
                ),
            }
        }
        Uasi::NotFunded => MetroWeight {
            w: None,
            class: "not_funded",
            label: format!(
                "{} is outside the 44 urban areas FEMA funds for terrorism preparedness, so an \
                 attack that closes your area is much less likely than in the big cities.",
                ctx.county_label()
            ),
        },
        Uasi::Unknown => MetroWeight {
            w: None,
            class: "unknown",
            label: "The data for where you live does not say whether it is in one of the 44 \
                    urban areas FEMA funds for terrorism preparedness, so the range runs from a \
                    small town to a large city."
                .to_owned(),
        },
    }
}

/// The nuclear family: serious local effects (blast or dangerous fallout) for this household.
fn nuclear(ctx: &Ctx<'_>, notes: &mut Notes) -> Row {
    let e = ctx.exposure();
    let class = e.strategic_class();
    let (f_s, why, class_id, label_sources): (Triple, String, String, Vec<&str>) = match class {
        Some(c) => {
            let def = strategic::class_def(c);
            let why = strategic::why_here(
                c,
                e.strategic_places(),
                e.strategic_distance(),
                &ctx.county_and_state(),
            );
            let mut src = vec![cite::STRATEGIC_SITES, cite::FEMA_NAPB90];
            match c {
                StrategicClass::A | StrategicClass::C1 => src.push(cite::FEMA_PNA_1985),
                StrategicClass::B => src.push(cite::PHILIPPE_2023),
                _ => {}
            }
            src.push(cite::RR_PRIORS);
            (def.f_s, why, c.as_str().to_owned(), src)
        }
        None => {
            notes.add(format!(
                "The strategic-site class for {} is not in the data yet, so the nuclear row uses \
                 the average over every county.",
                ctx.county_label()
            ));
            (
                STRATEGIC_FACTOR_UNKNOWN,
                strategic::WHY_HERE_UNKNOWN.to_owned(),
                "unknown".to_owned(),
                vec![cite::STRATEGIC_SITES, cite::RR_PRIORS],
            )
        }
    };
    let chain_sources = [
        cite::FRI_NUCLEAR,
        cite::XPT_2023,
        cite::RP_2019,
        cite::BARRETT_2013,
        cite::RR_PRIORS,
    ];
    let lambda_s = span(NUCLEAR_STRATEGIC_US, &chain_sources);
    let lambda_l = span(NUCLEAR_LIMITED_US, &chain_sources);
    let lambda_i = span(NUCLEAR_IND_US, &[cite::FRI_NUCLEAR, cite::RR_PRIORS]);
    let f = span(f_s, &label_sources);
    // Limited strikes weigh on the counterforce counties and the Pacific bases.
    let plausible_limited =
        class == Some(StrategicClass::A) || matches!(ctx.county.state_abbr.as_str(), "HI" | "GU");
    let w_l = if plausible_limited {
        LIMITED_STRIKE_WEIGHT
    } else {
        0.0
    };
    let mw = metro_weight(ctx);
    let s_uasi = mw.w.unwrap_or(0.0);
    let mut local = lambda_s.times_span(&f);
    if w_l > 0.0 {
        local = local.plus(&lambda_l.scaled(w_l * LIMITED_STRIKE_HOUSEHOLD_SHARE));
    }
    if s_uasi > 0.0 {
        local = local
            .plus(&lambda_i.scaled(s_uasi * IND_METRO_HOUSEHOLD_SHARE))
            .cite(&[cite::FEMA_UASI_FY2026]);
    }
    let local = local.cite(&[cite::FEMA_NUCLEAR_72H, cite::READY_NUCLEAR]);

    let hemp = lambda_s
        .times_span(&span(HEMP_GIVEN_STRATEGIC, &[cite::RR_PRIORS]))
        .plus(&lambda_l.times_span(&span(HEMP_GIVEN_LIMITED, &[cite::RR_PRIORS])));
    let national = lambda_s.plus(&lambda_l.scaled(0.5));
    let lower48 = !matches!(
        ctx.county.state_abbr.as_str(),
        "AK" | "HI" | "PR" | "GU" | "VI" | "AS" | "MP"
    );
    let emp_note = if lower48 {
        "Only as part of a nuclear attack: a burst high above the country could cut power and \
         phones over a wide area and damage some cars and electronics. EPRI's 2019 study found \
         months-long nationwide blackouts unlikely; the claim that most Americans would die is \
         not a published model."
    } else {
        "Only as part of a nuclear attack: a burst high above the lower 48 states could cut power \
         and phones there; where you live the effect depends on where the burst is. EPRI's 2019 \
         study found months-long nationwide blackouts unlikely."
    };
    let sub_causes = vec![
        sub(
            "limited_strike",
            "Limited strike on US territory",
            "One or a few weapons on US soil, for example a strike on Guam, Hawaii, Alaska or a \
             West Coast base. It adds to your chance only where your county is a plausible \
             target.",
            Some([lambda_l.low, lambda_l.high]),
            &[cite::FRI_NUCLEAR, cite::RR_PRIORS],
        ),
        sub(
            "nuclear_terrorism",
            "A crude nuclear device in a city",
            "Heavy damage within about a mile, and everyone within 50 miles told to get inside. \
             It adds to your chance by your metro area's share of FEMA's terrorism-preparedness \
             money.",
            Some([lambda_i.low, lambda_i.high]),
            &[cite::FRI_NUCLEAR, cite::FEMA_NUCLEAR_72H, cite::RR_PRIORS],
        ),
        sub(
            "emp",
            "Electromagnetic pulse (EMP) from a high-altitude burst",
            emp_note,
            Some([hemp.low, hemp.high]),
            &[cite::EPRI_HEMP, cite::RR_PRIORS],
        ),
        sub(
            "national_disruption",
            "Disruption across the country",
            "Shortages, power cuts and lost income even far from any target. Shown for everyone \
             and never added to your local chance.",
            Some([national.low, national.high]),
            &[cite::FRI_NUCLEAR, cite::RR_PRIORS],
        ),
        sub(
            "use_abroad",
            "A nuclear weapon used elsewhere in the world",
            "At home: market shocks, shortages and worry, with no fallout of concern. Never added \
             to your local chance.",
            Some([NUCLEAR_USE_WORLD.1, NUCLEAR_USE_WORLD.2]),
            &[
                cite::XPT_2023,
                cite::FRI_NUCLEAR,
                cite::RP_2019,
                cite::RR_PRIORS,
            ],
        ),
    ];
    let (severity, if_it_reaches_you) = match class {
        Some(StrategicClass::A | StrategicClass::C1) => (
            1.0,
            "Life-threatening: blast or heavy fallout near likely targets.",
        ),
        Some(StrategicClass::B | StrategicClass::C2 | StrategicClass::D) => (
            0.5,
            "Serious disruption: sheltering inside for a day or more against fallout, then \
             shortages and outages.",
        ),
        Some(StrategicClass::E) => (
            0.3,
            "Shortages, power cuts and lost income, not blast or heavy fallout.",
        ),
        None => (
            0.5,
            "Life-threatening near likely targets; elsewhere serious disruption, shortages and \
             power cuts.",
        ),
    };
    let what_it_changes = if class == Some(StrategicClass::E) {
        "Nothing beyond your basics."
    } else {
        "One free step: pick your shelter spot at home and at work (a basement, or the middle of \
         the building away from windows)."
    };
    Row {
        hazard: HazardId::NuclearAttack,
        est: local,
        verb: "be in a blast zone or under dangerous fallout",
        severity,
        confidence: DataConfidence::Prior,
        sub_causes,
        location_factor: Some(LocationFactor {
            class: class_id,
            label: why,
            multiplier: [f_s.1, f_s.0, f_s.2],
            sources: ids(&label_sources),
        }),
        if_it_reaches_you: if_it_reaches_you.to_owned(),
        what_it_changes: what_it_changes.to_owned(),
    }
}

/// The EMP rate for the lower 48 states (a sub-cause of the nuclear family), for the months-long
/// blackout row.
fn hemp_rate() -> Estimate {
    let chain = [cite::FRI_NUCLEAR, cite::XPT_2023, cite::RR_PRIORS];
    span(NUCLEAR_STRATEGIC_US, &chain)
        .times_span(&span(
            HEMP_GIVEN_STRATEGIC,
            &[cite::EPRI_HEMP, cite::RR_PRIORS],
        ))
        .plus(
            &span(NUCLEAR_LIMITED_US, &chain)
                .times_span(&span(HEMP_GIVEN_LIMITED, &[cite::RR_PRIORS])),
        )
}

/// A severe (Carrington-class) solar storm cutting this household's power for days.
fn geomagnetic(ctx: &Ctx<'_>) -> (Row, Estimate) {
    let geo = ctx.exposure().geomag();
    let ratio = geo.map_or(1.0, |g| g.factor / GMD_ALPHA_POP_MEAN);
    let (mid, lo, hi) = GMD_OUTAGE_GIVEN_STORM;
    let cap = |x: f64| (x * ratio).min(GMD_OUTAGE_CAP);
    // `cap` keeps the order of low, middle and high (it is monotone).
    let conditional = Estimate::prior(
        cap(mid),
        cap(lo),
        cap(hi),
        &[cite::LLOYDS_2013, cite::NERC_TPL007, cite::RR_PRIORS],
    );
    let storm = span(
        CARRINGTON_STORM,
        &[
            cite::RILEY_2012,
            cite::LOVE_CARRINGTON,
            cite::MORINA_2019,
            cite::LLOYDS_2013,
        ],
    );
    let mut est = storm.times_span(&conditional);
    if geo.is_some() {
        est = est.cite(&[cite::IGRF14]);
    }
    let (class, label) = match geo {
        Some(g) => {
            let band = if g.factor >= 0.4 {
                "high"
            } else if g.factor > 0.15 {
                "middle"
            } else {
                "low"
            };
            let lat = g
                .lat
                .map(|l| format!(" (about {l:.0}°)"))
                .unwrap_or_default();
            let times = if ratio >= 1.05 {
                format!("about {} times the national average", sentence::sig2(ratio))
            } else if ratio <= 0.95 {
                format!("about {} of the national average", sentence::sig2(ratio))
            } else {
                "about the national average".to_owned()
            };
            (
                band,
                format!(
                    "Your county is at a {band} geomagnetic latitude{lat}. Solar storms drive \
                     the strongest currents into long power lines nearer the magnetic pole, so \
                     the chance of a long outage here is {times}. The ground's conductivity also \
                     matters and is not counted."
                ),
            )
        }
        None => (
            "unknown",
            "The data for where you live does not include its geomagnetic latitude yet, so this \
             uses the national average."
                .to_owned(),
        ),
    };
    let row = Row {
        hazard: HazardId::GeomagneticStorm,
        est: est.clone(),
        verb: "lose power for days to a severe solar storm",
        severity: 0.5,
        confidence: DataConfidence::Prior,
        sub_causes: Vec::new(),
        location_factor: Some(LocationFactor {
            class: class.to_owned(),
            label,
            multiplier: [ratio, ratio, ratio],
            sources: ids(&[cite::NERC_TPL007, cite::IGRF14]),
        }),
        if_it_reaches_you: "Power out for days, and longer where large transformers fail."
            .to_owned(),
        what_it_changes: "Nothing beyond your power plan: a solar storm harms long power lines, \
                          not phones or radios."
            .to_owned(),
    };
    (row, est)
}

/// Near military sites and infrastructure (classes A, C1, C2) or far from them (B, D, E).
fn war(ctx: &Ctx<'_>) -> (Row, Estimate) {
    let class = ctx.exposure().strategic_class();
    let (k, class_id, label): (Triple, &str, &str) = match class {
        Some(StrategicClass::A | StrategicClass::C1 | StrategicClass::C2) => (
            (1.0, 1.0, 1.0),
            "near",
            "Your county has military sites, a big city, a port or a refinery nearby: the kinds \
             of places attacks on infrastructure aim at.",
        ),
        Some(_) => (
            (
                WAR_FAR_FROM_TARGETS,
                WAR_FAR_FROM_TARGETS,
                WAR_FAR_FROM_TARGETS,
            ),
            "far",
            "Your county is far from military sites, big cities, ports and refineries, so wartime \
             attacks on infrastructure are less likely to reach it.",
        ),
        None => (
            (0.71, WAR_FAR_FROM_TARGETS, 1.0),
            "unknown",
            "The data for where you live does not say how near it is to military sites and \
             infrastructure, so this uses the national average.",
        ),
    };
    let est = span(GREAT_POWER_WAR, &[cite::FRI_NUCLEAR, cite::RR_PRIORS])
        .times_span(&span(WAR_HOMELAND_ATTACKED, &[cite::RR_PRIORS]))
        .times_span(&span(k, &[cite::STRATEGIC_SITES]));
    let row = Row {
        hazard: HazardId::WarInfrastructure,
        est: est.clone(),
        verb: "lose power, water or phone service for days to attacks in a war",
        severity: 0.5,
        confidence: DataConfidence::Prior,
        sub_causes: Vec::new(),
        location_factor: Some(LocationFactor {
            class: class_id.to_owned(),
            label: label.to_owned(),
            multiplier: [k.1, k.0, k.2],
            sources: ids(&[cite::STRATEGIC_SITES, cite::RR_PRIORS]),
        }),
        if_it_reaches_you: "Outages of power, water or phones for days, and shortages across the \
                            country."
            .to_owned(),
        what_it_changes: "Nothing beyond your basics.".to_owned(),
    };
    (row, est)
}

/// Power out for two months or more from any cause: the solar-storm, EMP and war parts here;
/// the county's own outage record is added by [`crate::HazardAssessment::add_power_curve`] once
/// `rr-consequence` has built the power curve.
fn multi_month(gmd: &Estimate, war: &Estimate, lower48: bool) -> Row {
    let gmd_part = gmd.times_span(&span(
        GMD_SHARE_GE_60D,
        &[cite::LLOYDS_2013, cite::RR_PRIORS],
    ));
    let hemp_part = if lower48 {
        hemp_rate().times_span(&span(
            HEMP_SHARE_GE_60D,
            &[cite::EPRI_HEMP, cite::RR_PRIORS],
        ))
    } else {
        Estimate::exact(0.0)
    };
    let war_part = war.times_span(&span(WAR_SHARE_GE_60D, &[cite::RR_PRIORS]));
    let est = gmd_part
        .plus(&hemp_part)
        .plus(&war_part)
        .cite(&[cite::NPR_MARIA]);
    let sub_causes = vec![
        sub(
            "solar_storm",
            "A severe solar storm",
            "Long outages where large transformers fail; most places would be back within days.",
            Some([gmd_part.low, gmd_part.high]),
            &[cite::LLOYDS_2013, cite::RR_PRIORS],
        ),
        sub(
            "emp",
            "EMP from a nuclear attack",
            "Only as part of a nuclear attack; EPRI's 2019 study found months-long nationwide \
             blackouts unlikely.",
            Some([hemp_part.low, hemp_part.high]),
            &[cite::EPRI_HEMP, cite::RR_PRIORS],
        ),
        sub(
            "war",
            "Attacks on the grid in a war",
            "Cyberattacks, sabotage or missiles on power plants and lines.",
            Some([war_part.low, war_part.high]),
            &[cite::FRI_NUCLEAR, cite::RR_PRIORS],
        ),
    ];
    Row {
        hazard: HazardId::MultiMonthBlackout,
        est,
        verb: "be without power for two months or more",
        severity: 0.8,
        confidence: DataConfidence::Medium,
        sub_causes,
        location_factor: None,
        if_it_reaches_you: "No power for months: water, heat, medicine and money all affected. \
                            Puerto Rico waited 328 days after Hurricane Maria."
            .to_owned(),
        what_it_changes: "Nothing to stockpile for months. The long-horizon section lists what \
                          helps instead: a water filter with a water source, a way to cook, \
                          sanitation."
            .to_owned(),
    }
}

/// A chemical, biological or radiological attack disrupting daily life in this household's area.
fn cbrn(ctx: &Ctx<'_>) -> Row {
    let mw = metro_weight(ctx);
    let sources = [cite::START_POICN, cite::FEMA_UASI_FY2026, cite::RR_PRIORS];
    let est = match (mw.class, mw.w) {
        (_, Some(w)) => span(CBRN_US, &sources)
            .scaled(w)
            .times_span(&span(CBRN_METRO_SHARE, &[cite::RR_PRIORS])),
        ("unknown", None) => Estimate::prior(
            CBRN_US.0 * 0.01 * CBRN_METRO_SHARE.0,
            CBRN_NON_UASI.1,
            CBRN_US.2 * 0.05 * CBRN_METRO_SHARE.2,
            &sources,
        ),
        _ => span(CBRN_NON_UASI, &sources),
    };
    let multiplier = match mw.w {
        Some(w) => [w, w, w],
        None => {
            let base = CBRN_US.0 * CBRN_METRO_SHARE.0;
            [est.low / base, est.value / base, est.high / base]
        }
    };
    let sub_causes = vec![
        sub(
            "chemical",
            "Chemical",
            "About 3 in 4 such incidents worldwide involve chemicals: shelter inside for hours, \
             or leave the immediate area if told to.",
            None,
            &[cite::START_POICN],
        ),
        sub(
            "biological",
            "Biological",
            "A germ spread on purpose (the anthrax letters of 2001): stay home, and collect \
             preventive medicine at a public site if told to.",
            None,
            &[cite::START_POICN],
        ),
        sub(
            "radiological",
            "Radiological (\"dirty bomb\")",
            "Shelter inside for hours; the area may stay closed for weeks. None has caused mass \
             casualties anywhere.",
            None,
            &[cite::START_POICN],
        ),
    ];
    Row {
        hazard: HazardId::CbrnAttack,
        est,
        verb: "be told to stay inside, or to collect medicine at a public site, after a chemical, \
               biological or radiological attack",
        severity: 0.5,
        confidence: DataConfidence::Prior,
        sub_causes,
        location_factor: Some(LocationFactor {
            class: mw.class.to_owned(),
            label: mw.label,
            multiplier,
            sources: ids(&[cite::FEMA_UASI_FY2026, cite::RR_PRIORS]),
        }),
        if_it_reaches_you: "An order to stay inside for hours, closed buildings, or medicine \
                            handed out at public sites."
            .to_owned(),
        what_it_changes: "Nothing beyond your basics: your three-day supplies cover sheltering \
                          inside."
            .to_owned(),
    }
}

fn severe_pandemic() -> Row {
    Row {
        hazard: HazardId::SeverePandemic,
        est: span(
            SEVERE_PANDEMIC,
            &[cite::MARANI_2021, cite::CDC_PANDEMICS, cite::RR_PRIORS],
        ),
        verb: "live through a pandemic far deadlier than COVID-19",
        severity: 0.8,
        confidence: DataConfidence::Prior,
        sub_causes: vec![
            sub(
                "natural_1918_class",
                "A natural pandemic as deadly as 1918",
                "The 1918 flu killed about 675,000 Americans. Records of pandemics over four \
                 centuries give the chance.",
                None,
                &[cite::MARANI_2021, cite::CDC_PANDEMICS],
            ),
            sub(
                "engineered",
                "An engineered germ",
                "A germ made or changed on purpose. There is no record to count; it is part of \
                 the expert range.",
                None,
                &[cite::RR_PRIORS],
            ),
        ],
        location_factor: None,
        if_it_reaches_you: "Months of disruption, strained hospitals and lost income.".to_owned(),
        what_it_changes: "Nothing new: the pandemic row already sizes your food and medicine."
            .to_owned(),
    }
}

fn vei7() -> Row {
    let y = YELLOWSTONE;
    Row {
        hazard: HazardId::Vei7Eruption,
        est: span(VEI7_WORLD, &[cite::CASSIDY_MANI_2022, cite::RR_PRIORS]),
        verb: "live through a year or two of higher food prices after a very large eruption \
               somewhere in the world",
        severity: 0.3,
        confidence: DataConfidence::Medium,
        sub_causes: vec![sub(
            "yellowstone",
            "A Yellowstone super-eruption",
            "Devastation nearby and ash across much of the country. The USGS puts it at about 1 \
             in 730,000 a year.",
            Some([y.1, y.2]),
            &[cite::USGS_YVO],
        )],
        location_factor: None,
        if_it_reaches_you: "A year or two of higher food prices and some shortages.".to_owned(),
        what_it_changes: "Nothing beyond a two-week pantry.".to_owned(),
    }
}

fn financial() -> Row {
    Row {
        hazard: HazardId::FinancialCrisis,
        est: span(FINANCIAL_CRISIS, &[cite::FDIC_FAILURES, cite::RR_PRIORS]),
        verb: "have banks close for three days or more in a financial crisis",
        severity: 0.3,
        confidence: DataConfidence::Prior,
        sub_causes: vec![sub(
            "bank_failure",
            "Your own bank fails",
            "Insured deposits (up to $250,000 per person at each bank) move to another bank, \
             usually within days. The FDIC counts about 23 failures a year (2001–2025), most of \
             them small banks.",
            None,
            &[cite::FDIC_FAILURES],
        )],
        location_factor: None,
        if_it_reaches_you: "Cards and bank transfers stop for days.".to_owned(),
        what_it_changes: "Keep some cash in small bills (already in your plan) and a second \
                          account at another bank (free)."
            .to_owned(),
    }
}

fn mass_violence(ctx: &Ctx<'_>) -> Row {
    let people = ctx.people().max(1) as f64;
    Row {
        hazard: HazardId::MassViolence,
        est: Estimate::data(
            MASS_VIOLENCE_PER_PERSON.0,
            MASS_VIOLENCE_PER_PERSON.1,
            MASS_VIOLENCE_PER_PERSON.2,
            &[cite::FBI_ACTIVE_SHOOTER, cite::RR_PRIORS],
        )
        .scaled(people),
        verb: "have someone hurt in a mass shooting or bombing",
        severity: 0.9,
        confidence: DataConfidence::Medium,
        sub_causes: Vec::new(),
        location_factor: Some(LocationFactor {
            class: "not_modelled".to_owned(),
            label: "The data are too sparse to say where this is more likely, so the chance is \
                    the same everywhere."
                .to_owned(),
            multiplier: [1.0, 1.0, 1.0],
            sources: ids(&[cite::FBI_ACTIVE_SHOOTER]),
        }),
        if_it_reaches_you: "Injury or death.".to_owned(),
        what_it_changes: "Nothing to buy. Two free steps: know \"run, hide, fight\", and learn to \
                          stop bleeding."
            .to_owned(),
    }
}

/// The nine rare families for this household, in `HazardId` order.
pub(crate) fn assess(ctx: &Ctx<'_>, notes: &mut Notes) -> Vec<HazardRate> {
    let years = ctx.years();
    let lower48 = !matches!(
        ctx.county.state_abbr.as_str(),
        "AK" | "HI" | "PR" | "GU" | "VI" | "AS" | "MP"
    );
    let (gmd_row, gmd) = geomagnetic(ctx);
    let (war_row, war_rate) = war(ctx);
    let rows = vec![
        gmd_row,
        vei7(),
        nuclear(ctx, notes),
        multi_month(&gmd, &war_rate, lower48),
        war_row,
        cbrn(ctx),
        severe_pandemic(),
        financial(),
        mass_violence(ctx),
    ];
    let mut out: Vec<HazardRate> = rows.into_iter().map(|r| r.into_rate(years)).collect();
    out.sort_by_key(|r| r.hazard);
    out
}

/// "Also checked": the family sub-rows too rare to show here, with their rates.
pub(crate) fn also_checked() -> Vec<(&'static str, &'static str, Triple, &'static [&'static str])> {
    vec![
        (
            "asteroid",
            "an asteroid or comet impact",
            ASTEROID,
            &[cite::NASA_TUNGUSKA],
        ),
        (
            "yellowstone",
            "a Yellowstone super-eruption",
            YELLOWSTONE,
            &[cite::USGS_YVO],
        ),
    ]
}
