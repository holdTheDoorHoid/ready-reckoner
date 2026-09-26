//! What the allocator needs to know about each catalogue item, built from `rr-supply`'s
//! requirement lines (`docs/QUANTITY_RULES.md`) and the item catalogue (`content/items`).
//!
//! # The model
//!
//! A duration bucket (power, water, food, heat or cold, medicine, phones) is covered when every
//! **need line** `rr-supply` sized for it is met. Need lines are grouped into **parts** by the kind
//! of cover they are (their `item_class`: stored water, the toilet, food, heat, cold, lights,
//! batteries ...). The bucket's covered days are the weakest part's, because a household with a
//! month of food but no way to keep warm is not covered for a month of winter outage. The
//! allocator values each part separately with an equal share of the bucket's value, so it buys
//! the cheap parts of every bucket early.
//!
//! Within a part, each line is a **segment** with an equal share of the part's days, measured as
//! the share of the line's quantity the household has: four headlamps of the four lights the line
//! asks for cover the whole segment; one lantern covers a quarter of it. One part is a chain
//! instead: in `water_out`, stored water covers the first 14 days (`rr-supply` stores at most two
//! weeks) and water treatment the days after that, so the segments' spans are those day counts.
//!
//! # Which item meets which line
//!
//! 1. An item whose `quantity_rule` is the line's rule (bottled water and `water_gallons`, food
//!    and `food_kcal`). An item on an **alternative** line (reused bottles) meets the need line it
//!    stands in for. The item is **divisible** (bought in chunks toward the next step of the day
//!    ladder) when the line scales with days and the rule sizes the item in proportion, so a
//!    gallon of water is a quarter of a day for a family of four.
//! 2. [`ITEM_LINES`], for items sized by a generic rule (`once`, `per_person`): a headlamp is one
//!    of the lights the `lights` line asks for.
//! 3. For heat and cold items the table does not name, the item's `hazard_extras`: heat-wave
//!    items meet the first `thermal_heat` line, winter items the first `thermal_cold` line.
//!
//! Only water, food, medicine, generator fuel and baby formula are divisible
//! ([`DIVISIBLE_CLASSES`]); everything else is bought as one set, sized to meet its line. Bulk
//! staples ([`LONG_STORE_FOOD`]) count only for the days beyond the first month, as `rr-supply`'s
//! `long_term_staples` rule intends: the first month is food the household normally eats.
//!
//! Readiness buckets (leaving home, getting home, medical emergencies, fire, security) have no
//! days. An item that meets one of their need lines, or (meeting none) lists one first among its
//! buckets, is a step on that bucket's checklist and avoids the bucket's harm each time it is
//! needed ([`READINESS_HARM`], an expert estimate, as the research prototype valued each
//! capability). Life-safety follows DESIGN §4.7 (the catalogue's flags, plus any item that meets
//! a line `rr-supply` marks life-safety: water, a dependent's medicine, a medical device's power,
//! alarms, formula); a go-bag is not on that list, and the allocator's guardrail warns when an
//! evacuation-heavy household has none by month six.
//!
//! Items are offered only when the household needs them: the item's rule gives a quantity above
//! zero, it does not rest only on an optional line (a generator, a solar panel), and a
//! hazard-specific item names a hazard with at least a 2 % chance in ten years here.

use std::collections::{BTreeMap, BTreeSet};

use rr_budget::{CoverageRule, ItemMeta, ItemRole, ReadinessCredit};
use rr_content::Content;
use rr_supply::{ItemSizer, LineKind, SizedLine};
use rr_types::{BucketId, BucketKind, HazardId, Item, ItemId, PlanInput, Target};

/// Litres in a US gallon, as the catalogue's `volume_l_per_unit` values use it.
pub const LITRES_PER_GALLON: f64 = 3.785;

/// Stored water covers at most this many days (`rr-supply`'s `water_gallons` rule); treatment
/// covers the rest of a longer no-water target.
pub const STORED_WATER_MAX_DAYS: f64 = 14.0;

/// How many line units one item unit provides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Units {
    /// This many line units per item unit (a pair of radios is 2 radios).
    Per(f64),
    /// The quantity the item's own rule sizes meets the whole line (three power banks for three
    /// phone users meet the phone-power line).
    Fill,
    /// The item's `energy_kcal_per_unit` kilocalories per item unit, for food sized by a generic
    /// rule (three days of ordinary food per person). Unlike [`Units::Per`], the item keeps its
    /// own rule's quantity rather than growing to meet the whole line.
    Energy,
}

