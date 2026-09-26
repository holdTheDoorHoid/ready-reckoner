//! Inputs: the household and the dials (DESIGN §4.1).
//!
//! Every input struct denies unknown fields, so a typo in the web app, a fixture or an imported
//! plan fails loudly instead of being ignored. Optional fields are omitted from JSON when absent.

use serde::{Deserialize, Serialize};

use crate::{Date, ItemId};

/// The ZIP code in [`PlanInput::defaults`]. It is well formed, so the skeleton validates, but no
/// real ZIP code is 00000: an interview that forgets to replace it fails with `unknown_zip`
/// instead of quietly planning for somewhere else.
pub const PLACEHOLDER_ZIP: &str = "00000";

/// The planning date in [`PlanInput::defaults`]. The app replaces it with today's date; the engine
/// never reads the clock.
pub const PLACEHOLDER_PLANNING_DATE: Date = match Date::from_ymd(2026, 10, 1) {
    Some(d) => d,
    None => panic!("placeholder planning date is invalid"),
};

/// The `#[serde(default = ...)]` for fields that default to `true` when absent (plain `bool`'s own
/// `Default` is `false`).
const fn default_true() -> bool {
    true
}

/// Everything the engine needs about one household. `assess` turns this into a
/// [`crate::PlanOutput`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanInput {
    /// The day the plan starts. An input, never the wall clock: the app sets it to today.
    pub planning_date: Date,
    /// Where the household lives.
    pub location: LocationInput,
    /// The home itself.
    pub housing: Housing,
    /// Everyone in the household. At least one person.
    pub people: Vec<Person>,
    /// Animals the household looks after.
    pub pets: Pets,
    /// Vehicles.
    pub mobility: HouseholdMobility,
    /// Budget, savings, income and insurance.
    pub finances: Finances,
    /// What the household already has, and free actions already done. May be empty.
    pub existing: Vec<Owned>,
    /// Credit the household with everyday basics that almost every home has (blankets and warm
    /// layers, a cooking pot and can opener, a phone, a bag per person, three days of ordinary
    /// food) unless the Have screen says otherwise. On by default; the packet lists what was
    /// assumed. Items the allocator credits this way are marked [`crate::Item::assumed_basic`].
    #[serde(default = "default_true")]
    pub assume_basics: bool,
    /// The dials: how rare an event to be ready for, climate horizon, planning horizon, water
    /// level.
    pub dials: Dials,
    /// Where the household says it is with preparing (stages of change); tailors the copy.
    /// Absent if the question was skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<Stage>,
    /// "How confident are you that your household could handle an emergency?" on a 1 to 5 scale,
    /// asked before the plan and again after it (self-efficacy). Absent if skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_1to5: Option<u8>,
}

/// Where the household lives. Give a ZIP code, a county, or both.
///
/// If both are given, `county_fips` decides the county and `zip` is kept for display. That is how
/// the app records the county a user picked for a ZIP code that spans several counties
/// (`ambiguous_zip`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationInput {
    /// ISO 3166-1 alpha-2 country code. Only `"US"` is supported in v1.
    pub country: String,
    /// Five-digit ZIP code, for example `"19147"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip: Option<String>,
    /// Five-digit county FIPS code (state + county), for example `"42101"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub county_fips: Option<String>,
    /// Urban, suburban or rural, as the household describes it.
    pub setting: Setting,
}

string_enum! {
    /// How built-up the area around the home is.
    pub enum Setting: "setting" {
        /// A city.
        Urban = "urban",
        /// A suburb or town.
        Suburban = "suburban",
        /// The countryside.
        Rural = "rural",
    }
}

/// The home.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Housing {
    /// Kind of building.
    pub kind: HousingKind,
    /// Own or rent.
    pub tenure: Tenure,
    /// The floor the household lives on (1 is street level; negative is below ground).
    pub floor: i8,
    /// Whether the home has a basement.
    pub basement: bool,
    /// Where tap water comes from.
    pub water: WaterSource,
    /// Where wastewater goes (JSON field `sewer`).
    pub sewer: Wastewater,
    /// Main heating.
    pub heating: Heating,
    /// Cooling.
    pub cooling: Cooling,
    /// Backup power the household already has.
    pub backup_power: BackupPower,
    /// Fire and carbon monoxide safety equipment already in the home.
    pub alarms: Alarms,
}

