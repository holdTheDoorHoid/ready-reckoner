//! Power (research §6): powered medical devices and their batteries, lights and batteries, what a
//! power station, a generator, a fridge, a well pump or a solar panel takes, and spare wheelchair
//! batteries.

use rr_types::{Housing, HousingKind, Mobility, Per, Person, PoweredDevice, Tenure, WaterSource};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{ceil_count, count, day_adjective, days as fmt_days, gallons, num, round_dp};
use crate::household::{is_4_plus, is_13_plus};

/// Watt-hours a day for one person's powered medical device, with a short description.
fn device_wh_per_day(b: &mut Basis, d: PoweredDevice) -> Option<(f64, String)> {
    match d {
        PoweredDevice::None => None,
        PoweredDevice::Cpap => {
            let wh = b.k(keys::CPAP_WH_PER_NIGHT);
            let dry = b.k(keys::CPAP_WH_PER_NIGHT_DRY);
            Some((
                wh,
                format!(
                    "a CPAP uses about {} Wh a night with the humidifier on (about {} with it off)",
                    num(wh, 0),
                    num(dry, 0)
                ),
            ))
        }
        PoweredDevice::Oxygen => {
            let w = b.k(keys::OXYGEN_CONCENTRATOR_W);
            let hours = b.k(keys::DEVICE_HOURS_PER_DAY);
            Some((
                w * hours,
                format!(
                    "an oxygen concentrator draws about {} W (use the watts on its label) for {} hours a day",
                    num(w, 0),
                    num(hours, 0)
                ),
            ))
        }
        PoweredDevice::Other { watts } => {
            let hours = b.k(keys::DEVICE_HOURS_PER_DAY);
            let w = f64::from(watts).max(0.0);
            Some((
                w * hours,
                format!(
                    "a {} W medical device, counted as running {} hours a day (less if it runs part of the day)",
                    num(w, 0),
                    num(hours, 0)
                ),
            ))
        }
    }
}

/// The household's powered-device energy per day, in watt-hours.
pub(crate) fn devices_wh_per_day(b: &mut Basis, people_list: &[Person]) -> f64 {
    people_list
        .iter()
        .filter_map(|p| device_wh_per_day(b, p.medical.powered_device).map(|(wh, _)| wh))
        .sum()
}

/// (devices, devices plus one phone per person aged 13 and over), in watt-hours a day.
fn essential_wh_per_day(b: &mut Basis, people_list: &[Person]) -> (f64, f64) {
    let devices = devices_wh_per_day(b, people_list);
    let phones = people_list.iter().filter(|p| is_13_plus(p)).count() as f64;
    let phone_wh = if phones > 0.0 {
        b.k(keys::PHONE_WH_PER_DAY)
    } else {
        0.0
    };
    (devices, devices + phones * phone_wh)
}

/// Energy for powered medical devices through the outage target (life-safety). Rule
/// `medical_device_wh`.
pub fn medical_device_wh(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let mut b = Basis::new();
    let mut per_day = 0.0;
    let mut parts = Vec::new();
    let mut oxygen = false;
    for p in people_list {
        if let Some((wh, words)) = device_wh_per_day(&mut b, p.medical.powered_device) {
            per_day += wh;
            parts.push(words);
            oxygen |= p.medical.powered_device == PoweredDevice::Oxygen;
        }
    }
    if parts.is_empty() || days <= 0.0 {
        return None;
    }
    let q = per_day * days;
    let mut text = format!(
        "Backup power for medical equipment: {}. That is about {} Wh a day × {} = {} Wh. Keep spare batteries charged, and ask your utility about its list for customers who need power for medical equipment.",
        crate::format::and_list(&parts),
        num(per_day, 0),
        fmt_days(days),
        num(super::round_quantity("Wh", q), 0)
    );
    b.cite("ready_gov_disability");
    b.cite("ready_gov_power_outages");
    if oxygen {
        text.push_str(" Batteries cannot carry an oxygen concentrator for long: ask your supplier about backup oxygen.");
    }
    Some(
        Sizing::new(
            &b,
            "medical_device_wh",
            "medical_device_power",
            q,
            "Wh",
            Per::Person,
            text,
        )
        .per_day(days, per_day)
        .math(vec![format!(
            "{} Wh/day × {} days = {} Wh",
            num(per_day, 1),
            num(days, 2),
            num(q, 1)
        )]),
    )
}

/// A spare battery for each powered medical device (Ready.gov: keep extra batteries charged).
/// Rule `device_battery_units`.
pub fn device_battery_units(people_list: &[Person]) -> Option<Sizing> {
    let n = people_list
        .iter()
        .filter(|p| p.medical.powered_device != PoweredDevice::None)
        .count() as f64;
    if n == 0.0 {
        return None;
    }
    let mut b = Basis::new();
    b.cite("ready_gov_disability");
    b.cite("sil_cpap_power");
    let text = format!(
        "{} for each powered medical device: ask the maker which battery fits and how long it runs, and keep it charged.",
        count(n, "spare battery", "spare batteries")
    );
    Some(Sizing::new(
        &b,
        "device_battery_units",
        "device_battery",
        n,
        "battery",
        Per::Person,
        text,
    ))
}

