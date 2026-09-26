//! Ready Reckoner — `rr-supply`: turns a household and its bucket targets into cited requirement
//! lines (DESIGN §4.6), and sizes catalogue items by their quantity rule.
//!
//! ```text
//! PlanInput + [BucketAssessment] ──► requirements()   ──► [RequirementLine]   (per bucket)
//!                                 └─► ItemSizer::new() ──► quantity(rule)      (per catalogue item)
//! ```
//!
//! - **Quantity rules** ([`rules`], [`generic`]) are pure functions: water, food, medication,
//!   first aid, sanitation, power, heat and cold, communications, documents and money, getting
//!   home, leaving home and special needs, plus generic counts and `once_if_*` switches.
//!   `docs/QUANTITY_RULES.md` lists every rule id ([`rule_ids`]); catalogue items name the rule that
//!   sizes them.
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
pub mod generic;
mod household;
pub mod lines;
pub mod rules;
mod targets;
pub mod tiers;

use std::collections::BTreeSet;

use rr_types::{
    BucketAssessment, BucketId, CitationId, HazardId, Per, PlanInput, RequirementLine, Setting,
    Target, TierId, WaterSource,
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

/// The catalogue item for an electrician-installed generator inlet with an interlock or transfer
/// switch. A household that lists it in `existing` and owns a generator has pump power on a well,
/// so its animals' stored water only bridges the first days (`livestock_water`).
pub const TRANSFER_INTERLOCK_ITEM: &str = "power_transfer_interlock";

pub use rules::power::COLD_MEDICINE_POWER_CLASS;

/// Rules that emit requirement lines, in the order `docs/QUANTITY_RULES.md` lists them.
pub const LINE_RULES: &[&str] = &[
    "water_gallons",
    "water_reused_bottles",
    "water_treatment_capacity",
    "bleach_bottles",
    "boil_fuel",
    "livestock_water",
    "livestock_water_stored",
    "food_kcal",
    "food_cost_estimate",
    "food_kit_check",
    "long_term_staples",
    "cooking_fuel_canisters",
    "infant_formula_oz",
    "nursing_supplies",
    "pet_food_lb",
    "pet_food_days",
    "medication_days",
    "rx_cold_storage",
    "antibiotics_none",
    "epinephrine_check",
    "medical_device_wh",
    "device_battery_units",
    "lights",
    "battery_packs",
    "power_station_units",
    "generator_units",
    "generator_fuel_gallons",
    "generator_connection_units",
    "fuel_cans",
    "solar_panel_units",
    "fridge_wh",
    "fridge_thermometers",
    "well_pump_wh",
    "wheelchair_battery",
    "noaa_radio",
    "phone_power_wh",
    "two_way_radios",
    "contact_cards",
    "local_map",
    "cash_reserve_usd",
    "toilet_buckets",
    "toilet_bags",
    "toilet_cover_material",
    "toilet_paper_rolls",
    "soap_person_months",
    "menstrual_cycles",
    "diapers",
    "baby_wipes",
    "first_aid_kit",
    "otc_medicines",
    "n95_masks",
    "thermometer",
    "ors_packets",
    "bleeding_control_kit",
    "battery_fan",
    "cooling_towel",
    "cooling_plan",
    "room_thermometer",
    "blankets",
    "sleeping_bag_or_blanket",
    "warm_layers",
    "warm_room_plan",
    "fire_escape_plan",
    "smoke_alarm_count",
    "co_alarm_count",
    "extinguisher_count",
    "escape_ladder_count",
    "neighbour_contacts",
    "document_kit",
    "insurance_home_or_renters",
    "insurance_flood",
    "insurance_earthquake",
    "emergency_fund_months",
    "get_home_bag",
    "get_home_water",
    "get_home_food",
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
];

/// Every rule id rr-supply implements: the line rules and the generic ones
/// ([`generic::GENERIC_RULES`]). `docs/QUANTITY_RULES.md` lists the same ids; a test keeps the two
/// in step.
pub fn rule_ids() -> Vec<&'static str> {
    LINE_RULES
        .iter()
        .chain(generic::GENERIC_RULES)
        .copied()
        .collect()
}

/// Whether rr-supply implements a rule with this id.
pub fn is_rule(id: &str) -> bool {
    LINE_RULES.contains(&id) || generic::GENERIC_RULES.contains(&id)
}

