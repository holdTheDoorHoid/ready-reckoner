//! Ready Reckoner — `rr-supply`: turns a household and its bucket targets into cited requirement
//! lines (DESIGN §4.6).
//!
//! ```text
//! PlanInput + [BucketAssessment] ──► requirements() ──► [RequirementLine]
//! ```
//!
//! - **Quantity rules** ([`rules`]) are pure functions: water, food, medication, first aid,
//!   sanitation, power, heat and cold, communications, documents and money, getting home,
//!   leaving home and special needs. `docs/QUANTITY_RULES.md` lists every rule id; catalogue
//!   items name the rule that sizes them.
//! - **Every number comes from the constants registry** ([`constants()`], `constants.toml`), with
//!   its sources, range, alternatives and the note on where authorities disagree. Each line cites
//!   exactly the sources of the numbers it used, plus the bucket target's own sources; lines that
//!   rest on a planning estimate cite `rr_expert_prior` and say so.
//! - **Tier logic** ([`tiers`]): which tier covers a number of days, which tier is enough per
//!   bucket, and the tier the household should reach.
//! - **Line ids** say what kind of line each is ([`LineKind`]): needs, alternatives, optional
//!   lines and notes, so no consumer adds the same need twice.
//!
//! The engine is deterministic: no clock, no randomness, and only IEEE basic arithmetic with
//! `round`/`ceil` (no transcendental functions), so the CLI and the browser print the same lines.
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

mod basis;
pub mod constants;
mod format;
mod household;
pub mod lines;
pub mod rules;
mod targets;
pub mod tiers;

use std::collections::BTreeSet;

use rr_types::{
    BucketAssessment, BucketId, CitationId, HazardId, Per, PlanInput, RequirementLine, TierId,
};

pub use constants::{Constant, Constants, constants};
pub use lines::{LineKind, SizedLine};
pub use rules::Sizing;
pub use tiers::{ONE_MONTH_NOTE, tier_enough, tier_for_days, tier_recommended};

use household::Household;
use lines::{Shape, make};
use targets::Targets;

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-supply";

/// Every rule id rr-supply implements, including the generic ones (`once`, `per_person`, ...)
/// that the plan sizes with [`generic_quantity`]. `docs/QUANTITY_RULES.md` lists the same ids; a
/// test keeps the two in step.
pub const RULE_IDS: &[&str] = &[
    "water_gallons",
    "water_treatment_capacity",
    "boil_fuel",
    "livestock_water",
    "food_kcal",
    "food_cost_estimate",
    "food_kit_check",
    "long_term_staples",
    "infant_formula",
    "nursing_supplies",
    "pet_food",
    "medication_days",
    "rx_cold_storage",
    "antibiotics_none",
    "epinephrine_check",
    "medical_device_wh",
    "lights",
    "generator_fuel",
    "power_station_wh",
    "fridge_wh",
    "well_pump_wh",
    "solar_panel_watts",
    "wheelchair_battery",
    "noaa_radio",
    "phone_power_wh",
    "two_way_radios",
    "contact_cards",
    "local_map",
    "cash_days",
    "toilet_buckets",
    "toilet_bags",
    "toilet_cover_material",
    "soap_grams",
    "menstrual_products",
    "diapers",
    "baby_wipes",
    "first_aid_kit",
    "otc_medicines",
    "n95_masks",
    "thermometer",
    "ors_packets",
    "battery_fan",
    "cooling_towel",
    "cooling_plan",
    "sleeping_bag_or_blanket",
    "warm_layers",
    "warm_room_plan",
    "smoke_alarm",
    "co_alarm",
    "fire_extinguisher",
    "fire_escape_plan",
    "neighbour_contacts",
    "document_kit",
    "insurance_home_or_renters",
    "insurance_flood",
    "insurance_earthquake",
    "emergency_fund_months",
    "get_home_bag",
    "get_home_water",
    "car_kit",
    "go_bag",
    "go_bag_water",
    "go_bag_food",
    "pet_carrier",
    "pet_go_water",
    "pet_go_food",
    "fuel_half_tank",
    "evacuation_ride_plan",
    "evacuation_assistance_plan",
    "once",
    "per_person",
    "per_vehicle",
    "per_pet",
    "per_commuter",
];