/// Items sized by a generic rule, and the need lines they meet (`bucket.rule`, matching every
/// per-person line of that rule too). An empty list says the item meets no duration line (so the
/// heat-and-cold fallback does not apply). Checked against the catalogue and `rr-supply` by tests.
pub const ITEM_LINES: &[(&str, &[(&str, Units)])] = &[
    // Power.
    ("power_headlamp", &[("power.lights", Units::Per(1.0))]),
    ("power_lantern", &[("power.lights", Units::Per(1.0))]),
    ("power_station", &[("power.medical_device_wh", Units::Fill)]),
    (
        "power_device_battery",
        &[("power.medical_device_wh", Units::Fill)],
    ),
    // Phones, news and payments.
    ("power_bank", &[("comms.phone_power_wh", Units::Fill)]),
    ("comms_noaa_radio", &[("comms.noaa_radio", Units::Per(1.0))]),
    (
        "comms_frs_radios",
        &[("comms.two_way_radios", Units::Per(2.0))],
    ),
    (
        "comms_contact_card",
        &[("comms.contact_cards", Units::Fill)],
    ),
    // A phone is the household's own; the phone-power line is about keeping it charged.
    ("comms_phone_basic", &[]),
    // Water and the toilet.
    (
        "water_boil_method",
        &[("water_boil.water_treatment_capacity", Units::Fill)],
    ),
    (
        "san_twin_bucket_toilet",
        &[("water_out.toilet_buckets", Units::Per(2.0))],
    ),
    (
        "san_cover_material",
        &[("water_out.toilet_cover_material", Units::Fill)],
    ),
    // Heat and cold.
    (
        "thermal_battery_fan",
        &[("thermal.battery_fan", Units::Per(1.0))],
    ),
    (
        "thermal_cool_room_plan",
        &[("thermal.cooling_plan", Units::Fill)],
    ),
    (
        "thermal_sleeping_bag",
        &[("thermal.sleeping_bag_or_blanket", Units::Per(1.0))],
    ),
    (
        "thermal_warm_room_plan",
        &[("thermal.warm_room_plan", Units::Fill)],
    ),
    // Foil blankets are for go-bags and the car, not a week without heat.
    ("thermal_emergency_blankets", &[]),
    ("thermal_indoor_thermometer", &[]),
    // Food: three days of what the household normally eats, counted in kilocalories. A cooking pot
    // meets no line: the boil-water step (which includes a pot with a lid) meets the treatment
    // line, and the pot is an assumed basic.
    (
        "food_three_days_basic",
        &[("supplies.food_kcal", Units::Energy)],
    ),
    ("food_cooking_pot", &[]),
    // Medicine.
    (
        "med_cooler_refrigerated_rx",
        &[("medication.rx_cold_storage", Units::Fill)],
    ),
    // The set includes oral rehydration salts.
    (
        "med_otc_basics",
        &[
            ("medical_emergency.otc_medicines", Units::Fill),
            ("medical_emergency.ors_packets", Units::Fill),
        ],
    ),
    // Leaving home and getting home.
    ("evac_go_bag", &[("evacuate.go_bag", Units::Per(1.0))]),
    (
        "evac_half_tank",
        &[("evacuate.fuel_half_tank", Units::Fill)],
    ),
    // The leaving-home plan includes routes marked on a paper map.
    ("evac_know_zone", &[("comms.local_map", Units::Fill)]),
    (
        "special_access_needs_plan",
        &[("evacuate.evacuation_assistance_plan", Units::Fill)],
    ),
    (
        "special_pet_kit",
        &[("evacuate.pet_carrier", Units::Per(1.0))],
    ),
    ("gethome_bag", &[("get_home.get_home_bag", Units::Per(1.0))]),
    ("gethome_car_kit", &[("get_home.car_kit", Units::Per(1.0))]),
    // Documents, insurance and savings (money buckets: shown in `explain`, not bought).
    // The documents step includes checking insurance cover (home or renters, flood, earthquake).
    (
        "docs_effak",
        &[
            ("home_loss.document_kit", Units::Fill),
            ("home_loss.insurance_home_or_renters", Units::Fill),
            ("home_loss.insurance_flood", Units::Fill),
            ("home_loss.insurance_earthquake", Units::Fill),
        ],
    ),
    (
        "docs_start_emergency_fund",
        &[("income.emergency_fund_months", Units::Fill)],
    ),
    // Fire and security. The fire-safety step includes the escape plan.
    (
        "fire_test_alarms",
        &[("fire.fire_escape_plan", Units::Fill)],
    ),
    (
        "community_know_two_neighbours",
        &[("security.neighbour_contacts", Units::Fill)],
    ),
];

/// Day-equivalents of harm one step on a readiness bucket's checklist avoids each time it is
/// needed (DESIGN §4.7: V = 10 · w · r_need · harm), from the research prototype's capability
/// items (go-bag 2, get-home bag 0.5, first-aid kit 0.1, smoke-alarm test 5) and 0.5 for
/// security, which the research did not price. Expert estimates: they order readiness items
/// against each other and against supplies, and never reach the user as numbers. (A medical
/// emergency is needed about twice a year, so its small harm per use still adds up.)
pub const READINESS_HARM: [(BucketId, f64); 5] = [
    (BucketId::Evacuate, 2.0),
    (BucketId::GetHome, 0.5),
    (BucketId::MedicalEmergency, 0.1),
    (BucketId::Fire, 5.0),
    (BucketId::Security, 0.5),
];