/// Battery lights: one per person aged 4 and over, at least one (an estimate). Rule `lights`.
pub fn lights(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let each = b.k(keys::LIGHTS_PER_PERSON);
    b.cite("ready_gov_power_outages");
    let n = people_list.iter().filter(|p| is_4_plus(p)).count().max(1) as f64;
    let q = (n * each).max(1.0);
    let text = format!(
        "{}: a headlamp or lantern for each person aged 4 and over. Use lights, not candles.",
        count(q, "battery light", "battery lights")
    );
    Sizing::new(&b, "lights", "light", q, "light", Per::Person, text)
}

/// Packs of AA or AAA batteries for lights and the radio: one pack a week (an estimate). Rule
/// `battery_packs`.
pub fn battery_packs(days: f64) -> Option<Sizing> {
    if days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per = b.k(keys::BATTERY_PACK_DAYS);
    let q = ceil_count((days / per).max(1.0));
    let text = format!(
        "{} of AA or AAA batteries for the lights and the radio: one pack lasts a household about {}. Check which sizes your lights use.",
        count(q, "pack", "packs"),
        fmt_days(per)
    );
    Some(Sizing::new(
        &b,
        "battery_packs",
        "battery_pack",
        q,
        "pack",
        Per::Household,
        text,
    ))
}

/// Fuel for a generator the household owns: 2.8 gallons a day at light load, capped at what a home
/// may store. Rule `generator_fuel_gallons`.
pub fn generator_fuel_gallons(days: f64, housing: &Housing) -> Option<Sizing> {
    if days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per_day = b.k(keys::GENERATOR_GAL_PER_DAY);
    let (_, full) = b.range(keys::GENERATOR_GAL_PER_DAY);
    let max = b.k(keys::FUEL_STORAGE_MAX_GAL);
    let garage = b.k(keys::FUEL_STORAGE_ATTACHED_GARAGE_GAL);
    let indoor = b.k(keys::FUEL_STORAGE_INDOOR_GAL);
    let rotate = b.k(keys::FUEL_ROTATION_MONTHS);
    let clearance = b.k(keys::GENERATOR_CLEARANCE_FT);
    let need = per_day * days;
    let q = need.min(max);
    let indoors = if indoor == 0.0 {
        "none".to_owned()
    } else {
        num(indoor, 0)
    };
    let mut text = format!(
        "Your generator burns about {} of fuel a day at light load (up to {} at full load): {} is about {}.",
        gallons(per_day),
        gallons(full),
        fmt_days(days),
        gallons(super::round_quantity("gallon", need))
    );
    if need > max {
        text.push_str(&format!(
            " That is more than the {} most fire codes let a home store, so keep up to {} in approved cans ({} in an attached garage, {} indoors) and plan to refuel.",
            gallons(max),
            gallons(max),
            num(garage, 0),
            indoors
        ));
    } else {
        text.push_str(&format!(
            " Store it in approved cans, no more than {} in an attached garage and {} indoors.",
            num(garage, 0),
            indoors
        ));
    }
    text.push_str(&format!(
        " Use stored fuel within {}. Run the generator outdoors only, at least {} feet from windows, doors and vents.",
        count(rotate, "month", "months"),
        num(clearance, 0)
    ));
    if matches!(
        housing.kind,
        HousingKind::ApartmentHighRise | HousingKind::ApartmentLowRise
    ) {
        text.push_str(" Most apartments have no safe spot for that; a battery power station is the safer choice.");
    }
    Some(
        Sizing::new(
            &b,
            "generator_fuel_gallons",
            "generator_fuel",
            q,
            "gallon",
            Per::Household,
            text,
        )
        .per_day(days, per_day)
        .math(vec![format!(
            "min({} gal/day × {} days, {} gal) = {} gal",
            num(per_day, 2),
            num(days, 2),
            num(max, 0),
            num(q, 2)
        )]),
    )
}

/// The item class of the power station line when it keeps refrigerated medicine cold (a need, see
/// [`crate::rules::medication::rx_power_needed`]). rr-plan names the part after it ("power for cold
/// medicine"), so the station is its own part of the power bucket rather than the medical-device
/// part, and the budget's cold-chain guardrail looks for that part.
pub const COLD_MEDICINE_POWER_CLASS: &str = "power_for_cold_medicine";

