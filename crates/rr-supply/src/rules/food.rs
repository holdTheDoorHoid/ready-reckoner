//! Food: calories by age, cost by approach, long-term staples, infant formula and pet food
//! (research §2).

use rr_types::{AgeBand, CookingFuel, Heating, Housing, HousingKind, Per, Person, Pets};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::{ChildShareTable, DgaTable, constants, keys};
use crate::format::{DAYS_PER_YEAR, ceil_count, count, days as fmt_days, num, people, usd};
use crate::household::is_formula_fed;
use crate::rules::water::pets_phrase;

fn activity_column(activity: &str) -> usize {
    match activity {
        "sedentary" => 0,
        "active" => 2,
        _ => 1,
    }
}

/// kcal a day at one whole-year age: the average of the men's and women's rows (Table A2-1 for
/// age 1, Table A2-2 from age 2).
fn age_kcal(t: &DgaTable, age: u8) -> f64 {
    if age <= 1 {
        let rows = &t.month_rows;
        return rows.iter().map(|r| (r.male + r.female) / 2.0).sum::<f64>() / rows.len() as f64;
    }
    let col = activity_column(&t.activity);
    t.rows
        .iter()
        .find(|r| (r.ages[0]..=r.ages[1]).contains(&age))
        .map_or(0.0, |r| (r.male[col] + r.female[col]) / 2.0)
}

fn band_ages(t: &DgaTable, band: AgeBand) -> Option<[u8; 2]> {
    t.bands.iter().find(|b| b.band == band).map(|b| b.ages)
}

/// Food energy per day for one age band (DGA 2020–2025, moderately active, the average of the
/// men's and women's rows year by year over the band's ages). Babies under 1: 0 (breast milk or
/// formula).
pub fn band_kcal(band: AgeBand) -> f64 {
    band_kcal_with(&mut Basis::new(), band)
}

pub(crate) fn band_kcal_with(b: &mut Basis, band: AgeBand) -> f64 {
    let t = b.dga();
    let Some([lo, hi]) = band_ages(t, band) else {
        return 0.0;
    };
    let ages = lo..=hi;
    let n = f64::from(hi - lo + 1);
    ages.map(|a| age_kcal(t, a)).sum::<f64>() / n
}

/// One person's food energy per day, including the pregnancy or breastfeeding extra.
pub(crate) fn person_kcal(b: &mut Basis, p: &Person) -> f64 {
    let base = band_kcal_with(b, p.age_band);
    if p.pregnant_or_nursing {
        base + b.k(keys::KCAL_PREGNANT_OR_NURSING_ADD)
    } else {
        base
    }
}

/// The household's food energy per day.
pub(crate) fn household_kcal(b: &mut Basis, people_list: &[Person]) -> f64 {
    people_list.iter().map(|p| person_kcal(b, p)).sum()
}

fn band_words(band: AgeBand, n: f64) -> String {
    let (one, many) = match band {
        AgeBand::Infant => ("baby", "babies"),
        AgeBand::Toddler => ("toddler", "toddlers"),
        AgeBand::Child => ("child aged 4 to 12", "children aged 4 to 12"),
        AgeBand::Teen => ("teen", "teens"),
        AgeBand::Adult => ("adult", "adults"),
        AgeBand::Senior => ("person 65 or over", "people 65 or over"),
    };
    count(n, one, many)
}

/// People who eat food (everyone but babies under 1).
fn fed(people_list: &[Person]) -> usize {
    people_list
        .iter()
        .filter(|p| p.age_band != AgeBand::Infant)
        .count()
}