/// Citation ids behind the coverage model's own numbers (the readiness harm estimates come from
/// the research prototype; the harm weights they multiply are the allocator's).
pub const COVERAGE_CITATIONS: [&str; 2] = ["rr_research_risk_model", "prior_harm_weights"];

/// Item classes bought in chunks toward the next step of the day ladder; everything else is a set.
pub const DIVISIBLE_CLASSES: [&str; 6] = [
    "water_stored",
    "food",
    "prescription_medicine",
    "generator_fuel",
    "infant_formula",
    "pet_food",
];

/// Rules whose count grows with the target's days without a daily rate (one pack of batteries per
/// week): bought a unit at a time, so a long target does not arrive as one big purchase. (Bleach
/// is one bottle per household whatever the target, so it is an ordinary set.)
pub const DIVISIBLE_RULES: [&str; 1] = ["battery_packs"];

/// Classes bought at least a week's worth at a time (a bag of pet food, not a pound).
const WEEK_STEP_CLASSES: [&str; 1] = ["pet_food"];

/// Food for long storage that counts only for the days beyond the first month.
pub const LONG_STORE_FOOD: [&str; 1] = ["food_bulk_staples"];

/// Days of a supplies target that food the household normally eats must cover first.
pub const FIRST_MONTH_DAYS: f64 = 30.0;

/// Heat-wave hazards, for the heat-and-cold fallback.
const HEAT: [HazardId; 1] = [HazardId::HeatWave];
/// Winter hazards, for the heat-and-cold fallback.
const COLD: [HazardId; 3] = [
    HazardId::ColdWave,
    HazardId::WinterWeather,
    HazardId::IceStorm,
];

/// The power part that keeps a medical device running.
pub const DEVICE_PART: &str = "medical device power";

/// The water part that is stored drinking water (then treatment, for long targets).
pub const STORED_WATER_PART: &str = "stored water";

/// The plain name of the part a line's `item_class` belongs to. The allocator prints it in
/// brackets after the bucket's supply ("food and supplies (pet food)"), so it reads as words.
fn part_name(bucket: BucketId, class: &str) -> String {
    let name = match class {
        "light" => "lights",
        "battery_pack" => "batteries",
        "medical_device_power" | "device_battery" | "power_station" => DEVICE_PART,
        "generator_fuel" => "generator fuel",
        "wheelchair_battery" => "wheelchair battery",
        "water_stored" => STORED_WATER_PART,
        "water_treatment_capacity" if bucket == BucketId::WaterOut => "bleach",
        "water_treatment_capacity" => "water treatment",
        "livestock_water" => "water for animals",
        "toilet_bucket" | "toilet_bags" | "toilet_cover_material" => "toilet",
        "food" => "food",
        "infant_formula" => "baby formula",
        "nursing_supplies" => "nursing supplies",
        "pet_food" => "pet food",
        "toilet_paper" => "toilet paper",
        "soap" => "soap",
        "menstrual_products" => "period products",
        "diapers" | "baby_wipes" => "diapers and wipes",
        "thermal_heat" => "heat",
        "thermal_cold" => "cold",
        "prescription_medicine" => "prescriptions",
        "medicine_cooler" => "cold storage",
        "epinephrine_auto_injector" => "epinephrine",
        "weather_radio" | "power_bank" | "contact_card" | "paper_map" => "phone",
        "cash" => "payments",
        "two_way_radio" => "two-way radios",
        other => return other.replace('_', " "),
    };
    name.to_owned()
}

/// How an offered item meets one line.
#[derive(Debug, Clone, PartialEq)]
pub struct Join {
    /// The need line's id.
    pub line_id: String,
    /// Its bucket.
    pub bucket: BucketId,
    /// Line units one item unit provides.
    pub units_per_item: f64,
    /// The item meets the line through one of its alternatives: reused bottles for stored water,
    /// or a staging step (a pet's water packed in its go-kit) for the bag it is packed in. It
    /// counts toward that line, but a staging step is never the bag itself, so it takes none of
    /// the bag's guardrail roles.
    pub via_alternative: bool,
}

/// Why a catalogue item is not offered to this household.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Excluded {
    /// Its quantity rule gives zero for this household (no pets, no commuter, alarms owned ...).
    NotNeeded,
    /// It rests only on an optional line (a generator, a solar panel): not needed for the targets.
    OptionalOnly,
    /// It is a hazard-specific extra and none of its hazards reaches a 2 % ten-year chance here.
    HazardTooRare,
    /// Its quantity rule is not one `rr-supply` implements (the content validator prevents this).
    UnknownRule,
}

