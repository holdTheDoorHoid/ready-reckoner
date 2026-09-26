//! Water: stored water, making water safe, and livestock (research §1).

use rr_types::{Housing, HousingKind, Per, Person, Pets, RawWaterSource, WaterLevel, WaterSource};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::{constants, keys};
use crate::format::{
    DAYS_PER_MONTH, GAL_PER_SQFT_INCH, L_PER_GAL, OZ_PER_GAL, and_list, ceil_count, count,
    days as fmt_days, gallons, num, people, round_to,
};
use crate::household::is_formula_fed;

/// One day of water for the household, split into its parts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterDaily {
    /// The water level the household chose.
    pub level: WaterLevel,
    /// Whether the hot-climate amount applies.
    pub hot: bool,
    /// People in the household.
    pub people: usize,
    /// Gallons per person per day (the level's allowance, with the drinking share doubled in heat;
    /// the drinking share alone for [`Share::Drinking`]).
    pub allowance_gal: f64,
    /// The drinking part of the allowance, in gallons per person per day.
    pub drinking_gal: f64,
    /// People marked pregnant or nursing.
    pub nursing: usize,
    /// Extra gallons per pregnant or nursing person per day.
    pub nursing_gal_each: f64,
    /// Babies on formula.
    pub formula_infants: usize,
    /// Gallons to mix one baby's formula for a day.
    pub formula_gal_each: f64,
    /// Dogs, cats and small pets.
    pub pets: [u8; 3],
    /// Gallons per day for one dog, one cat and one small pet.
    pub pet_gal_each: [f64; 3],
}

impl WaterDaily {
    /// People's water per day, in gallons (allowances plus pregnancy, nursing and formula).
    pub fn people_gal(&self) -> f64 {
        self.people as f64 * self.allowance_gal
            + self.nursing as f64 * self.nursing_gal_each
            + self.formula_infants as f64 * self.formula_gal_each
    }

    /// Pets' water per day, in gallons.
    pub fn pets_gal(&self) -> f64 {
        (0..3)
            .map(|i| f64::from(self.pets[i]) * self.pet_gal_each[i])
            .sum()
    }

    /// Everything per day, in gallons.
    pub fn total_gal(&self) -> f64 {
        self.people_gal() + self.pets_gal()
    }

    /// Water that must be safe to swallow per day: people's drinking share, pregnancy and
    /// nursing, formula and pets.
    pub fn drinking_total_gal(&self) -> f64 {
        self.people as f64 * self.drinking_gal
            + self.nursing as f64 * self.nursing_gal_each
            + self.formula_infants as f64 * self.formula_gal_each
            + self.pets_gal()
    }
}

/// Which part of the day's water a rule counts, so that it reads (and cites) only the numbers
/// behind its own amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Share {
    /// Everything at the chosen level: drinking, cooking and washing.
    All,
    /// Only water that must be safe to swallow (a boil-water notice), whatever the level.
    Drinking,
}

/// One day of water for these people and pets (research §1.6), reading every number it uses
/// through `b`.
pub(crate) fn daily(
    b: &mut Basis,
    people: &[Person],
    pets: &Pets,
    level: WaterLevel,
    hot: bool,
) -> WaterDaily {
    daily_share(b, people, pets, level, hot, Share::All)
}

/// The household's water for one day at its level, in gallons (people, pregnancy, formula and
/// pets), without recording sources: for sizing that cites its own numbers.
pub(crate) fn daily_total_gal(people: &[Person], pets: &Pets, level: WaterLevel, hot: bool) -> f64 {
    daily(&mut Basis::new(), people, pets, level, hot).total_gal()
}

/// [`daily`] for one share of the water. The drinking share is cited only when the amount uses
/// it (in heat, or for [`Share::Drinking`]); otherwise it is filled in for reference.
pub(crate) fn daily_share(
    b: &mut Basis,
    people: &[Person],
    pets: &Pets,
    level: WaterLevel,
    hot: bool,
    share: Share,
) -> WaterDaily {
    // The drinking part of each level: Ready.gov's 3/4 gallon of the basic gallon; the survival
    // level is all drinking; Sphere's 15 L keeps the survival drinking amount (about 3 L) and adds
    // cooking and washing.
    let drinking_key = match level {
        WaterLevel::Basic => keys::WATER_DRINKING_SHARE_BASIC_GAL,
        WaterLevel::Survival | WaterLevel::Comfortable => keys::WATER_SURVIVAL_GAL,
    };
    let total_key = match level {
        WaterLevel::Survival => keys::WATER_SURVIVAL_GAL,
        WaterLevel::Basic => keys::WATER_BASIC_GAL,
        WaterLevel::Comfortable => keys::WATER_COMFORTABLE_GAL,
    };
    let (allowance_gal, drinking_gal) = if people.is_empty() {
        (0.0, 0.0)
    } else {
        let drinking = if hot || share == Share::Drinking {
            b.k(drinking_key)
        } else {
            constants().value(drinking_key)
        };
        let total = match share {
            Share::All => b.k(total_key),
            Share::Drinking => drinking,
        };
        if hot {
            let m = b.k(keys::WATER_HEAT_DRINKING_MULTIPLIER);
            b.k(keys::HOT_CLIMATE_DAYS_95F);
            (total + drinking * (m - 1.0), drinking * m)
        } else {
            (total, drinking)
        }
    };
    let nursing = people.iter().filter(|p| p.pregnant_or_nursing).count();
    let formula_infants = people.iter().filter(|p| is_formula_fed(p)).count();
    let nursing_gal_each = if nursing > 0 {
        b.k(keys::WATER_NURSING_ADD_GAL)
    } else {
        0.0
    };
    let formula_gal_each = if formula_infants > 0 {
        b.k(keys::WATER_FORMULA_INFANT_GAL)
    } else {
        0.0
    };
    let dog = if pets.dogs > 0 {
        b.k(keys::WATER_DOG_OZ_PER_LB_DAY) * b.k(keys::DOG_WEIGHT_LB) / OZ_PER_GAL
    } else {
        0.0
    };
    let cat = if pets.cats > 0 {
        b.k(keys::WATER_CAT_OZ_PER_LB_DAY) * b.k(keys::CAT_WEIGHT_LB) / OZ_PER_GAL
    } else {
        0.0
    };
    let small = if pets.small > 0 {
        b.k(keys::WATER_SMALL_PET_OZ_DAY) / OZ_PER_GAL
    } else {
        0.0
    };
    WaterDaily {
        level,
        hot,
        people: people.len(),
        allowance_gal,
        drinking_gal,
        nursing,
        nursing_gal_each,
        formula_infants,
        formula_gal_each,
        pets: [pets.dogs, pets.cats, pets.small],
        pet_gal_each: [dog, cat, small],
    }
}

/// "the dog", "the 2 dogs and the cat".
pub(crate) fn pets_phrase(pets: [u8; 3]) -> String {
    let names = [
        ("dog", "dogs"),
        ("cat", "cats"),
        ("small pet", "small pets"),
    ];
    let parts: Vec<String> = pets
        .iter()
        .zip(names)
        .filter(|(n, _)| **n > 0)
        .map(|(n, (one, many))| {
            if *n == 1 {
                format!("the {one}")
            } else {
                format!("the {n} {many}")
            }
        })
        .collect();
    and_list(&parts)
}

/// "1.75 gallons" for a daily rate (two decimals).
fn rate(g: f64) -> String {
    let s = num(g, 2);
    if s == "1" {
        "1 gallon".to_owned()
    } else {
        format!("{s} gallons")
    }
}