/// Food energy for `days` days (research §2.1). Rule `food_kcal`.
pub fn food_kcal(days: f64, people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let per_day = household_kcal(&mut b, people_list);
    let total = per_day * days;
    let mut parts = Vec::new();
    let order = [
        AgeBand::Adult,
        AgeBand::Senior,
        AgeBand::Teen,
        AgeBand::Child,
        AgeBand::Toddler,
    ];
    for band in order {
        let n = people_list.iter().filter(|p| p.age_band == band).count();
        if n == 0 {
            continue;
        }
        let each = band_kcal_with(&mut b, band);
        parts.push(format!(
            "{} at about {} kcal a day{}",
            band_words(band, n as f64),
            num(each, 0),
            if n > 1 { " each" } else { "" }
        ));
    }
    let nursing = people_list.iter().filter(|p| p.pregnant_or_nursing).count();
    let mut text = crate::format::and_list(&parts);
    if nursing > 0 {
        let add = b.k(keys::KCAL_PREGNANT_OR_NURSING_ADD);
        text.push_str(&format!(
            ", plus {} for pregnancy or breastfeeding",
            num(add, 0)
        ));
    }
    let q = super::round_quantity("kcal", total);
    text = format!(
        "{}: about {} kcal a day × {} = {} kcal. These are moderately active needs; the form does not ask sex, so men's and women's needs are averaged. Count calories, not servings.",
        capitalise(&text),
        num(per_day, 0),
        fmt_days(days),
        num(q, 0)
    );
    if people_list.iter().any(|p| p.age_band == AgeBand::Infant) {
        text.push_str(" Babies under 1 are fed breast milk or formula (see the formula line).");
    }
    let math = vec![format!(
        "per day = {} kcal; × {} days = {} kcal",
        num(per_day, 1),
        num(days, 2),
        num(total, 0)
    )];
    Sizing::new(&b, "food_kcal", "food", total, "kcal", Per::Person, text)
        .per_day(days, per_day)
        .math(math)
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The USDA household-size cost factor for `n` people.
fn size_factor(b: &mut Basis, n: usize) -> f64 {
    let t = b.tfp_size();
    let n = u16::try_from(n).unwrap_or(u16::MAX);
    t.rows
        .iter()
        .find(|r| (r.people[0]..=r.people[1]).contains(&n))
        .map_or(1.0, |r| r.factor)
}

/// What the food need costs three ways (research §2.7): everyday groceries at the USDA Thrifty
/// Food Plan, bulk staples, and freeze-dried meals. Cost comparison lines (rule
/// `food_cost_estimate`, unit usd); no catalogue item uses this rule.
pub fn food_cost_estimates(days: f64, people_list: &[Person]) -> Vec<Sizing> {
    let fed_n = fed(people_list);
    if fed_n == 0 || days <= 0.0 {
        return Vec::new();
    }
    let person_days = fed_n as f64 * days;

    let mut b = Basis::new();
    let pantry = b.k(keys::FOOD_COST_PANTRY_USD_PER_PERSON_DAY);
    // Babies on breast milk or formula do not change grocery economies, so they are not counted.
    let factor = size_factor(&mut b, fed_n);
    let cost = person_days * pantry * factor;
    let adjust = if (factor - 1.0).abs() < 1e-9 {
        String::new()
    } else {
        let pct = (factor - 1.0) * 100.0;
        format!(
            ", {} {} % for a household of {}",
            if pct > 0.0 { "plus" } else { "minus" },
            num(pct.abs(), 0),
            fed_n
        )
    };
    let text = format!(
        "As everyday groceries: about {} for {} person-days (USDA Thrifty Food Plan, {} a person a day{}). Buy what you already eat, keep it in the pantry and use the oldest first.",
        usd(cost),
        num(person_days, 1),
        usd_cents(pantry),
        adjust
    );
    let pantry_line = Sizing::new(
        &b,
        "food_cost_estimate",
        "food_pantry",
        cost,
        "usd",
        Per::Household,
        text,
    )
    .math(vec![format!(
        "{} person-days × ${} × {} = ${}",
        num(person_days, 2),
        num(pantry, 2),
        num(factor, 2),
        num(cost, 2)
    )]);

    let mut b = Basis::new();
    let staples = b.k(keys::FOOD_COST_STAPLES_USD_PER_PERSON_DAY);
    let (lo, hi) = b.range(keys::FOOD_COST_STAPLES_USD_PER_PERSON_DAY);
    let cost = person_days * staples;
    let text = format!(
        "As bulk staples such as wheat, rice and beans: about {} ({} to {} a person a day). Cheapest, but it needs cooking and is short on some vitamins, so it suits months rather than days.",
        usd(cost),
        usd_cents(lo),
        usd_cents(hi)
    );
    let staples_line = Sizing::new(
        &b,
        "food_cost_estimate",
        "food_bulk_staples",
        cost,
        "usd",
        Per::Household,
        text,
    );

    let mut b = Basis::new();
    let kcal = household_kcal(&mut b, people_list) * days;
    let per_2000 = b.k(keys::FOOD_COST_FREEZE_DRIED_USD_PER_2000KCAL);
    let (lo, hi) = b.range(keys::FOOD_COST_FREEZE_DRIED_USD_PER_2000KCAL);
    let water = b.k(keys::WATER_REHYDRATION_GAL_PER_PERSON_DAY);
    let cost = kcal / 2000.0 * per_2000;
    let text = format!(
        "As freeze-dried or dehydrated meals: about {} ({} to {} per 2,000 kcal, so {} to {}). They need about {} gallons of extra water a person a day to prepare.",
        usd(cost),
        usd(lo),
        usd(hi),
        usd(kcal / 2000.0 * lo),
        usd(kcal / 2000.0 * hi),
        num(water, 1)
    );
    let fd_line = Sizing::new(
        &b,
        "food_cost_estimate",
        "food_freeze_dried",
        cost,
        "usd",
        Per::Household,
        text,
    );
    vec![pantry_line, staples_line, fd_line]
}

fn usd_cents(x: f64) -> String {
    format!("${}", num(x, 2))
}

/// How long a retail "30-day" kit really lasts the average person here (research §2.7). Note
/// line (rule `food_kit_check`).
pub fn food_kit_check(people_list: &[Person]) -> Option<Sizing> {
    let fed_n = fed(people_list);
    if fed_n == 0 {
        return None;
    }
    let mut b = Basis::new();
    let avg = household_kcal(&mut b, people_list) / fed_n as f64;
    let label = b.k(keys::RETAIL_KIT_LABEL_DAYS);
    let (lo, hi) = b.range(keys::RETAIL_KIT_KCAL_PER_DAY);
    let days_lo = label * lo / avg;
    let days_hi = label * hi / avg;
    let text = format!(
        "A retail \"{}-day\" food kit supplies about {} to {} kcal a day. For the average person here (about {} kcal a day) one lasts about {} to {} days, not {}. Compare kits by total kcal.",
        num(label, 0),
        num(lo, 0),
        num(hi, 0),
        num(avg, 0),
        num(days_lo, 0),
        num(days_hi, 0),
        num(label, 0)
    );
    Some(Sizing::new(
        &b,
        "food_kit_check",
        "food_kit",
        days_lo,
        "day",
        Per::Person,
        text,
    ))
}

/// Share of an adult's staples amount for one age band, averaged over the band's ages after adding
/// the table's year offset (Ensign 2006). Babies under 1: 0 (nursing babies share their mother's
/// portion; formula-fed babies have their own line).
pub(crate) fn band_share(dga: &DgaTable, cs: &ChildShareTable, band: AgeBand) -> f64 {
    let Some([lo, hi]) = band_ages(dga, band) else {
        return 0.0;
    };
    let n = f64::from(hi - lo + 1);
    (lo..=hi)
        .map(|a| {
            let age = a.saturating_add(cs.age_offset_years);
            cs.rows
                .iter()
                .find(|r| (r.ages[0]..=r.ages[1]).contains(&age))
                .map_or(1.0, |r| r.share)
        })
        .sum::<f64>()
        / n
}

fn group_label(group: &str) -> &'static str {
    match group {
        "grains" => "Grains",
        "legumes" => "Dry beans, peas and lentils",
        "dry_milk" => "Nonfat dry milk",
        "sugar" => "Sugar",
        "fruit_veg" => "Dried fruit and vegetables",
        "salt_leavening" => "Salt and baking soda or powder",
        "oil" => "Cooking oil",
        "vitamin" => "Vitamin C tablets",
        _ => "Staples",
    }
}