/// A catalogue item as offered to this household.
#[derive(Debug, Clone, PartialEq)]
pub struct Offered {
    /// The item, with `buckets` widened to every bucket it meets a line in and `life_safety`
    /// raised when it meets a life-safety line.
    pub item: Item,
    /// Its quantity for this household, in the item's own unit.
    pub quantity: f64,
    /// The lines it meets.
    pub joins: Vec<Join>,
    /// Bought in chunks (water, food, medicine) rather than as one set.
    pub divisible: bool,
    /// The chunk size for a divisible item, in the item's unit.
    pub step: f64,
}

/// Everything built from the catalogue for one household.
#[derive(Debug, Clone)]
pub struct Offers {
    /// Items offered, in catalogue order.
    pub offered: Vec<Offered>,
    /// The allocator's metadata for them (same order).
    pub meta: Vec<ItemMeta>,
    /// The coverage rule.
    pub rule: PlanCoverage,
    /// Items left out, and why.
    pub excluded: Vec<(ItemId, Excluded)>,
    /// Hazard-specific extras offered but with nothing to measure them by (no line, no readiness
    /// bucket), for the packet's "extras" list.
    pub extras: Vec<ItemId>,
}

impl Offers {
    /// The offered item with this id.
    pub fn get(&self, id: &str) -> Option<&Offered> {
        self.offered.iter().find(|o| o.item.id == id)
    }

    /// The items as the allocator's catalogue.
    pub fn items(&self) -> Vec<Item> {
        self.offered.iter().map(|o| o.item.clone()).collect()
    }
}

/// Ten-year chance at a yearly rate.
fn p10(rate: f64) -> f64 {
    if rate <= 0.0 {
        0.0
    } else {
        -rr_types::math::exp_m1(-10.0 * rate)
    }
}

fn table_lines(item: &str) -> Option<&'static [(&'static str, Units)]> {
    ITEM_LINES
        .iter()
        .find(|(id, _)| *id == item)
        .map(|(_, lines)| *lines)
}

/// Whether a line id is the line `key` (`bucket.rule`) or one of its per-person lines.
fn line_matches(id: &str, key: &str) -> bool {
    id == key
        || id
            .strip_prefix(key)
            .is_some_and(|rest| rest.starts_with(".person_"))
}

/// The unit a rule's lines are counted in, when the budget must convert (gallons of water or
/// kilocalories of food into the item's own unit).
fn conversion(unit: &str, item: &Item) -> f64 {
    match unit {
        "gallon" => item
            .volume_l_per_unit
            .map_or(1.0, |l| f64::from(l) / LITRES_PER_GALLON),
        "kcal" => item.energy_kcal_per_unit.map_or(1.0, f64::from),
        _ => 1.0,
    }
}

