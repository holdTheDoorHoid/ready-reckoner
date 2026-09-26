//! Stable identifiers: hazards (DESIGN §4.2), consequence buckets (§4.3), plan tiers (§4.5), and
//! the open-ended citation and item ids.
//!
//! Every id serialises as its snake_case string. The strings are part of the engine contract:
//! renaming one breaks saved plans, fixtures, content files and the web app. Adding one is an
//! `ENGINE_API_VERSION` bump and must be mirrored in `web/src/engine/types.ts` (a test in this
//! crate compares the two).

use core::fmt;

/// An id string that did not match any known value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIdError {
    what: &'static str,
    value: String,
    expected: &'static [&'static str],
}

impl ParseIdError {
    /// Builds the error for `value`, which is not one of `expected` (`what` names the kind of id).
    pub fn new(what: &'static str, value: &str, expected: &'static [&'static str]) -> Self {
        Self {
            what,
            value: value.to_owned(),
            expected,
        }
    }

    /// The kind of id that was being parsed, for example `"hazard id"`.
    pub fn what(&self) -> &'static str {
        self.what
    }

    /// The string that failed to parse.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The strings that would have parsed.
    pub fn expected(&self) -> &'static [&'static str] {
        self.expected
    }
}

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown {} `{}`; expected one of: {}",
            self.what,
            self.value,
            self.expected.join(", ")
        )
    }
}

impl std::error::Error for ParseIdError {}

/// True when `s` follows the project id convention: starts with a lowercase ASCII letter, then
/// lowercase letters, digits or underscores, and is at most 64 bytes long.
pub fn is_well_formed_id(s: &str) -> bool {
    let bytes = s.as_bytes();
    match bytes.first() {
        Some(b) if b.is_ascii_lowercase() => {}
        _ => return false,
    }
    bytes.len() <= 64
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
}

string_enum! {
    /// Where a hazard comes from, which decides how its yearly chance is estimated (DESIGN §4.2).
    pub enum HazardTier: "hazard tier" {
        /// Weather, geology and fire: the 18 FEMA National Risk Index hazards.
        Natural = "natural",
        /// Failures of shared systems or of society: estimated from national base rates.
        Societal = "societal",
        /// Things that happen to one household: estimated from national base rates adjusted for
        /// the household.
        Personal = "personal",
    }
}