fn group_unit(unit: &str) -> &'static str {
    match unit {
        "gallon" => "gallon",
        "tablet" => "tablet",
        _ => "lb",
    }
}

fn unit_words(unit: &str, q: f64) -> String {
    match unit {
        "gallon" => crate::format::gallons(q),
        "tablet" => count(q, "tablet", "tablets"),
        _ => format!("{} lb", num(q, 1)),
    }
}

/// For supplies targets longer than the pantry month: the days beyond it as bulk staples, one
/// line per food group of the BYU 2019 list scaled by the household's adult-equivalents
/// (research §2.3). Alternative lines (rule `long_term_staples`).
pub fn long_term_staples(days: f64, people_list: &[Person]) -> Vec<Sizing> {
    let mut gate = Basis::new();
    let after = gate.k(keys::STAPLES_AFTER_DAYS);
    if days <= after {
        return Vec::new();
    }
    let beyond = days - after;
    let mut out = Vec::new();
    let mut probe = Basis::new();
    let st = probe.staples();
    let mut groups: Vec<&str> = Vec::new();
    for r in st.rows.iter().filter(|r| r.list == st.default_list) {
        if !groups.contains(&r.group.as_str()) {
            groups.push(&r.group);
        }
    }
    for (i, group) in groups.iter().enumerate() {
        let mut b = Basis::new();
        b.cite_all(gate.cites());
        let st = b.staples();
        let dga = b.dga();
        let cs = b.child_share();
        let shares: f64 = people_list
            .iter()
            .map(|p| band_share(dga, cs, p.age_band))
            .sum();
        if shares <= 0.0 {
            return Vec::new();
        }
        let rows: Vec<_> = st
            .rows
            .iter()
            .filter(|r| r.list == st.default_list && r.group == *group)
            .collect();
        let per_adult_year: f64 = rows.iter().map(|r| r.per_adult_year).sum();
        let unit = group_unit(&rows[0].unit);
        let q = per_adult_year * beyond / DAYS_PER_YEAR * shares;
        let names: Vec<String> = rows.iter().map(|r| r.label.clone()).collect();
        let children = people_list.iter().any(|p| {
            matches!(
                p.age_band,
                AgeBand::Infant | AgeBand::Toddler | AgeBand::Child
            )
        });
        let mut text = format!(
            "{}: about {} for days {} to {} as bulk staples ({}), for {}{}.",
            group_label(group),
            unit_words(unit, super::round_quantity(unit, q)),
            num(after + 1.0, 1),
            num(days, 1),
            crate::format::and_list(&names),
            people(people_list.len() as f64),
            if children {
                "; children count as half to nine tenths of an adult by age"
            } else {
                ""
            }
        );
        let shelf = b.shelf_life();
        let life = |key: &str| shelf.rows.iter().find(|r| r.key == key).map(|r| r.years);
        match *group {
            "grains" => {
                if let (Some(w), Some(hot)) = (life("wheat"), life("grains_hot_storage")) {
                    text.push_str(&format!(
                        " Packed dry, low in oxygen and below 75 °F, wheat and white rice keep about {} years; in a hot garage or attic, about {}.",
                        num(w, 0),
                        num(hot, 0)
                    ));
                }
            }
            "legumes" | "dry_milk" | "sugar" => {
                let key = match *group {
                    "legumes" => "legumes",
                    "dry_milk" => "nonfat_dry_milk",
                    _ => "sugar",
                };
                if let Some(y) = life(key) {
                    text.push_str(&format!(
                        " Packed dry and cool, it keeps about {} years.",
                        num(y, 0)
                    ));
                }
            }
            "vitamin" => {
                text.push_str(" Staples can fall short on calcium and vitamins A, C, B12 and E; the BYU list adds vitamin C.");
                for id in &st.note_sources {
                    b.cite(id.as_str());
                }
            }
            _ => {}
        }
        if i == 0 {
            text.push_str(" Never pack moist food without oxygen (botulism).");
        }
        let item = format!("staples_{group}");
        let per_day = per_adult_year / DAYS_PER_YEAR * shares;
        out.push(
            Sizing::new(&b, "long_term_staples", item, q, unit, Per::Person, text)
                .per_day(beyond, per_day),
        );
    }
    out
}