/// Builds the offers for one household.
///
/// `targets` holds each duration bucket's target in days; `register` each hazard's yearly rate.
pub fn build(
    content: &Content,
    sizer: &ItemSizer<'_>,
    targets: &BTreeMap<BucketId, f64>,
    register: &BTreeMap<HazardId, f64>,
) -> Offers {
    let lines = sizer.lines();
    let need = |l: &&SizedLine| l.kind == LineKind::Need && l.quantity > 0.0;
    let mut offered: Vec<Offered> = Vec::new();
    let mut excluded: Vec<(ItemId, Excluded)> = Vec::new();

    for item in &content.items {
        let rule = item.quantity_rule.as_str();
        let Some(q) = sizer.quantity(rule) else {
            excluded.push((item.id.clone(), Excluded::UnknownRule));
            continue;
        };
        let rule_lines: Vec<&SizedLine> = lines.iter().filter(|l| l.line.rule == rule).collect();
        let unit = rule_lines
            .first()
            .map(|l| l.line.unit.as_str())
            .unwrap_or("");
        let conv = conversion(unit, item);
        let quantity = q.quantity / conv;
        if !(quantity.is_finite() && quantity > 1e-9) {
            excluded.push((item.id.clone(), Excluded::NotNeeded));
            continue;
        }
        if !rule_lines.is_empty()
            && rule_lines
                .iter()
                .all(|l| matches!(l.kind, LineKind::Optional | LineKind::Note))
        {
            excluded.push((item.id.clone(), Excluded::OptionalOnly));
            continue;
        }
        if !item.hazard_extras.is_empty() && !item.rare_catastrophic {
            let relevant = item.hazard_extras.iter().any(|h| {
                p10(register.get(h).copied().unwrap_or(0.0))
                    >= rr_budget::value::READINESS_MIN_P_NEED_10YR
            });
            if !relevant {
                excluded.push((item.id.clone(), Excluded::HazardTooRare));
                continue;
            }
        }

        // Joins.
        let mut joins: Vec<Join> = Vec::new();
        let mut divisible = false;
        let mut step = 1.0_f64;
        let push = |line: &SizedLine, units: f64, via_alternative: bool, joins: &mut Vec<Join>| {
            if !joins.iter().any(|j| j.line_id == line.line.id) && units > 0.0 {
                joins.push(Join {
                    line_id: line.line.id.clone(),
                    bucket: line.line.bucket,
                    units_per_item: units,
                    via_alternative,
                });
            }
        };
        // 1. By quantity rule: need lines of the rule, and the need lines behind alternatives.
        let rule_need: Vec<&SizedLine> = rule_lines.iter().copied().filter(need).collect();
        let via_alt: Vec<&SizedLine> = rule_lines
            .iter()
            .filter(|l| l.kind == LineKind::Alternative)
            .filter_map(|alt| LineKind::alternative_to(&alt.line.id))
            .filter_map(|id| lines.iter().find(|l| l.line.id == id))
            .filter(need)
            .collect();
        if !rule_need.is_empty() {
            let per_person = rule_need.iter().any(|l| l.line.id.contains(".person_"));
            let expected = if per_person {
                rule_need.iter().map(|l| l.quantity).sum::<f64>()
            } else {
                rule_need.iter().map(|l| l.quantity).fold(0.0, f64::max)
            };
            let proportional = (q.quantity - expected).abs() <= 1e-6 * expected.max(1.0);
            let day_scaled = rule_need.iter().all(|l| l.per_day.is_some());
            for l in &rule_need {
                let units = if proportional {
                    conv
                } else {
                    l.quantity / quantity
                };
                push(l, units, false, &mut joins);
            }
            let class_ok = rule_need
                .iter()
                .all(|l| DIVISIBLE_CLASSES.contains(&l.line.item_class.as_str()));
            let by_rule = DIVISIBLE_RULES.contains(&rule);
            divisible = proportional && ((day_scaled && class_ok) || by_rule) && !item.free;
            if divisible {
                // A week of pet food at a time; everything else a unit at a time.
                step = rule_need
                    .iter()
                    .filter(|l| WEEK_STEP_CLASSES.contains(&l.line.item_class.as_str()))
                    .filter_map(|l| l.per_day)
                    .map(|per_day| (per_day * 7.0 / conv).ceil())
                    .fold(1.0_f64, f64::max);
            }
        }
        for l in via_alt {
            // An alternative (reused bottles) counts in the need line's unit.
            push(l, conv, true, &mut joins);
        }
        // 2. The table.
        let listed = table_lines(item.id.as_str());
        if let Some(entries) = listed {
            for (key, units) in entries {
                for l in lines
                    .iter()
                    .filter(need)
                    .filter(|l| line_matches(&l.line.id, key))
                {
                    let per_person = l.line.id.contains(".person_");
                    let u = match units {
                        Units::Per(u) => *u,
                        // Per-person lines: one item each; otherwise the sized quantity meets it.
                        Units::Fill if per_person => 1.0,
                        Units::Fill => l.quantity / quantity,
                        Units::Energy => item.energy_kcal_per_unit.map_or(0.0, f64::from),
                    };
                    push(l, u, false, &mut joins);
                }
            }
        }
        // 3. Heat and cold by hazard, for thermal items the table does not name.
        if joins.is_empty()
            && listed.is_none()
            && !item.free
            && item.buckets.contains(&BucketId::Thermal)
        {
            let class = if item.hazard_extras.iter().any(|h| HEAT.contains(h)) {
                Some("thermal_heat")
            } else if item.hazard_extras.iter().any(|h| COLD.contains(h)) {
                Some("thermal_cold")
            } else {
                None
            };
            if let Some(class) = class {
                if let Some(l) = lines
                    .iter()
                    .filter(need)
                    .find(|l| l.line.bucket == BucketId::Thermal && l.line.item_class == class)
                {
                    let u = l.quantity / quantity;
                    push(l, u, false, &mut joins);
                }
            }
        }

        // A set sized by a generic rule buys as many as its lines ask for (two fans, two pairs
        // of radios), so buying the set meets the line.
        let mut quantity = quantity;
        if !divisible {
            let needed = listed
                .unwrap_or(&[])
                .iter()
                .filter_map(|(key, units)| match units {
                    Units::Per(u) if *u > 0.0 => Some((key, *u)),
                    _ => None,
                })
                .flat_map(|(key, u)| {
                    lines
                        .iter()
                        .filter(need)
                        .filter(move |l| line_matches(&l.line.id, key))
                        .map(move |l| (l.line.id.contains(".person_"), l.quantity / u))
                })
                .fold((0.0_f64, 0.0_f64), |(people, most), (per_person, q)| {
                    if per_person {
                        (people + q, most)
                    } else {
                        (people, most.max(q))
                    }
                });
            let needed = needed.0.max(needed.1);
            if needed > 0.0 {
                quantity = needed.ceil();
            }
        }
        let mut item = item.clone();
        for j in &joins {
            if !item.buckets.contains(&j.bucket) {
                item.buckets.push(j.bucket);
            }
        }
        let life_safety_line = joins.iter().any(|j| {
            lines
                .iter()
                .any(|l| l.line.id == j.line_id && l.life_safety)
        });
        item.life_safety |= life_safety_line;
        offered.push(Offered {
            item,
            quantity,
            joins,
            divisible,
            step,
        });
    }

    let rule = PlanCoverage::new(lines, &offered, targets);
    let (meta, extras) = metadata(&offered, &rule);
    Offers {
        offered,
        meta,
        rule,
        excluded,
        extras,
    }
}