/// Stored water for `days` days (research §1.6): people at the chosen level, pregnancy and
/// nursing, formula, household pets, and optionally the extra water that dehydrated food needs.
/// Rule `water_gallons`.
pub fn water_gallons(
    days: f64,
    people_list: &[Person],
    pets: &Pets,
    level: WaterLevel,
    hot: bool,
    dehydrated_food_person_days: f64,
) -> Sizing {
    let mut b = Basis::new();
    let d = daily(&mut b, people_list, pets, level, hot);
    stored(&mut b, &d, days, dehydrated_food_person_days)
}

fn stored(b: &mut Basis, d: &WaterDaily, days: f64, dehydrated_person_days: f64) -> Sizing {
    let rehydrate_each = if dehydrated_person_days > 0.0 {
        b.k(keys::WATER_REHYDRATION_GAL_PER_PERSON_DAY)
    } else {
        0.0
    };
    let rehydrate = rehydrate_each * dehydrated_person_days;
    let total = d.total_gal() * days + rehydrate;
    let base = d.people as f64 * d.allowance_gal * days;
    let mut extras = Vec::new();
    if d.nursing > 0 {
        extras.push(format!(
            "{} for pregnancy or breastfeeding",
            gallons(d.nursing as f64 * d.nursing_gal_each * days)
        ));
    }
    if d.formula_infants > 0 {
        extras.push(format!(
            "{} to mix formula",
            gallons(d.formula_infants as f64 * d.formula_gal_each * days)
        ));
    }
    if d.pets_gal() > 0.0 {
        extras.push(format!(
            "{} for {}",
            gallons(d.pets_gal() * days),
            pets_phrase(d.pets)
        ));
    }
    if rehydrate > 0.0 {
        extras.push(format!("{} to prepare dried food", gallons(rehydrate)));
    }
    let q = super::round_quantity("gallon", total);
    let mut text = format!(
        "{} × {} a day × {} = {}",
        people(d.people as f64),
        rate(d.allowance_gal),
        fmt_days(days),
        gallons(base)
    );
    if extras.is_empty() {
        text.push('.');
    } else {
        text.push_str(&format!(
            ", plus {}: about {}.",
            extras.join(", plus "),
            gallons(q)
        ));
    }
    match d.level {
        WaterLevel::Survival => text.push_str(" This is the survival level: drinking water only."),
        WaterLevel::Basic => {}
        WaterLevel::Comfortable => text.push_str(&format!(
            " This is the comfortable level: about {} L a person a day for drinking, cooking and washing.",
            num(d.allowance_gal * L_PER_GAL, 0)
        )),
    }
    if d.hot {
        let threshold = b.k(keys::HOT_CLIMATE_DAYS_95F);
        text.push_str(&format!(
            " Your county has at least {} days a year at 95 °F or more, and drinking water can double in heat, so each person gets {} a day.",
            num(threshold, 0),
            rate(d.allowance_gal)
        ));
    }
    let rotate = b.k(keys::WATER_ROTATION_MONTHS);
    text.push_str(&format!(
        " Replace water you store yourself every {}.",
        count(rotate, "month", "months")
    ));
    let math = vec![
        format!(
            "per day = {} × {} + extras {} = {} gal",
            d.people,
            num(d.allowance_gal, 3),
            num(d.total_gal() - d.people as f64 * d.allowance_gal, 3),
            num(d.total_gal(), 4)
        ),
        format!(
            "total = {} gal × {} days{} = {} gal",
            num(d.total_gal(), 4),
            num(days, 2),
            if rehydrate > 0.0 {
                format!(" + {} gal", num(rehydrate, 2))
            } else {
                String::new()
            },
            num(total, 3)
        ),
    ];
    Sizing::new(
        b,
        "water_gallons",
        "water_stored",
        total,
        "gallon",
        Per::Person,
        text,
    )
    .per_day(days, d.total_gal())
    .math(math)
}

/// Where water beyond the stored days would come from, for the treatment line's wording and for
/// whether a filter counts (round-2 review P-03, S6; contract v2's `Housing.raw_water_source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawSource {
    /// The household's own well, which needs pump power (a generator through an interlock or
    /// transfer switch) or a hand pump.
    Well,
    /// A source the household named in the interview.
    Named(RawWaterSource),
    /// The rain barrel the plan suggests (the rain line), where the state allows drinking it.
    PlannedBarrel,
    /// No source known: a filter adds nothing, and the water beyond the stored days has to be
    /// carried in.
    None,
}

impl RawSource {
    /// Whether there is water to put through a filter.
    pub fn exists(self) -> bool {
        self != RawSource::None
    }

    /// The source for a household: its well, then the source it named, then a planned rain barrel.
    pub fn of(housing: &Housing, planned_barrel: bool) -> RawSource {
        if housing.water == WaterSource::Well {
            return RawSource::Well;
        }
        match housing.raw_water_source {
            Some(s) if s != RawWaterSource::None => RawSource::Named(s),
            _ if planned_barrel => RawSource::PlannedBarrel,
            _ => RawSource::None,
        }
    }
}

/// Stored water for a no-tap-water target, capped at the stored-water days (14) when the target
/// is longer; beyond that, the second sizing is the water to make safe with a filter and a source
/// (`water_treatment_capacity`, per household). Research §1.6 and §9.5; the brief's rule for
/// the Coos Bay case ("prefer a filter and a source to 100+ gallons").
pub fn water_storage(
    target_days: f64,
    people_list: &[Person],
    pets: &Pets,
    level: WaterLevel,
    hot: bool,
    source: RawSource,
) -> (Sizing, Option<Sizing>) {
    let cap = constants().value(keys::WATER_STORED_CAP_DAYS);
    let mut base = Basis::new();
    let d = daily(&mut base, people_list, pets, level, hot);
    if target_days <= cap {
        return (stored(&mut base.clone(), &d, target_days, 0.0), None);
    }
    let line = stored(&mut base.clone(), &d, cap, 0.0);
    let mut why = Basis::new();
    let cap = why.k(keys::WATER_STORED_CAP_DAYS);
    let all = super::round_quantity("gallon", d.total_gal() * target_days);
    let sentence = format!(
        "Your target is {}. Storing all of it would take about {}, so the plan stores the first {} and makes the rest safe with a filter and a water source (see the next line).",
        fmt_days(target_days),
        gallons(all),
        fmt_days(cap)
    );
    let line = line.extend(&sentence, &why);
    let treat = treatment_beyond(&mut base, &d, target_days, source);
    (line, Some(treat))
}

/// Water to make safe after the stored days run out (rule `water_treatment_capacity`). A filter
/// counts only with a raw-water source, and the line names it: the household's own well (with a
/// way to run its pump), the source it named, or the rain barrel the plan suggests; with none, the
/// line says a filter adds nothing and the water has to be carried in.
fn treatment_beyond(b: &mut Basis, d: &WaterDaily, target_days: f64, source: RawSource) -> Sizing {
    let cap = b.k(keys::WATER_STORED_CAP_DAYS);
    let extra_days = target_days - cap;
    let q = d.total_gal() * extra_days;
    let how = how_to_treat(b);
    let source_words = match source {
        RawSource::Well => {
            "water from your well, which needs a way to run its pump in a power cut (a generator connected through an interlock or transfer switch an electrician installs, or a hand pump)"
        }
        RawSource::Named(RawWaterSource::SurfaceNearby) => {
            "water from the stream, river or pond near you that you named"
        }
        RawSource::Named(RawWaterSource::NeighbourWell) => {
            "water from your neighbour's well, which you said you may use"
        }
        RawSource::Named(RawWaterSource::RainBarrel) => "rain from your rain barrel",
        RawSource::Named(_) => "the raw-water source you named",
        RawSource::PlannedBarrel => {
            "rain from a barrel on your downspout (see the rain line), where your state allows drinking it"
        }
        RawSource::None => "",
    };
    let text = if source.exists() {
        format!(
            "After day {}, for the other {}: make about {} of water safe each day, about {} in all, with a water filter and {source_words}, instead of storing more. Filter it, then {}.",
            num(cap, 1),
            fmt_days(extra_days),
            gallons(d.total_gal()),
            gallons(super::round_quantity("gallon", q)),
            how
        )
    } else {
        format!(
            "After day {}, for the other {}: your household needs about {} of safe water each day, about {} in all. You have not named a raw-water source, so a filter adds nothing yet: pick one in your water plan (a stream or pond you can reach, or a neighbour's well you may use), or plan to carry water from a distribution point (see the carriers line). Water you collect must be made safe: filter it, then {}.",
            num(cap, 1),
            fmt_days(extra_days),
            gallons(d.total_gal()),
            gallons(super::round_quantity("gallon", q)),
            how
        )
    };
    let math = vec![format!(
        "treat = {} gal/day × ({} − {}) days = {} gal; source: {}",
        num(d.total_gal(), 4),
        num(target_days, 2),
        num(cap, 2),
        num(q, 3),
        match source {
            RawSource::Well => "the well",
            RawSource::Named(_) => "named by the household",
            RawSource::PlannedBarrel => "a planned rain barrel",
            RawSource::None => "none",
        }
    )];
    Sizing::new(
        b,
        "water_treatment_capacity",
        "water_treatment_capacity",
        q,
        "gallon",
        Per::Household,
        text,
    )
    .per_day(extra_days, d.total_gal())
    .math(math)
}

