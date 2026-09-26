//! The Philadelphia worked example (research risk-model §8) as stub inputs: curves tabulated from
//! the research prototype's own parameters (`rm_proto/params.py`, `PHL`) and a catalogue mirroring
//! `rm_proto/items_phl.py` with the research's illustrative prices. Hazard shares are stub values
//! read off the prototype's event list; the real ones come from `rr-consequence`.

use std::collections::BTreeMap;

use rr_budget::{Contributes, ItemMeta, ItemRole, ReadinessCredit, Risks};
use rr_types::{BucketId, HazardId, Item, Target, TierId};

use super::{EventClass, assessment, days_target, ev, item, readiness_target, tabulate};

/// The research default dial: 90 % sure nothing in ten years is worse, Λ = −ln(0.9)/10
/// (research risk-model §3.2). Exactly 1/100 puts the water target at 3.003 days, which rounds up
/// to 5 on the day ladder; see the report.
pub const DIAL_RATE: f64 = 0.010_536_051_565_782_628;

const H: f64 = 1.0 / 24.0;

pub fn power() -> Vec<EventClass> {
    vec![
        ev(0.67, 1.5 * H, 4.0 * H),
        ev(0.2, 3.0 * H, 10.0 * H),
        ev(0.065, 2.0 * H, 20.0 * H),
        ev(0.024, 1.5, 5.0),
        ev(0.003, 5.0, 14.0),
        ev(0.005, 1.0, 3.0),
    ]
}

pub fn water_out() -> Vec<EventClass> {
    vec![
        ev(0.10, 6.0 * H, 24.0 * H),
        ev(0.02, 1.5, 7.0),
        ev(0.004, 7.0, 30.0),
        ev(0.002, 1.0, 3.0),
    ]
}

pub fn water_boil() -> Vec<EventClass> {
    vec![ev(0.05, 2.0, 6.0)]
}

pub fn supplies() -> Vec<EventClass> {
    vec![
        ev(0.5, 1.0, 2.0),
        ev(0.2, 2.0, 5.0),
        ev(0.03, 2.0, 5.0),
        ev(0.01, 14.0, 45.0),
        ev(0.003, 5.0, 14.0),
    ]
}

pub fn medication() -> Vec<EventClass> {
    vec![
        ev(0.1, 1.5, 4.0),
        ev(0.02, 5.0, 21.0),
        ev(0.008, 5.0, 30.0),
        ev(0.01, 7.0, 30.0),
        ev(0.003, 7.0, 21.0),
    ]
}

pub fn thermal() -> Vec<EventClass> {
    vec![
        ev(0.25 * 0.15, 4.0 * H, 20.0 * H),
        ev(0.024 * 0.3, 1.5, 5.0),
        ev(0.02 * 0.3, 1.0, 3.0),
        ev(0.024 * 0.4, 1.5, 5.0),
    ]
}

pub fn comms() -> Vec<EventClass> {
    vec![ev(0.05, 0.5, 2.0), ev(0.003, 3.0, 10.0)]
}

/// Ten-year chance from an annual rate.
fn p10(rate: f64) -> f64 {
    1.0 - rr_types::math::exp(-10.0 * rate)
}