string_enum! {
    /// Kind of building.
    pub enum HousingKind: "housing kind" {
        /// An apartment in a tall building (elevators; water may need pumps).
        ApartmentHighRise = "apartment_high_rise",
        /// An apartment in a low building.
        ApartmentLowRise = "apartment_low_rise",
        /// A rowhouse or townhouse.
        Rowhouse = "rowhouse",
        /// A detached house.
        Detached = "detached",
        /// A mobile or manufactured home.
        MobileHome = "mobile_home",
        /// A house on rural land, such as a farm.
        RuralProperty = "rural_property",
    }
}

string_enum! {
    /// Whether the household owns or rents the home.
    pub enum Tenure: "tenure" {
        /// The household owns the home.
        Own = "own",
        /// The household rents the home.
        Rent = "rent",
    }
}

string_enum! {
    /// Where tap water comes from.
    pub enum WaterSource: "water source" {
        /// A public water system.
        Municipal = "municipal",
        /// A private well (usually needs power to pump).
        Well = "well",
    }
}

string_enum! {
    /// Where wastewater goes.
    pub enum Wastewater: "wastewater" {
        /// A public sewer.
        Sewer = "sewer",
        /// A septic system.
        Septic = "septic",
    }
}

string_enum! {
    /// Main heating.
    pub enum Heating: "heating" {
        /// Natural gas.
        Gas = "gas",
        /// Electric resistance heaters or baseboards.
        ElectricResistance = "electric_resistance",
        /// Electric heat pump.
        HeatPump = "heat_pump",
        /// Heating oil.
        Oil = "oil",
        /// Propane.
        Propane = "propane",
        /// Wood stove or fireplace.
        Wood = "wood",
        /// Steam or hot water from a building or district plant.
        District = "district",
        /// No heating.
        None = "none",
    }
}

string_enum! {
    /// Cooling.
    pub enum Cooling: "cooling" {
        /// Central air conditioning.
        Central = "central",
        /// Window or portable air conditioners.
        Window = "window",
        /// No air conditioning.
        None = "none",
    }
}

string_enum! {
    /// Backup power the household already has.
    pub enum BackupPower: "backup power" {
        /// None.
        None = "none",
        /// A battery power station.
        PowerStation = "power_station",
        /// A fuel generator.
        Generator = "generator",
        /// Solar panels with a home battery.
        SolarBattery = "solar_battery",
    }
}

/// Fire and carbon monoxide safety equipment already in the home.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alarms {
    /// Working smoke alarms.
    pub smoke: bool,
    /// Working carbon monoxide alarms.
    pub co: bool,
    /// A fire extinguisher.
    pub extinguisher: bool,
}

/// One member of the household.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    /// Age band.
    pub age_band: AgeBand,
    /// Pregnant or breastfeeding (more water and food; prenatal care).
    pub pregnant_or_nursing: bool,
    /// Medical needs.
    pub medical: Medical,
    /// Whether this person earns money for the household.
    pub earner: bool,
    /// The person's usual trip to work or school, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commute: Option<Commute>,
}

string_enum! {
    /// Age band.
    pub enum AgeBand: "age band" {
        /// Under 1 year.
        Infant = "infant",
        /// 1 to 3 years.
        Toddler = "toddler",
        /// 4 to 12 years.
        Child = "child",
        /// 13 to 17 years.
        Teen = "teen",
        /// 18 to 64 years.
        Adult = "adult",
        /// 65 years and over.
        Senior = "senior",
    }
}

/// A person's medical needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Medical {
    /// Takes prescription medicine every day.
    pub daily_rx: bool,
    /// Has a prescription medicine that must stay cold (for example insulin).
    pub refrigerated_rx: bool,
    /// A medical device that needs electricity.
    pub powered_device: PoweredDevice,
    /// How well the person gets around.
    pub mobility: Mobility,
    /// Dietary needs in the person's own words (for example `"vegetarian"`, `"formula"`).
    pub dietary: Vec<String>,
    /// Carries an epinephrine auto-injector.
    pub epinephrine: bool,
}

