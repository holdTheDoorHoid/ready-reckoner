//! Plain-language wording shared by the sentences, drivers and summaries. Eighth-grade reading
//! level; natural frequencies before percentages; no false precision (docs/PRINCIPLES.md §3, §10,
//! research §7.1–§7.2).

use rr_types::{BucketId, HazardId};

/// A hazard as a plural noun phrase, for "how often … happen where you live".
#[allow(deprecated)] // the retired `terrorism` still needs an arm while the id exists
pub fn hazard_plural(h: HazardId) -> &'static str {
    use HazardId::*;
    match h {
        Avalanche => "avalanches",
        CoastalFlooding => "floods from the sea",
        ColdWave => "cold waves",
        Drought => "droughts",
        Earthquake => "earthquakes",
        Hail => "hailstorms",
        HeatWave => "heat waves",
        Hurricane => "hurricanes and tropical storms",
        IceStorm => "ice storms",
        Landslide => "landslides",
        Lightning => "lightning storms",
        RiverineFlooding => "floods from rivers and heavy rain",
        StrongWind => "windstorms",
        Tornado => "tornadoes",
        Tsunami => "tsunamis",
        VolcanicActivity => "volcanic eruptions",
        Wildfire => "wildfires",
        WinterWeather => "snow and ice storms",
        Pandemic => "pandemics",
        GridFailure => "regional blackouts",
        CyberOutage => "cyberattacks on services",
        CivilUnrest => "curfews and unrest",
        SupplyChainDisruption => "store shortages",
        HazmatRelease => "chemical spills and releases",
        NuclearPlantIncident => "nuclear plant accidents",
        NuclearAttack => "nuclear attacks",
        Terrorism => "terrorist attacks",
        JobLoss => "job losses",
        HouseFire => "home fires",
        MedicalEmergency => "medical emergencies",
        VehicleStranding => "breakdowns and road closures",
        LocalUtilityOutage => "local water and gas problems",
        Burglary => "break-ins",
        EarnerDeathOrDisability => "the death or disability of an earner",
        ExtendedHouseholdIllness => "long illnesses at home",
        WildfireSmoke => "spells of wildfire smoke",
        DustStorm => "dust storms",
        Sinkhole => "sinkholes and ground collapses",
        GeomagneticStorm => "severe solar storms",
        Vei7Eruption => "very large volcanic eruptions",
        DamFailure => "dam or levee failures",
        NetworkOutage => "phone and internet outages",
        DrugShortage => "medicine shortages",
        BenefitInterruption => "lapses in pay or benefits",
        AttackDisruption => "attacks or threats that close an area",
        MultiMonthBlackout => "power cuts lasting months",
        WarInfrastructure => "wars with attacks on US infrastructure",
        CbrnAttack => "chemical, biological or radiological attacks",
        SeverePandemic => "severe pandemics",
        FinancialCrisis => "financial crises with bank closures",
        MassViolence => "mass shootings or bombings",
        WaterDamage => "burst pipes and leaks",
        Eviction => "evictions",
        ArrestOrDetention => "arrests or detentions",
    }
}