string_enum! {
    /// A hazard: an event that can happen to a household (DESIGN §4.2). There are 35.
    ///
    /// Hazards explain *why* a consequence bucket matters where you live; supplies are sized by
    /// bucket, never by hazard.
    pub enum HazardId: "hazard id" {
        /// Snow avalanche.
        Avalanche = "avalanche",
        /// Flooding from the sea: storm surge and high-tide flooding.
        CoastalFlooding = "coastal_flooding",
        /// A spell of dangerously cold weather.
        ColdWave = "cold_wave",
        /// A long dry spell that strains water supplies.
        Drought = "drought",
        /// Earthquake.
        Earthquake = "earthquake",
        /// Hailstorm.
        Hail = "hail",
        /// A spell of dangerously hot weather.
        HeatWave = "heat_wave",
        /// Hurricane or tropical storm.
        Hurricane = "hurricane",
        /// Freezing rain that coats trees and power lines in ice.
        IceStorm = "ice_storm",
        /// Landslide or debris flow.
        Landslide = "landslide",
        /// Lightning strike.
        Lightning = "lightning",
        /// Flooding from rivers, streams and heavy rain (National Risk Index v1.20 "inland
        /// flooding", `IFLD`; the id keeps the older name).
        RiverineFlooding = "riverine_flooding",
        /// Damaging wind that is not a tornado or hurricane.
        StrongWind = "strong_wind",
        /// Tornado.
        Tornado = "tornado",
        /// Tsunami.
        Tsunami = "tsunami",
        /// Volcanic eruption, ash fall or lahar.
        VolcanicActivity = "volcanic_activity",
        /// Wildfire, including the planned power shutoffs that utilities use to prevent one.
        Wildfire = "wildfire",
        /// Snow, blizzards and freezing rain (the National Risk Index calls it winter weather).
        WinterWeather = "winter_weather",
        /// A disease outbreak that spreads widely.
        Pandemic = "pandemic",
        /// A regional, multi-day failure of the power grid that is not caused by weather.
        GridFailure = "grid_failure",
        /// A cyberattack or computer failure that takes down utilities, payments, or pharmacy and
        /// insurer systems.
        CyberOutage = "cyber_outage",
        /// Riots or widespread unrest.
        CivilUnrest = "civil_unrest",
        /// Store shortages: port, rail, trucking or fuel disruption, or runs on stores before a
        /// storm.
        SupplyChainDisruption = "supply_chain_disruption",
        /// A chemical spill or release from a plant, rail car or truck.
        HazmatRelease = "hazmat_release",
        /// An accident at a nuclear power plant.
        NuclearPlantIncident = "nuclear_plant_incident",
        /// A nuclear attack, including an electromagnetic pulse (EMP).
        NuclearAttack = "nuclear_attack",
        /// A terrorist attack.
        Terrorism = "terrorism",
        /// Losing a job or a main source of income.
        JobLoss = "job_loss",
        /// A fire in the home.
        HouseFire = "house_fire",
        /// A sudden serious illness or injury at home.
        MedicalEmergency = "medical_emergency",
        /// Being stuck in a vehicle by weather, a breakdown or closed roads.
        VehicleStranding = "vehicle_stranding",
        /// A local water main break, boil-water notice or gas leak.
        LocalUtilityOutage = "local_utility_outage",
        /// A break-in at the home.
        Burglary = "burglary",
        /// The death or long-term disability of someone who earns money for the household.
        EarnerDeathOrDisability = "earner_death_or_disability",
        /// A long illness that keeps someone in the household sick at home.
        ExtendedHouseholdIllness = "extended_household_illness",
    }
}

impl HazardId {
    /// Natural, societal or personal (DESIGN §4.2).
    pub const fn tier(self) -> HazardTier {
        use HazardId::*;
        match self {
            Avalanche | CoastalFlooding | ColdWave | Drought | Earthquake | Hail | HeatWave
            | Hurricane | IceStorm | Landslide | Lightning | RiverineFlooding | StrongWind
            | Tornado | Tsunami | VolcanicActivity | Wildfire | WinterWeather => {
                HazardTier::Natural
            }
            Pandemic
            | GridFailure
            | CyberOutage
            | CivilUnrest
            | SupplyChainDisruption
            | HazmatRelease
            | NuclearPlantIncident
            | NuclearAttack
            | Terrorism => HazardTier::Societal,
            JobLoss
            | HouseFire
            | MedicalEmergency
            | VehicleStranding
            | LocalUtilityOutage
            | Burglary
            | EarnerDeathOrDisability
            | ExtendedHouseholdIllness => HazardTier::Personal,
        }
    }

    /// The plain-English name shown to users (sentence case).
    pub const fn name(self) -> &'static str {
        use HazardId::*;
        match self {
            Avalanche => "Avalanche",
            CoastalFlooding => "Coastal flooding",
            ColdWave => "Cold wave",
            Drought => "Drought",
            Earthquake => "Earthquake",
            Hail => "Hail",
            HeatWave => "Heat wave",
            Hurricane => "Hurricane",
            IceStorm => "Ice storm",
            Landslide => "Landslide",
            Lightning => "Lightning",
            RiverineFlooding => "Flooding from rivers or heavy rain",
            StrongWind => "Strong wind",
            Tornado => "Tornado",
            Tsunami => "Tsunami",
            VolcanicActivity => "Volcanic eruption",
            Wildfire => "Wildfire",
            WinterWeather => "Winter storm",
            Pandemic => "Pandemic",
            GridFailure => "Regional blackout",
            CyberOutage => "Cyberattack on services",
            CivilUnrest => "Civil unrest",
            SupplyChainDisruption => "Supply chain disruption",
            HazmatRelease => "Chemical spill or release",
            NuclearPlantIncident => "Nuclear power plant accident",
            NuclearAttack => "Nuclear attack",
            Terrorism => "Terrorist attack",
            JobLoss => "Job loss",
            HouseFire => "House fire",
            MedicalEmergency => "Medical emergency",
            VehicleStranding => "Stranded in a vehicle",
            LocalUtilityOutage => "Local water or gas outage",
            Burglary => "Break-in",
            EarnerDeathOrDisability => "Death or disability of an earner",
            ExtendedHouseholdIllness => "Long illness in the household",
        }
    }
}