/// A medical device that needs electricity. In JSON the simple cases are strings (`"cpap"`) and
/// the open case is an object: `{"other": {"watts": 60}}`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PoweredDevice {
    /// No powered device.
    None,
    /// A CPAP or BiPAP breathing machine.
    Cpap,
    /// An oxygen concentrator.
    Oxygen,
    /// Another device.
    Other {
        /// How much power it draws, in watts (more than zero).
        watts: f32,
    },
}

string_enum! {
    /// How well a person gets around (JSON field `medical.mobility`). Not to be confused with
    /// [`HouseholdMobility`], the household's vehicles.
    pub enum Mobility: "mobility" {
        /// No limits.
        None = "none",
        /// Walks with difficulty or needs help on stairs.
        Limited = "limited",
        /// Uses a wheelchair.
        Wheelchair = "wheelchair",
    }
}

/// A person's usual trip to work or school.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commute {
    /// One-way distance in kilometres.
    pub distance_km: f32,
    /// How the person usually travels.
    pub mode: CommuteMode,
    /// Whether the person could work or study from home.
    pub remote_possible: bool,
}

string_enum! {
    /// How a person usually travels to work or school.
    pub enum CommuteMode: "commute mode" {
        /// Car.
        Car = "car",
        /// Bus, train or other public transit.
        Transit = "transit",
        /// Walking.
        Walk = "walk",
        /// Bicycle.
        Bike = "bike",
    }
}

/// Animals the household looks after.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pets {
    /// Dogs.
    pub dogs: u8,
    /// Cats.
    pub cats: u8,
    /// Small animals: birds, rabbits, reptiles, fish tanks.
    pub small: u8,
    /// Livestock and horses.
    pub large_animals: u8,
}

/// The household's vehicles (JSON field `mobility`). Not to be confused with [`Mobility`], a
/// person's ability to get around.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholdMobility {
    /// Every vehicle the household can use. May be empty.
    pub vehicles: Vec<Vehicle>,
}

/// One vehicle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vehicle {
    /// What it runs on.
    pub fuel: Fuel,
}

string_enum! {
    /// What a vehicle runs on.
    pub enum Fuel: "fuel" {
        /// Gasoline.
        Gas = "gas",
        /// Diesel.
        Diesel = "diesel",
        /// Gasoline-electric hybrid.
        Hybrid = "hybrid",
        /// Battery electric.
        Ev = "ev",
    }
}

/// Budget, savings, income and insurance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finances {
    /// Money for preparing each month, in US dollars. Zero is fine: the plan is then free actions.
    pub monthly_budget_usd: f32,
    /// A one-off amount available now, in US dollars.
    pub one_off_budget_usd: f32,
    /// Months of expenses already saved.
    pub emergency_fund_months: f32,
    /// Monthly household expenses in US dollars, if the household gave them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_expenses_usd: Option<f32>,
    /// Income.
    pub income: Income,
    /// Insurance held.
    pub insurance: Insurance,
}

/// Household income.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Income {
    /// How many people earn money. Must equal the number of people marked `earner`.
    pub earners: u8,
    /// How steady the income is.
    pub stability: IncomeStability,
}

string_enum! {
    /// How steady the household's income is.
    pub enum IncomeStability: "income stability" {
        /// Tenured, public sector or a pension: the most protected from job loss.
        VeryStable = "very_stable",
        /// A regular salary or pension.
        Stable = "stable",
        /// Changes from month to month.
        Variable = "variable",
        /// Depends on the season.
        Seasonal = "seasonal",
        /// Gig or app-based work.
        Gig = "gig",
    }
}

/// Insurance the household holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Insurance {
    /// Homeowners or renters insurance.
    pub home_or_renters: bool,
    /// Flood insurance (standard home policies do not cover floods).
    pub flood: bool,
    /// Earthquake insurance.
    pub earthquake: bool,
}

/// Something the household already has, or a free action it has already done.
///
/// A free action counts as done when it appears here with `qty` 1 or more. Bought items record
/// what was paid so the plan can use real prices instead of the price band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Owned {
    /// The catalogue item.
    pub item_id: ItemId,
    /// How many, in the item's unit (for example gallons of water). Zero means none.
    pub qty: f32,
    /// What the household paid for this quantity in total, in US dollars, if recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid_usd: Option<f32>,
}