/// "boil it for 1 minute (3 minutes above 5,000 feet), or add 8 drops of 6 % unscented bleach
/// per gallon (6 drops of 8.25 %) and wait 30 minutes; double the bleach for cloudy or very cold
/// water". Every number from the registry (EPA, CDC).
pub(crate) fn how_to_treat(b: &mut Basis) -> String {
    let boil = b.k(keys::BOIL_MINUTES);
    let boil_high = b.k(keys::BOIL_MINUTES_HIGH_ALTITUDE);
    let altitude = b.k(keys::BOIL_ALTITUDE_FT);
    let d6 = b.k(keys::BLEACH_DROPS_PER_GAL_6PCT);
    let d8 = b.k(keys::BLEACH_DROPS_PER_GAL_8PCT);
    let wait = b.k(keys::DISINFECT_WAIT_MINUTES);
    let cloudy = b.k(keys::BLEACH_CLOUDY_MULTIPLIER);
    let cloudy_words = if (cloudy - 2.0).abs() < 1e-9 {
        "double the bleach".to_owned()
    } else {
        format!("use {} times the bleach", num(cloudy, 1))
    };
    format!(
        "boil it for {} ({} above {} feet), or add {} drops of 6 % unscented bleach per gallon ({} drops of 8.25 %) and wait {}; {} for cloudy or very cold water",
        count(boil, "minute", "minutes"),
        count(boil_high, "minute", "minutes"),
        num(altitude, 0),
        num(d6, 0),
        num(d8, 0),
        count(wait, "minute", "minutes"),
        cloudy_words
    )
}

/// Water to make safe during a boil-water notice of `days` days: the drinking share (Ready.gov's
/// 3/4 gallon a person, doubled in heat) plus pregnancy, nursing, formula and pets. It does not
/// depend on the water level, which only changes how much washing water is stored. Rule
/// `water_treatment_capacity` (bucket `water_boil`).
pub fn water_treatment_boil(days: f64, people_list: &[Person], pets: &Pets, hot: bool) -> Sizing {
    let mut b = Basis::new();
    let d = daily_share(
        &mut b,
        people_list,
        pets,
        WaterLevel::Basic,
        hot,
        Share::Drinking,
    );
    let per_day = d.drinking_total_gal();
    let q = per_day * days;
    let how = how_to_treat(&mut b);
    b.cite("cdc_co_basics");
    let mut text = format!(
        "For a boil-water notice of up to {}: make about {} of drinking water safe each day, about {} in all. To make it safe, {}. A gas stove can boil water while the gas flows; never use a camp stove or grill indoors.",
        fmt_days(days),
        gallons(per_day),
        gallons(super::round_quantity("gallon", q)),
        how
    );
    if d.nursing > 0 {
        text.push_str(" If you are pregnant or breastfeeding, don't use iodine tablets.");
    }
    let math = vec![format!(
        "per day = {} × {} + extras {} = {} gal; × {} days = {} gal",
        d.people,
        num(d.drinking_gal, 3),
        num(per_day - d.people as f64 * d.drinking_gal, 3),
        num(per_day, 4),
        num(days, 2),
        num(q, 3)
    )];
    Sizing::new(
        &b,
        "water_treatment_capacity",
        "water_treatment_capacity",
        q,
        "gallon",
        Per::Household,
        text,
    )
    .per_day(days, per_day)
    .math(math)
}

/// Tap water in clean reused drink bottles: the household's water for the days it stores (the
/// no-water target, at most the 14 stored days), but no more than it can bottle (6 gallons, an
/// estimate). A free way to meet part, or for a small household all, of `water_gallons` (round-2
/// review P-13: the zero-budget student was told to buy water it could fill for free). The line
/// cites the household's water sources only when its water is under the cap (then that is the
/// amount); otherwise the cap is the amount, and only its sources stand behind it. Rule
/// `water_reused_bottles`.
pub fn water_reused_bottles(
    stored_days: f64,
    people_list: &[Person],
    pets: &Pets,
    level: WaterLevel,
    hot: bool,
) -> Sizing {
    let mut need = Basis::new();
    let d = daily(&mut need, people_list, pets, level, hot);
    let days = stored_days.max(0.0);
    let all_days = d.total_gal() * days;
    let cap = constants().value(keys::REUSED_BOTTLES_MAX_GAL);
    let mut b = Basis::new();
    let (q, amount) = if all_days >= cap {
        let max = b.k(keys::REUSED_BOTTLES_MAX_GAL);
        (
            max,
            format!(
                "about {}, the most a household can usually gather this way",
                gallons(max)
            ),
        )
    } else {
        b.cite_all(need.cites());
        (
            all_days,
            format!(
                "about {} (all {} of your water)",
                gallons(super::round_quantity("gallon", all_days)),
                fmt_days(days)
            ),
        )
    };
    let rotate = b.k(keys::WATER_ROTATION_MONTHS);
    b.cite("church_emergency_prep_manual");
    let text = format!(
        "Free first step: fill clean drink bottles you already have with tap water, {amount}. Not milk jugs: they leak. Label them and replace the water every {}.",
        count(rotate, "month", "months")
    );
    Sizing::new(
        &b,
        "water_reused_bottles",
        "water_stored",
        q,
        "gallon",
        Per::Household,
        text,
    )
    .math(vec![format!(
        "min({} gal/day × {} days, {} gal) = {} gal",
        num(d.total_gal(), 4),
        num(days, 2),
        num(cap, 1),
        num(q, 3)
    )])
}