/// A hazard as a single event with an article, for "depends mostly on one event: …".
#[allow(deprecated)] // the retired `terrorism` still needs an arm while the id exists
pub fn hazard_one(h: HazardId) -> &'static str {
    use HazardId::*;
    match h {
        Avalanche => "an avalanche",
        CoastalFlooding => "a flood from the sea",
        ColdWave => "a cold wave",
        Drought => "a drought",
        Earthquake => "an earthquake",
        Hail => "a hailstorm",
        HeatWave => "a heat wave",
        Hurricane => "a hurricane",
        IceStorm => "an ice storm",
        Landslide => "a landslide",
        Lightning => "a lightning storm",
        RiverineFlooding => "a flood",
        StrongWind => "a windstorm",
        Tornado => "a tornado",
        Tsunami => "a tsunami",
        VolcanicActivity => "a volcanic eruption",
        Wildfire => "a wildfire",
        WinterWeather => "a winter storm",
        Pandemic => "a pandemic",
        GridFailure => "a regional blackout",
        CyberOutage => "a cyberattack on services",
        CivilUnrest => "a curfew",
        SupplyChainDisruption => "a store shortage",
        HazmatRelease => "a chemical spill",
        NuclearPlantIncident => "a nuclear plant accident",
        NuclearAttack => "a nuclear attack",
        Terrorism => "a terrorist attack",
        JobLoss => "a job loss",
        HouseFire => "a home fire",
        MedicalEmergency => "a medical emergency",
        VehicleStranding => "a breakdown",
        LocalUtilityOutage => "a local water or gas problem",
        Burglary => "a break-in",
        EarnerDeathOrDisability => "the death or disability of an earner",
        ExtendedHouseholdIllness => "a long illness",
        WildfireSmoke => "a spell of wildfire smoke",
        DustStorm => "a dust storm",
        Sinkhole => "a sinkhole",
        GeomagneticStorm => "a severe solar storm",
        Vei7Eruption => "a very large volcanic eruption",
        DamFailure => "a dam or levee failure",
        NetworkOutage => "a phone or internet outage",
        DrugShortage => "a medicine shortage",
        BenefitInterruption => "a lapse in pay or benefits",
        AttackDisruption => "an attack or threat that closes the area",
        MultiMonthBlackout => "a power cut lasting months",
        WarInfrastructure => "a war with attacks on US infrastructure",
        CbrnAttack => "a chemical, biological or radiological attack",
        SeverePandemic => "a severe pandemic",
        FinancialCrisis => "a financial crisis with bank closures",
        MassViolence => "a mass shooting or bombing",
        WaterDamage => "a burst pipe or leak",
        Eviction => "an eviction",
        ArrestOrDetention => "an arrest or detention",
    }
}

/// "how often X {verb}" for a bucket.
pub fn bucket_cause_verb(b: BucketId) -> &'static str {
    use BucketId::*;
    match b {
        Power => "cut the power",
        WaterBoil => "bring a boil-water notice",
        WaterOut => "stop the tap water",
        Supplies => "keep people from shopping",
        Thermal => "leave homes dangerously hot or cold",
        Medication => "interrupt medicine",
        Comms => "cut phone, internet or card payments",
        Evacuate => "force people to leave home",
        GetHome => "strand people away from home",
        MedicalEmergency => "bring a medical emergency",
        Fire => "start a fire",
        Security => "threaten home security",
        CleanAir => "fill homes with smoke, dust or fumes",
        Income => "cut income",
        HomeLoss => "damage homes",
    }
}

/// The disruption as a plural noun, for "how long {noun} from X last".
pub fn bucket_noun_plural(b: BucketId) -> &'static str {
    use BucketId::*;
    match b {
        Power => "power cuts",
        WaterBoil => "boil-water notices",
        WaterOut => "water outages",
        Supplies => "times without shopping",
        Thermal => "spells of dangerous heat or cold",
        Medication => "gaps in medicine",
        Comms => "phone and payment outages",
        Evacuate => "times away from home",
        GetHome => "strandings",
        MedicalEmergency => "medical emergencies",
        Fire => "fires",
        Security => "security problems",
        CleanAir => "spells of unhealthy air",
        Income => "income gaps",
        HomeLoss => "displacements",
    }
}

/// The disruption as a singular noun, for "facing a longer {noun}".
pub fn bucket_noun(b: BucketId) -> &'static str {
    use BucketId::*;
    match b {
        Power => "power cut",
        WaterBoil => "boil-water notice",
        WaterOut => "water outage",
        Supplies => "stretch without shopping",
        Thermal => "spell of dangerous heat or cold",
        Medication => "gap in medicine",
        Comms => "outage",
        Evacuate => "time away",
        GetHome => "stranding",
        MedicalEmergency => "emergency",
        Fire => "fire",
        Security => "problem",
        CleanAir => "spell of unhealthy air",
        Income => "income gap",
        HomeLoss => "displacement",
    }
}

