//! Water: stored water, making water safe, and livestock (research §1).

use rr_types::{Per, Person, Pets, WaterLevel};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::{constants, keys};
use crate::format::{
    L_PER_GAL, OZ_PER_GAL, and_list, count, days as fmt_days, gallons, num, people, round_to,
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
    let treat = treatment_beyond(&mut base, &d, target_days);
    (line, Some(treat))
}

/// Water to make safe after the stored days run out (rule `water_treatment_capacity`).
fn treatment_beyond(b: &mut Basis, d: &WaterDaily, target_days: f64) -> Sizing {
    let cap = b.k(keys::WATER_STORED_CAP_DAYS);
    let extra_days = target_days - cap;
    let q = d.total_gal() * extra_days;
    let how = how_to_treat(b);
    let text = format!(
        "After day {}, for the other {}: make about {} of water safe each day, about {} in all, with a water filter and a source such as rain barrels, a stream or a pond, instead of storing more. Filter it, then {}. If there is no water source near you, store more water instead.",
        num(cap, 1),
        fmt_days(extra_days),
        gallons(d.total_gal()),
        gallons(super::round_quantity("gallon", q)),
        how
    );
    let math = vec![format!(
        "treat = {} gal/day × ({} − {}) days = {} gal",
        num(d.total_gal(), 4),
        num(target_days, 2),
        num(cap, 2),
        num(q, 3)
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

/// Tap water in clean reused drink bottles: three days of the household's water, but no more than
/// it can bottle (6 gallons, an estimate). A free way to meet part of `water_gallons`. The line
/// cites the household's water sources only when three days of water is under the cap (then that
/// is the amount); otherwise the cap is the amount, and only its sources stand behind it. Rule
/// `water_reused_bottles`.
pub fn water_reused_bottles(
    people_list: &[Person],
    pets: &Pets,
    level: WaterLevel,
    hot: bool,
) -> Sizing {
    let mut need = Basis::new();
    let d = daily(&mut need, people_list, pets, level, hot);
    let days = f64::from(rr_types::TierId::H72.days());
    let three_days = d.total_gal() * days;
    let cap = constants().value(keys::REUSED_BOTTLES_MAX_GAL);
    let mut b = Basis::new();
    let (q, amount) = if three_days >= cap {
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
            three_days,
            format!(
                "about {} ({} of your water)",
                gallons(super::round_quantity("gallon", three_days)),
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
        num(days, 0),
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
/// well pump or a plan to haul water instead of more tank space. Rule `livestock_water`.
pub fn livestock_water(days: f64, large_animals: u8) -> Option<Sizing> {
    if large_animals == 0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let l_each = b.k(keys::WATER_LIVESTOCK_L_DAY);
    let (lo, hi) = b.range(keys::WATER_LIVESTOCK_L_DAY);
    b.cite("aspca_disaster_prep");
    // The cap stands behind the line only when it binds.
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
    if capped {
        text.push_str(&format!(
            " Your target is {}; store the first {}, as for people, and beyond that keep the well pump powered (a generator or battery sized for it) or plan to haul water, instead of buying more tank space.",
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
        let (stored, treat) = water_storage(50.0, &p.people, &p.pets, WaterLevel::Basic, false);
        let treat = treat.expect("a treatment line beyond 14 days");
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
        let (short, none) = water_storage(14.0, &p.people, &p.pets, WaterLevel::Basic, false);
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
        let s = livestock_water(3.0, 12).unwrap();
        // 12 × 25 × 3 = 900 L = 237.8 gal
        assert_eq!(s.quantity, 237.8);
        assert_eq!(s.citations[0], "sphere_2018");
        assert!(!s.prior && !s.plain.contains("well pump"));
        assert!(livestock_water(3.0, 0).is_none());
    }

    #[test]
    fn livestock_water_is_capped_at_the_stored_days() {
        // 12 large animals × 25 L × min(target, 14) days: a 60-day target stores 14 days,
        // 4,200 L = 1,109.5 gal (it was 18,000 L, 4,755 gal).
        let s = livestock_water(60.0, 12).unwrap();
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
        let at = livestock_water(14.0, 12).unwrap();
        assert_eq!(at.quantity, s.quantity);
        assert!(!at.prior && !at.plain.contains("well pump"));
    }

    #[test]
    fn reused_bottles_cite_only_what_their_amount_uses() {
        let p = philly();
        let r = water_reused_bottles(&p.people, &p.pets, WaterLevel::Basic, false);
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
        let one = water_reused_bottles(&p.people[..1], &none, WaterLevel::Basic, false);
        assert_eq!(one.quantity, 3.0);
        assert!(one.plain.contains("3 days of your water"), "{}", one.plain);
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

    #[test]
    fn pet_phrases() {
        assert_eq!(pets_phrase([1, 0, 0]), "the dog");
        assert_eq!(pets_phrase([2, 1, 0]), "the 2 dogs and the cat");
        assert_eq!(pets_phrase([0, 0, 3]), "the 3 small pets");
    }
}