/// Unscented bleach for making water safe and for cleaning: one bottle per household whatever the
/// target, because at half a millilitre a gallon one bottle treats thousands of gallons (CDC), and a
/// fresh bottle every six months, because bleach weakens in storage (an estimate). Rule
/// `bleach_bottles`.
pub fn bleach_bottles() -> Sizing {
    let mut b = Basis::new();
    let q = b.k(keys::BLEACH_BOTTLES_PER_HOUSEHOLD);
    let (lo, hi) = b.range(keys::BLEACH_STRENGTH_PCT);
    let ml = b.k(keys::BLEACH_ML_PER_GAL);
    let cloudy = b.k(keys::BLEACH_CLOUDY_MULTIPLIER);
    let replace = b.k(keys::BLEACH_REPLACE_MONTHS);
    b.cite("cdc_bleach_disinfecting");
    // Gallons of clear water one fluid ounce of bleach makes safe.
    let gal_per_oz = L_PER_GAL * 1000.0 / OZ_PER_GAL / ml;
    let text = format!(
        "{} of plain unscented bleach ({} to {} % sodium hypochlorite, not the splashless kind), for making water safe and for cleaning, however long your target is. At about {} mL a gallon, each ounce treats about {} gallons of clear water ({} if it is cloudy or very cold), so one bottle treats thousands of gallons. Bleach weakens in storage: buy a fresh bottle every {}, when you replace the water you store yourself.",
        count(q, "bottle", "bottles"),
        num(lo, 0),
        num(hi, 0),
        num(ml, 1),
        num(round_to(gal_per_oz, 10.0), 0),
        num(round_to(gal_per_oz / cloudy, 10.0), 0),
        count(replace, "month", "months")
    );
    Sizing::new(
        &b,
        "bleach_bottles",
        "water_treatment_capacity",
        q,
        "bottle",
        Per::Household,
        text,
    )
    .math(vec![format!(
        "{} bottle per household; {} L/gal × 1000 mL/L ÷ {} oz/gal ÷ {} mL per gal of water = {} gal of water per oz",
        num(q, 0),
        num(L_PER_GAL, 3),
        num(OZ_PER_GAL, 0),
        num(ml, 2),
        num(gal_per_oz, 1)
    )])
}

/// Propane to boil `gallons_to_boil` on a camp stove outdoors (an estimate). Rule `boil_fuel`.
pub fn boil_fuel(gallons_to_boil: f64) -> Sizing {
    let mut b = Basis::new();
    let per_lb = b.k(keys::PROPANE_L_BOILED_PER_LB);
    let (lo, hi) = b.range(keys::PROPANE_L_BOILED_PER_LB);
    let max_small = b.k(keys::PROPANE_SMALL_CYLINDERS_INDOOR_MAX);
    b.cite("cdc_co_basics");
    let litres = gallons_to_boil * L_PER_GAL;
    let lb = litres / per_lb;
    let text = format!(
        "If you boil on a camp stove outdoors, about {} lb of propane brings {} L to a boil (about {} to {} L a pound). Never use a camp stove indoors, and keep no more than {} one-pound cylinders inside.",
        num(super::round_quantity("lb", lb), 1),
        num(litres, 0),
        num(lo, 0),
        num(hi, 0),
        num(max_small, 0)
    );
    Sizing::new(
        &b,
        "boil_fuel",
        "water_treatment_capacity",
        lb,
        "lb",
        Per::Household,
        text,
    )
    .math(vec![format!(
        "{} L ÷ {} L/lb = {} lb",
        num(litres, 1),
        num(per_lb, 1),
        num(lb, 2)
    )])
}

/// Water for livestock and horses (Sphere): 25 L a day per large animal for the target's days, but
/// no more than the days of water a household stores (`water_stored_cap_days`, 14), the same cap
/// as people's stored water (an estimate for animals). Beyond that the line points to power for the
/// well pump or a plan to haul water instead of more tank space.
///
/// `pump`: the household is on a well and pump power actually exists: it owns a generator and the
/// interlock or transfer switch that connects it to the pump (a generator merely planned does not
/// count, round-2 review P-02). The animals' stored water then only has to last until the
/// generator runs the pump (`livestock_pump_bridge_days`, 3, an estimate), and two weeks in stock
/// tanks becomes an alternative ([`livestock_water_stored`]). `drought`: a drought drives the
/// no-water target, and the line says that hauling water in is the answer to a dry well. Rule
/// `livestock_water`.
pub fn livestock_water(days: f64, large_animals: u8, pump: bool, drought: bool) -> Option<Sizing> {
    if large_animals == 0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let l_each = b.k(keys::WATER_LIVESTOCK_L_DAY);
    let (lo, hi) = b.range(keys::WATER_LIVESTOCK_L_DAY);
    b.cite("aspca_disaster_prep");
    // Only the limit that binds stands behind the line.
    let bridge = constants().value(keys::LIVESTOCK_PUMP_BRIDGE_DAYS);
    let bridged = pump && days > bridge;
    let capped = !bridged && days > constants().value(keys::WATER_STORED_CAP_DAYS);
    let stored_days = if bridged {
        b.cite(crate::constants::PRIOR_SOURCE);
        b.k(keys::LIVESTOCK_PUMP_BRIDGE_DAYS)
    } else if capped {
        b.cite(crate::constants::PRIOR_SOURCE);
        b.k(keys::WATER_STORED_CAP_DAYS)
    } else {
        days
    };
    let n = f64::from(large_animals);
    let litres = n * l_each * stored_days;
    let gal = litres / L_PER_GAL;
    let mut text = format!(
        "{} × {} L a day ({} to {}) × {} = {} L, about {}. Fill tubs or stock tanks before a storm or an outage.",
        count(n, "large animal", "large animals"),
        num(l_each, 0),
        num(lo, 0),
        num(hi, 0),
        fmt_days(stored_days),
        num(litres, 0),
        gallons(super::round_quantity("gallon", gal))
    );
    if bridged {
        text.push_str(&format!(
            " Your target is {}; store the first {}, until the generator runs the well pump, and in a longer power cut let it keep the pump going (see the generator line) instead of storing weeks of water.",
            fmt_days(days),
            fmt_days(stored_days)
        ));
        if drought {
            text.push_str(
                " A drought that lowers the well is different: plan to haul water in, and a stock tank holds what you haul.",
            );
        }
    } else if capped {
        text.push_str(&format!(
            " Your target is {}; store the first {}, as for people, instead of buying more tank space. A power cut longer than that needs a generator that can start the well pump, connected through an interlock or transfer switch an electrician installs; in a drought that lowers the well, haul water in, and a stock tank holds what you haul.",
            fmt_days(days),
            fmt_days(stored_days)
        ));
    }
    Some(
        Sizing::new(
            &b,
            "livestock_water",
            "livestock_water",
            gal,
            "gallon",
            Per::Pet,
            text,
        )
        .per_day(stored_days, n * l_each / L_PER_GAL),
    )
}

/// Two weeks of stored water for large animals, as an alternative to running the well pump on a
/// generator (see [`livestock_water`]): 25 L a day per animal × min(target, 14) days. No catalogue
/// item is sized by it; it says what storing instead of pumping would take. Rule
/// `livestock_water_stored`.
pub fn livestock_water_stored(days: f64, large_animals: u8) -> Option<Sizing> {
    if large_animals == 0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let l_each = b.k(keys::WATER_LIVESTOCK_L_DAY);
    b.cite("aspca_disaster_prep");
    let capped = days > constants().value(keys::WATER_STORED_CAP_DAYS);
    let stored_days = if capped {
        b.cite(crate::constants::PRIOR_SOURCE);
        b.k(keys::WATER_STORED_CAP_DAYS)
    } else {
        days
    };
    let n = f64::from(large_animals);
    let litres = n * l_each * stored_days;
    let gal = litres / L_PER_GAL;
    let text = format!(
        "Instead of relying on the generator, you could store {} for the animals in stock tanks: {} × {} L a day × {} = {} L, about {} of tank space. A stock tank also holds water you haul in.",
        fmt_days(stored_days),
        count(n, "large animal", "large animals"),
        num(l_each, 0),
        fmt_days(stored_days),
        num(litres, 0),
        gallons(super::round_quantity("gallon", gal))
    );
    Some(
        Sizing::new(
            &b,
            "livestock_water_stored",
            "livestock_water",
            gal,
            "gallon",
            Per::Pet,
            text,
        )
        .per_day(stored_days, n * l_each / L_PER_GAL),
    )
}

/// Whether the housing is a house with a roof and downspouts of its own.
fn is_house(housing: &Housing) -> bool {
    matches!(
        housing.kind,
        HousingKind::Detached
            | HousingKind::Rowhouse
            | HousingKind::MobileHome
            | HousingKind::RuralProperty
    )
}