/// Ready-to-feed formula for the first days (round-2 review P-17; item N-11): it needs no water, so
/// it is the safest choice in an emergency (CDC). Up to 32 oz of prepared formula a day (the AAP's
/// most) × `baby_supply_min_days` (3) for each formula-fed baby. Rule `infant_formula_rtf`.
pub fn infant_formula_rtf(people_list: &[Person]) -> Option<Sizing> {
    let n = people_list.iter().filter(|p| is_formula_fed(p)).count() as f64;
    if n == 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per_day = b.k(keys::INFANT_FORMULA_OZ_DAY);
    let days = b.k(keys::BABY_SUPPLY_MIN_DAYS);
    b.cite("cdc_infant_feeding_disaster");
    let q = n * per_day * days;
    let text = format!(
        "Ready-to-feed formula for the first {}: up to {} oz a day for each baby on formula × {} = {} fluid ounces. It needs no water, so it is the safest formula in an emergency; keep the kind your baby already takes, and check the dates every month.",
        fmt_days(days),
        num(per_day, 0),
        count(n, "baby", "babies"),
        num(q, 0)
    );
    Some(
        Sizing::new(
            &b,
            "infant_formula_rtf",
            "infant_formula",
            q,
            "fl oz",
            Per::Person,
            text,
        )
        .per_day(days, n * per_day),
    )
}

/// Powdered formula for the days after the ready-to-feed formula's first three: 32 oz of prepared
/// formula a day at most (AAP), about 5 oz of powder (labels differ), for each formula-fed baby. No
/// line when the target is three days or less. Rule `infant_formula_oz`.
pub fn infant_formula_oz(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let n = people_list.iter().filter(|p| is_formula_fed(p)).count() as f64;
    let rtf_days = constants().value(keys::BABY_SUPPLY_MIN_DAYS);
    if n == 0.0 || days.is_nan() || days <= rtf_days {
        return None;
    }
    let mut b = Basis::new();
    let prepared = b.k(keys::INFANT_FORMULA_OZ_DAY);
    let powder = b.k(keys::FORMULA_POWDER_OZ_PER_DAY);
    let first = b.k(keys::BABY_SUPPLY_MIN_DAYS);
    b.cite("cdc_infant_feeding_disaster");
    let d = days - first;
    let q = n * powder * d;
    let text = format!(
        "Powdered formula for the days after the ready-to-feed formula's first {}: {}: up to {} oz of prepared formula a day, about {} oz of powder (labels differ), × {} = {} oz of powder. Powder needs safe water (counted in the water line). Check the amount every month as the baby grows.",
        fmt_days(first),
        count(n, "baby on formula", "babies on formula"),
        num(prepared, 0),
        num(powder, 0),
        fmt_days(d),
        num(q, 0)
    );
    Some(
        Sizing::new(
            &b,
            "infant_formula_oz",
            "infant_formula",
            q,
            "oz",
            Per::Person,
            text,
        )
        .per_day(d, n * powder),
    )
}

/// A manual pump and nursing pads for each breastfed baby (CDC checklist). Rule
/// `nursing_supplies`.
pub fn nursing_supplies(people_list: &[Person]) -> Option<Sizing> {
    let n = people_list
        .iter()
        .filter(|p| p.age_band == AgeBand::Infant && !is_formula_fed(p))
        .count();
    if n == 0 {
        return None;
    }
    let mut b = Basis::new();
    let pumps = b.k(keys::MANUAL_PUMPS_PER_NURSING_INFANT);
    let (lo, hi) = b.range(keys::NURSING_PAD_BOXES);
    let text = format!(
        "For {}: {} manual breast pump and {} to {} boxes of nursing pads, in case you are apart or the power is out.",
        count(n as f64, "breastfed baby", "breastfed babies"),
        num(pumps, 0),
        num(lo, 0),
        num(hi, 0)
    );
    Some(Sizing::new(
        &b,
        "nursing_supplies",
        "nursing_supplies",
        n as f64,
        "kit",
        Per::Person,
        text,
    ))
}

/// Food for household pets, in pet-days, for the supplies target but at least a week (ASPCA 7–10
/// days; no per-pet calorie figure is published). Rule `pet_food_days`.
pub fn pet_food_days(days: f64, pets: &Pets) -> Option<Sizing> {
    let n = u32::from(pets.dogs) + u32::from(pets.cats) + u32::from(pets.small);
    if n == 0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let min_days = b.k(keys::PET_FOOD_MIN_DAYS);
    b.cite("ready_gov_pets");
    let d = days.max(min_days);
    let q = f64::from(n) * d;
    let text = format!(
        "Food for {} for {} ({} pet-days, at least a week): what {} now, in an airtight, waterproof container.",
        pets_phrase([pets.dogs, pets.cats, pets.small]),
        fmt_days(d),
        num(q, 1),
        if n == 1 { "it eats" } else { "they eat" }
    );
    Some(
        Sizing::new(
            &b,
            "pet_food_days",
            "pet_food",
            q,
            "pet_day",
            Per::Pet,
            text,
        )
        .per_day(d, f64::from(n)),
    )
}

/// Dry pet food in pounds for the supplies target, at least a week: a 40 lb dog 0.7 lb a day, a
/// 10 lb cat 0.15, a small pet 0.05 (estimates; the sizes the water rule assumes). Rule
/// `pet_food_lb`.
pub fn pet_food_lb(days: f64, pets: &Pets) -> Option<Sizing> {
    let n = u32::from(pets.dogs) + u32::from(pets.cats) + u32::from(pets.small);
    if n == 0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let min_days = b.k(keys::PET_FOOD_MIN_DAYS);
    let mut per_day = 0.0;
    let mut parts = Vec::new();
    for (count_of, key, what) in [
        (pets.dogs, keys::DOG_FOOD_LB_PER_DAY, "dog"),
        (pets.cats, keys::CAT_FOOD_LB_PER_DAY, "cat"),
        (pets.small, keys::SMALL_PET_FOOD_LB_PER_DAY, "small pet"),
    ] {
        if count_of > 0 {
            let each = b.k(key);
            per_day += f64::from(count_of) * each;
            parts.push(format!("{} lb a day for each {what}", num(each, 2)));
        }
    }
    b.cite("ready_gov_pets");
    let d = days.max(min_days);
    let q = per_day * d;
    let text = format!(
        "Dry food for {} for {} (at least a week): about {} = {} lb. Feed what the label says for yours, and keep it in an airtight, waterproof container.",
        pets_phrase([pets.dogs, pets.cats, pets.small]),
        fmt_days(d),
        crate::format::and_list(&parts),
        num(super::round_quantity("lb", q), 1)
    );
    Some(Sizing::new(&b, "pet_food_lb", "pet_food", q, "lb", Per::Pet, text).per_day(d, per_day))
}