/// Facts about the location that sharpen the sizing. Everything is optional: without them the
/// water is sized for a temperate climate and there is no winter-solar note.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SupplyContext {
    /// Days a year at or above 95 °F in the county under the chosen climate dial (NCA5 Atlas /
    /// CMRA `tmax_days_ge_95f`). At 30 or more (an estimate) water uses the hot-climate amount.
    pub days_at_or_above_95f: Option<f64>,
    /// The county centroid's latitude, for the winter-solar note.
    pub latitude: Option<f64>,
}

impl SupplyContext {
    /// Whether the hot-climate water amounts apply.
    pub fn hot(&self) -> bool {
        let threshold = constants().value(constants::keys::HOT_CLIMATE_DAYS_95F);
        self.days_at_or_above_95f
            .is_some_and(|d| d.is_finite() && d >= threshold)
    }
}

/// The household's requirement lines for its bucket targets, in bucket order (DESIGN §4.6). The
/// same as [`requirements_with`] without location context.
pub fn requirements(input: &PlanInput, buckets: &[BucketAssessment]) -> Vec<RequirementLine> {
    requirements_with(input, buckets, &SupplyContext::default())
}

/// The household's requirement lines, using what is known about the location.
pub fn requirements_with(
    input: &PlanInput,
    buckets: &[BucketAssessment],
    ctx: &SupplyContext,
) -> Vec<RequirementLine> {
    sized_requirements(input, buckets, ctx)
        .into_iter()
        .map(|l| l.line)
        .collect()
}

/// A generic rule's quantity for a catalogue item (`once`, `per_person`, `per_vehicle`,
/// `per_pet`, `per_commuter`): no requirement line is emitted for these; the plan sizes the item
/// directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenericQuantity {
    /// How many.
    pub quantity: f64,
    /// What it scales with.
    pub per: Per,
}

/// The quantity for an item that uses a generic rule, or `None` if `rule` is not generic.
pub fn generic_quantity(rule: &str, input: &PlanInput) -> Option<GenericQuantity> {
    let h = Household::new(input);
    let (quantity, per) = match rule {
        "once" => (1.0, Per::Household),
        "per_person" => (h.n() as f64, Per::Person),
        "per_vehicle" => (h.vehicles() as f64, Per::Household),
        "per_pet" => (f64::from(h.household_pets()), Per::Pet),
        "per_commuter" => (h.commuters().len() as f64, Per::Commuter),
        _ => return None,
    };
    Some(GenericQuantity { quantity, per })
}

/// Every citation id rr-supply can emit on its own (the `[[source]]` table of the constants
/// registry). Each must resolve to `content/citations.toml`. Lines also carry their bucket
/// target's sources, which the hazard and consequence crates own.
pub fn citations_used() -> BTreeSet<CitationId> {
    constants().sources.iter().map(|s| s.id.clone()).collect()
}

const HEAT: &[HazardId] = &[HazardId::HeatWave];
const COLD: &[HazardId] = &[
    HazardId::ColdWave,
    HazardId::WinterWeather,
    HazardId::IceStorm,
];

/// Collects lines in output order.
struct Out {
    lines: Vec<SizedLine>,
    /// Whether the one-month note has been said.
    noted: bool,
}

impl Out {
    fn push(
        &mut self,
        bucket: BucketId,
        s: Option<Sizing>,
        shape: Shape<'_>,
        tier: Option<TierId>,
        life_safety: bool,
    ) {
        let Some(mut s) = s else { return };
        let tier = tier.unwrap_or_else(|| s.days.map_or(TierId::H72, tier_for_days));
        // Say once that the one-month tier is an interpolation, on the first need line that
        // reaches it (DESIGN §4.5).
        if !self.noted && tier == TierId::M1 && matches!(shape, Shape::Need | Shape::PerPerson(_)) {
            s = s.extend(ONE_MONTH_NOTE, &tiers::one_month_basis());
            self.noted = true;
        }
        self.lines
            .push(make(bucket, s, shape, Some(tier), life_safety));
    }
}