/// What the rain line is: a need (the raw-water source for the filter), an optional line (washing
/// and flushing only, where the state does not let a household drink rainwater, or no state rule is
/// known), or a note (the driest months bring too little rain for barrels to help).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RainKind {
    /// The raw-water source for the filter beyond the stored days.
    Need,
    /// For washing and flushing only, or where the state's rule is not known.
    Optional,
    /// Barrels would not refill in the driest months.
    Note,
}

/// Rain barrels as the raw-water source for a long no-water target (round-2 review P-03; item
/// N-04): for a house, a target longer than the 14 stored days, no drought behind it (a drought
/// brings no rain), and no raw-water source already (a well, or one the household named). Barrels
/// are sized so the rain of the state's driest three months between April and October (NOAA
/// nClimDiv normals; barrels freeze in winter) on the roof area that drains to each downspout
/// (`rain_catchment_sqft_per_barrel`, an estimate) brings in the household's monthly water, from 1
/// to `rain_barrels_max` (4) and within the state's limit; if even that many would bring in less
/// than `rain_dry_season_min_share` (a quarter) of it, the line is a note that points to hauling
/// instead. The state's rule comes from NCSL's map: where it does not let a household drink
/// rainwater, the barrels are optional, for washing and flushing. Returns the sizing and its kind.
/// Rule `rain_catchment_units`.
pub fn rain_catchment_units(
    target_days: f64,
    per_day_gal: f64,
    housing: &Housing,
    drought: bool,
    state_fips: Option<u8>,
) -> Option<(Sizing, RainKind)> {
    let cap = constants().value(keys::WATER_STORED_CAP_DAYS);
    let named = housing
        .raw_water_source
        .is_some_and(|s| s != RawWaterSource::None);
    if !is_house(housing)
        || !(target_days > cap)
        || drought
        || named
        || housing.water == WaterSource::Well
        || per_day_gal <= 0.0
    {
        return None;
    }
    let mut b = Basis::new();
    let barrel = b.k(keys::RAIN_BARREL_GAL);
    let area = b.k(keys::RAIN_CATCHMENT_SQFT_PER_BARREL);
    let most = b.k(keys::RAIN_BARRELS_MAX);
    b.cite("cdc_water_disinfection");
    let per_inch = area * GAL_PER_SQFT_INCH;
    let month_need = per_day_gal * DAYS_PER_MONTH;
    let row = state_fips.and_then(|s| b.rain_row(s));
    let allowed_barrels = row.map_or(most, |r| {
        let by_count = r.max_barrels.unwrap_or(most);
        let by_volume = r.max_gal.map_or(most, |g| (g / barrel).floor().max(1.0));
        most.min(by_count).min(by_volume)
    });
    let mut math = vec![format!(
        "{} sq ft × {:.3} gal per sq ft per inch = {} gal an inch",
        num(area, 0),
        GAL_PER_SQFT_INCH,
        num(per_inch, 1)
    )];
    // Barrels, and the rain each brings in a dry month, where the state's figure is known.
    let (barrels, dry) = match row.and_then(|r| r.dry_in) {
        Some(dry_in) => {
            let month_rain = dry_in / 3.0;
            let per_barrel = per_inch * month_rain;
            let wanted = ceil_count(month_need / per_barrel.max(f64::MIN_POSITIVE));
            let n = wanted.clamp(1.0, allowed_barrels);
            math.push(format!(
                "driest 3 months {} in ÷ 3 = {} in a month; {} gal a month per barrel; need {} gal a month; barrels = min(max(1, ceil({} ÷ {})), {}) = {}",
                num(dry_in, 2),
                num(month_rain, 2),
                num(per_barrel, 1),
                num(month_need, 1),
                num(month_need, 1),
                num(per_barrel, 1),
                num(allowed_barrels, 0),
                num(n, 0)
            ));
            (n, Some((dry_in, per_barrel)))
        }
        None => (1.0, None),
    };
    let state = row.map_or("your state", |r| r.state.as_str());
    let too_dry = dry.is_some_and(|(_, per_barrel)| {
        let share = constants().value(keys::RAIN_DRY_SEASON_MIN_SHARE);
        allowed_barrels * per_barrel < share * month_need
    });
    if too_dry {
        b.k(keys::RAIN_DRY_SEASON_MIN_SHARE);
        let (dry_in, _) = dry.unwrap_or((0.0, 0.0));
        let text = format!(
            "Rain barrels will not carry you through the dry season: {state}'s driest three months between April and October bring only about {} inches of rain, too little to refill them. Plan to carry water from a distribution point or a neighbour's well instead (see the carriers line).",
            num(dry_in, 1)
        );
        let s = Sizing::new(
            &b,
            "rain_catchment_units",
            "raw_water_source",
            1.0,
            "barrel",
            Per::Household,
            text,
        )
        .math(math);
        return Some((s, RainKind::Note));
    }
    let drinking = row.is_some_and(|r| r.drinking);
    let mut text = format!(
        "{} with a screened lid and a downspout diverter, about {} gallons each: an inch of rain on the roughly {} square feet of roof that drains to one downspout gives about {} gallons",
        count(barrels, "rain barrel", "rain barrels"),
        num(barrel, 0),
        num(area, 0),
        num(per_inch, 0)
    );
    match dry {
        Some((dry_in, per_barrel)) => text.push_str(&format!(
            ", and even {state}'s driest three months between April and October bring about {} inches, about {} a month for each barrel against the {} your household uses.",
            num(dry_in, 1),
            gallons(super::round_quantity("gallon", per_barrel)),
            gallons(super::round_quantity("gallon", month_need))
        )),
        None => text.push('.'),
    }
    let kind = if drinking {
        text.push_str(" This is the raw-water source for your filter: filter the rain, then disinfect it before drinking.");
        RainKind::Need
    } else {
        match row.and_then(|r| r.note.as_deref()) {
            Some(rule) => text.push_str(&format!(
                " {state} limits rainwater to {rule}, so count it for washing and flushing, not drinking, unless you get the permit it asks for."
            )),
            None => text.push_str(" Check your state's and town's rules on catching rainwater before you count on drinking it."),
        }
        RainKind::Optional
    };
    if row.is_some_and(|r| r.max_gal.is_some() || r.max_barrels.is_some()) {
        text.push_str(" The count stays within your state's limit.");
    }
    let s = Sizing::new(
        &b,
        "rain_catchment_units",
        "raw_water_source",
        barrels,
        "barrel",
        Per::Household,
        text,
    )
    .math(math);
    Some((s, kind))
}

/// Water carriers for hauling (round-2 review P-03; item N-05): two 5-gallon carriers, four for a
/// household of four or more (estimates). A need when the no-water target is longer than the 14
/// stored days (the rest comes from a distribution point, a neighbour's well or a spring, as it did
/// for weeks after Hurricane Helene in Asheville); an option for a well or a tall building with a
/// shorter target. Returns the sizing and whether it is a need. Rule `water_carriers`.
pub fn water_carriers(
    target_days: f64,
    people: usize,
    housing: &Housing,
) -> Option<(Sizing, bool)> {
    let cap = constants().value(keys::WATER_STORED_CAP_DAYS);
    let long = target_days > cap;
    let well = housing.water == WaterSource::Well;
    let high_rise = housing.kind == HousingKind::ApartmentHighRise;
    if target_days <= 0.0 || !(long || well || high_rise) {
        return None;
    }
    let mut b = Basis::new();
    let each = b.k(keys::WATER_CARRIER_GAL);
    let q = if people >= 4 {
        b.k(keys::WATER_CARRIERS_LARGE_HOUSEHOLD)
    } else {
        b.k(keys::WATER_CARRIERS_PER_HOUSEHOLD)
    };
    let pounds = each * WATER_LB_PER_GAL;
    if long {
        b.cite("epa_asheville_boil_notice_2024");
    }
    let why = if long {
        format!(
            "your target of {} runs past the {} of water you store, and the rest has to come from somewhere",
            fmt_days(target_days),
            fmt_days(cap)
        )
    } else if high_rise {
        "in a tall building the pumps stop with the power, and water may have to come up the stairs"
            .to_owned()
    } else {
        "a well stops with the power, and water may have to come from somewhere else".to_owned()
    };
    let text = format!(
        "{} of about {} gallons each for hauling water from a distribution point, a neighbour's well or a spring, because {why}. A full one weighs about {} pounds, so choose a size you can carry, with a good handle.",
        count(q, "water carrier", "water carriers"),
        num(each, 0),
        num(pounds, 0)
    );
    Some((
        Sizing::new(
            &b,
            "water_carriers",
            "water_carrier",
            q,
            "carrier",
            Per::Household,
            text,
        ),
        long,
    ))
}