string_enum! {
    /// How a bucket's target is expressed; the `kind` tag of [`crate::Target`].
    pub enum TargetKind: "target kind" {
        /// A number of days, with a low–high range (duration buckets).
        Days = "days",
        /// A number of months, with a low–high range (`income`).
        Months = "months",
        /// The chance of having to leave, the warning to expect and the days away (`evacuate`).
        Evacuate = "evacuate",
        /// The chance of needing it and a checklist of steps (the other readiness buckets and
        /// `home_loss`).
        Readiness = "readiness",
    }
}

string_enum! {
    /// The three kinds of consequence bucket, which decide how a bucket is sized and paid for.
    pub enum BucketKind: "bucket kind" {
        /// Measured in days of going without; covered by supplies from the preparedness budget.
        Duration = "duration",
        /// Covered by a checklist of steps and a few items; not measured in days.
        Readiness = "readiness",
        /// Covered by savings and insurance, tracked apart from the supplies budget.
        Money = "money",
    }
}

string_enum! {
    /// A consequence bucket: a plain consequence that many hazards share. Supplies and actions are
    /// sized against buckets, so two hazards that cut the power share one plan for "no power".
    /// There are 14.
    pub enum BucketId: "bucket id" {
        /// No grid power at home.
        Power = "power",
        /// Tap water must be treated (boil-water notice).
        WaterBoil = "water_boil",
        /// No tap water at all.
        WaterOut = "water_out",
        /// Can't get to a store: stores or roads closed, or shelves empty.
        Supplies = "supplies",
        /// Dangerous heat or cold indoors.
        Thermal = "thermal",
        /// Medication and medical-supply continuity.
        Medication = "medication",
        /// No phone, internet or card payments.
        Comms = "comms",
        /// Must leave home quickly.
        Evacuate = "evacuate",
        /// Stranded away from home (the get-home bag lives here).
        GetHome = "get_home",
        /// Medical emergency when help is slow.
        MedicalEmergency = "medical_emergency",
        /// House fire.
        Fire = "fire",
        /// Home and personal security.
        Security = "security",
        /// Loss of income.
        Income = "income",
        /// Home damaged or uninhabitable.
        HomeLoss = "home_loss",
    }
}