/// The requirement lines with everything the plan and budget crates need: kind, tier, days,
/// per-day rate, formula, estimate tag and life-safety flag. Buckets missing from `buckets` get
/// no lines; lines come in [`BucketId::ALL`] order, and within a bucket in a fixed order.
pub fn sized_requirements(
    input: &PlanInput,
    buckets: &[BucketAssessment],
    ctx: &SupplyContext,
) -> Vec<SizedLine> {
    use Shape::{Need, Note, Optional};
    use rules::{
        comms, evacuate, fire, first_aid, food, get_home, medication, money, power, sanitation,
        thermal, water,
    };

    let h = Household::new(input);
    let t = Targets::new(buckets);
    let hot = ctx.hot();
    let level = input.dials.water_level;
    let people = h.people();
    let pets = h.pets();
    let housing = &input.housing;
    let now = Some(TierId::Now);
    let h72 = Some(TierId::H72);
    let mut out = Out {
        lines: Vec::new(),
        noted: false,
    };
    let days_of = |b: BucketId| t.days(b).filter(|d| *d > 0.0);
    // Duration lines also cite the target's own sources (the hazard and duration data behind the
    // days).
    let cited = |b: BucketId, s: Option<Sizing>| s.map(|s| s.also_cite(t.sources(b)));

    for &bucket in BucketId::ALL {
        if t.get(bucket).is_none() {
            continue;
        }
        match bucket {
            BucketId::Power => {
                let Some(days) = days_of(bucket) else {
                    continue;
                };
                out.push(
                    bucket,
                    cited(bucket, power::medical_device_wh(days, people)),
                    Need,
                    None,
                    true,
                );
                out.push(bucket, Some(power::lights(people)), Need, h72, false);
                if h.has_generator() {
                    out.push(
                        bucket,
                        cited(bucket, power::generator_fuel(days, housing)),
                        Need,
                        None,
                        false,
                    );
                }
                out.push(bucket, power::wheelchair_battery(people), Need, h72, false);
                out.push(
                    bucket,
                    cited(bucket, power::power_station_wh(days, people)),
                    Optional,
                    None,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, power::fridge_wh(days)),
                    Optional,
                    None,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, power::well_pump_wh(days, housing)),
                    Optional,
                    None,
                    false,
                );
                if let Some(lat) = ctx.latitude {
                    out.push(
                        bucket,
                        power::solar_panel_watts(lat, people),
                        Note,
                        h72,
                        false,
                    );
                }
            }
            BucketId::WaterBoil => {
                let Some(days) = days_of(bucket) else {
                    continue;
                };
                let treat = water::water_treatment_boil(days, people, pets, hot);
                let fuel = water::boil_fuel(treat.quantity);
                out.push(bucket, cited(bucket, Some(treat)), Need, None, false);
                out.push(
                    bucket,
                    Some(fuel),
                    Optional,
                    Some(tier_for_days(days)),
                    false,
                );
            }
            BucketId::WaterOut => {
                let Some(days) = days_of(bucket) else {
                    continue;
                };
                let (stored, treat) = water::water_storage(days, people, pets, level, hot);
                out.push(bucket, cited(bucket, Some(stored)), Need, None, true);
                out.push(
                    bucket,
                    cited(bucket, treat),
                    Need,
                    Some(tier_for_days(days)),
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, water::livestock_water(days, pets.large_animals)),
                    Need,
                    None,
                    false,
                );
                out.push(bucket, Some(sanitation::toilet_buckets()), Need, h72, false);
                out.push(
                    bucket,
                    cited(bucket, sanitation::toilet_bags(days, people)),
                    Need,
                    None,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, sanitation::toilet_cover_material(days, people)),
                    Need,
                    None,
                    false,
                );
            }
            BucketId::Supplies => {
                let Some(days) = days_of(bucket) else {
                    continue;
                };
                out.push(
                    bucket,
                    cited(bucket, Some(food::food_kcal(days, people))),
                    Need,
                    None,
                    false,
                );
                let tier = Some(tier_for_days(days));
                for s in food::food_cost_estimates(days, people) {
                    let variant = s.item_class.trim_start_matches("food_").to_owned();
                    out.push(
                        bucket,
                        cited(bucket, Some(s)),
                        Shape::Alternative {
                            of: "food_kcal",
                            variant: &variant,
                        },
                        tier,
                        false,
                    );
                }
                for s in food::long_term_staples(days, people) {
                    let variant = s.item_class.clone();
                    out.push(
                        bucket,
                        cited(bucket, Some(s)),
                        Shape::Alternative {
                            of: "food_kcal",
                            variant: &variant,
                        },
                        tier,
                        false,
                    );
                }
                out.push(bucket, food::food_kit_check(people), Note, tier, false);
                out.push(
                    bucket,
                    cited(bucket, food::infant_formula(days, people)),
                    Need,
                    None,
                    true,
                );
                out.push(bucket, food::nursing_supplies(people), Need, h72, false);
                out.push(
                    bucket,
                    cited(bucket, food::pet_food(days, pets)),
                    Need,
                    None,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, sanitation::soap_grams(days, people)),
                    Need,
                    None,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, sanitation::menstrual_products(days, people)),
                    Need,
                    tier,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, sanitation::diapers(days, people)),
                    Need,
                    None,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, sanitation::baby_wipes(days, people)),
                    Need,
                    tier,
                    false,
                );
            }
            BucketId::Thermal => {
                if days_of(bucket).is_none() {
                    continue;
                }
                let heat = t.driven_by(bucket, HEAT);
                let cold = t.driven_by(bucket, COLD);
                // With no heat or cold hazard named (or no contributions at all), cover both.
                let neither = heat != Some(true) && cold != Some(true);
                if heat == Some(true) || neither {
                    out.push(bucket, Some(thermal::battery_fan(people)), Need, h72, false);
                    out.push(
                        bucket,
                        Some(thermal::cooling_towel(people)),
                        Need,
                        h72,
                        false,
                    );
                    out.push(bucket, Some(thermal::cooling_plan()), Need, now, false);
                }
                if cold == Some(true) || neither {
                    out.push(
                        bucket,
                        Some(thermal::sleeping_bag_or_blanket(people)),
                        Need,
                        h72,
                        false,
                    );
                    out.push(bucket, Some(thermal::warm_layers(people)), Need, h72, false);
                    out.push(
                        bucket,
                        Some(thermal::warm_room_plan(housing, people)),
                        Need,
                        now,
                        false,
                    );
                }
            }
            BucketId::Medication => {
                let days = days_of(bucket);
                out.push(
                    bucket,
                    cited(bucket, medication::medication_days(days, people)),
                    Need,
                    None,
                    true,
                );
                let cold_days = days_of(BucketId::Power).or(days);
                if let Some(cd) = cold_days {
                    let src = if days_of(BucketId::Power).is_some() {
                        BucketId::Power
                    } else {
                        bucket
                    };
                    out.push(
                        bucket,
                        cited(src, medication::rx_cold_storage(cd, people)),
                        Need,
                        None,
                        true,
                    );
                }
                out.push(
                    bucket,
                    Some(medication::antibiotics_none()),
                    Need,
                    now,
                    false,
                );
                out.push(
                    bucket,
                    medication::epinephrine_check(people),
                    Need,
                    now,
                    false,
                );
            }
            BucketId::Comms => {
                let Some(days) = days_of(bucket) else {
                    continue;
                };
                out.push(bucket, Some(comms::noaa_radio()), Need, h72, false);
                let (phone_days, src) = match days_of(BucketId::Power) {
                    Some(p) => (p, BucketId::Power),
                    None => (days, bucket),
                };
                out.push(
                    bucket,
                    cited(src, comms::phone_power_wh(phone_days, people)),
                    Need,
                    None,
                    false,
                );
                out.push(bucket, comms::two_way_radios(people), Need, h72, false);
                out.push(bucket, Some(comms::contact_cards(people)), Need, now, false);
                out.push(bucket, Some(comms::local_map()), Need, now, false);
                out.push(
                    bucket,
                    cited(bucket, Some(comms::cash_days(days))),
                    Need,
                    None,
                    false,
                );
            }
            BucketId::Evacuate => {
                let Some(e) = t.evacuate() else { continue };
                out.push(
                    bucket,
                    cited(
                        bucket,
                        Some(evacuate::go_bag(people, e.notice_hours_low, e.days_away)),
                    ),
                    Need,
                    h72,
                    false,
                );
                out.push(
                    bucket,
                    cited(
                        bucket,
                        Some(evacuate::go_bag_water(people, level, hot, e.days_away)),
                    ),
                    Need,
                    h72,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, evacuate::go_bag_food(people, e.days_away)),
                    Need,
                    h72,
                    false,
                );
                out.push(bucket, evacuate::pet_carrier(pets), Need, h72, false);
                out.push(bucket, evacuate::pet_go_water(pets), Need, h72, false);
                out.push(bucket, evacuate::pet_go_food(pets), Need, h72, false);
                out.push(
                    bucket,
                    evacuate::fuel_half_tank(h.vehicles(), h.evs()),
                    Need,
                    now,
                    false,
                );
                out.push(
                    bucket,
                    evacuate::evacuation_ride_plan(h.vehicles()),
                    Need,
                    now,
                    false,
                );
                out.push(
                    bucket,
                    evacuate::evacuation_assistance_plan(people),
                    Need,
                    now,
                    false,
                );
            }
            BucketId::GetHome => {
                for (index, commute) in h.commuters() {
                    out.push(
                        bucket,
                        Some(get_home::get_home_bag(index, commute, hot)),
                        Shape::PerPerson(index),
                        h72,
                        false,
                    );
                    out.push(
                        bucket,
                        Some(get_home::get_home_water(commute, hot)),
                        Shape::PerPerson(index),
                        h72,
                        false,
                    );
                }
                out.push(bucket, get_home::car_kit(h.vehicles()), Need, h72, false);
            }
            BucketId::MedicalEmergency => {
                let days = days_of(BucketId::Supplies).unwrap_or(f64::from(TierId::W2.days()));
                out.push(
                    bucket,
                    Some(first_aid::first_aid_kit(people)),
                    Need,
                    h72,
                    false,
                );
                out.push(
                    bucket,
                    Some(first_aid::otc_medicines(days, people)),
                    Need,
                    h72,
                    false,
                );
                out.push(bucket, first_aid::n95_masks(people), Need, h72, false);
                out.push(
                    bucket,
                    Some(first_aid::thermometer(people)),
                    Need,
                    h72,
                    false,
                );
                out.push(
                    bucket,
                    Some(first_aid::ors_packets(days, people)),
                    Need,
                    h72,
                    false,
                );
            }
            BucketId::Fire => {
                out.push(
                    bucket,
                    Some(fire::fire_escape_plan(housing)),
                    Need,
                    now,
                    false,
                );
                out.push(bucket, fire::smoke_alarm(housing), Need, h72, true);
                out.push(bucket, fire::co_alarm(housing), Need, h72, true);
                out.push(bucket, fire::fire_extinguisher(housing), Need, h72, false);
            }
            BucketId::Security => {
                out.push(bucket, Some(fire::neighbour_contacts()), Need, now, false);
            }
            BucketId::Income => {
                let months = t.months(bucket);
                let s = money::emergency_fund_months(months, &input.finances);
                let s = if months.is_some() {
                    s.also_cite(t.sources(bucket))
                } else {
                    s
                };
                out.push(bucket, Some(s), Need, Some(TierId::M3), false);
            }
            BucketId::HomeLoss => {
                out.push(bucket, Some(money::document_kit()), Need, now, false);
                out.push(
                    bucket,
                    money::insurance_home_or_renters(&input.finances, housing),
                    Need,
                    now,
                    false,
                );
                out.push(
                    bucket,
                    money::insurance_flood(&input.finances, housing),
                    Need,
                    now,
                    false,
                );
                if t.driven_by(bucket, &[HazardId::Earthquake]) == Some(true) {
                    out.push(
                        bucket,
                        money::insurance_earthquake(&input.finances),
                        Need,
                        now,
                        false,
                    );
                }
            }
        }
    }

    out.lines
}