/// A haulable tote for large animals' water (round-2 review P-02, P-03; item N-06): one food-grade
/// tote of about 275 gallons that fits a pickup, when there are horses or livestock and a drought
/// drives the no-water target (a dry well is met by hauling) or the target runs past the 14 stored
/// days. Its item class is the animals' water, so it is part of the same cover as the stock tank.
/// Rule `livestock_haul_tank`.
pub fn livestock_haul_tank(days: f64, large_animals: u8, drought: bool) -> Option<Sizing> {
    let cap = constants().value(keys::WATER_STORED_CAP_DAYS);
    if large_animals == 0 || days <= 0.0 || !(drought || days > cap) {
        return None;
    }
    let mut b = Basis::new();
    let tote = b.k(keys::LIVESTOCK_HAUL_TANK_GAL);
    let l_each = b.k(keys::WATER_LIVESTOCK_L_DAY);
    b.cite("aspca_disaster_prep");
    let n = f64::from(large_animals);
    let per_day = n * l_each / L_PER_GAL;
    let why = if drought {
        "when a drought dries the well"
    } else {
        "when the power or the water stays off longer than the stored water lasts"
    };
    let text = format!(
        "1 food-grade tote of about {} gallons that fits a pickup, to haul water {why}: the {} drink about {} a day, so one load lasts about {}.",
        num(tote, 0),
        count(n, "large animal", "large animals"),
        gallons(super::round_quantity("gallon", per_day)),
        fmt_days(round_dp_1(tote / per_day))
    );
    Some(Sizing::new(
        &b,
        "livestock_haul_tank",
        "livestock_water",
        1.0,
        "tote",
        Per::Household,
        text,
    ))
}

/// A hand pump for the well (round-2 review P-02, P-03; item N-07): optional, for a household on a
/// well with a no-water target of more than `hand_pump_min_days` (30, an estimate), as the research
/// flagged for Coos Bay (risk-model §9.6): it draws water with no power where the water level is not
/// too deep. Rule `well_hand_pump`.
pub fn well_hand_pump(days: f64, housing: &Housing) -> Option<Sizing> {
    let min = constants().value(keys::HAND_PUMP_MIN_DAYS);
    if housing.water != WaterSource::Well || !(days > min) {
        return None;
    }
    let mut b = Basis::new();
    b.k(keys::HAND_PUMP_MIN_DAYS);
    let text = format!(
        "Optional for a {} no-water target: a hand pump fitted to your well draws water with no power at all, where the water level is not too deep for it; deep wells need a costlier pump, so ask a well contractor for a quote.",
        crate::format::day_adjective(days)
    );
    Some(Sizing::new(
        &b,
        "well_hand_pump",
        "well_hand_pump",
        1.0,
        "pump",
        Per::Household,
        text,
    ))
}

/// Pounds in a gallon of water (8.34, a physical constant for water near room temperature).
const WATER_LB_PER_GAL: f64 = 8.34;