/// The item class of the cooking lines (a way to cook without power, and its fuel): one part of
/// the supplies bucket, so the stove and its fuel are valued together.
pub const COOKING_CLASS: &str = "cooking";

/// What the cooking line is for this household.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cooking {
    /// A way to cook without power is needed and must be bought: a camp stove and fuel.
    Need,
    /// The home can already cook without power (a wood stove, or a gas range while the gas flows):
    /// a note, nothing to buy.
    Covered,
}

/// A way to cook and boil water without power (round-2 review P-07, REVIEW K1): needed when the
/// can't-get-to-a-store target is at least `cooking_capability_min_days` (14, an estimate; Oregon's
/// 2 Weeks Ready asks households to know how to prepare two weeks of food without electricity or
/// gas), or when a formula-fed baby's powder must be mixed during a boil-water target. A wood stove
/// or a gas range already covers it (a note); otherwise a one-burner camp stove, burning propane where
/// a cold hazard drives the heat-or-cold bucket (butane stops turning to gas near 32 °F; NIST),
/// outdoors only (CDC). Rule `cooking_capability`.
pub fn cooking_capability(
    supplies_days: Option<f64>,
    boil_days: Option<f64>,
    people_list: &[Person],
    housing: &Housing,
    cold: bool,
) -> Option<(Sizing, Cooking)> {
    let min = constants().value(keys::COOKING_CAPABILITY_MIN_DAYS);
    let long = supplies_days.is_some_and(|d| d >= min);
    let formula = people_list.iter().any(is_formula_fed) && boil_days.is_some_and(|d| d > 0.0);
    if !long && !formula {
        return None;
    }
    let mut b = Basis::new();
    if long {
        b.k(keys::COOKING_CAPABILITY_MIN_DAYS);
    }
    if formula {
        b.cite("cdc_infant_feeding_disaster");
    }
    let why = match (long, formula) {
        (true, true) => format!(
            "your plan runs {} without a store, and a baby's powdered formula needs boiled water during a boil-water notice",
            fmt_days(supplies_days.unwrap_or(min))
        ),
        (true, false) => format!(
            "your plan runs {} without a store, longer than food that needs no cooking comfortably covers",
            fmt_days(supplies_days.unwrap_or(min))
        ),
        _ => "a baby's powdered formula needs boiled water during a boil-water notice".to_owned(),
    };
    if housing.heating == Heating::Wood {
        b.cite("oregon_2_weeks_ready");
        let text = format!(
            "A way to cook without power, because {why}: your wood stove can boil water and cook when the power is out. Keep dry wood and a pot that fits on it."
        );
        let s = Sizing::new(
            &b,
            "cooking_capability",
            COOKING_CLASS,
            1.0,
            "stove",
            Per::Household,
            text,
        );
        return Some((s, Cooking::Covered));
    }
    if housing.cooking == Some(CookingFuel::Gas) {
        b.cite("cdc_co_basics");
        b.cite("ready_gov_winter");
        let text = format!(
            "A way to cook without power, because {why}: your gas range can boil water and cook while the gas still flows. If its igniter needs power, light a burner with a long match (check the manual first). Never use the oven or burners to heat the home."
        );
        let s = Sizing::new(
            &b,
            "cooking_capability",
            COOKING_CLASS,
            1.0,
            "stove",
            Per::Household,
            text,
        );
        return Some((s, Cooking::Covered));
    }
    b.cite("cdc_co_basics");
    let fuel_words = if cold {
        let butane = b.k(keys::BUTANE_BOIL_F);
        let propane = b.k(keys::PROPANE_BOIL_F);
        format!(
            "one that burns propane, because winter outages come where you live: butane stops turning to gas near {} °F, propane not until about {} °F",
            num(butane, 0),
            num(propane, 0)
        )
    } else {
        "a propane or butane one-burner stove".to_owned()
    };
    let text = format!(
        "A way to cook and boil water without power, because {why}: a camp stove, {fuel_words}. Use it outdoors only, never indoors or in a garage (carbon monoxide), and keep a lighter or waterproof matches with it."
    );
    let s = Sizing::new(
        &b,
        "cooking_capability",
        COOKING_CLASS,
        1.0,
        "stove",
        Per::Household,
        text,
    );
    Some((s, Cooking::Need))
}

/// Fuel for a camp stove, in pounds: about 0.2 lb a person a day to boil drinking water and heat
/// one meal for the longer of the store and boil-water targets, plus about 0.3 lb per 2,000 kcal of
/// dry staples the plan counts beyond the first month (estimates; round-2 review P-07). Returns the
/// pounds and the staples' share.
fn fuel_lb(b: &mut Basis, fuel_days: f64, people_list: &[Person], staples_kcal: f64) -> (f64, f64) {
    let n = people_list.len() as f64;
    let per_person_day = b.k(keys::COOKING_FUEL_LB_PER_PERSON_DAY);
    let base = n * fuel_days * per_person_day;
    let staples = if staples_kcal > 0.0 {
        staples_kcal / 2000.0 * b.k(keys::STAPLES_FUEL_LB_PER_2000KCAL)
    } else {
        0.0
    };
    (base + staples, staples)
}