pub fn risks() -> Risks {
    use BucketId as B;
    use HazardId as Hz;
    let mut curves = BTreeMap::new();
    curves.insert(B::Power, tabulate(&power(), DIAL_RATE));
    curves.insert(B::WaterOut, tabulate(&water_out(), DIAL_RATE));
    curves.insert(B::WaterBoil, tabulate(&water_boil(), DIAL_RATE));
    curves.insert(B::Supplies, tabulate(&supplies(), DIAL_RATE));
    curves.insert(B::Medication, tabulate(&medication(), DIAL_RATE));
    let mut t = tabulate(&thermal(), DIAL_RATE);
    t.part_shares = BTreeMap::from([("heat".to_owned(), 0.6), ("cold".to_owned(), 0.4)]);
    curves.insert(B::Thermal, t);
    let mut c = tabulate(&comms(), DIAL_RATE);
    c.part_shares = BTreeMap::from([("phone".to_owned(), 0.7), ("payments".to_owned(), 0.3)]);
    curves.insert(B::Comms, c);

    let mut assessments = BTreeMap::new();
    let mut put = |b: B, target: Target, shares: &[(Hz, f32)]| {
        assessments.insert(b, assessment(b, target, shares));
    };
    let days = |b: B| days_target(curves[&b].target_days);
    put(
        B::Power,
        days(B::Power),
        &[
            (Hz::StrongWind, 0.40),
            (Hz::WinterWeather, 0.20),
            (Hz::IceStorm, 0.15),
            (Hz::Hurricane, 0.10),
            (Hz::GridFailure, 0.10),
            (Hz::HeatWave, 0.05),
        ],
    );
    put(
        B::WaterOut,
        days(B::WaterOut),
        &[
            (Hz::LocalUtilityOutage, 0.70),
            (Hz::HazmatRelease, 0.20),
            (Hz::GridFailure, 0.05),
            (Hz::RiverineFlooding, 0.05),
        ],
    );
    put(
        B::WaterBoil,
        days(B::WaterBoil),
        &[
            (Hz::LocalUtilityOutage, 0.75),
            (Hz::RiverineFlooding, 0.15),
            (Hz::Hurricane, 0.10),
        ],
    );
    put(
        B::Supplies,
        days(B::Supplies),
        &[
            (Hz::WinterWeather, 0.55),
            (Hz::SupplyChainDisruption, 0.30),
            (Hz::Pandemic, 0.10),
            (Hz::CivilUnrest, 0.05),
        ],
    );
    put(
        B::Medication,
        days(B::Medication),
        &[
            (Hz::CyberOutage, 0.35),
            (Hz::WinterWeather, 0.30),
            (Hz::Pandemic, 0.20),
            (Hz::HouseFire, 0.15),
        ],
    );
    put(
        B::Thermal,
        days(B::Thermal),
        &[
            (Hz::HeatWave, 0.55),
            (Hz::IceStorm, 0.25),
            (Hz::GridFailure, 0.20),
        ],
    );
    put(
        B::Comms,
        days(B::Comms),
        &[
            (Hz::StrongWind, 0.55),
            (Hz::Hurricane, 0.25),
            (Hz::CyberOutage, 0.20),
        ],
    );
    put(
        B::Evacuate,
        Target::Evacuate {
            p_need_10yr: p10(0.0082),
            notice_hours_low: 0.25,
            notice_hours_high: 24.0,
            days_away: 3.0,
        },
        &[
            (Hz::HouseFire, 0.65),
            (Hz::RiverineFlooding, 0.20),
            (Hz::HazmatRelease, 0.15),
        ],
    );
    put(
        B::GetHome,
        readiness_target(p10(0.1)),
        &[(Hz::VehicleStranding, 0.70), (Hz::WinterWeather, 0.30)],
    );
    put(
        B::MedicalEmergency,
        readiness_target(p10(0.5)),
        &[(Hz::MedicalEmergency, 1.0)],
    );
    put(
        B::Fire,
        readiness_target(p10(0.0052)),
        &[(Hz::HouseFire, 1.0)],
    );
    put(
        B::HomeLoss,
        readiness_target(0.05),
        &[(Hz::HouseFire, 0.6), (Hz::RiverineFlooding, 0.4)],
    );
    let mut income = assessment(
        B::Income,
        Target::Months {
            value: 4.0,
            low: 2.2,
            high: 9.5,
        },
        &[(Hz::JobLoss, 0.85), (Hz::EarnerDeathOrDisability, 0.15)],
    );
    income.frequency_sentences = vec![
        "About 15 of 100 households like yours have an income gap of more than 3 months in the next 10 years."
            .into(),
    ];
    assessments.insert(B::Income, income);
    Risks {
        curves,
        assessments,
    }
}