/// "will {verb} for a day or more".
pub fn bucket_verb(b: BucketId) -> &'static str {
    use BucketId::*;
    match b {
        Power => "lose grid power",
        WaterBoil => "have to boil or treat their tap water",
        WaterOut => "have no usable tap water",
        Supplies => "be unable to shop for food and supplies",
        Thermal => "face dangerous heat or cold at home with no working cooling or heating",
        Medication => "have trouble getting medicine",
        Comms => "lose phone, internet or card payments",
        Evacuate => "have to leave home quickly",
        GetHome => "have someone stranded away from home",
        MedicalEmergency => "have a medical emergency",
        Fire => "have a home fire",
        Security => "have a break-in or a nearby curfew",
        CleanAir => "need to keep smoke, dust or fumes out of the home",
        Income => "lose income",
        HomeLoss => "have to leave home for a while because of damage",
    }
}

/// Short label used in scenario summaries ("water: 14 days → 60 days").
pub fn bucket_short(b: BucketId) -> &'static str {
    use BucketId::*;
    match b {
        Power => "Power",
        WaterBoil => "Boil-water",
        WaterOut => "Tap water",
        Supplies => "Food and supplies",
        Thermal => "Heat or cold",
        Medication => "Medicine",
        Comms => "Phone and payments",
        Evacuate => "Leaving home",
        GetHome => "Getting home",
        MedicalEmergency => "Medical emergency",
        Fire => "Fire",
        Security => "Security",
        CleanAir => "Clean air",
        Income => "Income gap",
        HomeLoss => "Home damage",
    }
}

/// First letter lower-cased ("A magnitude 9 …" → "a magnitude 9 …").
pub fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The generic cause labels the data pack's outage job gives events it cannot name
/// (`rr-etl`, `jobs::outage_model::class_label`): "Winter storm, March 2018".
pub const GENERIC_EVENT_LABELS: [&str; 10] = [
    "Hurricane or tropical storm",
    "Ice storm",
    "Winter storm",
    "Wind and thunderstorms",
    "Wildfire or fire-weather shutoff",
    "Heat",
    "Cold and a grid emergency",
    "Flooding",
    "Grid failure",
    "Power cut, cause not recorded",
];

/// An event's name mid-sentence: a generic cause label loses its capital ("was winter storm,
/// March 2018"); a named storm or a dated event keeps its own ("was Hurricane Helene", "was
/// November 2018 Camp Fire").
pub fn event_mid_sentence(name: &str) -> String {
    let generic = GENERIC_EVENT_LABELS.iter().any(|g| {
        name.strip_prefix(g)
            .is_some_and(|rest| rest.chars().next().is_none_or(|c| !c.is_alphanumeric()))
    });
    if generic {
        lower_first(name)
    } else {
        name.to_owned()
    }
}

/// A share as the packet prints percentages, with no space before the sign: "30%".
pub fn percent(share: f64) -> String {
    format!("{}%", round_nice(100.0 * share))
}

/// A scenario's name as one event with an article, mid-sentence: "Magnitude 9 Cascadia
/// earthquake" becomes "a magnitude 9 Cascadia earthquake"; a name that already starts with an
/// article keeps it.
pub fn scenario_one(name: &str) -> String {
    let lower = lower_first(name.trim());
    if ["a ", "an ", "the "].iter().any(|a| lower.starts_with(a)) {
        return lower;
    }
    let article = match lower.chars().next() {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    };
    format!("{article} {lower}")
}

/// First letter upper-cased.
pub fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A ladder value in days, in words: "half a day", "a day", "3 days", "a week", "2 weeks",
/// "a month", "6 weeks", "2 months", "3 months", "6 months", "a year".
pub fn ladder_phrase(days: f32) -> String {
    const NAMED: [(f32, &str); 10] = [
        (1.0, "a day"),
        (7.0, "a week"),
        (14.0, "2 weeks"),
        (21.0, "3 weeks"),
        (30.0, "a month"),
        (45.0, "6 weeks"),
        (60.0, "2 months"),
        (90.0, "3 months"),
        (180.0, "6 months"),
        (365.0, "a year"),
    ];
    if days.is_nan() || days <= 0.0 {
        return "no time".to_owned();
    }
    if days < 1.0 {
        return "half a day".to_owned();
    }
    if days > 365.0 {
        return "a year".to_owned();
    }
    match NAMED.iter().find(|(d, _)| *d == days) {
        Some((_, name)) => (*name).to_owned(),
        None => days_phrase(f64::from(days)),
    }
}