/// How many power stations: enough for a powered medical device through the outage (at most two,
/// an estimate); one to keep refrigerated medicine cool when the power target is 2 days or more and
/// the household has no backup power (a need: a cooler bag lasts about a day); one as an option
/// for refrigerated medicine otherwise; else one as an optional upgrade when the outage target is
/// 3 days or more. The flag says whether it is needed. Rule `power_station_units`.
pub fn power_station_units(
    days: f64,
    people_list: &[Person],
    housing: &Housing,
) -> Option<(Sizing, bool)> {
    if days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let (devices, essential) = essential_wh_per_day(&mut b, people_list);
    let station = b.k(keys::POWER_STATION_WH);
    let usable = station * b.k(keys::POWER_STATION_USABLE_SHARE);
    let fridge = b.k(keys::FRIDGE_WH_PER_DAY);
    let cold_rx = people_list.iter().any(|p| p.medical.refrigerated_rx);
    let cold_needed = super::medication::rx_power_needed(Some(days), people_list, housing);
    let mut class = "power_station";
    let (units, needed, why) = if devices > 0.0 {
        let max_units = b.k(keys::POWER_STATION_MAX_UNITS);
        let u = ceil_count(devices * days / usable).clamp(1.0, max_units);
        (
            u,
            true,
            "to run the medical equipment through the outage".to_owned(),
        )
    } else if cold_needed {
        b.k(keys::RX_POWER_MIN_DAYS);
        let hold = b.k(keys::COOLER_HOLD_DAYS);
        class = COLD_MEDICINE_POWER_CLASS;
        (
            1.0,
            true,
            format!(
                "to keep refrigerated medicine cool through a {} power cut, because a cooler bag lasts only about {}",
                day_adjective(days),
                fmt_days(hold)
            ),
        )
    } else if cold_rx {
        b.cite("ready_gov_power_outages");
        (
            1.0,
            false,
            "to keep refrigerated medicine cold in a small cooler".to_owned(),
        )
    } else {
        let from = b.k(keys::POWER_STATION_UPGRADE_DAYS);
        if days < from {
            return None;
        }
        (
            1.0,
            false,
            "as an optional upgrade for lights, phones and a radio".to_owned(),
        )
    };
    let load = if devices > 0.0 {
        "medical equipment and phones"
    } else {
        "phones"
    };
    let mut text = format!(
        "{} (about {} Wh each) {why}. Your essential load ({load}) is about {} Wh a day, {} Wh for {}; one station gives about {} Wh after losses, enough for about {} of that load. A full-size fridge alone would drain one in about {}.",
        count(units, "battery power station", "battery power stations"),
        num(station, 0),
        num(essential, 0),
        num(super::round_quantity("Wh", essential * days), 0),
        fmt_days(days),
        num(usable, 0),
        fmt_days(round_dp(usable / essential.max(f64::MIN_POSITIVE), 1)),
        fmt_days(round_dp(usable / fridge, 1))
    );
    if cold_needed && devices <= 0.0 {
        b.cite("cdc_co_basics");
        text.push_str(" It can run a small 12-volt cooler for the medicine, or the fridge in short spells. Recharge it from a car, gas or electric, with the engine running outdoors only, or from a solar panel.");
    } else if cold_rx && devices > 0.0 {
        text.push_str(" It can also keep refrigerated medicine cool in a small 12-volt cooler.");
    }
    let sizing = Sizing::new(
        &b,
        "power_station_units",
        class,
        units,
        "power station",
        Per::Household,
        text,
    )
    .math(vec![if devices <= 0.0 && cold_rx {
        format!(
            "refrigerated medicine, {} days of power target: {} station ({})",
            num(days, 2),
            num(units, 0),
            if needed { "a need" } else { "optional" }
        )
    } else {
        format!(
            "devices {} Wh/day × {} days ÷ {} usable Wh = {} stations",
            num(devices, 1),
            num(days, 2),
            num(usable, 1),
            num(units, 0)
        )
    }]);
    Some((sizing, needed))
}

/// A generator for a house: offered when the outage target is 3 days or more, or 1 day or more on
/// a well; never for apartments (no safe spot 20 feet from openings). Optional line (rule
/// `generator_units`).
pub fn generator_units(days: f64, housing: &Housing) -> Option<Sizing> {
    if days <= 0.0
        || matches!(
            housing.kind,
            HousingKind::ApartmentHighRise | HousingKind::ApartmentLowRise
        )
    {
        return None;
    }
    let mut b = Basis::new();
    b.cite("cdc_co_basics");
    b.cite("ready_gov_power_outages");
    let well = housing.water == WaterSource::Well;
    let from = if well {
        b.k(keys::GENERATOR_MIN_DAYS_WELL)
    } else {
        b.k(keys::GENERATOR_MIN_DAYS)
    };
    if days < from {
        return None;
    }
    let clearance = b.k(keys::GENERATOR_CLEARANCE_FT);
    let mut text = format!(
        "Optional: a portable generator can run the fridge and lights through a {} outage. It must run outdoors, at least {} feet from windows, doors and vents, never in a garage, and it needs fuel (see the fuel line).",
        day_adjective(days),
        num(clearance, 0)
    );
    if well {
        let (s_lo, s_hi) = b.range(keys::WELL_PUMP_START_MULTIPLIER);
        b.cite("cpsc_generator_alert_2021");
        b.cite("osha_portable_generators");
        text.push_str(&format!(
            " To run the well pump it must supply the pump's starting watts, {} to {} times its running watts, and 240 volts if the pump runs on 240 (check the nameplate), and it reaches the pump only through an inlet with an interlock or transfer switch that an electrician installs. Never plug a generator into a wall outlet: backfeeding can electrocute utility workers.",
            num(s_lo, 0),
            num(s_hi, 0)
        ));
    }
    if housing.tenure == Tenure::Rent {
        text.push_str(" Ask your landlord first.");
    }
    Some(Sizing::new(
        &b,
        "generator_units",
        "generator",
        1.0,
        "generator",
        Per::Household,
        text,
    ))
}