/// The allocator's metadata: sets and steps, readiness credits, guardrail roles.
fn metadata(offered: &[Offered], rule: &PlanCoverage) -> (Vec<ItemMeta>, Vec<ItemId>) {
    let harm_share = |b: BucketId| -> f64 {
        READINESS_HARM
            .iter()
            .find(|(x, _)| *x == b)
            .map_or(0.0, |(_, h)| *h)
    };

    let mut meta = Vec::new();
    let mut extras = Vec::new();
    for o in offered {
        let mut m = ItemMeta::new(o.item.id.clone());
        if o.divisible {
            m.step = Some(o.step);
        } else {
            m.set_quantity = Some(set_quantity(o.quantity));
        }
        // Readiness credit: every readiness bucket where the item meets a line; an item that
        // meets none gets credit for the first readiness bucket its catalogue entry lists, its
        // main purpose (a tornado shelter spot is security first, not a medical kit).
        let lined: Vec<BucketId> = READINESS_HARM
            .iter()
            .map(|(b, _)| *b)
            .filter(|b| o.joins.iter().any(|j| j.bucket == *b))
            .collect();
        let credited: Vec<BucketId> = if !lined.is_empty() {
            lined
        } else {
            o.item
                .buckets
                .iter()
                .copied()
                .find(|b| READINESS_HARM.iter().any(|(x, _)| x == b))
                .into_iter()
                .collect()
        };
        for bucket in credited {
            let share = harm_share(bucket);
            if share > 0.0 {
                m.readiness.push(ReadinessCredit {
                    bucket,
                    harm_day_equivalents: share,
                });
            }
        }
        // Roles come from the lines an item meets itself, never through a staging alternative.
        let meets = |key: &str| {
            o.joins
                .iter()
                .any(|j| !j.via_alternative && line_matches(&j.line_id, key))
        };
        if meets("power.medical_device_wh")
            || meets("power.device_battery_units")
            || meets("power.power_station_units")
        {
            m.roles.push(ItemRole::DevicePower);
        }
        if meets("medication.rx_cold_storage") {
            m.roles.push(ItemRole::ColdChain);
        }
        if meets("evacuate.go_bag") {
            m.roles.push(ItemRole::GoBag);
        }
        // Stored water itself, not the free step of refilling bottles (an alternative).
        if meets("water_out.water_gallons") && !o.item.free {
            m.roles.push(ItemRole::StoredWater);
        }
        let measured = !m.readiness.is_empty() || o.joins.iter().any(|j| rule.covers(j));
        if !measured && !o.item.hazard_extras.is_empty() && !o.item.free {
            extras.push(o.item.id.clone());
        }
        meta.push(m);
    }
    (meta, extras)
}

/// A set's size in whole units (a set is bought at once).
fn set_quantity(q: f64) -> f64 {
    let r = q.round();
    if (q - r).abs() < 1e-6 {
        r.max(1.0)
    } else {
        q.ceil().max(1.0)
    }
}

/// One segment of a part: a need line (or a stretch of one) and the items that meet it.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    /// The line's id.
    pub line_id: String,
    /// Days of the bucket this segment stands for when fully met.
    pub span: f64,
    /// The quantity that meets the segment, in line units.
    pub requirement: f64,
    /// Line units per item unit, per item.
    pub contrib: BTreeMap<ItemId, f64>,
    /// Counts only once the segment before is fully met, and receives whatever that segment
    /// holds beyond its own requirement (bulk staples count only after a month of normal food,
    /// and normal food beyond a month counts here too).
    pub after_previous: bool,
}

impl Segment {
    fn have(&self, qty: &dyn Fn(&ItemId) -> f64) -> f64 {
        self.contrib.iter().map(|(id, u)| qty(id) * u).sum::<f64>()
    }
}

/// One part of a bucket: the lines of one kind of cover.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// Plain name ("stored water", "toilet", "heat").
    pub name: String,
    /// Its segments.
    pub segments: Vec<Segment>,
}