/// "about 0.2 lb of fuel a person a day × 4 people × 10 days = 8 lb, plus 1.2 lb to cook the dry
/// staples".
fn fuel_words(people_list: &[Person], fuel_days: f64, total: f64, staples: f64) -> String {
    let n = people_list.len() as f64;
    let per_person_day = constants().value(keys::COOKING_FUEL_LB_PER_PERSON_DAY);
    let base = total - staples;
    let mut s = format!(
        "about {} lb of fuel a person a day × {} × {} = {} lb",
        num(per_person_day, 1),
        count(n, "person", "people"),
        fmt_days(fuel_days),
        num(base, 1)
    );
    if staples > 0.0 {
        s.push_str(&format!(
            ", plus {} lb to cook the dry staples, {} lb in all",
            num(staples, 1),
            num(total, 1)
        ));
    }
    s
}

/// Butane canisters for a camp stove where no cold hazard drives the heat-or-cold bucket: the
/// pounds [`fuel_lb`] gives in 8-ounce canisters, at least two (estimates). A need when the household
/// must buy a way to cook ([`Cooking::Need`]), else optional. Rule `cooking_fuel_canisters`.
pub fn cooking_fuel_canisters(
    fuel_days: f64,
    people_list: &[Person],
    staples_kcal: f64,
) -> Option<Sizing> {
    if people_list.is_empty() || fuel_days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    b.cite("cdc_co_basics");
    let (lb, staples) = fuel_lb(&mut b, fuel_days, people_list, staples_kcal);
    let per_canister = b.k(keys::FUEL_CANISTER_LB);
    let min = b.k(keys::COOKING_FUEL_MIN_CANISTERS);
    let q = ceil_count(lb / per_canister).max(min);
    let text = format!(
        "Fuel for a camp stove: {}, or {} of {} lb each. Use it outdoors only, never indoors (carbon monoxide), and store it cool, away from heat.",
        fuel_words(people_list, fuel_days, lb, staples),
        count(q, "butane canister", "butane canisters"),
        num(per_canister, 1)
    );
    Some(
        Sizing::new(
            &b,
            "cooking_fuel_canisters",
            COOKING_CLASS,
            q,
            "canister",
            Per::Household,
            text,
        )
        .math(vec![format!(
            "max({}, ceil({} lb ÷ {} lb)) = {}",
            num(min, 0),
            num(lb, 2),
            num(per_canister, 2),
            num(q, 0)
        )]),
    )
}

/// One-pound propane cylinders for a camp stove where a cold hazard drives the heat-or-cold bucket
/// (propane works far below freezing; butane does not, NIST): the pounds [`fuel_lb`] gives, in 1 lb
/// cylinders, at least two; no more than two kept inside (the fire-code limit as Lehi states it).
/// A need when the household must buy a way to cook, else optional (round-2 review P-07; item N-08).
/// Rule `propane_cylinders`.
pub fn propane_cylinders(
    fuel_days: f64,
    people_list: &[Person],
    staples_kcal: f64,
) -> Option<Sizing> {
    if people_list.is_empty() || fuel_days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    b.cite("cdc_co_basics");
    let (lb, staples) = fuel_lb(&mut b, fuel_days, people_list, staples_kcal);
    let per_cylinder = b.k(keys::PROPANE_CYLINDER_LB);
    let min = b.k(keys::COOKING_FUEL_MIN_CANISTERS);
    let inside = b.k(keys::PROPANE_SMALL_CYLINDERS_INDOOR_MAX);
    let q = ceil_count(lb / per_cylinder).max(min);
    let text = format!(
        "Fuel for a propane camp stove: {}, or {}. Use the stove outdoors only (carbon monoxide), and store the cylinders outside the living space, with no more than {} inside the home or an attached garage.",
        fuel_words(people_list, fuel_days, lb, staples),
        count(
            q,
            "one-pound propane cylinder",
            "one-pound propane cylinders"
        ),
        num(inside, 0)
    );
    Some(
        Sizing::new(
            &b,
            "propane_cylinders",
            COOKING_CLASS,
            q,
            "cylinder",
            Per::Household,
            text,
        )
        .math(vec![format!(
            "max({}, ceil({} lb ÷ {} lb)) = {}",
            num(min, 0),
            num(lb, 2),
            num(per_cylinder, 2),
            num(q, 0)
        )]),
    )
}

/// A spare filled 20-lb cylinder for an outdoor gas grill (round-2 review P-07; item N-09): the
/// cheapest boiling and cooking fuel for a house that has a grill. The interview does not ask about
/// a grill, so it stays optional. Rule `grill_propane_tank`.
pub fn grill_propane_tank(housing: &Housing) -> Option<Sizing> {
    let house = matches!(
        housing.kind,
        HousingKind::Detached
            | HousingKind::Rowhouse
            | HousingKind::MobileHome
            | HousingKind::RuralProperty
    );
    if !house {
        return None;
    }
    let mut b = Basis::new();
    b.cite("lehi_fuel_storage");
    b.cite("cdc_co_basics");
    let text = "Optional, if you have an outdoor gas grill: a spare filled 20-pound cylinder is the cheapest way to boil water and cook in a long outage. Use the grill outdoors only, away from the house, and keep the cylinder outside, never indoors or in the garage.".to_owned();
    Some(Sizing::new(
        &b,
        "grill_propane_tank",
        COOKING_CLASS,
        1.0,
        "cylinder",
        Per::Household,
        text,
    ))
}

