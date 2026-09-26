//! Food: calories by age, cost by approach, long-term staples, infant formula and pet food
//! (research §2).

use rr_types::{AgeBand, Per, Person, Pets};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::{ChildShareTable, DgaTable, keys};
use crate::format::{DAYS_PER_YEAR, count, days as fmt_days, num, people, usd};
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

/// Prepared formula for formula-fed babies (AAP maximum of 32 oz a day). Rule `infant_formula`.
pub fn infant_formula(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let n = people_list.iter().filter(|p| is_formula_fed(p)).count();
    if n == 0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let oz = b.k(keys::INFANT_FORMULA_OZ_DAY);
    b.cite("cdc_infant_feeding_disaster");
    b.cite("cdc_infant_emergency_checklist");
    let q = n as f64 * oz * days;
    let text = format!(
        "{} × {} oz of prepared formula a day (the most babies usually drink) × {} = {} oz. Ready-to-feed formula is safest in an emergency; powder needs safe water (counted in the water line). Check the amount every month as the baby grows.",
        count(n as f64, "baby on formula", "babies on formula"),
        num(oz, 0),
        fmt_days(days),
        num(q, 0)
    );
    Some(
        Sizing::new(
            &b,
            "infant_formula",
            "infant_formula",
            q,
            "oz",
            Per::Person,
            text,
        )
        .per_day(days, n as f64 * oz),
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

/// Food for household pets for `days` days, counted in pet-days (no per-pet calorie figure is
/// published). Rule `pet_food`.
pub fn pet_food(days: f64, pets: &Pets) -> Option<Sizing> {
    let n = u32::from(pets.dogs) + u32::from(pets.cats) + u32::from(pets.small);
    if n == 0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    b.cite("ready_gov_pets");
    b.cite("aspca_disaster_prep");
    let q = f64::from(n) * days;
    let text = format!(
        "Food for {} for {} ({} pet-days): what {} now, in an airtight, waterproof container.",
        pets_phrase([pets.dogs, pets.cats, pets.small]),
        fmt_days(days),
        num(q, 1),
        if n == 1 { "it eats" } else { "they eat" }
    );
    Some(
        Sizing::new(&b, "pet_food", "pet_food", q, "pet_day", Per::Pet, text)
            .per_day(days, f64::from(n)),
    )
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
        assert_eq!(s.citations, ["dga_2020_2025"]);
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

    #[test]
    fn formula_and_nursing() {
        let p = fixtures::get("sugar-land-ev-household-3").unwrap();
        let f = infant_formula(7.0, &p.people).unwrap();
        assert_eq!(f.quantity, 224.0);
        assert!(nursing_supplies(&p.people).is_none());
        let mut breastfed = p.people.clone();
        breastfed[2].medical.dietary.clear();
        assert!(infant_formula(7.0, &breastfed).is_none());
        assert_eq!(nursing_supplies(&breastfed).unwrap().quantity, 1.0);
    }

    #[test]
    fn pet_food_counts_pet_days() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        let s = pet_food(17.0, &p.pets).unwrap();
        assert_eq!(s.quantity, 51.0);
        assert!(s.plain.contains("the 2 dogs and the cat"));
        assert!(pet_food(17.0, &Pets::default()).is_none());
    }
}