/// A solar panel, offered when the outage target is 7 days or more, with what it makes in December
/// at the county's latitude when that is known (NREL PVWatts cities as a latitude-band proxy).
/// Optional line (rule `solar_panel_units`).
pub fn solar_panel_units(
    days: f64,
    latitude: Option<f64>,
    people_list: &[Person],
) -> Option<Sizing> {
    let mut b = Basis::new();
    let from = b.k(keys::SOLAR_PANEL_MIN_DAYS);
    if days < from {
        return None;
    }
    let (_, essential) = essential_wh_per_day(&mut b, people_list);
    let fridge = b.k(keys::FRIDGE_WH_PER_DAY);
    let table = b.solar();
    let row = latitude.filter(|l| l.is_finite()).and_then(|lat| {
        table
            .rows
            .iter()
            .find(|r| lat >= r.lat[0] && lat < r.lat[1])
            .or(table.rows.last())
    });
    let mut text = format!(
        "Optional for a {} outage: a portable solar panel with a battery.",
        day_adjective(days)
    );
    let mut math = Vec::new();
    match row {
        Some(r) => {
            let dec = r.dec_wh_per_100w;
            let watts = essential / dec * 100.0;
            let fridge_watts = fridge / dec * 100.0;
            text.push_str(&format!(
                " In December a 100 W panel near your latitude makes about {} Wh a day (like {}). Your essential load of about {} Wh a day needs about {} W of panels in full sun; a full-size fridge would need about {} W. Panels laid flat or in shade make less.",
                num(dec, 0),
                r.city,
                num(essential, 0),
                num(super::round_quantity("watt", watts), 0),
                num(super::round_quantity("watt", fridge_watts), 0)
            ));
            math.push(format!(
                "{} Wh/day ÷ {} Wh per 100 W × 100 = {} W",
                num(essential, 1),
                num(dec, 0),
                num(watts, 1)
            ));
        }
        None => text.push_str(" Winter sun is weak: a 100 to 200 W panel keeps phones, lights, a radio and a CPAP going, but not a full-size fridge in a northern winter."),
    }
    Some(
        Sizing::new(
            &b,
            "solar_panel_units",
            "solar_panel",
            1.0,
            "panel",
            Per::Household,
            text,
        )
        .math(math),
    )
}

/// Energy to keep a refrigerator running (optional: shelf-stable food avoids the need). Rule
/// `fridge_wh`.
pub fn fridge_wh(days: f64) -> Option<Sizing> {
    if days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per_day = b.k(keys::FRIDGE_WH_PER_DAY);
    let chest = b
        .constant(keys::FRIDGE_WH_PER_DAY)
        .alternatives
        .iter()
        .find(|a| a.label == "Chest freezer")
        .map(|a| a.value);
    let fridge_hours = b.k(keys::FRIDGE_COLD_HOURS);
    let freezer_hours = b.k(keys::FREEZER_COLD_HOURS);
    let q = per_day * days;
    let text = format!(
        "Optional: keeping a fridge running takes about {} Wh a day{} × {} = {} Wh. A closed fridge keeps food cold about {} hours and a full freezer about {}; shelf-stable food avoids the need.",
        num(per_day, 0),
        chest.map_or(String::new(), |c| format!(
            " (a chest freezer about {})",
            num(c, 0)
        )),
        fmt_days(days),
        num(super::round_quantity("Wh", q), 0),
        num(fridge_hours, 0),
        num(freezer_hours, 0)
    );
    Some(
        Sizing::new(
            &b,
            "fridge_wh",
            "fridge_power",
            q,
            "Wh",
            Per::Household,
            text,
        )
        .per_day(days, per_day),
    )
}