/// Facts about the location that sharpen the sizing. Everything is optional: without them the
/// water is sized for a temperate climate, there is no winter-solar figure, and the nuclear-plant
/// switch is off.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SupplyContext {
    /// Days a year at or above 95 °F in the county under the chosen climate dial (NCA5 Atlas /
    /// CMRA `tmax_days_ge_95f`). At 30 or more (an estimate) water uses the hot-climate amount.
    pub days_at_or_above_95f: Option<f64>,
    /// The county centroid's latitude, for the winter-solar figure.
    pub latitude: Option<f64>,
    /// A nuclear power plant within 16 km (`LocationResolved.facility_flags`), for
    /// `once_if_near_nuclear_plant`.
    pub nuclear_plant_within_16km: Option<bool>,
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

/// Every citation id rr-supply can emit on its own (the `[[source]]` table of the constants
/// registry). Each must resolve to `content/citations.toml`. Lines also carry their bucket
/// target's sources, which the hazard and consequence crates own.
pub fn citations_used() -> BTreeSet<CitationId> {
    constants().sources.iter().map(|s| s.id.clone()).collect()
}

/// How many of a catalogue item a household needs, by the item's quantity rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemQuantity {
    /// In the item's own unit, except that `gallon` rules give gallons of water and `kcal` rules
    /// give food energy (convert with the item's `volume_l_per_unit` / `energy_kcal_per_unit`).
    /// Zero means the item is not needed by this household.
    pub quantity: f64,
    /// What the quantity scales with.
    pub per: Per,
}

/// Sizes catalogue items for one household: computes the requirement lines once, then answers
/// [`ItemSizer::quantity`] for any rule id (docs/QUANTITY_RULES.md).
#[derive(Debug, Clone)]
pub struct ItemSizer<'a> {
    input: &'a PlanInput,
    ctx: SupplyContext,
    lines: Vec<SizedLine>,
    water_out_days: Option<f64>,
}

impl<'a> ItemSizer<'a> {
    /// Computes the household's lines for these targets.
    pub fn new(input: &'a PlanInput, buckets: &[BucketAssessment], ctx: &SupplyContext) -> Self {
        let lines = sized_requirements(input, buckets, ctx);
        let water_out_days = Targets::new(buckets).days(BucketId::WaterOut);
        Self {
            input,
            ctx: *ctx,
            lines,
            water_out_days,
        }
    }

    /// The lines behind the answers.
    pub fn lines(&self) -> &[SizedLine] {
        &self.lines
    }

    /// The quantity for an item that uses `rule`, or `None` if rr-supply has no such rule.
    ///
    /// Generic rules count people, pets, vehicles or commuters, or switch an item on (1) or off (0).
    /// Line rules give the quantity of the household's line for that rule: summed over people for
    /// per-person lines (each commuter needs a bag), and the largest over buckets otherwise (one
    /// stock serves every bucket that needs it). A rule with no line for this household gives 0.
    /// `water_treatment_capacity` gives 1 filter when stored water cannot cover the no-water target
    /// (over 14 days) or the home is on a well, else 0: the lines state the gallons to make safe,
    /// which one family filter far exceeds.
    pub fn quantity(&self, rule: &str) -> Option<ItemQuantity> {
        if let Some((quantity, per)) = generic::quantity(rule, self.input, &self.ctx) {
            return Some(ItemQuantity { quantity, per });
        }
        if !LINE_RULES.contains(&rule) {
            return None;
        }
        if rule == "water_treatment_capacity" {
            let cap = constants().value(constants::keys::WATER_STORED_CAP_DAYS);
            let long = self.water_out_days.is_some_and(|d| d > cap);
            let well = self.input.housing.water == WaterSource::Well;
            let quantity = if long || well { 1.0 } else { 0.0 };
            return Some(ItemQuantity {
                quantity,
                per: Per::Household,
            });
        }
        let mine: Vec<&SizedLine> = self.lines.iter().filter(|l| l.line.rule == rule).collect();
        let Some(first) = mine.first() else {
            return Some(ItemQuantity {
                quantity: 0.0,
                per: Per::Household,
            });
        };
        let per_person = mine.iter().any(|l| l.line.id.contains(".person_"));
        let quantity = if per_person {
            mine.iter().map(|l| l.quantity).sum()
        } else {
            mine.iter().map(|l| l.quantity).fold(0.0, f64::max)
        };
        Some(ItemQuantity {
            quantity,
            per: first.line.per,
        })
    }
}