/// The dials the user can turn on the risks screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dials {
    /// How severe an event to be ready for, as a return period.
    pub return_period: ReturnPeriod,
    /// Today's climate or the 2050 projection.
    pub climate: ClimateHorizon,
    /// Years used for the natural-frequency sentences ("in the next ten years"): 1 to 50, usually
    /// 10.
    pub horizon_years: u8,
    /// How much water per person per day to plan for. Defaults to `basic` when absent.
    #[serde(default)]
    pub water_level: WaterLevel,
    /// The user's on/off choices for named scenarios (for example `cascadia_m9`). The engine
    /// decides which scenarios apply to a location and reports them in
    /// [`crate::PlanOutput::scenarios`]; a toggle here overrides its default. Empty when absent.
    #[serde(default)]
    pub scenario_overrides: Vec<ScenarioToggle>,
    /// Allow up to 10% of the monthly budget for rare-catastrophe items (a radiation meter,
    /// potassium iodide only on official instruction, Faraday storage). Off by default: those
    /// items otherwise get $0 (`Item.rare_catastrophic`).
    #[serde(default)]
    pub rare_catastrophic_opt_in: bool,
}

string_enum! {
    /// How severe an event to be ready for: an event this bad happens about once in this many
    /// years where you live.
    #[derive(Default)]
    pub enum ReturnPeriod: "return period" {
        /// About once in 10 years: common disruptions.
        OneIn10 = "one_in_10",
        /// About once in 50 years: serious.
        OneIn50 = "one_in_50",
        /// About once in 100 years: very serious. The default.
        #[default]
        OneIn100 = "one_in_100",
        /// About once in 500 years: rare catastrophes.
        OneIn500 = "one_in_500",
    }
}

impl ReturnPeriod {
    /// The return period in years: 10, 50, 100 or 500.
    pub const fn years(self) -> u16 {
        match self {
            ReturnPeriod::OneIn10 => 10,
            ReturnPeriod::OneIn50 => 50,
            ReturnPeriod::OneIn100 => 100,
            ReturnPeriod::OneIn500 => 500,
        }
    }

    /// The label on the dial.
    pub const fn name(self) -> &'static str {
        match self {
            ReturnPeriod::OneIn10 => "Common disruptions",
            ReturnPeriod::OneIn50 => "Serious",
            ReturnPeriod::OneIn100 => "Very serious",
            ReturnPeriod::OneIn500 => "Rare catastrophes",
        }
    }
}

/// The user's choice for one named scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioToggle {
    /// The scenario id, as reported in [`crate::ScenarioInfo::id`] (for example `cascadia_m9`).
    pub id: String,
    /// Plan for it (`true`) or leave it out (`false`).
    pub on: bool,
}

string_enum! {
    /// Which climate the hazard chances describe.
    pub enum ClimateHorizon: "climate horizon" {
        /// Today's climate.
        Today = "today",
        /// The 2050 projection (Fifth National Climate Assessment regional multipliers), labelled
        /// as a projection.
        Y2050 = "y2050",
    }
}

string_enum! {
    /// How much water per person per day to plan for. The litres behind each level are defined and
    /// cited in `rr-supply` (roughly 3 L, 4 L, which is one US gallon, and 15 L).
    #[derive(Default)]
    pub enum WaterLevel: "water level" {
        /// Drinking only.
        Survival = "survival",
        /// Drinking plus basic hygiene: the usual guidance of one gallon per person per day. The
        /// default.
        #[default]
        Basic = "basic",
        /// Drinking, cooking and washing.
        Comfortable = "comfortable",
    }
}

string_enum! {
    /// Where the household says it is with preparing (stages of change). Tailors the copy; never
    /// changes the numbers.
    pub enum Stage: "stage" {
        /// Has not thought about it (precontemplation).
        NotThoughtAbout = "not_thought_about",
        /// Thinking about starting (contemplation).
        Thinking = "thinking",
        /// Has some things put by (preparation).
        HaveSomeThings = "have_some_things",
        /// Has a plan and is working through it (action).
        HaveAPlan = "have_a_plan",
        /// Keeping supplies and plans up to date (maintenance).
        Maintaining = "maintaining",
    }
}