impl BucketId {
    /// The plain name shown to users.
    pub const fn name(self) -> &'static str {
        use BucketId::*;
        match self {
            Power => "No grid power at home",
            WaterBoil => "Tap water must be treated",
            WaterOut => "No tap water at all",
            Supplies => "Can't get to a store",
            Thermal => "Dangerous heat or cold indoors",
            Medication => "Medication and medical-supply continuity",
            Comms => "No phone, internet or card payments",
            Evacuate => "Must leave home quickly",
            GetHome => "Stranded away from home",
            MedicalEmergency => "Medical emergency when help is slow",
            Fire => "House fire",
            Security => "Home and personal security",
            Income => "Loss of income",
            HomeLoss => "Home damaged or uninhabitable",
        }
    }

    /// Duration, readiness or money.
    pub const fn kind(self) -> BucketKind {
        use BucketId::*;
        match self {
            Power | WaterBoil | WaterOut | Supplies | Thermal | Medication | Comms => {
                BucketKind::Duration
            }
            Evacuate | GetHome | MedicalEmergency | Fire | Security => BucketKind::Readiness,
            Income | HomeLoss => BucketKind::Money,
        }
    }

    /// How this bucket's [`crate::Target`] is expressed: days for the duration buckets, months
    /// for `income` (the savings goal), `evacuate` for `evacuate`, and a readiness checklist for
    /// the other readiness buckets and for `home_loss`, which is an insurance-and-documents
    /// decision with no stockpile target (DESIGN §4.4).
    pub const fn target_kind(self) -> TargetKind {
        use BucketId::*;
        match self {
            Power | WaterBoil | WaterOut | Supplies | Thermal | Medication | Comms => {
                TargetKind::Days
            }
            Income => TargetKind::Months,
            Evacuate => TargetKind::Evacuate,
            GetHome | MedicalEmergency | Fire | Security | HomeLoss => TargetKind::Readiness,
        }
    }
}

string_enum! {
    /// A plan tier, in plan order. The derived ordering follows this order, so the tier a
    /// household should reach is the maximum over its buckets. (The get-home bag is an item in the
    /// `get_home` bucket, not a tier.)
    pub enum TierId: "tier id" {
        /// Free actions, always first.
        Now = "now",
        /// Three days.
        H72 = "h72",
        /// Two weeks.
        W2 = "w2",
        /// One month.
        M1 = "m1",
        /// Three months.
        M3 = "m3",
        /// Six months.
        M6 = "m6",
        /// One year.
        Y1 = "y1",
    }
}

impl TierId {
    /// The plain name shown to users.
    pub const fn name(self) -> &'static str {
        use TierId::*;
        match self {
            Now => "Free actions",
            H72 => "Three days",
            W2 => "Two weeks",
            M1 => "One month",
            M3 => "Three months",
            M6 => "Six months",
            Y1 => "One year",
        }
    }

    /// The days of self-sufficiency the tier stands for.
    pub const fn days(self) -> u16 {
        use TierId::*;
        match self {
            Now => 0,
            H72 => 3,
            W2 => 14,
            M1 => 30,
            M3 => 90,
            M6 => 180,
            Y1 => 365,
        }
    }
}

string_newtype! {
    /// A citation id: a key into `content/citations.toml` (for example `ready_gov_water`). Every
    /// user-visible number carries at least one.
    pub struct CitationId;
}