/// Any duration in days, in words, without false precision (research §7.2): hours below a day,
/// whole days to two weeks, then weeks and months.
pub fn days_phrase(days: f64) -> String {
    if days.is_nan() || days <= 0.0 {
        return "no time".to_owned();
    }
    let hours = days * 24.0;
    if hours < 1.5 {
        return "about an hour".to_owned();
    }
    if days < 1.0 {
        return format!("{} hours", round_nice(hours));
    }
    if days < 1.5 {
        return "a day".to_owned();
    }
    if days < 10.5 {
        return format!("{} days", (days + 0.5).floor() as i64);
    }
    if days < 60.0 {
        return format!("{} weeks", (days / 7.0 + 0.5).floor() as i64);
    }
    if days < 365.0 {
        return format!("{} months", (days / 30.44 + 0.5).floor() as i64);
    }
    let years = days / 365.25;
    if years < 1.5 {
        "a year".to_owned()
    } else {
        format!("{} years", (years + 0.5).floor() as i64)
    }
}

/// Hours in words ("2 hours", "20 hours", "3 days").
pub fn hours_phrase(hours: f64) -> String {
    days_phrase(hours / 24.0)
}

/// A yearly rate in words: "3 times a year", "once a year", "once every 15 years".
pub fn rate_phrase(rate: f64) -> String {
    if rate.is_nan() || rate <= 0.0 {
        return "never".to_owned();
    }
    if rate >= 1.5 {
        return format!("{} times a year", round_nice(rate));
    }
    if rate >= 0.75 {
        return "once a year".to_owned();
    }
    format!("once every {} years", round_nice(1.0 / rate))
}

/// Rounds to two significant figures for display (research §7.2: at most 2 significant figures):
/// whole numbers below 10 (at least 1), then two figures ("4,600").
pub fn round_sig2(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    if x < 10.0 {
        return (x + 0.5).floor().max(1.0);
    }
    let mut mag = 1.0;
    while x / mag >= 100.0 {
        mag *= 10.0;
    }
    ((x / mag) + 0.5).floor() * mag
}

/// [`round_sig2`] in words, with thousands separators: "7", "45", "1,200"; "many" when infinite.
pub fn round_nice(x: f64) -> String {
    if !(x.is_finite()) {
        return "many".to_owned();
    }
    rr_types::money::thousands(round_sig2(x) as u64)
}

/// An amount of money for a sentence: two significant figures, whole dollars with thousands
/// separators ("$4,600"), the shared `rr_types::money` format.
pub fn usd(x: f64) -> String {
    rr_types::money::usd(if x.is_finite() && x > 0.0 {
        round_sig2(x)
    } else {
        x
    })
}

/// Natural frequency in words (research §7.1): under 1 → "fewer than 1"; 1 to 10 → a whole
/// number; above 10 → the nearest 5.
pub fn per_100(n: f64) -> String {
    if n.is_nan() || n < 1.0 {
        return "fewer than 1".to_owned();
    }
    if n <= 10.0 {
        return format!("{}", (n + 0.5).floor() as i64);
    }
    let r = ((n / 5.0) + 0.5).floor() * 5.0;
    format!("{}", r.min(100.0) as i64)
}

/// A count out of 100 that rounds to all of them ([`per_100`] rounds 97.5 and up to 100) is
/// said as "nearly all", never "about 100 of 100".
pub const NEARLY_ALL_PER_100: f64 = 97.5;

/// A natural frequency with its range: "about 9 (5–15)"; the range is dropped when it rounds to
/// the same words. Below 1 in 100 the central value has no "about": "fewer than 1 (up to 2)".
/// At the top: "nearly all (90 or more)".
pub fn per_100_range(n: f64, lo: f64, hi: f64) -> String {
    let (c, l, h) = (per_100(n), per_100(lo), per_100(hi));
    if n < 1.0 && hi < 1.0 {
        return "fewer than 1".to_owned();
    }
    if n.is_nan() || n < 1.0 {
        return format!("fewer than 1 (up to {h})");
    }
    if n >= NEARLY_ALL_PER_100 {
        return if lo >= NEARLY_ALL_PER_100 {
            "nearly all".to_owned()
        } else {
            let l = if lo < 1.0 { "0".to_owned() } else { l };
            format!("nearly all ({l} or more)")
        };
    }
    if l == h {
        return format!("about {c}");
    }
    let l = if lo < 1.0 { "0".to_owned() } else { l };
    format!("about {c} ({l}–{h})")
}