/// A generator for a household on a well with horses or livestock: a need, not an option, when
/// a power cut can outlast the animals' stored water. In a power cut it runs the well pump so the
/// animals have water after the `stored_days` their stored water covers (`livestock_water`), and
/// the fridge and lights too. It must be pump-rated (the pump's starting watts, 240 volts if the
/// pump runs on 240) and reaches the hardwired pump only through an electrician-installed inlet
/// with an interlock or transfer switch (`generator_connection_units`), never a wall outlet (CPSC,
/// OSHA). The same homes as [`generator_units`] (a house, and a power target of a day or more on a
/// well); `None` without large animals, off a well, or where a generator is not offered. Rule
/// `generator_units`.
pub fn generator_for_well_pump(
    days: f64,
    housing: &Housing,
    large_animals: u8,
    stored_days: f64,
) -> Option<Sizing> {
    if large_animals == 0 || housing.water != WaterSource::Well {
        return None;
    }
    generator_units(days, housing)?;
    let mut b = Basis::new();
    b.cite("cdc_co_basics");
    b.cite("ready_gov_power_outages");
    b.cite("aspca_disaster_prep");
    b.cite(crate::constants::PRIOR_SOURCE);
    let w = b.k(keys::WELL_PUMP_W);
    let (s_lo, s_hi) = b.range(keys::WELL_PUMP_START_MULTIPLIER);
    let clearance = b.k(keys::GENERATOR_CLEARANCE_FT);
    b.cite("cpsc_generator_alert_2021");
    b.cite("osha_portable_generators");
    let n = f64::from(large_animals);
    let mut text = format!(
        "A generator that can start your well pump, so your {} have water in a power cut that outlasts their {} of stored water; it runs the fridge and lights too. Starting a pump takes {} to {} times its running watts (about {} W running): check the pump's nameplate, and if it runs on 240 volts, choose a generator with a 240-volt outlet. It reaches the pump only through an inlet with an interlock or transfer switch that an electrician installs (see that line); never plug it into a wall outlet, because backfeeding can electrocute utility workers and neighbours. It must run outdoors, at least {} feet from windows, doors and vents, never in a garage, and its fuel goes in approved cans (see the fuel lines).",
        count(n, "large animal", "large animals"),
        fmt_days(stored_days),
        num(s_lo, 0),
        num(s_hi, 0),
        num(w, 0),
        num(clearance, 0)
    );
    if housing.tenure == Tenure::Rent {
        text.push_str(" Ask your landlord first.");
    }
    Some(Sizing::new(
        &b,
        "generator_units",
        "generator",
        1.0,
        "generator",
        Per::Household,
        text,
    ))
}

/// Whether the generator in [`generator_connection_units`] is one the household owns or one its
/// plan buys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorFor {
    /// A generator the household already has (`backup_power = generator`).
    Owned,
    /// The generator the plan buys to run the well pump for large animals.
    Planned,
}

/// An inlet with an interlock or transfer switch, installed by a licensed electrician, so a
/// generator can run a hardwired well pump safely: 1 for a house on a well that its household owns,
/// with a generator owned or planned. OSHA: connect a generator to a building only through a
/// transfer switch a qualified electrician installed; CPSC: plugging one into a wall outlet
/// ("backfeeding") can electrocute utility workers and neighbours. Renters ask the landlord, so
/// the line is only for owners. Rule `generator_connection_units`.
pub fn generator_connection_units(housing: &Housing, generator: GeneratorFor) -> Option<Sizing> {
    let house = matches!(
        housing.kind,
        HousingKind::Detached
            | HousingKind::Rowhouse
            | HousingKind::MobileHome
            | HousingKind::RuralProperty
    );
    if !house || housing.water != WaterSource::Well || housing.tenure != Tenure::Own {
        return None;
    }
    let mut b = Basis::new();
    b.cite("osha_portable_generators");
    b.cite("cpsc_generator_alert_2021");
    let whose = match generator {
        GeneratorFor::Owned => "Your generator",
        GeneratorFor::Planned => "The generator for the well pump",
    };
    let text = format!(
        "{whose} can run the well pump, which is wired into the house, only through an inlet with an interlock or transfer switch that a licensed electrician installs: 1 installed kit, with the pump's circuit on it. Never plug a generator into a wall outlet or connect it to the house wiring any other way: backfeeding can electrocute utility workers and neighbours, and OSHA says to connect a generator to a building only through a transfer switch a qualified electrician installed."
    );
    // For a generator the household owns, the connection is its own part of the power bucket (a
    // life-safety need: the backfeed risk is there now). For the generator the plan buys, it
    // shares the generator's part with the generator and its fuel cans, so the three are valued
    // as one purchase and the connection never outranks the rest of the plan before there is a
    // generator to connect.
    let class = match generator {
        GeneratorFor::Owned => "generator_connection",
        GeneratorFor::Planned => "generator",
    };
    Some(Sizing::new(
        &b,
        "generator_connection_units",
        class,
        1.0,
        "installed kit",
        Per::Household,
        text,
    ))
}