string_newtype! {
    /// A catalogue item id: a key into `content/items/*.toml` (for example `water_stored`).
    pub struct ItemId;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::str::FromStr;

    fn check_string_enum<T>(all: &[T], strs: &[&str])
    where
        T: Copy + Eq + Ord + fmt::Debug + fmt::Display + FromStr + serde::Serialize,
        T: serde::de::DeserializeOwned,
        <T as FromStr>::Err: fmt::Debug,
    {
        assert_eq!(all.len(), strs.len());
        let unique: BTreeSet<&str> = strs.iter().copied().collect();
        assert_eq!(unique.len(), strs.len(), "duplicate strings");
        for (i, (v, s)) in all.iter().zip(strs).enumerate() {
            assert!(is_well_formed_id(s), "{s} is not snake_case");
            assert_eq!(&v.to_string(), s);
            assert_eq!(T::from_str(s).unwrap(), *v);
            let json = serde_json::to_string(v).unwrap();
            assert_eq!(json, format!("\"{s}\""));
            assert_eq!(serde_json::from_str::<T>(&json).unwrap(), *v);
            if i > 0 {
                assert!(all[i - 1] < *v, "ALL must be in declaration (Ord) order");
            }
        }
    }

    #[test]
    fn every_id_enum_round_trips_through_its_string() {
        check_string_enum(HazardId::ALL, HazardId::STRS);
        check_string_enum(HazardTier::ALL, HazardTier::STRS);
        check_string_enum(BucketId::ALL, BucketId::STRS);
        check_string_enum(TargetKind::ALL, TargetKind::STRS);
        check_string_enum(BucketKind::ALL, BucketKind::STRS);
        check_string_enum(TierId::ALL, TierId::STRS);
        for h in HazardId::ALL {
            assert_eq!(h.as_str(), h.to_string());
        }
    }

    #[test]
    fn there_are_35_hazards_18_natural_9_societal_8_personal() {
        assert_eq!(HazardId::ALL.len(), 35);
        let count = |t| HazardId::ALL.iter().filter(|h| h.tier() == t).count();
        assert_eq!(count(HazardTier::Natural), 18);
        assert_eq!(count(HazardTier::Societal), 9);
        assert_eq!(count(HazardTier::Personal), 8);
    }

    #[test]
    fn hazard_ids_match_design_section_4_2() {
        let natural = "avalanche coastal_flooding cold_wave drought earthquake hail heat_wave \
            hurricane ice_storm landslide lightning riverine_flooding strong_wind tornado tsunami \
            volcanic_activity wildfire winter_weather";
        let societal = "pandemic grid_failure cyber_outage civil_unrest supply_chain_disruption \
            hazmat_release nuclear_plant_incident nuclear_attack terrorism";
        let personal = "job_loss house_fire medical_emergency vehicle_stranding \
            local_utility_outage burglary earner_death_or_disability extended_household_illness";
        for (list, tier) in [
            (natural, HazardTier::Natural),
            (societal, HazardTier::Societal),
            (personal, HazardTier::Personal),
        ] {
            for s in list.split_whitespace() {
                let h: HazardId = s.parse().unwrap();
                assert_eq!(h.tier(), tier, "{s}");
            }
        }
    }

    #[test]
    fn buckets_match_the_v1_contract() {
        let by_kind = |k: BucketKind| -> Vec<&str> {
            BucketId::ALL
                .iter()
                .filter(|b| b.kind() == k)
                .map(|b| b.as_str())
                .collect()
        };
        assert_eq!(BucketId::ALL.len(), 14);
        assert_eq!(
            by_kind(BucketKind::Duration),
            [
                "power",
                "water_boil",
                "water_out",
                "supplies",
                "thermal",
                "medication",
                "comms"
            ]
        );
        assert_eq!(
            by_kind(BucketKind::Readiness),
            [
                "evacuate",
                "get_home",
                "medical_emergency",
                "fire",
                "security"
            ]
        );
        assert_eq!(by_kind(BucketKind::Money), ["income", "home_loss"]);
        for gone in ["water", "shelter_in_place", "medical", "supply_chain"] {
            assert!(gone.parse::<BucketId>().is_err(), "{gone} was removed");
        }
        assert_eq!(BucketId::Power.name(), "No grid power at home");
        assert_eq!(BucketId::Supplies.name(), "Can't get to a store");
        assert_eq!(
            BucketId::Comms.name(),
            "No phone, internet or card payments"
        );
        assert_eq!(BucketId::HomeLoss.name(), "Home damaged or uninhabitable");
    }

    #[test]
    fn target_kind_follows_bucket_kind() {
        for b in BucketId::ALL {
            let want = match (b.kind(), b) {
                (BucketKind::Duration, _) => TargetKind::Days,
                (BucketKind::Money, BucketId::Income) => TargetKind::Months,
                // An insurance-and-documents decision, no stockpile target (DESIGN §4.4).
                (BucketKind::Money, _) => TargetKind::Readiness,
                (BucketKind::Readiness, BucketId::Evacuate) => TargetKind::Evacuate,
                (BucketKind::Readiness, _) => TargetKind::Readiness,
            };
            assert_eq!(b.target_kind(), want, "{b}");
        }
        assert_eq!(BucketId::Income.target_kind(), TargetKind::Months);
        assert_eq!(BucketId::HomeLoss.target_kind(), TargetKind::Readiness);
        assert_eq!(BucketId::Evacuate.target_kind(), TargetKind::Evacuate);
        assert_eq!(BucketId::GetHome.target_kind(), TargetKind::Readiness);
        assert_eq!(BucketId::WaterOut.target_kind(), TargetKind::Days);
    }

    #[test]
    fn tiers_match_the_v1_contract() {
        let tiers: Vec<(&str, u16, &str)> = TierId::ALL
            .iter()
            .map(|t| (t.as_str(), t.days(), t.name()))
            .collect();
        assert_eq!(
            tiers,
            [
                ("now", 0, "Free actions"),
                ("h72", 3, "Three days"),
                ("w2", 14, "Two weeks"),
                ("m1", 30, "One month"),
                ("m3", 90, "Three months"),
                ("m6", 180, "Six months"),
                ("y1", 365, "One year"),
            ]
        );
        assert!(
            "get_home".parse::<TierId>().is_err(),
            "get_home is a bucket, not a tier"
        );
        let days: Vec<u16> = TierId::ALL.iter().map(|t| t.days()).collect();
        assert!(
            days.windows(2).all(|w| w[0] < w[1]),
            "tiers grow with plan order"
        );
        assert!(TierId::Now < TierId::Y1);
    }

    #[test]
    fn names_are_present_and_plain() {
        let names = HazardId::ALL
            .iter()
            .map(|h| h.name())
            .chain(BucketId::ALL.iter().map(|b| b.name()))
            .chain(TierId::ALL.iter().map(|t| t.name()));
        for n in names {
            assert!(!n.is_empty());
            assert!(n.chars().next().unwrap().is_uppercase(), "{n}");
            assert!(!n.contains('_'), "{n} looks like an id, not a name");
            assert!(n.len() <= 48, "{n} is too long for a card title");
        }
        let unique: BTreeSet<&str> = HazardId::ALL.iter().map(|h| h.name()).collect();
        assert_eq!(unique.len(), HazardId::ALL.len());
    }

    #[test]
    fn unknown_ids_fail_loudly() {
        let err = "hurrican".parse::<HazardId>().unwrap_err();
        assert_eq!(err.what(), "hazard id");
        assert_eq!(err.value(), "hurrican");
        assert!(err.to_string().contains("hurricane"));
        let err = serde_json::from_str::<BucketId>("\"Power\"").unwrap_err();
        assert!(err.to_string().contains("unknown variant `Power`"), "{err}");
        assert!(serde_json::from_str::<TierId>("3").is_err());
    }

    #[test]
    fn ids_work_as_json_map_keys() {
        let mut m = std::collections::BTreeMap::new();
        m.insert(HazardId::HeatWave, 0.25_f64);
        m.insert(HazardId::Avalanche, 0.5_f64);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"avalanche":0.5,"heat_wave":0.25}"#);
        let back: std::collections::BTreeMap<HazardId, f64> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn newtype_ids() {
        let c = CitationId::from("ready_gov_water");
        assert!(c.is_well_formed());
        assert_eq!(c, "ready_gov_water");
        assert_eq!(serde_json::to_string(&c).unwrap(), "\"ready_gov_water\"");
        let back: CitationId = serde_json::from_str("\"ready_gov_water\"").unwrap();
        assert_eq!(back, c);
        let mut set = std::collections::BTreeSet::new();
        set.insert(ItemId::new("water_stored"));
        assert!(set.contains("water_stored"));
        for bad in [
            "",
            "Water",
            "water stored",
            "9volt",
            "water-stored",
            &"a".repeat(65),
        ] {
            assert!(!ItemId::new(bad).is_well_formed(), "{bad:?}");
        }
        for good in ["a", "water_stored", "nws_2024", &"a".repeat(64)] {
            assert!(ItemId::new(good).is_well_formed(), "{good:?}");
        }
    }
}