/// A count of households or customers out of 100 as the subject of a sentence: "about 9 of
/// 100", "fewer than 1 of 100", "nearly all".
pub fn count_of_100(n: f64) -> String {
    if n >= NEARLY_ALL_PER_100 {
        "nearly all".to_owned()
    } else if n.is_nan() || n < 1.0 {
        "fewer than 1 of 100".to_owned()
    } else {
        format!("about {} of 100", per_100(n))
    }
}

/// How a count out of 1,000 reads: "fewer than 1", "5", "45", "1,200".
fn per_1000(n: f64) -> String {
    if n.is_nan() || n < 0.5 {
        "fewer than 1".to_owned()
    } else if n < 10.0 {
        format!("{}", (n + 0.5).floor() as i64)
    } else {
        round_nice(n)
    }
}

/// Two chances over the horizon (0 to 1) on one scale, for a sentence that sets them side by
/// side: tenths while the smaller is at least about 1 in 10 (the default setting's "about 1 in
/// 10 … about 3 in 10"), hundredths down to about 1 in 100, then thousandths ("about 2 in 1,000
/// … about 9 in 1,000"), as the rest of the engine writes small chances; "nearly all" at the top.
pub fn chance_pair(small: f64, large: f64) -> (String, String) {
    let tenths = |p: f64| {
        if p >= 0.95 {
            "nearly all".to_owned()
        } else {
            format!("about {} in 10", ((p * 10.0) + 0.5).floor().max(1.0) as i64)
        }
    };
    let hundredths = |p: f64| {
        let n = 100.0 * p;
        if n >= NEARLY_ALL_PER_100 {
            "nearly all".to_owned()
        } else if n < 1.0 {
            "fewer than 1 in 100".to_owned()
        } else {
            format!("about {} in 100", per_100(n))
        }
    };
    let thousandths = |p: f64| {
        let n = 1000.0 * p;
        if n >= 10.0 * NEARLY_ALL_PER_100 {
            "nearly all".to_owned()
        } else if n < 0.5 {
            "fewer than 1 in 1,000".to_owned()
        } else {
            format!("about {} in 1,000", per_1000(n))
        }
    };
    let small = if small.is_finite() {
        small.max(0.0)
    } else {
        0.0
    };
    let large = if large.is_finite() {
        large.max(small)
    } else {
        small
    };
    if small >= 0.095 {
        (tenths(small), tenths(large))
    } else if small >= 0.0095 {
        (hundredths(small), hundredths(large))
    } else {
        (thousandths(small), thousandths(large))
    }
}

/// "the next 10 years" / "the next year".
pub fn horizon_phrase(years: u8) -> String {
    if years <= 1 {
        "the next year".to_owned()
    } else {
        format!("the next {years} years")
    }
}

/// Miles and kilometres for a distance: "12 miles (19 km)".
pub fn distance_phrase(km: f64) -> String {
    let miles = km / 1.609_344;
    let m = (miles + 0.5).floor().max(1.0) as i64;
    let k = (km + 0.5).floor().max(1.0) as i64;
    if m == 1 {
        format!("1 mile ({k} km)")
    } else {
        format!("{m} miles ({k} km)")
    }
}

/// A distance as an adjective: "12-mile (19 km)".
pub fn distance_adjective(km: f64) -> String {
    let miles = km / 1.609_344;
    let m = (miles + 0.5).floor().max(1.0) as i64;
    let k = (km + 0.5).floor().max(1.0) as i64;
    format!("{m}-mile ({k} km)")
}

/// The county-scale episodes a row counts, in words ("damaging river floods").
pub fn county_event_plural(keys: &[String]) -> String {
    let named: Vec<&str> = keys
        .iter()
        .map(|k| match k.as_str() {
            "flood" => "damaging river floods",
            "flash_flood" => "damaging flash floods",
            "coastal_flood" | "storm_surge" => "damaging coastal floods",
            _ => "damaging storms",
        })
        .collect();
    let mut out: Vec<&str> = Vec::new();
    for n in named {
        if !out.contains(&n) {
            out.push(n);
        }
    }
    out.join(" and ")
}