/// Approved gasoline cans for a generator's fuel: ceil(gallons ÷ 5), with fuel stabilizer, stored
/// outside the living space (CPSC) and used within six months. For a generator the household owns
/// the cans go with the fuel it buys (item class `generator_fuel`); for the generator the plan buys
/// for the well pump they are part of that purchase (item class `generator`). Rule `fuel_cans`.
pub fn fuel_cans(fuel_gallons: f64, generator: GeneratorFor) -> Option<Sizing> {
    if fuel_gallons <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per_can = b.k(keys::FUEL_CAN_GAL);
    let rotate = b.k(keys::FUEL_ROTATION_MONTHS);
    b.cite("cpsc_generator_alert_2021");
    let q = ceil_count(fuel_gallons / per_can);
    let text = format!(
        "{} for the generator's {} of fuel, with fuel stabilizer added: keep them outside the living space, away from a gas water heater, in labeled safety containers, and use the fuel within {}.",
        count(
            q,
            &format!("approved {}-gallon gasoline can", num(per_can, 0)),
            &format!("approved {}-gallon gasoline cans", num(per_can, 0))
        ),
        gallons(fuel_gallons),
        count(rotate, "month", "months")
    );
    let class = match generator {
        GeneratorFor::Owned => "generator_fuel",
        GeneratorFor::Planned => "generator",
    };
    Some(
        Sizing::new(&b, "fuel_cans", class, q, "can", Per::Household, text).math(vec![format!(
            "ceil({} gal ÷ {} gal a can) = {} cans",
            num(fuel_gallons, 2),
            num(per_can, 0),
            num(q, 0)
        )]),
    )
}

/// Thermometers for the fridge and the freezer whenever there is a power target: after a power
/// cut, the plan's own 40 °F rule (keep refrigerated food only if it stayed at 40 °F or below;
/// throw out perishable food above 40 °F for 2 hours or more) needs them (Ready.gov). Rule
/// `fridge_thermometers`.
pub fn fridge_thermometers() -> Sizing {
    let mut b = Basis::new();
    let q = b.k(keys::FRIDGE_THERMOMETER_PAIRS);
    let max_f = b.k(keys::FOOD_SAFE_MAX_F);
    let hours = b.k(keys::FOOD_UNSAFE_HOURS);
    let text = format!(
        "{} of appliance thermometers, one in the fridge and one in the freezer: after a power cut, keep refrigerated food only if it stayed at {} °F or below, and throw out perishable food that was above {} °F for {} or more.",
        count(q, "pair", "pairs"),
        num(max_f, 0),
        num(max_f, 0),
        count(hours, "hour", "hours")
    );
    Sizing::new(
        &b,
        "fridge_thermometers",
        "fridge_thermometer",
        q,
        "pair",
        Per::Household,
        text,
    )
}

/// Energy to run a well pump for essential water (an estimate; optional, because stored water
/// covers the same need more cheaply). Rule `well_pump_wh`.
pub fn well_pump_wh(days: f64, housing: &Housing) -> Option<Sizing> {
    if housing.water != WaterSource::Well || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let w = b.k(keys::WELL_PUMP_W);
    let (lo, hi) = b.range(keys::WELL_PUMP_W);
    let (s_lo, s_hi) = b.range(keys::WELL_PUMP_START_MULTIPLIER);
    let hours = b.k(keys::WELL_PUMP_HOURS_PER_DAY);
    let per_day = w * hours;
    let q = per_day * days;
    let text = format!(
        "Optional: your well pump needs about {} W to run ({} to {}) and {} to {} times that to start; check its nameplate. At {} a day that is {} Wh for {}. A battery power station usually cannot start it; a generator or a hand pump can. Stored water is the cheaper way to cover this.",
        num(w, 0),
        num(lo, 0),
        num(hi, 0),
        num(s_lo, 0),
        num(s_hi, 0),
        count(hours, "hour", "hours"),
        num(super::round_quantity("Wh", q), 0),
        fmt_days(days)
    );
    Some(
        Sizing::new(
            &b,
            "well_pump_wh",
            "well_pump_power",
            q,
            "Wh",
            Per::Household,
            text,
        )
        .per_day(days, per_day),
    )
}

