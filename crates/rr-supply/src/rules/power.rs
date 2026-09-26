//! Power (research §6): powered medical devices, lights, generator fuel, what a power station or a
//! fridge takes, a well pump, winter solar, and spare wheelchair batteries.

use rr_types::{Housing, HousingKind, Mobility, Per, Person, PoweredDevice, WaterSource};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{count, days as fmt_days, gallons, num};
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

/// Battery lights: one per person aged 4 and over, at least one (an estimate). Rule `lights`.
pub fn lights(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let each = b.k(keys::LIGHTS_PER_PERSON);
    let n = people_list.iter().filter(|p| is_4_plus(p)).count().max(1) as f64;
    let q = (n * each).max(1.0);
    let text = format!(
        "{}: a headlamp or lantern for each person aged 4 and over, plus spare batteries. Use lights, not candles.",
        count(q, "battery light", "battery lights")
    );
    Sizing::new(&b, "lights", "light", q, "light", Per::Person, text)
}

/// Fuel for a generator the household owns, capped at what a home may store. Rule
/// `generator_fuel`.
pub fn generator_fuel(days: f64, housing: &Housing) -> Option<Sizing> {
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
            if indoor == 0.0 { "none".to_owned() } else { num(indoor, 0) }
        ));
    } else {
        text.push_str(&format!(
            " Store it in approved cans, no more than {} in an attached garage and {} indoors.",
            num(garage, 0),
            if indoor == 0.0 {
                "none".to_owned()
            } else {
                num(indoor, 0)
            }
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
            "generator_fuel",
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

/// The essential load a power station would carry: medical devices plus one phone per person
/// aged 13 and over. Optional line (rule `power_station_wh`).
pub fn power_station_wh(days: f64, people_list: &[Person]) -> Option<Sizing> {
    if days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let devices = devices_wh_per_day(&mut b, people_list);
    let phones = people_list.iter().filter(|p| is_13_plus(p)).count() as f64;
    let phone_wh = if phones > 0.0 {
        b.k(keys::PHONE_WH_PER_DAY)
    } else {
        0.0
    };
    let per_day = devices + phones * phone_wh;
    if per_day <= 0.0 {
        return None;
    }
    let station = b.k(keys::POWER_STATION_WH);
    let usable_share = b.k(keys::POWER_STATION_USABLE_SHARE);
    let fridge = b.k(keys::FRIDGE_WH_PER_DAY);
    let usable = station * usable_share;
    let q = per_day * days;
    let text = format!(
        "If you want a battery power station: your essential load (medical devices and phones) is about {} Wh a day, {} Wh for {}. A {} Wh station gives about {} Wh after losses, enough for about {} of that load; a full-size fridge alone would drain it in about {}.",
        num(per_day, 0),
        num(super::round_quantity("Wh", q), 0),
        fmt_days(days),
        num(station, 0),
        num(usable, 0),
        fmt_days(crate::format::round_dp(usable / per_day, 1)),
        fmt_days(crate::format::round_dp(usable / fridge, 1))
    );
    Some(
        Sizing::new(
            &b,
            "power_station_wh",
            "power_station",
            q,
            "Wh",
            Per::Household,
            text,
        )
        .per_day(days, per_day),
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
    if chest.is_some() {
        b.cite("energy_star_freezers");
    }
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

/// Solar panel watts to cover the essential load on a December day at the county's latitude
/// (NREL PVWatts cities as a latitude-band proxy). Note line (rule `solar_panel_watts`).
pub fn solar_panel_watts(latitude: f64, people_list: &[Person]) -> Option<Sizing> {
    if !latitude.is_finite() {
        return None;
    }
    let mut b = Basis::new();
    let table = b.solar();
    let row = table
        .rows
        .iter()
        .find(|r| latitude >= r.lat[0] && latitude < r.lat[1])
        .or(table.rows.last())?;
    let dec = row.dec_wh_per_100w;
    let devices = devices_wh_per_day(&mut b, people_list);
    let phones = people_list.iter().filter(|p| is_13_plus(p)).count() as f64;
    let phone_wh = if phones > 0.0 {
        b.k(keys::PHONE_WH_PER_DAY)
    } else {
        0.0
    };
    let essential = devices + phones * phone_wh;
    let fridge = b.k(keys::FRIDGE_WH_PER_DAY);
    let watts = essential / dec * 100.0;
    let fridge_watts = fridge / dec * 100.0;
    let text = format!(
        "Solar in winter: in December a 100 W panel near your latitude makes about {} Wh a day (like {}). Your essential load of about {} Wh a day needs about {} W of panels in full sun; a full-size fridge would need about {} W. Panels laid flat or in shade make less.",
        num(dec, 0),
        row.city,
        num(essential, 0),
        num(super::round_quantity("watt", watts), 0),
        num(super::round_quantity("watt", fridge_watts), 0)
    );
    Some(
        Sizing::new(
            &b,
            "solar_panel_watts",
            "solar_panel",
            watts,
            "watt",
            Per::Household,
            text,
        )
        .math(vec![format!(
            "{} Wh/day ÷ {} Wh per 100 W × 100 = {} W",
            num(essential, 1),
            num(dec, 0),
            num(watts, 1)
        )]),
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
        let s = generator_fuel(13.0, &p.housing).unwrap();
        // 13 × 2.8 = 36.4 gal, more than 25 gal: store 25.
        assert_eq!(s.quantity, 25.0);
        assert!(s.plain.contains("36.4 gallons"), "{}", s.plain);
        assert!(s.plain.contains("20 feet"));
        let short = generator_fuel(3.0, &p.housing).unwrap();
        assert_eq!(short.quantity, 8.4);
    }

    #[test]
    fn optional_loads() {
        assert_eq!(fridge_wh(3.0).unwrap().quantity, 2990.0); // 995 × 3 = 2,985 → 2,990
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
        assert_eq!(
            well_pump_wh(13.0, &coos.housing).unwrap().quantity,
            16_250.0
        );
        let philly = fixtures::get("philadelphia-renters-4").unwrap();
        assert!(well_pump_wh(3.0, &philly.housing).is_none());
        let station = power_station_wh(3.0, &philly.people).unwrap();
        assert_eq!(station.quantity, 140.0); // 3 phones × 15 × 3 = 135 → 140
    }

    #[test]
    fn solar_uses_the_latitude_band() {
        let philly = fixtures::get("philadelphia-renters-4").unwrap();
        // 45 Wh a day of phones ÷ 261 Wh per 100 W in December at 40° N = 17 W
        let s = solar_panel_watts(39.95, &philly.people).unwrap();
        assert_eq!(s.quantity, 20.0);
        assert!(s.plain.contains("261 Wh"));
        assert!(
            s.plain.contains("380 W"),
            "a fridge needs 995 ÷ 261 × 100 = 381 W: {}",
            s.plain
        );
        let seattle = solar_panel_watts(47.6, &philly.people).unwrap();
        assert!(seattle.plain.contains("121 Wh"));
        assert!(s.prior);
    }

    #[test]
    fn lights_and_batteries() {
        let philly = fixtures::get("philadelphia-renters-4").unwrap();
        assert_eq!(lights(&philly.people).quantity, 4.0);
        assert!(wheelchair_battery(&philly.people).is_none());
    }
}