/// The catalogue and its coverage metadata.
pub fn catalogue() -> (Vec<Item>, Vec<ItemMeta>) {
    use BucketId as B;
    let mut items = Vec::new();
    let mut meta = Vec::new();
    let mut add = |it: Item, m: ItemMeta| {
        items.push(it);
        meta.push(m);
    };
    let hh = Contributes::per_household;
    let pp = Contributes::per_person;
    let with = |id: &str, contributes: Vec<Contributes>| {
        let mut m = ItemMeta::new(id);
        m.contributes = contributes;
        m
    };
    let ready = |id: &str, bucket: B, harm: f64| {
        let mut m = ItemMeta::new(id);
        m.readiness = vec![ReadinessCredit {
            bucket,
            harm_day_equivalents: harm,
        }];
        m
    };
    let free = (0.0, 0.0);

    // ---- Free actions (research §4.3 and §8.6 month 0). ----
    add(
        item(
            "family_plan",
            "Household plan: meeting places, out-of-area contact, contact cards",
            "plan",
            &[B::Comms, B::Evacuate],
            TierId::Now,
            true,
            false,
            free,
        ),
        with("family_plan", vec![hh(B::Comms, 0.5)]),
    );
    add(
        item(
            "documents_copied",
            "Photos and paper copies of IDs, lease, insurance and medicine list in a zip bag",
            "set",
            &[B::HomeLoss, B::Evacuate],
            TierId::Now,
            true,
            false,
            free,
        ),
        ItemMeta::new("documents_copied"),
    );
    add(
        item(
            "alerts_signup",
            "Sign up for local text alerts and turn on emergency alerts",
            "action",
            &[B::Comms],
            TierId::Now,
            true,
            false,
            free,
        ),
        ItemMeta::new("alerts_signup"),
    );
    let mut bottles = with(
        "water_reused_bottles",
        vec![pp(B::WaterOut, 1.0), pp(B::WaterBoil, 1.0)],
    );
    bottles.set_quantity = Some(6.0);
    add(
        item(
            "water_reused_bottles",
            "Fill clean reused drink bottles with tap water (replace every 6 months)",
            "gallon",
            &[B::WaterOut, B::WaterBoil],
            TierId::Now,
            true,
            true,
            free,
        ),
        bottles,
    );
    add(
        item(
            "water_heater_reserve",
            "Learn to draw water from the water heater; close its inlet on a do-not-drink notice",
            "action",
            &[B::WaterOut],
            TierId::Now,
            true,
            false,
            free,
        ),
        with("water_heater_reserve", vec![hh(B::WaterOut, 1.0)]),
    );
    add(
        item(
            "stove_boil",
            "Boil water on the stove during a boil-water notice",
            "action",
            &[B::WaterBoil],
            TierId::Now,
            true,
            false,
            free,
        ),
        with("stove_boil", vec![hh(B::WaterBoil, 14.0)]),
    );
    add(
        item(
            "medication_refill_rule",
            "Refill medicine when 7 days remain; ask about 90-day fills; keep a written list",
            "action",
            &[B::Medication],
            TierId::Now,
            true,
            true,
            free,
        ),
        with("medication_refill_rule", vec![hh(B::Medication, 7.0)]),
    );
    add(
        item(
            "heat_cold_plan",
            "Heat and cold plan: cooling centre, coolest and warmest rooms, check on the grandparent",
            "plan",
            &[B::Thermal],
            TierId::Now,
            true,
            false,
            free,
        ),
        with("heat_cold_plan", vec![hh(B::Thermal, 0.5)]),
    );
    add(
        item(
            "alarm_test",
            "Test smoke alarms, confirm a carbon monoxide alarm, know two ways out",
            "action",
            &[B::Fire],
            TierId::Now,
            true,
            true,
            free,
        ),
        ready("alarm_test", B::Fire, 5.0),
    );
    add(
        item(
            "neighbours_numbers",
            "Swap phone numbers with two neighbours",
            "action",
            &[B::Security, B::MedicalEmergency],
            TierId::Now,
            true,
            false,
            free,
        ),
        ItemMeta::new("neighbours_numbers"),
    );
    add(
        item(
            "half_tank_rule",
            "Keep the car's tank at least half full",
            "action",
            &[B::Evacuate, B::GetHome],
            TierId::Now,
            true,
            false,
            free,
        ),
        ItemMeta::new("half_tank_rule"),
    );
    add(
        item(
            "renters_insurance_check",
            "Check whether you have renters insurance",
            "action",
            &[B::HomeLoss],
            TierId::Now,
            true,
            false,
            free,
        ),
        ItemMeta::new("renters_insurance_check"),
    );

    // ---- Purchases (research §8.6 prices, illustrative). ----
    add(
        item(
            "lights",
            "Lights: 2 headlamps, 2 lanterns and spare batteries",
            "set",
            &[B::Power],
            TierId::H72,
            false,
            false,
            (25.0, 45.0),
        ),
        with("lights", vec![hh(B::Power, 3.0)]),
    );
    add(
        item(
            "power_bank",
            "Power bank and charging cables",
            "each",
            &[B::Comms],
            TierId::H72,
            false,
            false,
            (20.0, 30.0),
        ),
        with("power_bank", vec![hh(B::Comms, 3.0).part("phone")]),
    );
    add(
        item(
            "water_containers",
            "Two 5-gallon water containers and unscented bleach",
            "set",
            &[B::WaterOut],
            TierId::H72,
            false,
            true,
            (25.0, 45.0),
        ),
        with("water_containers", vec![hh(B::WaterOut, 2.5)]),
    );
    let mut food = with("food_pantry", vec![pp(B::Supplies, 1.0)]);
    food.step = Some(1.0);
    add(
        item(
            "food_pantry",
            "Extra shelf-stable food you already eat",
            "person-day",
            &[B::Supplies],
            TierId::H72,
            false,
            false,
            (2.15, 2.85),
        ),
        food,
    );
    add(
        item(
            "first_aid_kit",
            "First-aid kit and basic over-the-counter supplies",
            "kit",
            &[B::MedicalEmergency],
            TierId::H72,
            false,
            false,
            (25.0, 35.0),
        ),
        ready("first_aid_kit", B::MedicalEmergency, 0.1),
    );
    add(
        item(
            "battery_fans",
            "Two battery fans and cooling towels",
            "set",
            &[B::Thermal],
            TierId::H72,
            false,
            false,
            (25.0, 35.0),
        ),
        with("battery_fans", vec![hh(B::Thermal, 1.5).part("heat")]),
    );
    add(
        item(
            "blankets",
            "Blankets or sleeping bags for everyone (if not owned)",
            "set",
            &[B::Thermal],
            TierId::H72,
            false,
            false,
            (30.0, 50.0),
        ),
        with("blankets", vec![hh(B::Thermal, 3.0).part("cold")]),
    );
    add(
        item(
            "weather_radio",
            "Hand-crank or battery weather radio",
            "each",
            &[B::Comms],
            TierId::H72,
            false,
            false,
            (25.0, 35.0),
        ),
        with("weather_radio", vec![hh(B::Comms, 7.0).part("phone")]),
    );
    let mut cushion = with("medicine_cushion", vec![hh(B::Medication, 1.0)]);
    cushion.step = Some(1.0);
    add(
        item(
            "medicine_cushion",
            "Extra days of the grandparent's daily medicine (copay-dependent)",
            "day",
            &[B::Medication],
            TierId::W2,
            false,
            true,
            (0.5, 2.36),
        ),
        cushion,
    );
    add(
        item(
            "get_home_bag",
            "Get-home bag for the commuter (shoes, water, snacks, poncho, light)",
            "bag",
            &[B::GetHome],
            TierId::W2,
            false,
            false,
            (30.0, 50.0),
        ),
        ready("get_home_bag", B::GetHome, 0.5),
    );
    let mut go_bag = ready("go_bag", B::Evacuate, 2.0);
    go_bag.roles = vec![ItemRole::GoBag];
    add(
        item(
            "go_bag",
            "Go-bag additions: bag, masks, toiletries, copies, small bills",
            "bag",
            &[B::Evacuate],
            TierId::W2,
            false,
            false,
            (20.0, 40.0),
        ),
        go_bag,
    );
    add(
        item(
            "cash_reserve",
            "Cash at home: $100 in small bills (kept, not spent)",
            "set",
            &[B::Comms],
            TierId::W2,
            false,
            false,
            (100.0, 100.0),
        ),
        with("cash_reserve", vec![hh(B::Comms, 3.0).part("payments")]),
    );
    add(
        item(
            "more_water_containers",
            "Two more 5-gallon water containers",
            "set",
            &[B::WaterOut],
            TierId::W2,
            false,
            true,
            (25.0, 35.0),
        ),
        with("more_water_containers", vec![hh(B::WaterOut, 2.5)]),
    );
    add(
        item(
            "kn95_masks",
            "KN95 masks for smoke and illness",
            "box",
            &[B::MedicalEmergency],
            TierId::W2,
            false,
            false,
            (15.0, 25.0),
        ),
        ready("kn95_masks", B::MedicalEmergency, 0.04),
    );
    add(
        item(
            "power_station",
            "Portable power station, about 300 Wh",
            "each",
            &[B::Power, B::Comms, B::Thermal],
            TierId::W2,
            false,
            false,
            (200.0, 300.0),
        ),
        with(
            "power_station",
            vec![
                hh(B::Power, 2.0),
                hh(B::Comms, 3.0).part("phone"),
                hh(B::Thermal, 1.0),
            ],
        ),
    );
    let mut meter = item(
        "radiation_meter",
        "Radiation meter",
        "each",
        &[B::Security],
        TierId::Y1,
        false,
        false,
        (100.0, 200.0),
    );
    meter.rare_catastrophic = true;
    add(meter, ItemMeta::new("radiation_meter"));
    (items, meta)
}