impl PlanInput {
    /// A valid skeleton with every section filled in; the interview starts from it (the engine's
    /// `defaults()` function returns it).
    ///
    /// `planning_date` ([`PLACEHOLDER_PLANNING_DATE`]) and `location.zip` ([`PLACEHOLDER_ZIP`])
    /// are placeholders the app must replace. Safety equipment defaults to absent, so a skipped
    /// question leads to a recommendation rather than an assumption.
    pub fn defaults() -> PlanInput {
        PlanInput {
            planning_date: PLACEHOLDER_PLANNING_DATE,
            location: LocationInput {
                country: "US".to_owned(),
                zip: Some(PLACEHOLDER_ZIP.to_owned()),
                county_fips: None,
                setting: Setting::Suburban,
            },
            housing: Housing {
                kind: HousingKind::Detached,
                tenure: Tenure::Own,
                floor: 1,
                basement: false,
                water: WaterSource::Municipal,
                sewer: Wastewater::Sewer,
                heating: Heating::Gas,
                cooling: Cooling::Central,
                backup_power: BackupPower::None,
                alarms: Alarms::default(),
            },
            people: vec![Person {
                age_band: AgeBand::Adult,
                pregnant_or_nursing: false,
                medical: Medical {
                    daily_rx: false,
                    refrigerated_rx: false,
                    powered_device: PoweredDevice::None,
                    mobility: Mobility::None,
                    dietary: Vec::new(),
                    epinephrine: false,
                },
                earner: true,
                commute: None,
            }],
            pets: Pets::default(),
            mobility: HouseholdMobility::default(),
            finances: Finances {
                monthly_budget_usd: 0.0,
                one_off_budget_usd: 0.0,
                emergency_fund_months: 0.0,
                monthly_expenses_usd: None,
                income: Income {
                    earners: 1,
                    stability: IncomeStability::Stable,
                },
                insurance: Insurance::default(),
            },
            existing: Vec::new(),
            assume_basics: true,
            dials: Dials {
                return_period: ReturnPeriod::OneIn100,
                climate: ClimateHorizon::Today,
                horizon_years: 10,
                water_level: WaterLevel::Basic,
                scenario_overrides: Vec::new(),
                rare_catastrophic_opt_in: false,
            },
            stage: None,
            confidence_1to5: None,
        }
    }
}