/// The quantity for one catalogue item (see [`ItemSizer::quantity`]); build an [`ItemSizer`] when
/// sizing many items for the same household.
pub fn item_quantity(
    rule: &str,
    input: &PlanInput,
    buckets: &[BucketAssessment],
    ctx: &SupplyContext,
) -> Option<ItemQuantity> {
    ItemSizer::new(input, buckets, ctx).quantity(rule)
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
    // Horses or livestock on a well (round-2 review P-02). Their stored water covers the no-water
    // target up to 14 days, like people's. Pump power exists only when the household owns a
    // generator and the interlock or transfer switch that connects it to the pump; then the
    // stored water only bridges the first days (`livestock_water`), with two weeks in stock tanks
    // as the alternative. A power cut that can outlast the stored water makes a pump-rated
    // generator a need (unless one is owned), with its connection and fuel cans. A drought that
    // dries the well is met by hauling water, not by pump power.
    let well = housing.water == WaterSource::Well;
    let animals_on_well = well && pets.large_animals > 0 && days_of(BucketId::WaterOut).is_some();
    let owns_interlock = input
        .existing
        .iter()
        .any(|o| o.item_id == TRANSFER_INTERLOCK_ITEM && o.qty > 0.0);
    let pump_power = animals_on_well && h.has_generator() && owns_interlock;
    let stored_cap = constants().value(constants::keys::WATER_STORED_CAP_DAYS);
    let animals_stored_days = days_of(BucketId::WaterOut).map_or(stored_cap, |d| d.min(stored_cap));
    let generator_for_pump = animals_on_well
        && !h.has_generator()
        && days_of(BucketId::Power).is_some_and(|d| {
            d > animals_stored_days && power::generator_units(d, housing).is_some()
        });
    let longest = |a: BucketId, b: BucketId| match (days_of(a), days_of(b)) {
        (Some(x), Some(y)) => Some((x.max(y), if x >= y { a } else { b })),
        (Some(x), None) => Some((x, a)),
        (None, Some(y)) => Some((y, b)),
        (None, None) => None,
    };

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
                out.push(bucket, power::device_battery_units(people), Need, h72, true);
                out.push(bucket, Some(power::lights(people)), Need, h72, false);
                out.push(
                    bucket,
                    cited(bucket, power::battery_packs(days)),
                    Need,
                    h72,
                    false,
                );
                // The plan's 40 °F rule for food after a power cut needs a fridge thermometer.
                out.push(bucket, Some(power::fridge_thermometers()), Need, h72, false);
                let fuel = power::generator_fuel_gallons(days, housing);
                let fuel_gallons = fuel.as_ref().map_or(0.0, |f| f.quantity);
                if h.has_generator() {
                    out.push(bucket, cited(bucket, fuel), Need, None, false);
                    // Gasoline the plan buys needs approved cans first (CPSC): life-safety, so the
                    // cans come before the fuel, never after it.
                    out.push(
                        bucket,
                        cited(
                            bucket,
                            power::fuel_cans(fuel_gallons, power::GeneratorFor::Owned),
                        ),
                        Need,
                        Some(tier_for_days(days)),
                        true,
                    );
                } else if generator_for_pump {
                    // The fuel to keep for the generator the plan buys for the pump: said, but not
                    // a need of its own, so the plan never buys fuel before the generator. The
                    // cans to keep it in are part of the generator's purchase.
                    out.push(bucket, cited(bucket, fuel), Note, None, false);
                    out.push(
                        bucket,
                        cited(
                            bucket,
                            power::fuel_cans(fuel_gallons, power::GeneratorFor::Planned),
                        ),
                        Need,
                        Some(tier_for_days(days)),
                        false,
                    );
                }
                // A generator reaches a hardwired well pump only through an electrician-installed
                // interlock or transfer switch. For a generator the household owns it is
                // life-safety (the backfeed risk is there now); for the one the plan buys it is
                // part of that purchase.
                let generator = if h.has_generator() {
                    Some(power::GeneratorFor::Owned)
                } else if generator_for_pump {
                    Some(power::GeneratorFor::Planned)
                } else {
                    None
                };
                if let (true, Some(g)) = (well, generator) {
                    out.push(
                        bucket,
                        power::generator_connection_units(housing, g),
                        Need,
                        Some(tier_for_days(days)),
                        g == power::GeneratorFor::Owned,
                    );
                }
                out.push(bucket, power::wheelchair_battery(people), Need, h72, false);
                if let Some((s, needed)) = power::power_station_units(days, people, housing) {
                    let shape = if needed { Need } else { Optional };
                    out.push(
                        bucket,
                        cited(bucket, Some(s)),
                        shape,
                        Some(tier_for_days(days)),
                        needed,
                    );
                }
                if generator_for_pump {
                    out.push(
                        bucket,
                        cited(
                            bucket,
                            power::generator_for_well_pump(
                                days,
                                housing,
                                pets.large_animals,
                                animals_stored_days,
                            ),
                        ),
                        Need,
                        Some(tier_for_days(days)),
                        false,
                    );
                } else {
                    out.push(
                        bucket,
                        cited(bucket, power::generator_units(days, housing)),
                        Optional,
                        Some(tier_for_days(days)),
                        false,
                    );
                }
                out.push(
                    bucket,
                    cited(bucket, power::solar_panel_units(days, ctx.latitude, people)),
                    Optional,
                    Some(tier_for_days(days)),
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
            }
            BucketId::WaterBoil => {
                let Some(days) = days_of(bucket) else {
                    continue;
                };
                let treat = water::water_treatment_boil(days, people, pets, hot);
                let fuel = water::boil_fuel(treat.quantity);
                out.push(bucket, cited(bucket, Some(treat)), Need, None, false);
                // One bottle whatever the target: it does not depend on the days.
                out.push(bucket, Some(water::bleach_bottles()), Need, h72, false);
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
                let (stored, treat) = water::water_storage(days, people, pets, level, hot, well);
                out.push(bucket, cited(bucket, Some(stored)), Need, None, true);
                out.push(
                    bucket,
                    Some(water::water_reused_bottles(people, pets, level, hot)),
                    Shape::Alternative {
                        of: "water_gallons",
                        variant: "reused_bottles",
                    },
                    now,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, treat),
                    Need,
                    Some(tier_for_days(days)),
                    false,
                );
                // Bleach goes with the boil-water lines when there are any, otherwise here.
                if days_of(BucketId::WaterBoil).is_none() {
                    out.push(bucket, Some(water::bleach_bottles()), Need, h72, false);
                }
                let drought = t.driven_by(bucket, &[HazardId::Drought]) == Some(true);
                out.push(
                    bucket,
                    cited(
                        bucket,
                        water::livestock_water(days, pets.large_animals, pump_power, drought),
                    ),
                    Need,
                    None,
                    false,
                );
                if pump_power {
                    out.push(
                        bucket,
                        cited(
                            bucket,
                            water::livestock_water_stored(days, pets.large_animals),
                        ),
                        Shape::Alternative {
                            of: "livestock_water",
                            variant: "stock_tank",
                        },
                        None,
                        false,
                    );
                }
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
                    cited(bucket, food::infant_formula_oz(days, people)),
                    Need,
                    None,
                    true,
                );
                out.push(bucket, food::nursing_supplies(people), Need, h72, false);
                out.push(
                    bucket,
                    cited(bucket, food::pet_food_lb(days, pets)),
                    Need,
                    None,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, food::pet_food_days(days, pets)),
                    Shape::Alternative {
                        of: "pet_food_lb",
                        variant: "pet_days",
                    },
                    None,
                    false,
                );
                if let Some((fuel_days, src)) = longest(BucketId::Supplies, BucketId::WaterBoil) {
                    out.push(
                        bucket,
                        cited(src, food::cooking_fuel_canisters(fuel_days, people)),
                        Optional,
                        tier,
                        false,
                    );
                }
                if let Some((tp_days, src)) = longest(BucketId::Supplies, BucketId::WaterOut) {
                    out.push(
                        bucket,
                        cited(src, sanitation::toilet_paper_rolls(tp_days, people)),
                        Need,
                        Some(tier_for_days(tp_days)),
                        false,
                    );
                }
                out.push(
                    bucket,
                    cited(bucket, sanitation::soap_person_months(days, people)),
                    Need,
                    tier,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, sanitation::menstrual_cycles(days, people)),
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
                let Some(days) = days_of(bucket) else {
                    continue;
                };
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
                    out.push(bucket, Some(thermal::room_thermometer()), Need, h72, false);
                }
                if cold == Some(true) || neither {
                    // Blankets and warm layers first (most homes have them); a sleeping bag or an
                    // extra heavy blanket only for a long target or someone 65 or over.
                    out.push(bucket, thermal::blankets(people), Need, h72, false);
                    out.push(bucket, Some(thermal::warm_layers(people)), Need, h72, false);
                    if let Some((s, needed)) =
                        thermal::sleeping_bag_or_blanket(days, people, housing)
                    {
                        // The target's sources stand behind it only when its days decided it.
                        let s = if s.days.is_some() {
                            s.also_cite(t.sources(bucket))
                        } else {
                            s
                        };
                        out.push(
                            bucket,
                            Some(s),
                            if needed { Need } else { Optional },
                            Some(tier_for_days(days)),
                            false,
                        );
                    }
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
                        cited(
                            src,
                            medication::rx_cold_storage(
                                cd,
                                days_of(BucketId::Power),
                                people,
                                housing,
                            ),
                        ),
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
                    cited(bucket, Some(comms::cash_reserve_usd(days))),
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
                // The go-bags' water and food, and the pet go-kit's, are staged from the
                // household's own supplies: alternative lines of the bag, never additions.
                out.push(
                    bucket,
                    cited(
                        bucket,
                        Some(evacuate::go_bag_water(people, level, hot, e.days_away)),
                    ),
                    Shape::Alternative {
                        of: "go_bag",
                        variant: "staged_water",
                    },
                    h72,
                    false,
                );
                out.push(
                    bucket,
                    cited(bucket, evacuate::go_bag_food(people, e.days_away)),
                    Shape::Alternative {
                        of: "go_bag",
                        variant: "staged_food",
                    },
                    h72,
                    false,
                );
                out.push(bucket, evacuate::pet_carrier(pets), Need, h72, false);
                out.push(
                    bucket,
                    evacuate::pet_go_water(pets),
                    Shape::Alternative {
                        of: "pet_carrier",
                        variant: "staged_water",
                    },
                    h72,
                    false,
                );
                out.push(
                    bucket,
                    evacuate::pet_go_food(pets),
                    Shape::Alternative {
                        of: "pet_carrier",
                        variant: "staged_food",
                    },
                    h72,
                    false,
                );
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
                // Each commuter's bag, with its water and snacks staged from home: alternative
                // lines of that person's bag, never additions.
                for (index, commute) in h.commuters() {
                    out.push(
                        bucket,
                        Some(get_home::get_home_bag(index, commute)),
                        Shape::PerPerson(index),
                        h72,
                        false,
                    );
                    out.push(
                        bucket,
                        Some(get_home::get_home_water(index, commute, hot)),
                        Shape::PersonAlternative {
                            of: "get_home_bag",
                            variant: "staged_water",
                            person: index,
                        },
                        h72,
                        false,
                    );
                    out.push(
                        bucket,
                        Some(get_home::get_home_food(index, commute)),
                        Shape::PersonAlternative {
                            of: "get_home_bag",
                            variant: "staged_food",
                            person: index,
                        },
                        h72,
                        false,
                    );
                }
                out.push(bucket, get_home::car_kit(h.vehicles()), Need, h72, false);
            }
            BucketId::MedicalEmergency => {
                let supplies = days_of(BucketId::Supplies);
                let days = supplies.unwrap_or(f64::from(TierId::W2.days()));
                // The supplies target's sources stand behind the medicine counts only when its days
                // size them (two weeks otherwise).
                let by_days = |s: Sizing| {
                    if supplies.is_some() {
                        s.also_cite(t.sources(BucketId::Supplies))
                    } else {
                        s
                    }
                };
                out.push(
                    bucket,
                    Some(first_aid::first_aid_kit(people)),
                    Need,
                    h72,
                    false,
                );
                out.push(
                    bucket,
                    Some(by_days(first_aid::otc_medicines(days, people))),
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
                    Some(by_days(first_aid::ors_packets(days, people))),
                    Need,
                    h72,
                    false,
                );
                // Life-safety where the household is rural or a medical emergency is likely.
                let p_medical = t
                    .get(BucketId::MedicalEmergency)
                    .and_then(|a| match a.target {
                        Target::Readiness { p_need_10yr, .. } => Some(p_need_10yr),
                        _ => None,
                    });
                let (kit, life_safety) = first_aid::bleeding_control_kit(
                    input.location.setting == Setting::Rural,
                    p_medical,
                );
                out.push(bucket, Some(kit), Need, h72, life_safety);
            }
            BucketId::Fire => {
                out.push(
                    bucket,
                    Some(fire::fire_escape_plan(housing)),
                    Need,
                    now,
                    false,
                );
                // Renters ask the landlord first: a note, not a purchase (round-2 review RR-P16).
                if let Some((s, need)) = fire::smoke_alarm_count(housing, people.len()) {
                    out.push(bucket, Some(s), if need { Need } else { Note }, h72, need);
                }
                out.push(bucket, fire::co_alarm_count(housing), Need, h72, true);
                out.push(bucket, fire::extinguisher_count(housing), Need, h72, false);
                out.push(bucket, fire::escape_ladder_count(housing), Need, h72, false);
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