impl Part {
    fn days(&self, qty: &dyn Fn(&ItemId) -> f64) -> f64 {
        let mut total = 0.0;
        let mut overflow = 0.0;
        let mut previous_full = true;
        for s in &self.segments {
            let mut have = s.have(qty);
            if s.after_previous {
                if !previous_full {
                    previous_full = false;
                    overflow = 0.0;
                    continue;
                }
                have += overflow;
            }
            if s.requirement > 0.0 {
                total += s.span * (have / s.requirement).clamp(0.0, 1.0);
            }
            previous_full = have + 1e-9 >= s.requirement;
            overflow = (have - s.requirement).max(0.0);
        }
        total + 0.0
    }
}

/// [`CoverageRule`] for the allocator: see the module docs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlanCoverage {
    buckets: BTreeMap<BucketId, (f64, Vec<Part>)>,
}

impl PlanCoverage {
    /// Builds the parts of every duration bucket with a target.
    pub fn new(
        lines: &[SizedLine],
        offered: &[Offered],
        targets: &BTreeMap<BucketId, f64>,
    ) -> Self {
        let mut buckets = BTreeMap::new();
        for (&bucket, &target) in targets {
            if bucket.kind() != BucketKind::Duration || target <= 0.0 {
                continue;
            }
            // Need lines of this bucket that some offered item meets.
            let coverable: Vec<&SizedLine> = lines
                .iter()
                .filter(|l| l.line.bucket == bucket && l.kind == LineKind::Need && l.quantity > 0.0)
                .filter(|l| {
                    offered
                        .iter()
                        .any(|o| o.joins.iter().any(|j| j.line_id == l.line.id))
                })
                .collect();
            let contrib_for = |line: &SizedLine| -> BTreeMap<ItemId, f64> {
                offered
                    .iter()
                    .filter_map(|o| {
                        o.joins
                            .iter()
                            .find(|j| j.line_id == line.line.id)
                            .map(|j| (o.item.id.clone(), j.units_per_item))
                    })
                    .collect()
            };
            let mut parts: Vec<Part> = Vec::new();
            let mut used: BTreeSet<&str> = BTreeSet::new();
            // The drinking-water chain: stored water for the first days, treatment after that.
            if bucket == BucketId::WaterOut {
                let stored = coverable.iter().find(|l| l.line.rule == "water_gallons");
                let treated = coverable
                    .iter()
                    .find(|l| l.line.rule == "water_treatment_capacity" && l.per_day.is_some());
                if let Some(s) = stored {
                    let stored_days = s.days.unwrap_or(target).min(target);
                    let mut segments = vec![Segment {
                        line_id: s.line.id.clone(),
                        span: stored_days,
                        requirement: s.quantity,
                        contrib: contrib_for(s),
                        after_previous: false,
                    }];
                    used.insert(s.line.id.as_str());
                    if let Some(t) = treated {
                        segments.push(Segment {
                            line_id: t.line.id.clone(),
                            span: (target - stored_days).max(0.0),
                            requirement: t.quantity,
                            contrib: contrib_for(t),
                            after_previous: false,
                        });
                        used.insert(t.line.id.as_str());
                    }
                    let total: f64 = segments.iter().map(|x| x.span).sum();
                    if total > 0.0 {
                        for x in &mut segments {
                            x.span *= target / total;
                        }
                    }
                    parts.push(Part {
                        name: part_name(bucket, "water_stored"),
                        segments,
                    });
                }
            }
            // Food: what the household normally eats covers the first month; bulk staples only
            // the days after it.
            if bucket == BucketId::Supplies {
                if let Some(food) = coverable.iter().find(|l| l.line.rule == "food_kcal") {
                    let all = contrib_for(food);
                    let (tail, first): (BTreeMap<ItemId, f64>, BTreeMap<ItemId, f64>) = all
                        .into_iter()
                        .partition(|(id, _)| LONG_STORE_FOOD.contains(&id.as_str()));
                    let per_day = food.per_day.unwrap_or(food.quantity / target.max(1.0));
                    let head_days = target.min(FIRST_MONTH_DAYS);
                    let mut segments = vec![Segment {
                        line_id: food.line.id.clone(),
                        span: head_days,
                        requirement: per_day * head_days,
                        contrib: first.clone(),
                        after_previous: false,
                    }];
                    if target > FIRST_MONTH_DAYS {
                        let mut both = first;
                        both.extend(tail);
                        // The head's items already count toward the head; only their excess
                        // (the overflow) and the staples count here.
                        both.retain(|id, _| LONG_STORE_FOOD.contains(&id.as_str()));
                        segments.push(Segment {
                            line_id: food.line.id.clone(),
                            span: target - FIRST_MONTH_DAYS,
                            requirement: per_day * (target - FIRST_MONTH_DAYS),
                            contrib: both,
                            after_previous: true,
                        });
                    }
                    used.insert(food.line.id.as_str());
                    parts.push(Part {
                        name: part_name(bucket, "food"),
                        segments,
                    });
                }
            }
            // Everything else, grouped by kind of cover, in line order.
            let mut groups: Vec<(String, Vec<&SizedLine>)> = Vec::new();
            for l in coverable
                .iter()
                .filter(|l| !used.contains(l.line.id.as_str()))
            {
                let name = part_name(bucket, &l.line.item_class);
                match groups.iter_mut().find(|(n, _)| *n == name) {
                    Some((_, v)) => v.push(l),
                    None => groups.push((name, vec![l])),
                }
            }
            for (name, group) in groups {
                let n = group.len() as f64;
                parts.push(Part {
                    name,
                    segments: group
                        .into_iter()
                        .map(|l| Segment {
                            line_id: l.line.id.clone(),
                            span: target / n,
                            requirement: l.quantity,
                            contrib: contrib_for(l),
                            after_previous: false,
                        })
                        .collect(),
                });
            }
            buckets.insert(bucket, (target, parts));
        }
        Self { buckets }
    }