impl Default for PlanInput {
    fn default() -> Self {
        PlanInput::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_serialise_every_section_and_round_trip() {
        let d = PlanInput::defaults();
        let v = serde_json::to_value(&d).unwrap();
        for key in [
            "planning_date",
            "location",
            "housing",
            "people",
            "pets",
            "mobility",
            "finances",
            "existing",
            "assume_basics",
            "dials",
        ] {
            assert!(v.get(key).is_some(), "defaults() is missing {key}");
        }
        assert_eq!(v["assume_basics"], true);
        assert_eq!(v["dials"]["water_level"], "basic");
        assert_eq!(v["dials"]["return_period"], "one_in_100");
        assert_eq!(v["dials"]["scenario_overrides"], serde_json::json!([]));
        assert_eq!(v["dials"]["rare_catastrophic_opt_in"], false);
        assert_eq!(v["location"]["zip"], PLACEHOLDER_ZIP);
        assert_eq!(v["planning_date"], "2026-10-01");
        assert!(v.get("stage").is_none() && v.get("confidence_1to5").is_none());
        let back: PlanInput = serde_json::from_value(v).unwrap();
        assert_eq!(back, d);
        assert_eq!(PlanInput::default(), d);
    }

    #[test]
    fn powered_device_json_shapes() {
        assert_eq!(
            serde_json::to_string(&PoweredDevice::Cpap).unwrap(),
            "\"cpap\""
        );
        assert_eq!(
            serde_json::to_string(&PoweredDevice::None).unwrap(),
            "\"none\""
        );
        let other = PoweredDevice::Other { watts: 60.0 };
        let json = serde_json::to_string(&other).unwrap();
        assert_eq!(json, r#"{"other":{"watts":60.0}}"#);
        assert_eq!(serde_json::from_str::<PoweredDevice>(&json).unwrap(), other);
        assert!(
            serde_json::from_str::<PoweredDevice>(r#"{"other":{"watts":60,"volts":120}}"#).is_err()
        );
        assert!(serde_json::from_str::<PoweredDevice>("\"other\"").is_err());
        assert!(serde_json::from_str::<PoweredDevice>("\"CPAP\"").is_err());
    }

    #[test]
    fn optional_dials_default_when_absent() {
        let dials: Dials = serde_json::from_str(
            r#"{"return_period":"one_in_100","climate":"today","horizon_years":10}"#,
        )
        .unwrap();
        assert_eq!(dials.water_level, WaterLevel::Basic);
        assert!(dials.scenario_overrides.is_empty());
        assert!(!dials.rare_catastrophic_opt_in);
        let dials: Dials = serde_json::from_str(
            r#"{"return_period":"one_in_500","climate":"y2050","horizon_years":10,"water_level":"survival",
                "scenario_overrides":[{"id":"cascadia_m9","on":false}],"rare_catastrophic_opt_in":true}"#,
        )
        .unwrap();
        assert_eq!(dials.water_level, WaterLevel::Survival);
        assert_eq!(dials.return_period, ReturnPeriod::OneIn500);
        assert_eq!(
            dials.scenario_overrides,
            [ScenarioToggle {
                id: "cascadia_m9".into(),
                on: false
            }]
        );
        assert!(dials.rare_catastrophic_opt_in);
        for bad in [
            r#"{"return_period":"one_in_100","climate":"today","horizon_years":10,"water_level":"lots"}"#,
            r#"{"climate":"today","horizon_years":10}"#,
            r#"{"confidence":"nine_in_ten","climate":"today","horizon_years":10}"#,
            r#"{"return_period":"one_in_1000","climate":"today","horizon_years":10}"#,
        ] {
            assert!(serde_json::from_str::<Dials>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn assume_basics_defaults_to_true_when_absent() {
        let mut v = serde_json::to_value(PlanInput::defaults()).unwrap();
        assert_eq!(v["assume_basics"], true);
        v.as_object_mut().unwrap().remove("assume_basics");
        let parsed: PlanInput = serde_json::from_value(v.clone()).unwrap();
        assert!(
            parsed.assume_basics,
            "absent assume_basics must default to true"
        );

        v["assume_basics"] = serde_json::json!(false);
        let parsed: PlanInput = serde_json::from_value(v).unwrap();
        assert!(!parsed.assume_basics);
        assert_eq!(
            serde_json::to_value(&parsed).unwrap()["assume_basics"],
            false
        );
    }

    #[test]
    fn unknown_fields_are_rejected_at_every_level() {
        let mut v = serde_json::to_value(PlanInput::defaults()).unwrap();
        v["housing"]["alarms"]["sprinklers"] = serde_json::json!(true);
        let err = serde_json::from_value::<PlanInput>(v).unwrap_err();
        assert!(
            err.to_string().contains("unknown field `sprinklers`"),
            "{err}"
        );

        let mut v = serde_json::to_value(PlanInput::defaults()).unwrap();
        v["setings"] = serde_json::json!({});
        assert!(serde_json::from_value::<PlanInput>(v).is_err());

        let mut v = serde_json::to_value(PlanInput::defaults()).unwrap();
        v["people"][0]["medical"]["allergies"] = serde_json::json!([]);
        assert!(serde_json::from_value::<PlanInput>(v).is_err());
    }

    #[test]
    fn return_periods() {
        let years: Vec<u16> = ReturnPeriod::ALL.iter().map(|r| r.years()).collect();
        assert_eq!(years, [10, 50, 100, 500]);
        let names: Vec<&str> = ReturnPeriod::ALL.iter().map(|r| r.name()).collect();
        assert_eq!(
            names,
            [
                "Common disruptions",
                "Serious",
                "Very serious",
                "Rare catastrophes"
            ]
        );
        assert_eq!(ReturnPeriod::default(), ReturnPeriod::OneIn100);
        assert_eq!(ReturnPeriod::OneIn10.as_str(), "one_in_10");
    }

    #[test]
    fn very_stable_income_is_first_and_defaults_are_unchanged() {
        // `very_stable` (tenured, public sector, pension) is the newest, most-protected option and
        // sorts before `stable`; `defaults()` still starts a household at the ordinary `stable`.
        assert_eq!(IncomeStability::ALL[0], IncomeStability::VeryStable);
        assert_eq!(IncomeStability::VeryStable.as_str(), "very_stable");
        assert_eq!(
            PlanInput::defaults().finances.income.stability,
            IncomeStability::Stable
        );
        assert_eq!(
            "very_stable".parse::<IncomeStability>().unwrap(),
            IncomeStability::VeryStable
        );
    }
}