/// A multiplier in words: "about twice", "about 3 times", "about half".
pub fn factor_phrase(f: f64) -> String {
    if !f.is_finite() || f <= 0.0 {
        return "about as".to_owned();
    }
    if f < 0.4 {
        "about a third".to_owned()
    } else if f < 0.75 {
        "about half".to_owned()
    } else if f < 1.25 {
        "about the same".to_owned()
    } else if f < 1.75 {
        "about one and a half times".to_owned()
    } else if f < 2.5 {
        "about twice".to_owned()
    } else {
        format!("about {} times", round_nice(f))
    }
}

/// The event a hand-copied historic restoration curve (`historic:<id>`) records.
pub fn historic_event(class: &str) -> &'static str {
    match class {
        "historic:maria_2017_pr" => "Hurricane Maria (2017)",
        "historic:irma_maria_2017_vi" => "Hurricanes Irma and Maria (2017)",
        _ => "the worst storm on record",
    }
}

/// Citation id for a hand-copied historic restoration curve (the data pack's historic table reads
/// Maria from the Department of Energy's situation reports).
pub fn historic_source(class: &str) -> &'static str {
    match class {
        "historic:maria_2017_pr" | "historic:irma_maria_2017_vi" => "doe_maria_situation_reports",
        _ => "ornl_eagle_i_outages",
    }
}