/// A spare battery for each wheelchair user, if the chair is powered (Ready.gov). Rule
/// `wheelchair_battery`.
pub fn wheelchair_battery(people_list: &[Person]) -> Option<Sizing> {
    let n = people_list
        .iter()
        .filter(|p| p.medical.mobility == Mobility::Wheelchair)
        .count();
    if n == 0 {
        return None;
    }
    let mut b = Basis::new();
    let each = b.k(keys::WHEELCHAIR_SPARE_BATTERIES);
    let q = n as f64 * each;
    let text = format!(
        "If the wheelchair is powered, keep {} charged and ready.",
        count(q, "spare battery", "spare batteries")
    );
    Some(Sizing::new(
        &b,
        "wheelchair_battery",
        "wheelchair_battery",
        q,
        "battery",
        Per::Person,
        text,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn cpap_for_three_days() {
        let p = fixtures::get("phoenix-apartment-cpap-1").unwrap();
        let s = medical_device_wh(3.0, &p.people).unwrap();
        assert_eq!(s.quantity, 510.0); // 170 × 3
        assert!(s.citations.iter().any(|c| c == "sil_cpap_power"));
        assert!(!s.prior);
        assert_eq!(device_battery_units(&p.people).unwrap().quantity, 1.0);
        // 170 Wh × 3 nights = 510 Wh: one 1,070 Wh station (909.5 usable) covers it, and it is needed.
        let (station, needed) = power_station_units(3.0, &p.people, &p.housing).unwrap();
        assert_eq!(station.quantity, 1.0);
        assert!(needed);
        // 14 nights = 2,380 Wh: three stations' worth, capped at two.
        assert_eq!(
            power_station_units(14.0, &p.people, &p.housing)
                .unwrap()
                .0
                .quantity,
            2.0
        );
    }

    #[test]
    fn oxygen_is_an_estimate_and_warns() {
        let mut p = fixtures::get("phoenix-apartment-cpap-1").unwrap();
        p.people[0].medical.powered_device = PoweredDevice::Oxygen;
        let s = medical_device_wh(2.0, &p.people).unwrap();
        assert_eq!(s.quantity, 14_400.0); // 300 W × 24 h × 2
        assert!(s.prior);
        assert!(s.plain.contains("backup oxygen"));
        p.people[0].medical.powered_device = PoweredDevice::Other { watts: 60.0 };
        assert_eq!(medical_device_wh(1.0, &p.people).unwrap().quantity, 1440.0);
    }

    #[test]
    fn generator_fuel_is_capped_at_the_storage_limit() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        let s = generator_fuel_gallons(13.0, &p.housing).unwrap();
        // 13 × 2.8 = 36.4 gal, more than 25 gal: store 25.
        assert_eq!(s.quantity, 25.0);
        assert!(s.plain.contains("36.4 gallons"), "{}", s.plain);
        assert!(s.plain.contains("20 feet"));
        assert_eq!(
            generator_fuel_gallons(3.0, &p.housing).unwrap().quantity,
            8.4
        );
    }

    #[test]
    fn stations_generators_and_panels_follow_the_content_rules() {
        let philly = fixtures::get("philadelphia-renters-4").unwrap();
        // No device: an optional upgrade at 3 days or more, nothing below.
        let (s, needed) = power_station_units(3.0, &philly.people, &philly.housing).unwrap();
        assert_eq!((s.quantity, needed), (1.0, false));
        assert!(power_station_units(2.0, &philly.people, &philly.housing).is_none());
        // Houses get a generator from 3 days (1 on a well); apartments never.
        assert_eq!(generator_units(3.0, &philly.housing).unwrap().quantity, 1.0);
        assert!(generator_units(2.0, &philly.housing).is_none());
        assert!(
            generator_units(3.0, &philly.housing)
                .unwrap()
                .plain
                .contains("landlord")
        );
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
        let g = generator_units(1.0, &coos.housing).unwrap();
        assert!(
            g.plain.contains("well pump") && g.plain.contains("1-day outage"),
            "{}",
            g.plain
        );
        let miami = fixtures::get("miami-condo-retiree-1").unwrap();
        assert!(generator_units(30.0, &miami.housing).is_none());
        // Solar from 7 days.
        assert!(solar_panel_units(5.0, Some(40.0), &philly.people).is_none());
        let s = solar_panel_units(13.0, Some(39.95), &philly.people).unwrap();
        assert!(
            s.plain.contains("261 Wh") && s.plain.contains("380 W"),
            "{}",
            s.plain
        );
        assert!(s.prior);
        let seattle = solar_panel_units(13.0, Some(47.6), &philly.people).unwrap();
        assert!(seattle.plain.contains("121 Wh"));
        let unknown = solar_panel_units(13.0, None, &philly.people).unwrap();
        assert!(unknown.plain.contains("northern winter"));
    }

    #[test]
    fn optional_loads_and_batteries() {
        assert_eq!(fridge_wh(3.0).unwrap().quantity, 2990.0); // 995 × 3 = 2,985 → 2,990
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
        assert_eq!(
            well_pump_wh(13.0, &coos.housing).unwrap().quantity,
            16_250.0
        );
        let philly = fixtures::get("philadelphia-renters-4").unwrap();
        assert!(well_pump_wh(3.0, &philly.housing).is_none());
        assert_eq!(lights(&philly.people).quantity, 4.0);
        assert!(wheelchair_battery(&philly.people).is_none());
        assert_eq!(battery_packs(3.0).unwrap().quantity, 1.0);
        assert_eq!(battery_packs(13.0).unwrap().quantity, 2.0);
    }

    #[test]
    fn refrigerated_medicine_makes_a_power_station_a_need() {
        let sl = fixtures::get("sugar-land-ev-household-3").unwrap();
        // Insulin, a 5-day power target, no backup power (the electric car does not count):
        // one station, a need, in its own part of the power bucket.
        let (s, needed) = power_station_units(5.0, &sl.people, &sl.housing).unwrap();
        assert!(needed);
        assert_eq!(s.quantity, 1.0);
        assert_eq!(s.item_class, COLD_MEDICINE_POWER_CLASS);
        assert!(
            s.plain.contains("refrigerated medicine cool"),
            "{}",
            s.plain
        );
        assert!(
            s.plain.contains("engine running outdoors only"),
            "{}",
            s.plain
        );
        assert!(s.citations.iter().any(|c| c == "cdc_co_basics"));
        assert!(s.prior);
        // Under 2 days the bag is enough: the station is an option again.
        let (s, needed) = power_station_units(1.5, &sl.people, &sl.housing).unwrap();
        assert!(!needed);
        assert_eq!(s.item_class, "power_station");
        // A generator of their own already runs the fridge.
        let mut owns = sl.clone();
        owns.housing.backup_power = rr_types::BackupPower::Generator;
        let (_, needed) = power_station_units(5.0, &owns.people, &owns.housing).unwrap();
        assert!(!needed);
        // A CPAP user who also keeps insulin: the device sizes the stations; still a need.
        let mut both = fixtures::get("phoenix-apartment-cpap-1").unwrap();
        both.people[0].medical.refrigerated_rx = true;
        let (s, needed) = power_station_units(3.0, &both.people, &both.housing).unwrap();
        assert!(needed);
        assert_eq!(s.item_class, "power_station");
        assert!(s.plain.contains("refrigerated medicine"), "{}", s.plain);
    }

    #[test]
    fn the_pump_generator_is_pump_rated_and_connects_through_an_interlock() {
        let hays = fixtures::get("hays-kansas-farm-5").unwrap();
        let g = generator_for_well_pump(30.0, &hays.housing, 12, 14.0).unwrap();
        assert!(g.plain.contains("240-volt outlet"), "{}", g.plain);
        assert!(
            g.plain.contains("2 to 3 times its running watts"),
            "{}",
            g.plain
        );
        assert!(
            g.plain.contains("interlock or transfer switch"),
            "{}",
            g.plain
        );
        assert!(
            g.plain.contains("never plug it into a wall outlet"),
            "{}",
            g.plain
        );
        assert!(g.plain.contains("14 days of stored water"), "{}", g.plain);
        for id in ["cpsc_generator_alert_2021", "osha_portable_generators"] {
            assert!(g.citations.iter().any(|c| c == id), "{id}");
        }
        // The optional line on a well says the same about the pump; off a well it does not.
        let opt = generator_units(3.0, &hays.housing).unwrap();
        assert!(
            opt.plain.contains("interlock or transfer switch"),
            "{}",
            opt.plain
        );
        let philly = fixtures::get("philadelphia-renters-4").unwrap();
        assert!(
            !generator_units(3.0, &philly.housing)
                .unwrap()
                .plain
                .contains("interlock")
        );

        // The connection: owners of a house on a well, with a generator owned or planned.
        let c = generator_connection_units(&hays.housing, GeneratorFor::Planned).unwrap();
        assert_eq!((c.quantity, c.unit), (1.0, "installed kit"));
        assert_eq!(
            c.item_class, "generator",
            "valued with the generator it connects"
        );
        assert!(
            c.plain.starts_with("The generator for the well pump"),
            "{}",
            c.plain
        );
        assert!(c.plain.contains("licensed electrician"), "{}", c.plain);
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
        let c = generator_connection_units(&coos.housing, GeneratorFor::Owned).unwrap();
        assert!(c.plain.starts_with("Your generator"), "{}", c.plain);
        assert_eq!(c.item_class, "generator_connection");
        let mut renter = coos.clone();
        renter.housing.tenure = Tenure::Rent;
        assert!(generator_connection_units(&renter.housing, GeneratorFor::Owned).is_none());
        assert!(generator_connection_units(&philly.housing, GeneratorFor::Owned).is_none());
    }

    #[test]
    fn fuel_cans_hold_the_fuel_and_thermometers_check_the_fridge() {
        // 25 gallons of fuel: 5 cans; 8.4 gallons: 2 cans.
        let c = fuel_cans(25.0, GeneratorFor::Owned).unwrap();
        assert_eq!((c.quantity, c.unit), (5.0, "can"));
        assert_eq!(c.item_class, "generator_fuel");
        assert!(
            c.plain.contains("5 approved 5-gallon gasoline cans"),
            "{}",
            c.plain
        );
        assert!(c.plain.contains("fuel stabilizer"), "{}", c.plain);
        let p = fuel_cans(8.4, GeneratorFor::Planned).unwrap();
        assert_eq!(p.quantity, 2.0);
        assert_eq!(p.item_class, "generator");
        assert!(fuel_cans(0.0, GeneratorFor::Owned).is_none());
        let t = fridge_thermometers();
        assert_eq!((t.quantity, t.unit), (1.0, "pair"));
        assert!(t.plain.contains("40 °F or below"), "{}", t.plain);
        assert!(t.citations.iter().any(|c| c == "ready_gov_food"));
    }
}