/// The dry staples' food energy the plan counts for a store target: the days beyond the first month
/// (`staples_after_days`, as [`long_term_staples`] counts them) × the household's kcal a day.
pub fn staples_kcal(days: f64, people_list: &[Person]) -> f64 {
    let after = constants().value(keys::STAPLES_AFTER_DAYS);
    if days <= after {
        return 0.0;
    }
    household_kcal(&mut Basis::new(), people_list) * (days - after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn band_values_follow_dga_table_a2_2() {
        // Moderately active, average of men and women, year by year (hand-computed from the table).
        assert!((band_kcal(AgeBand::Toddler) - 3200.0 / 3.0).abs() < 1e-9); // 900, 1000, 1300
        assert!((band_kcal(AgeBand::Child) - 15000.0 / 9.0).abs() < 1e-9);
        assert!((band_kcal(AgeBand::Teen) - 2280.0).abs() < 1e-9);
        assert!((band_kcal(AgeBand::Adult) - 106_300.0 / 47.0).abs() < 1e-9); // 2,261.7
        assert!((band_kcal(AgeBand::Senior) - 22_100.0 / 11.0).abs() < 1e-9); // 2,009.1
        assert_eq!(band_kcal(AgeBand::Infant), 0.0);
    }

    #[test]
    fn philadelphia_ten_days_is_about_82_000_kcal() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        let s = food_kcal(10.0, &p.people);
        // 2 × 2,261.7 + 1,666.7 + 2,009.1 = 8,199.2 kcal a day
        assert_eq!(s.quantity, 82_000.0);
        assert!((s.per_day.unwrap() - 8199.16).abs() < 0.01);
        assert_eq!(s.citations, ["usda_dga_2020_2025"]);
        assert!(
            s.plain.contains("2 adults at about 2,262 kcal a day each"),
            "{}",
            s.plain
        );
        // The household average sits close to Sphere's 2,100 population figure.
        let avg = s.per_day.unwrap() / 4.0;
        assert!((avg - 2100.0).abs() < 100.0, "{avg}");
    }

    #[test]
    fn pregnancy_adds_four_hundred() {
        let hays = fixtures::get("hays-kansas-farm-5").unwrap();
        let mut b = Basis::new();
        let with = household_kcal(&mut b, &hays.people);
        let mut people = hays.people.clone();
        for p in &mut people {
            p.pregnant_or_nursing = false;
        }
        let without = household_kcal(&mut b, &people);
        assert!((with - without - 400.0).abs() < 1e-9);
    }

    #[test]
    fn cost_estimates_use_the_cited_rates() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        let lines = food_cost_estimates(10.0, &p.people);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].quantity, 338.0); // 40 × $8.44 × 1.00
        assert_eq!(lines[1].quantity, 100.0); // 40 × $2.50
        // 81,991.6 kcal ÷ 2,000 × $24 = $983.90
        assert_eq!(lines[2].quantity, 984.0);
        assert!(
            lines
                .iter()
                .all(|l| l.unit == "usd" && l.rule == "food_cost_estimate")
        );
        let single = fixtures::get("chicago-student-zero-budget-1").unwrap();
        let one = food_cost_estimates(10.0, &single.people);
        assert_eq!(one[0].quantity, 101.0); // 10 × 8.44 × 1.20 = 101.28
        assert!(
            one[0].plain.contains("plus 20 % for a household of 1"),
            "{}",
            one[0].plain
        );
    }

    #[test]
    fn retail_kits_last_about_nineteen_days() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        let s = food_kit_check(&p.people).unwrap();
        // 30 × 1,290 ÷ 2,049.8 = 18.9
        assert_eq!(s.quantity, 18.9);
        assert!(s.plain.contains("not 30"));
    }

    #[test]
    fn staples_only_beyond_the_pantry_month() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        assert!(long_term_staples(30.0, &p.people).is_empty());
        let lines = long_term_staples(54.0, &p.people);
        assert_eq!(lines.len(), 8);
        let grains = &lines[0];
        // (132 + 65 + 29 + 21) lb × 24 / 365 × 2 adults = 32.5 lb
        assert_eq!(grains.quantity, 32.5);
        assert_eq!(grains.item_class, "staples_grains");
        let oil = lines
            .iter()
            .find(|l| l.item_class == "staples_oil")
            .unwrap();
        assert_eq!(oil.unit, "gallon");
        assert_eq!(oil.quantity, 0.3); // 2 gal × 24/365 × 2
    }

    #[test]
    fn child_shares_add_a_year_and_average_the_band() {
        let mut b = Basis::new();
        let dga = b.dga();
        let cs = b.child_share();
        assert!((band_share(dga, cs, AgeBand::Toddler) - 1.7 / 3.0).abs() < 1e-12);
        assert!((band_share(dga, cs, AgeBand::Child) - 8.0 / 9.0).abs() < 1e-12);
        assert_eq!(band_share(dga, cs, AgeBand::Adult), 1.0);
        assert_eq!(band_share(dga, cs, AgeBand::Infant), 0.0);
    }

    /// Round-2 review P-17: three days of ready-to-feed formula, then powder for the rest.
    #[test]
    fn formula_and_nursing() {
        let p = fixtures::get("sugar-land-ev-household-3").unwrap();
        let rtf = infant_formula_rtf(&p.people).unwrap();
        assert_eq!((rtf.quantity, rtf.unit), (96.0, "fl oz")); // 32 fl oz × 3 days
        assert!(rtf.plain.contains("needs no water"), "{}", rtf.plain);
        assert!(rtf.citations.iter().any(|c| c == "aap_formula_amounts"));
        let f = infant_formula_oz(7.0, &p.people).unwrap();
        assert_eq!(f.quantity, 20.0); // 5 oz of powder × (7 − 3) days
        assert!(
            infant_formula_oz(3.0, &p.people).is_none(),
            "3 days are ready-to-feed"
        );
        assert!(infant_formula_oz(1.0, &p.people).is_none());
        assert!(nursing_supplies(&p.people).is_none());
        let mut breastfed = p.people.clone();
        breastfed[2].medical.dietary.clear();
        assert!(infant_formula_oz(7.0, &breastfed).is_none());
        assert!(infant_formula_rtf(&breastfed).is_none());
        assert_eq!(nursing_supplies(&breastfed).unwrap().quantity, 1.0);
    }

    /// Round-2 review P-07: no household could cook or boil without power. A long store target (or
    /// formula under a boil notice) needs a way to cook; a wood stove or gas range already is one;
    /// propane where winter comes; staples need fuel to cook.
    #[test]
    fn a_way_to_cook_without_power() {
        let philly = fixtures::get("philadelphia-renters-4").unwrap();
        // 10 days, no baby: not needed.
        assert!(
            cooking_capability(Some(10.0), Some(4.0), &philly.people, &philly.housing, true)
                .is_none()
        );
        // 14 days, no gas range recorded: a propane stove in a cold county.
        let (s, kind) =
            cooking_capability(Some(14.0), None, &philly.people, &philly.housing, true).unwrap();
        assert_eq!(kind, Cooking::Need);
        assert!(
            s.plain.contains("propane") && s.plain.contains("32 °F"),
            "{}",
            s.plain
        );
        assert!(s.citations.iter().any(|c| c == "nist_butane"));
        assert_eq!(s.item_class, COOKING_CLASS);
        // A gas range or a wood stove already covers it.
        let mut gas = philly.housing.clone();
        gas.cooking = Some(CookingFuel::Gas);
        let (s, kind) = cooking_capability(Some(14.0), None, &philly.people, &gas, true).unwrap();
        assert_eq!(kind, Cooking::Covered);
        assert!(s.plain.contains("gas range"), "{}", s.plain);
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
        let (s, kind) =
            cooking_capability(Some(60.0), None, &coos.people, &coos.housing, false).unwrap();
        assert_eq!(kind, Cooking::Covered);
        assert!(s.plain.contains("wood stove"));
        // A formula-fed baby under a boil-water target, even for a short store target.
        let sl = fixtures::get("sugar-land-ev-household-3").unwrap();
        let (s, _) =
            cooking_capability(Some(7.0), Some(3.0), &sl.people, &sl.housing, false).unwrap();
        assert!(s.plain.contains("formula"), "{}", s.plain);
        // Fuel: 2 people × 60 days × 0.2 lb = 24 lb, plus 30 days of staples (4,524 kcal a day ×
        // 30 = 135,700 kcal ÷ 2,000 × 0.3 = 20.4 lb): 45 one-pound cylinders.
        let staples = staples_kcal(60.0, &coos.people);
        assert!((staples - 30.0 * 106_300.0 / 47.0 * 2.0).abs() < 1e-6);
        let p = propane_cylinders(60.0, &coos.people, staples).unwrap();
        assert_eq!(p.quantity, 45.0);
        assert!(p.plain.contains("dry staples"), "{}", p.plain);
        assert!(p.prior);
        assert_eq!(staples_kcal(30.0, &coos.people), 0.0);
        let one = fixtures::get("chicago-student-zero-budget-1").unwrap();
        assert_eq!(
            propane_cylinders(1.0, &one.people, 0.0).unwrap().quantity,
            2.0
        );
        assert!(grill_propane_tank(&philly.housing).is_some());
        assert!(grill_propane_tank(&one.housing).is_none());
    }

    #[test]
    fn pet_food_counts_pet_days() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        let s = pet_food_days(17.0, &p.pets).unwrap();
        assert_eq!(s.quantity, 51.0);
        assert!(s.plain.contains("the 2 dogs and the cat"));
        assert_eq!(
            pet_food_days(3.0, &p.pets).unwrap().quantity,
            21.0,
            "at least a week"
        );
        assert!(pet_food_days(17.0, &Pets::default()).is_none());
    }

    #[test]
    fn pet_food_in_pounds() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        // (2 dogs × 0.7 + 1 cat × 0.15) lb a day × 10 days = 15.5 lb
        let s = pet_food_lb(10.0, &p.pets).unwrap();
        assert_eq!(s.quantity, 15.5);
        assert!(s.prior);
        // One dog for a 3-day target still gets a week: 0.7 × 7 = 4.9 lb.
        let one_dog = Pets {
            dogs: 1,
            ..Pets::default()
        };
        assert_eq!(pet_food_lb(3.0, &one_dog).unwrap().quantity, 4.9);
        assert!(pet_food_lb(3.0, &Pets::default()).is_none());
    }

    #[test]
    fn camp_stove_canisters() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        // 4 × 10 days × 0.2 lb = 8 lb ÷ 0.5 lb = 16 canisters
        let s = cooking_fuel_canisters(10.0, &p.people, 0.0).unwrap();
        assert_eq!(s.quantity, 16.0);
        assert!(s.prior && s.plain.contains("never indoors"));
        let one = fixtures::get("chicago-student-zero-budget-1").unwrap();
        assert_eq!(
            cooking_fuel_canisters(1.0, &one.people, 0.0)
                .unwrap()
                .quantity,
            2.0,
            "at least 2"
        );
    }
}