    /// Whether a join counts toward a duration bucket's coverage.
    pub fn covers(&self, join: &Join) -> bool {
        self.buckets.get(&join.bucket).is_some_and(|(_, parts)| {
            parts
                .iter()
                .any(|p| p.segments.iter().any(|s| s.line_id == join.line_id))
        })
    }

    /// The parts of a bucket, for `explain` and the packet.
    pub fn parts_of(&self, bucket: BucketId) -> &[Part] {
        self.buckets
            .get(&bucket)
            .map(|(_, p)| p.as_slice())
            .unwrap_or(&[])
    }

    /// Days one part of a bucket covers with these items (0 for an unknown part).
    pub fn part_days(&self, bucket: BucketId, part: &str, items: &[(ItemId, f64)]) -> f64 {
        let qty = lookup(items);
        self.part(bucket, part).map_or(0.0, |p| p.days(&qty))
    }

    fn part(&self, bucket: BucketId, part: &str) -> Option<&Part> {
        self.buckets
            .get(&bucket)
            .and_then(|(_, parts)| parts.iter().find(|p| p.name == part))
    }
}

fn lookup(items: &[(ItemId, f64)]) -> impl Fn(&ItemId) -> f64 + '_ {
    move |id: &ItemId| {
        items
            .iter()
            .filter(|(i, _)| i == id)
            .map(|(_, q)| q.max(0.0))
            .sum::<f64>()
    }
}

impl CoverageRule for PlanCoverage {
    fn coverage(&self, bucket: BucketId, items: &[(ItemId, f64)], _household: &PlanInput) -> f64 {
        let Some((target, parts)) = self.buckets.get(&bucket) else {
            return 0.0;
        };
        if parts.is_empty() {
            return 0.0;
        }
        let qty = lookup(items);
        parts
            .iter()
            .map(|p| p.days(&qty))
            .fold(f64::INFINITY, f64::min)
            .min(*target)
            + 0.0
    }

    fn parts(&self, bucket: BucketId) -> Vec<String> {
        match self.buckets.get(&bucket) {
            Some((_, parts)) if parts.len() > 1 => parts.iter().map(|p| p.name.clone()).collect(),
            _ => Vec::new(),
        }
    }

    fn part_coverage(
        &self,
        bucket: BucketId,
        part: &str,
        items: &[(ItemId, f64)],
        _household: &PlanInput,
    ) -> f64 {
        let qty = lookup(items);
        self.part(bucket, part).map_or(0.0, |p| p.days(&qty)) + 0.0
    }

    fn gain(
        &self,
        bucket: BucketId,
        part: Option<&str>,
        items: &[(ItemId, f64)],
        item: &ItemId,
        qty: f64,
        household: &PlanInput,
    ) -> f64 {
        let base = lookup(items);
        let more = |id: &ItemId| base(id) + if id == item { qty.max(0.0) } else { 0.0 };
        let measure = |q: &dyn Fn(&ItemId) -> f64| -> f64 {
            match part {
                Some(p) => self.part(bucket, p).map_or(0.0, |p| p.days(q)),
                None => {
                    let Some((target, parts)) = self.buckets.get(&bucket) else {
                        return 0.0;
                    };
                    if parts.is_empty() {
                        return 0.0;
                    }
                    parts
                        .iter()
                        .map(|p| p.days(q))
                        .fold(f64::INFINITY, f64::min)
                        .min(*target)
                }
            }
        };
        let _ = household;
        (measure(&more) - measure(&base)).max(0.0) + 0.0
    }
}

/// The days a target stands for (duration buckets), else 0.
pub fn target_days(target: &Target) -> f64 {
    match *target {
        Target::Days { value, .. } => f64::from(value),
        _ => 0.0,
    }
}