/// Citation id for a county event record type.
pub fn event_source(event_type: &str) -> &'static str {
    match event_type {
        "boil_water_notice" | "boil_water" => "county_boil_water_records",
        _ => "noaa_storm_events",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_names_get_an_article() {
        assert_eq!(
            scenario_one("Magnitude 9 Cascadia earthquake"),
            "a magnitude 9 Cascadia earthquake"
        );
        assert_eq!(
            scenario_one("A magnitude 9 Cascadia earthquake"),
            "a magnitude 9 Cascadia earthquake"
        );
        assert_eq!(scenario_one("Ice age"), "an ice age");
        assert_eq!(
            scenario_one("Direct hit by a major hurricane"),
            "a direct hit by a major hurricane"
        );
    }

    #[test]
    fn natural_frequencies_follow_the_rounding_rules() {
        assert_eq!(per_100(0.4), "fewer than 1");
        assert_eq!(per_100(1.0), "1");
        assert_eq!(per_100(9.4), "9");
        assert_eq!(per_100(10.0), "10");
        assert_eq!(per_100(12.4), "10");
        assert_eq!(per_100(12.6), "15");
        assert_eq!(per_100(26.0), "25");
        assert_eq!(per_100(99.9), "100");
        assert_eq!(per_100_range(9.0, 5.2, 14.0), "about 9 (5–15)");
        assert_eq!(per_100_range(9.0, 9.2, 9.4), "about 9");
        assert_eq!(per_100_range(0.2, 0.1, 0.5), "fewer than 1");
        assert_eq!(per_100_range(3.0, 0.4, 6.0), "about 3 (0–6)");
        // Never "about fewer than 1 (0–2)".
        assert_eq!(per_100_range(0.4, 0.1, 2.3), "fewer than 1 (up to 2)");
        // Never "about 100 (90–100)".
        assert_eq!(per_100_range(99.2, 91.0, 100.0), "nearly all (90 or more)");
        assert_eq!(per_100_range(99.8, 98.0, 100.0), "nearly all");
        assert_eq!(per_100_range(96.0, 88.0, 99.0), "about 95 (90–100)");
        assert_eq!(count_of_100(100.0), "nearly all");
        assert_eq!(count_of_100(30.0), "about 30 of 100");
        assert_eq!(count_of_100(0.3), "fewer than 1 of 100");
    }

    #[test]
    fn chances_side_by_side_share_one_scale() {
        // The default setting: 1 in 10 for one kind, about 3 in 10 for any.
        assert_eq!(
            chance_pair(0.10, 0.28),
            ("about 1 in 10".to_owned(), "about 3 in 10".to_owned())
        );
        // 1 in 500 over ten years: 2 in 100, not "1 in 10".
        assert_eq!(
            chance_pair(0.0198, 0.068),
            ("about 2 in 100".to_owned(), "about 7 in 100".to_owned())
        );
        // One year at 1 in 500: thousandths.
        assert_eq!(
            chance_pair(0.002, 0.009),
            ("about 2 in 1,000".to_owned(), "about 9 in 1,000".to_owned())
        );
        assert_eq!(chance_pair(0.0001, 0.0003).0, "fewer than 1 in 1,000");
        assert_eq!(
            chance_pair(0.65, 0.99),
            ("about 7 in 10".to_owned(), "nearly all".to_owned())
        );
    }

    #[test]
    fn event_names_and_percentages_read_mid_sentence() {
        assert_eq!(
            event_mid_sentence("Winter storm, March 2018"),
            "winter storm, March 2018"
        );
        assert_eq!(
            event_mid_sentence("Power cut, cause not recorded, February 2021"),
            "power cut, cause not recorded, February 2021"
        );
        assert_eq!(
            event_mid_sentence("Hurricane or tropical storm, September 2020"),
            "hurricane or tropical storm, September 2020"
        );
        assert_eq!(event_mid_sentence("Heat, July 2022"), "heat, July 2022");
        // Named storms and dated events keep their capitals.
        assert_eq!(event_mid_sentence("Hurricane Helene"), "Hurricane Helene");
        assert_eq!(
            event_mid_sentence("November 2018 Camp Fire"),
            "November 2018 Camp Fire"
        );
        assert_eq!(
            event_mid_sentence("Hurricanes Irma and Maria (US Virgin Islands)"),
            "Hurricanes Irma and Maria (US Virgin Islands)"
        );
        assert_eq!(event_mid_sentence("Heatwave Kerberos"), "Heatwave Kerberos");
        assert_eq!(percent(0.3), "30%");
        assert_eq!(percent(0.065), "7%");
    }

    #[test]
    fn money_and_big_numbers_group_thousands() {
        assert_eq!(usd(4614.0), "$4,600");
        assert_eq!(usd(16_812.0), "$17,000");
        assert_eq!(usd(820.0), "$820");
        assert_eq!(round_nice(1000.0), "1,000");
        assert_eq!(round_nice(7.4), "7");
        assert_eq!(rate_phrase(0.001), "once every 1,000 years");
    }

    #[test]
    fn durations_read_plainly() {
        assert_eq!(ladder_phrase(0.5), "half a day");
        assert_eq!(ladder_phrase(3.0), "3 days");
        assert_eq!(ladder_phrase(14.0), "2 weeks");
        assert_eq!(ladder_phrase(365.0), "a year");
        assert_eq!(days_phrase(2.0 / 24.0), "2 hours");
        assert_eq!(days_phrase(20.0 / 24.0), "20 hours");
        assert_eq!(days_phrase(2.8), "3 days");
        assert_eq!(days_phrase(45.0), "6 weeks");
        assert_eq!(days_phrase(14.0), "2 weeks");
        assert_eq!(days_phrase(10.0), "10 days");
        assert_eq!(days_phrase(180.0), "6 months");
        assert_eq!(days_phrase(1095.0), "3 years");
        assert_eq!(distance_phrase(19.0), "12 miles (19 km)");
        assert_eq!(rate_phrase(0.0102), "once every 98 years");
        assert_eq!(rate_phrase(1.09), "once a year");
        assert_eq!(rate_phrase(2.5), "3 times a year");
        assert_eq!(distance_adjective(19.0), "12-mile (19 km)");
        assert_eq!(rate_phrase(0.065), "once every 15 years");
        assert_eq!(round_nice(123.0), "120");
        assert_eq!(horizon_phrase(10), "the next 10 years");
    }

    #[test]
    fn every_id_has_words() {
        for h in HazardId::ALL {
            assert!(!hazard_plural(*h).is_empty() && !hazard_one(*h).is_empty());
        }
        for b in BucketId::ALL {
            for s in [
                bucket_cause_verb(*b),
                bucket_noun(*b),
                bucket_noun_plural(*b),
                bucket_verb(*b),
                bucket_short(*b),
            ] {
                assert!(!s.is_empty());
                assert!(!s.contains('_'), "{s}");
            }
        }
    }
}