fn round_dp_1(x: f64) -> f64 {
    crate::format::round_dp(x, 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    fn philly() -> rr_types::PlanInput {
        fixtures::get("philadelphia-renters-4").unwrap()
    }

    #[test]
    fn philadelphia_three_days_basic_is_twelve_to_thirteen_gallons() {
        let p = philly();
        let s = water_gallons(3.0, &p.people, &p.pets, WaterLevel::Basic, false, 0.0);
        // 4 people × 1 gal × 3 days = 12, plus a 40 lb dog at 1 oz/lb/day = 0.3125 gal/day × 3.
        assert_eq!(s.quantity, 12.9);
        assert_eq!(s.per_day, Some(4.3125));
        assert!(s.plain.starts_with("4 people × 1 gallon a day × 3 days = 12 gallons, plus 0.9 gallons for the dog: about 12.9 gallons."), "{}", s.plain);
        assert!(s.plain.contains("every 6 months"));
        for id in [
            "ready_gov_water",
            "cdc_water_storage",
            "petmd_dog_water",
            "rr_expert_prior",
        ] {
            assert!(
                s.citations.iter().any(|c| c == id),
                "missing {id}: {:?}",
                s.citations
            );
        }
        assert!(s.prior, "the 40 lb dog is an estimate");
    }

    #[test]
    fn levels_and_heat_follow_the_research() {
        let p = philly();
        let none = Pets::default();
        let per_person = |level, hot| {
            let s = water_gallons(1.0, &p.people[..1], &none, level, hot, 0.0);
            s.per_day.unwrap()
        };
        assert_eq!(per_person(WaterLevel::Survival, false), 0.8);
        assert_eq!(per_person(WaterLevel::Basic, false), 1.0);
        assert_eq!(per_person(WaterLevel::Comfortable, false), 4.0);
        // Hot: the drinking share doubles (0.75 → 1.5), sanitation stays 0.25: 1.75 (research §1.2).
        assert_eq!(per_person(WaterLevel::Basic, true), 1.75);
        assert_eq!(per_person(WaterLevel::Survival, true), 1.6);
        assert_eq!(per_person(WaterLevel::Comfortable, true), 4.8);
    }

    #[test]
    fn add_ons_match_research_1_6() {
        let p = fixtures::get("sugar-land-ev-household-3").unwrap();
        let mut b = Basis::new();
        let d = daily(&mut b, &p.people, &p.pets, WaterLevel::Basic, false);
        assert_eq!(d.formula_infants, 1);
        assert_eq!(d.formula_gal_each, 0.25);
        assert_eq!(d.nursing, 0);
        // 3 people + formula 0.25 + dog 0.3125
        assert!((d.total_gal() - 3.5625).abs() < 1e-12);
        let hays = fixtures::get("hays-kansas-farm-5").unwrap();
        let mut b = Basis::new();
        let d = daily(&mut b, &hays.people, &hays.pets, WaterLevel::Basic, false);
        assert_eq!(d.nursing, 1);
        // 5 people + nursing 0.29 + 2 dogs × 0.3125 + 3 cats × 10 × 0.8 / 128
        assert!((d.total_gal() - (5.0 + 0.29 + 0.625 + 0.1875)).abs() < 1e-12);
    }

    #[test]
    fn dehydrated_food_adds_six_tenths_per_person_day() {
        let p = philly();
        let none = Pets::default();
        let a = water_gallons(3.0, &p.people, &none, WaterLevel::Basic, false, 0.0);
        let b = water_gallons(3.0, &p.people, &none, WaterLevel::Basic, false, 12.0);
        assert!(
            (b.quantity - a.quantity - 7.2).abs() < 0.051,
            "{} vs {}",
            a.quantity,
            b.quantity
        );
        assert!(b.prior);
    }

    #[test]
    fn coos_bay_fifty_days_prefers_a_filter_to_a_hundred_gallons() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        let (stored, treat) = water_storage(
            50.0,
            &p.people,
            &p.pets,
            WaterLevel::Basic,
            false,
            RawSource::Well,
        );
        let treat = treat.expect("a treatment line beyond 14 days");
        // The filter counts only with a raw-water source, and the line names it: their well.
        assert!(
            treat.plain.contains("water from your well"),
            "{}",
            treat.plain
        );
        assert!(
            treat.plain.contains("interlock or transfer switch"),
            "{}",
            treat.plain
        );
        // Town water with no source named: a filter adds nothing, and the line says to carry water.
        let (_, town) = water_storage(
            50.0,
            &p.people,
            &p.pets,
            WaterLevel::Basic,
            false,
            RawSource::None,
        );
        let town = town.unwrap();
        assert!(
            town.plain.contains("a filter adds nothing yet"),
            "{}",
            town.plain
        );
        assert!(town.plain.contains("carriers line"), "{}", town.plain);
        // A planned rain barrel, or a source the household named, is named in the line.
        let (_, barrel) = water_storage(
            50.0,
            &p.people,
            &p.pets,
            WaterLevel::Basic,
            false,
            RawSource::PlannedBarrel,
        );
        assert!(barrel.unwrap().plain.contains("rain line"));
        let (_, creek) = water_storage(
            50.0,
            &p.people,
            &p.pets,
            WaterLevel::Basic,
            false,
            RawSource::Named(RawWaterSource::SurfaceNearby),
        );
        assert!(creek.unwrap().plain.contains("stream, river or pond"));
        // 2 people + 2 dogs × 0.3125 + 1 cat × 0.0625 = 2.6875 gal/day
        assert_eq!(stored.quantity, 37.6, "14 days stored");
        assert!(stored.quantity < 100.0);
        assert_eq!(treat.quantity, 96.8, "36 days × 2.6875");
        assert_eq!(treat.per, Per::Household);
        assert_eq!(treat.rule, "water_treatment_capacity");
        assert!(stored.plain.contains("134.4 gallons"), "{}", stored.plain);
        assert!(treat.plain.contains("filter"));
        assert!(
            treat
                .citations
                .iter()
                .any(|c| c == "byu_longer_term_storage_2019")
        );
        assert!(
            treat
                .citations
                .iter()
                .any(|c| c == "epa_emergency_disinfection")
        );
        // At or under the cap there is no treatment line.
        let (short, none) = water_storage(
            14.0,
            &p.people,
            &p.pets,
            WaterLevel::Basic,
            false,
            RawSource::Well,
        );
        assert!(none.is_none());
        assert_eq!(short.quantity, 37.6);
    }

    #[test]
    fn boil_notice_treats_the_drinking_share() {
        let p = philly();
        let s = water_treatment_boil(4.0, &p.people, &p.pets, false);
        // 4 × 0.75 + dog 0.3125 = 3.3125 gal/day × 4 = 13.25
        assert_eq!(s.quantity, 13.3);
        assert!(
            s.plain.contains("1 minute")
                && s.plain.contains("8 drops")
                && s.plain.contains("6 drops")
        );
        assert!(s.plain.contains("5,000 feet"));
        let fuel = boil_fuel(s.quantity);
        // 13.3 gal × 3.785 = 50.3 L ÷ 27.5 = 1.8 lb
        assert_eq!(fuel.quantity, 1.8);
        assert!(fuel.prior);
    }

    #[test]
    fn livestock_uses_sphere_twenty_five_litres() {
        let s = livestock_water(3.0, 12, false, false).unwrap();
        // 12 × 25 × 3 = 900 L = 237.8 gal
        assert_eq!(s.quantity, 237.8);
        assert_eq!(s.citations[0], "sphere_2018");
        assert!(!s.prior && !s.plain.contains("well pump"));
        assert!(livestock_water(3.0, 0, false, false).is_none());
    }

    #[test]
    fn livestock_water_is_capped_at_the_stored_days() {
        // 12 large animals × 25 L × min(target, 14) days: a 60-day target stores 14 days,
        // 4,200 L = 1,109.5 gal (it was 18,000 L, 4,755 gal).
        let s = livestock_water(60.0, 12, false, false).unwrap();
        assert_eq!(s.quantity, 1109.5);
        assert_eq!(s.days, Some(14.0));
        assert_eq!(s.per_day, Some(12.0 * 25.0 / L_PER_GAL));
        assert!(s.plain.contains("× 14 days = 4,200 L"), "{}", s.plain);
        assert!(s.plain.contains("well pump") && s.plain.contains("haul water"));
        assert!(s.prior, "the human cap applied to animals is an estimate");
        assert!(
            s.citations
                .iter()
                .any(|c| c == "byu_longer_term_storage_2019")
        );
        // Exactly 14 days is not capped; the cap never lowers a shorter target.
        let at = livestock_water(14.0, 12, false, false).unwrap();
        assert_eq!(at.quantity, s.quantity);
        assert!(!at.prior && !at.plain.contains("well pump"));
    }

    #[test]
    fn livestock_water_bridges_three_days_when_a_generator_runs_the_pump() {
        // 12 × 25 L × 3 days = 900 L = 237.8 gal, instead of 14 days (1,109.5 gal).
        let s = livestock_water(60.0, 12, true, false).unwrap();
        assert_eq!(s.quantity, 237.8);
        assert_eq!(s.days, Some(3.0));
        assert!(s.prior, "the three-day bridge is an estimate");
        assert!(
            s.plain.contains("until the generator runs the well pump"),
            "{}",
            s.plain
        );
        assert!(
            !s.plain.contains("haul water"),
            "no drought, no hauling: {}",
            s.plain
        );
        assert!(
            !s.citations
                .iter()
                .any(|c| c == "byu_longer_term_storage_2019")
        );
        let dry = livestock_water(60.0, 12, true, true).unwrap();
        assert!(dry.plain.contains("haul water in"), "{}", dry.plain);
        // A target the bridge already covers is stored whole, with no generator sentence.
        let short = livestock_water(2.0, 12, true, true).unwrap();
        assert_eq!(short.days, Some(2.0));
        assert!(!short.prior && !short.plain.contains("generator"));
        // The alternative: two weeks in stock tanks.
        let alt = livestock_water_stored(60.0, 12).unwrap();
        assert_eq!(alt.quantity, 1109.5);
        assert_eq!(alt.rule, "livestock_water_stored");
        assert!(livestock_water_stored(60.0, 0).is_none());
    }

    #[test]
    fn reused_bottles_cite_only_what_their_amount_uses() {
        let p = philly();
        let r = water_reused_bottles(3.0, &p.people, &p.pets, WaterLevel::Basic, false);
        // 3 days would be 12.9 gallons; a household can gather about 6, so the cap is the amount
        // and the dog's water (PetMD, the 40 lb estimate) is not behind the line.
        assert_eq!(r.quantity, 6.0);
        assert!(r.prior && r.plain.contains("milk jugs"));
        assert!(r.plain.contains("the most a household can usually gather"));
        for id in ["petmd_dog_water", "merck_vet_maintenance_fluids"] {
            assert!(!r.citations.iter().any(|c| c == id), "{id}");
        }
        assert!(r.citations.iter().any(|c| c == "rr_research_risk_model"));
        // One person: 3 days is 3 gallons, under the cap, so the water sources stand behind it
        // and the cap does not.
        let none = Pets::default();
        let one = water_reused_bottles(3.0, &p.people[..1], &none, WaterLevel::Basic, false);
        assert_eq!(one.quantity, 3.0);
        assert!(
            one.plain.contains("all 3 days of your water"),
            "{}",
            one.plain
        );
        // Round-2 review P-13: the whole stored target, not three days, up to the 6-gallon cap.
        // One person, a 5-day target: 5 gallons, free.
        let five = water_reused_bottles(5.0, &p.people[..1], &none, WaterLevel::Basic, false);
        assert_eq!(five.quantity, 5.0);
        let week = water_reused_bottles(7.0, &p.people[..1], &none, WaterLevel::Basic, false);
        assert_eq!(week.quantity, 6.0);
        assert!(!one.prior, "{:?}", one.citations);
        assert!(!one.citations.iter().any(|c| c == "rr_research_risk_model"));
        assert!(one.citations.iter().any(|c| c == "ready_gov_water"));
    }

    #[test]
    fn one_bottle_of_bleach_whatever_the_target() {
        let s = bleach_bottles();
        assert_eq!(s.quantity, 1.0);
        assert_eq!(s.unit, "bottle");
        assert!(
            s.plain
                .starts_with("1 bottle of plain unscented bleach (5 to 9 %")
        );
        // 3,785 mL ÷ 128 oz ÷ 0.5 mL a gallon: about 60 gallons an ounce, 30 when cloudy.
        assert!(
            s.plain.contains("about 60 gallons of clear water (30 if"),
            "{}",
            s.plain
        );
        assert!(s.plain.contains("thousands of gallons"));
        assert!(s.plain.contains("every 6 months"));
        assert!(s.prior, "the six-month replacement is an estimate");
        for id in [
            "cdc_water_storage",
            "epa_emergency_disinfection",
            "cdc_water_disinfection",
            "cdc_bleach_disinfecting",
            "rr_expert_prior",
        ] {
            assert!(s.citations.iter().any(|c| c == id), "missing {id}");
        }
    }

    #[test]
    fn the_boil_line_cites_the_drinking_share_only() {
        let p = philly();
        let none = Pets::default();
        let s = water_treatment_boil(3.0, &p.people, &none, false);
        // 4 × 0.75 gal (Ready.gov); the basic gallon's other source (CDC storage) is not used.
        assert_eq!(s.quantity, 9.0);
        assert!(s.citations.iter().any(|c| c == "ready_gov_water"));
        assert!(!s.citations.iter().any(|c| c == "cdc_water_storage"));
        // Stored water at the comfortable level uses Sphere's 15 L, not the survival drinking
        // amount (WHO), unless it is hot.
        let c = water_gallons(3.0, &p.people, &none, WaterLevel::Comfortable, false, 0.0);
        assert!(!c.citations.iter().any(|x| x == "who_wedc_tn9"));
        let hot = water_gallons(3.0, &p.people, &none, WaterLevel::Comfortable, true, 0.0);
        assert!(hot.citations.iter().any(|x| x == "who_wedc_tn9"));
    }

    /// Round-2 review P-03: rain barrels as the raw-water source, sized to the state's driest
    /// months and within its rules.
    #[test]
    fn rain_barrels_follow_the_state_rain_and_rules() {
        let mut house = fixtures::get("sugar-land-ev-household-3").unwrap().housing;
        house.raw_water_source = None;
        // Texas (48): driest April-October quarter 7.94 in → 2.65 in a month × 155.8 gal an inch
        // = 412 gal a month per barrel against 3.56 × 30 = 107: one barrel, a need.
        let (s, kind) = rain_catchment_units(50.0, 3.5625, &house, false, Some(48)).unwrap();
        assert_eq!(kind, RainKind::Need);
        assert_eq!(s.quantity, 1.0);
        assert!(s.plain.contains("TX's driest three months"), "{}", s.plain);
        assert!(s.citations.iter().any(|c| c == "noaa_nclimdiv"));
        assert!(s.citations.iter().any(|c| c == "ncsl_rainwater"));
        assert!(s.prior, "the roof area is an estimate");
        // California (6) limits rainwater to outdoor uses: optional, and within its 360 gallons.
        let (s, kind) = rain_catchment_units(50.0, 3.5625, &house, false, Some(6)).unwrap();
        assert_eq!(kind, RainKind::Optional);
        assert!(s.plain.contains("360 gallons"), "{}", s.plain);
        // A household that uses 60 gallons a day in California's dry summer (0.64 in over
        // Jul-Sep): four barrels bring in about 133 gallons a month against 1,800, less than a
        // quarter, so the line is a note that points to hauling.
        let (s, kind) = rain_catchment_units(50.0, 60.0, &house, false, Some(6)).unwrap();
        assert_eq!(kind, RainKind::Note);
        assert!(s.plain.contains("carriers line"), "{}", s.plain);
        // Oregon (41): enough rain, but drinking rainwater may need a permit: optional, washing and
        // flushing.
        let (s, kind) = rain_catchment_units(50.0, 2.6875, &house, false, Some(41)).unwrap();
        assert_eq!(kind, RainKind::Optional);
        assert!(s.plain.contains("washing and flushing"), "{}", s.plain);
        // Colorado (8) caps the count at two barrels whatever the need.
        let (s, _) = rain_catchment_units(50.0, 40.0, &house, false, Some(8)).unwrap();
        assert_eq!(s.quantity, 2.0);
        // No barrel for a short target, a drought, an apartment, a well or a named source.
        assert!(rain_catchment_units(14.0, 3.0, &house, false, Some(48)).is_none());
        assert!(rain_catchment_units(50.0, 3.0, &house, true, Some(48)).is_none());
        let mut named = house.clone();
        named.raw_water_source = Some(RawWaterSource::SurfaceNearby);
        assert!(rain_catchment_units(50.0, 3.0, &named, false, Some(48)).is_none());
        let flat = fixtures::get("miami-condo-retiree-1").unwrap().housing;
        assert!(rain_catchment_units(50.0, 3.0, &flat, false, Some(12)).is_none());
        // Without a state figure: one barrel, optional until the rule is known.
        let (s, kind) = rain_catchment_units(50.0, 3.0, &house, false, None).unwrap();
        assert_eq!((s.quantity, kind), (1.0, RainKind::Optional));
    }

    #[test]
    fn carriers_totes_and_hand_pumps() {
        let philly = fixtures::get("philadelphia-renters-4").unwrap().housing;
        // A long target: a need; four people carry four.
        let (c, need) = water_carriers(30.0, 4, &philly).unwrap();
        assert!(need);
        assert_eq!(c.quantity, 4.0);
        assert!(c.plain.contains("42 pounds"), "{}", c.plain);
        assert!(
            c.citations
                .iter()
                .any(|x| x == "epa_asheville_boil_notice_2024")
        );
        assert!(water_carriers(3.0, 4, &philly).is_none());
        // A tall building or a well: optional for a short target.
        let miami = fixtures::get("miami-condo-retiree-1").unwrap().housing;
        let (c, need) = water_carriers(3.0, 1, &miami).unwrap();
        assert!(!need && c.quantity == 2.0);
        assert!(c.plain.contains("up the stairs"));
        // Hays: 12 animals in a drought: one tote, 79.3 gallons a day, about 3.5 days a load.
        let t = livestock_haul_tank(60.0, 12, true).unwrap();
        assert_eq!(t.quantity, 1.0);
        assert_eq!(t.item_class, "livestock_water");
        assert!(t.plain.contains("3.5 days"), "{}", t.plain);
        assert!(livestock_haul_tank(10.0, 12, false).is_none());
        assert!(livestock_haul_tank(60.0, 0, true).is_none());
        // A hand pump is optional past 30 days on a well.
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap().housing;
        assert!(well_hand_pump(50.0, &coos).is_some());
        assert!(well_hand_pump(30.0, &coos).is_none());
        assert!(well_hand_pump(50.0, &philly).is_none());
    }

    #[test]
    fn pet_phrases() {
        assert_eq!(pets_phrase([1, 0, 0]), "the dog");
        assert_eq!(pets_phrase([2, 1, 0]), "the 2 dogs and the cat");
        assert_eq!(pets_phrase([0, 0, 3]), "the 3 small pets");
    }
}
