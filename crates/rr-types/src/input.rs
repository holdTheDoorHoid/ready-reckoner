//! Inputs: the household and the dials (DESIGN §4.1).
//!
//! Every input struct denies unknown fields, so a typo in the web app, a fixture or an imported
//! plan fails loudly instead of being ignored. Optional fields are omitted from JSON when absent.

use serde::{Deserialize, Serialize};

use crate::{Date, HazardId, ItemId, RARE_FAMILY_ALL, rare_family_ids};

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
    /// The household's own emergency plan (meeting places, contacts, shut-offs, the trusted
    /// circle), captured on a device-only screen and printed in the packet and on wallet cards.
    /// Never used for computation. Absent if the household has not filled it in (contract v2;
    /// REVIEW N1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family_plan: Option<FamilyPlan>,
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
    /// Someone sleeps below street level (a basement bedroom or flat), where flash floods trap
    /// people (contract v2; REVIEW R3, backtest M-09). Defaults to false when absent.
    #[serde(default)]
    pub below_grade_bedroom: bool,
    /// What the main stove runs on (contract v2; REVIEW K1). Absent if not asked; the engine
    /// then assumes nothing about cooking without power.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooking: Option<CookingFuel>,
    /// Untreated water the household could filter or treat if the taps stop (contract v2; REVIEW
    /// S6). A water filter's days count only with a source. Absent if not asked, which counts as
    /// no source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_water_source: Option<RawWaterSource>,
    /// What the household knows of its public water system's record (contract v2; REVIEW R2).
    /// Absent if not asked, which counts as `unknown`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_system_record: Option<WaterSystemRecord>,
}

string_enum! {
    /// What the home's main stove runs on (REVIEW K1). A gas range can still boil water in a
    /// power cut while the gas flows.
    pub enum CookingFuel: "cooking fuel" {
        /// An electric range or cooktop (coil or smooth top).
        Electric = "electric",
        /// A gas range (natural gas or propane).
        Gas = "gas",
        /// An induction cooktop.
        Induction = "induction",
        /// No stove: a microwave, a hot plate or nothing.
        None = "none",
    }
}

string_enum! {
    /// A source of untreated water the household could filter or treat if the taps stop (REVIEW
    /// S6).
    pub enum RawWaterSource: "raw water source" {
        /// None within reach.
        None = "none",
        /// The household's own well, drawn by hand or with backup power.
        Well = "well",
        /// A stream, river, pond or lake within walking distance.
        SurfaceNearby = "surface_nearby",
        /// A rain barrel or cistern.
        RainBarrel = "rain_barrel",
        /// A neighbour's well the household may use.
        NeighbourWell = "neighbour_well",
    }
}

string_enum! {
    /// What the household knows of its public water system's record (REVIEW R2). The engine
    /// combines it with the county's EPA drinking-water violations.
    pub enum WaterSystemRecord: "water system record" {
        /// No boil-water notices or problems the household remembers.
        Fine = "fine",
        /// A boil-water notice or a problem now and then.
        OccasionalNotices = "occasional_notices",
        /// Frequent notices, outages or water-quality problems.
        FrequentProblems = "frequent_problems",
        /// The household does not know.
        Unknown = "unknown",
    }
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
    /// Needs that change how the person gets warnings, help or care in an emergency (CMIST;
    /// contract v2, REVIEW N3). Defaults to none when absent.
    #[serde(default)]
    pub access_needs: Vec<AccessNeed>,
}

string_enum! {
    /// A need that changes how a person gets warnings, help or care in an emergency: the CMIST
    /// framework of communication, maintaining health, independence, support and safety, and
    /// transportation (REVIEW N3). The plan uses them for the communication plan, registries and
    /// evacuation help. Mobility and medical devices have their own fields in [`Medical`].
    pub enum AccessNeed: "access need" {
        /// Deaf or hard of hearing: alerts must be seen or felt, not only heard.
        Hearing = "hearing",
        /// Blind or low vision.
        Vision = "vision",
        /// Speaks or reads little English.
        LimitedEnglish = "limited_english",
        /// A cognitive or intellectual disability, or dementia.
        Cognitive = "cognitive",
        /// Needs someone with them at all times.
        Supervision = "supervision",
        /// Has a service animal.
        ServiceAnimal = "service_animal",
        /// Needs dialysis on a schedule.
        Dialysis = "dialysis",
        /// Relies on home health care or a personal care aide.
        HomeHealth = "home_health",
    }
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
    /// Pay or benefits the household relies on that a government shutdown or lapse can stop
    /// (contract v2; REVIEW H7). Only households that tick one see the benefit-interruption
    /// hazard. Defaults to none when absent.
    #[serde(default)]
    pub benefits: Vec<Benefit>,
}

string_enum! {
    /// Pay or a benefit that a government shutdown or a funding lapse can stop (REVIEW H7).
    pub enum Benefit: "benefit" {
        /// Federal pay: a federal job or a federal contract.
        FederalPay = "federal_pay",
        /// SNAP or WIC food benefits.
        SnapWic = "snap_wic",
        /// Supplemental Security Income or Social Security Disability Insurance.
        SsiSsdi = "ssi_ssdi",
        /// Veterans' benefits.
        Va = "va",
        /// Unemployment benefits.
        Unemployment = "unemployment",
    }
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
    /// Sewer or water backup cover (usually an add-on to a home policy). Absent if not asked
    /// (contract v2; REVIEW N4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sewer_backup: Option<bool>,
    /// Life or disability insurance for the earners. Absent if not asked (contract v2; REVIEW
    /// N4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life_or_disability: Option<bool>,
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
    /// When the household last tried it and it worked, for items that need testing (a jump pack,
    /// a generator, a key safe, flashlights: those with [`crate::Item::test_interval_months`]).
    /// Absent if never recorded (contract v2; the Deviant Ollam lessons).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tested_on: Option<Date>,
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
    /// items otherwise get $0 (`Item.rare_catastrophic`). Since contract v2 this switch means
    /// `rare_opt_in = ["all"]`; read both through [`Dials::rare_families`].
    #[serde(default)]
    pub rare_catastrophic_opt_in: bool,
    /// The rare-event families the rare allowance may buy for, by family id
    /// ([`HazardId::family`]), or `["all"]` for every family. Empty (the default) keeps the
    /// allowance off unless `rare_catastrophic_opt_in` is on (contract v2; REVIEW §2.4, H8).
    #[serde(default)]
    pub rare_opt_in: Vec<String>,
    /// Bare-minimum mode: schedule the smallest kit that covers three days of water, light,
    /// warmth and medicine first (contract v2; REVIEW R6). The engine also warns when a plan runs
    /// past 36 months. Defaults to false when absent.
    #[serde(default)]
    pub minimum_kit: bool,
    /// Show the long-horizon section (rain catchment, fuel storage, sanitation for months) even
    /// when no target passes 30 days; it is on anyway when one does (contract v2). Defaults to
    /// false when absent.
    #[serde(default)]
    pub long_horizon: bool,
}

impl Dials {
    /// The rare-event families the household allows the rare allowance to buy for, as family ids
    /// in [`HazardId::RARE`] order: every family when `rare_catastrophic_opt_in` is on (the v1
    /// switch maps to `rare_opt_in = ["all"]`) or `rare_opt_in` holds `"all"`; otherwise the
    /// families `rare_opt_in` names, each once. Ids that name no family are left out
    /// ([`PlanInput::validate`] reports them). Empty means the allowance stays off (the default).
    pub fn rare_families(&self) -> Vec<&'static str> {
        let all =
            self.rare_catastrophic_opt_in || self.rare_opt_in.iter().any(|f| f == RARE_FAMILY_ALL);
        rare_family_ids()
            .filter(|family| all || self.rare_opt_in.iter().any(|f| f == family))
            .collect()
    }

    /// True when the rare allowance may buy for `hazard`'s family (see [`Dials::rare_families`]);
    /// false for a hazard that heads no family.
    pub fn allows_rare(&self, hazard: HazardId) -> bool {
        hazard
            .family()
            .is_some_and(|family| self.rare_families().contains(&family))
    }
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

/// Longest free-text family-plan note the engine keeps, in characters; [`FamilyPlan::tidy`] cuts
/// anything longer. The web form uses the same limit.
pub const FAMILY_PLAN_TEXT_MAX: usize = 300;

/// Longest name, phone number or number-by-heart the engine keeps, in characters.
pub const FAMILY_PLAN_SHORT_MAX: usize = 80;

/// Most people in the trusted circle.
pub const TRUSTED_CIRCLE_MAX: usize = 4;

/// Most routes out of the area.
pub const ROUTES_MAX: usize = 2;

/// Most numbers known by heart.
pub const NUMBERS_BY_HEART_MAX: usize = 5;

/// The household's own emergency plan (REVIEW N1; DESIGN-DELTA §1.1), captured on a device-only
/// screen and printed after the packet's summary and on wallet cards.
///
/// Every field is optional free text. The engine never computes with it and never requires any
/// of it: [`PlanInput::from_json`] only trims it and caps its length ([`FamilyPlan::tidy`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyPlan {
    /// Where to meet near home if the household cannot get in (for example "the mailbox at the
    /// corner").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meeting_place_near: Option<String>,
    /// Where to meet outside the neighbourhood if the household cannot get home.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meeting_place_far: Option<String>,
    /// Someone out of the area everyone checks in with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_area_contact: Option<Contact>,
    /// Who picks the children up from school or child care, and the school's release plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub school_pickup: Option<String>,
    /// What each worker does in an emergency: stay put, come home, the workplace's own plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_plans: Option<String>,
    /// The safest place to shelter at home (for example "the inner hallway downstairs").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shelter_spot_home: Option<String>,
    /// The safest place to shelter at work or school.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shelter_spot_work: Option<String>,
    /// Where the household would go if it had to leave (a friend, a relative, a town with hotels).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_we_would_go: Option<String>,
    /// Two different routes out of the area (at most [`ROUTES_MAX`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<String>,
    /// Neighbours who check on the household, and whom it checks on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neighbours_who_check: Option<String>,
    /// Who takes the animals if the household cannot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub who_takes_animals: Option<String>,
    /// Where the gas shut-off is, and the tool that turns it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shutoff_gas: Option<String>,
    /// Where the main water shut-off is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shutoff_water: Option<String>,
    /// Where the electrical panel or main breaker is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shutoff_electric: Option<String>,
    /// People who have agreed in advance to help each other (at most [`TRUSTED_CIRCLE_MAX`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusted_circle: Vec<TrustedPerson>,
    /// A lawyer to call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lawyer: Option<Contact>,
    /// The roadside-assistance number (an insurer, an auto club or a card).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roadside_assistance: Option<String>,
    /// Phone numbers everyone knows by heart (at most [`NUMBERS_BY_HEART_MAX`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numbers_by_heart: Vec<String>,
}

/// Someone to call: a name and a phone number, both free text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    /// The person's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Their phone number, as the household writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

/// Someone in the trusted circle, and what they hold for the household.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPerson {
    /// The person's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Their phone number, as the household writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// What they hold for the household.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holds: Vec<Holds>,
}

string_enum! {
    /// What a member of the trusted circle holds for the household (the Deviant Ollam lessons;
    /// owner decision 2026-09-26).
    pub enum Holds: "held item" {
        /// A spare key to the home.
        SpareKey = "spare_key",
        /// Copies of key documents.
        Documents = "documents",
        /// A medical power of attorney: they may speak with doctors when someone cannot.
        MedicalPoa = "medical_poa",
        /// Backup codes for important accounts, for signing in without the phone.
        BackupCodes = "backup_codes",
    }
}

/// `text` without surrounding whitespace and at most `max` characters long; `None` when nothing
/// is left.
fn tidy_str(text: &str, max: usize) -> Option<String> {
    let kept: String = text.trim().chars().take(max).collect();
    let kept = kept.trim_end();
    (!kept.is_empty()).then(|| kept.to_owned())
}

fn tidy_opt(field: &mut Option<String>, max: usize) {
    *field = field.as_deref().and_then(|t| tidy_str(t, max));
}

fn tidy_list(list: &mut Vec<String>, max_len: usize, max_items: usize) {
    *list = list
        .iter()
        .filter_map(|t| tidy_str(t, max_len))
        .take(max_items)
        .collect();
}

impl Contact {
    /// Trims both fields and caps them at [`FAMILY_PLAN_SHORT_MAX`] characters.
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.name, FAMILY_PLAN_SHORT_MAX);
        tidy_opt(&mut self.phone, FAMILY_PLAN_SHORT_MAX);
    }

    /// True when neither a name nor a phone number is given.
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.phone.is_none()
    }
}

impl TrustedPerson {
    /// Trims the name and phone number and caps them at [`FAMILY_PLAN_SHORT_MAX`] characters.
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.name, FAMILY_PLAN_SHORT_MAX);
        tidy_opt(&mut self.phone, FAMILY_PLAN_SHORT_MAX);
    }

    /// True when the entry says nothing: no name, no phone number, nothing held.
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.phone.is_none() && self.holds.is_empty()
    }
}

impl FamilyPlan {
    /// Trims every field and caps its length, which is all the engine ever does to the family
    /// plan: notes at [`FAMILY_PLAN_TEXT_MAX`] characters, names and numbers at
    /// [`FAMILY_PLAN_SHORT_MAX`], lists at [`ROUTES_MAX`], [`TRUSTED_CIRCLE_MAX`] and
    /// [`NUMBERS_BY_HEART_MAX`] entries. Blank text becomes absent and empty entries are dropped;
    /// nothing is ever required, and nothing is reported as a problem.
    pub fn tidy(&mut self) {
        for note in [
            &mut self.meeting_place_near,
            &mut self.meeting_place_far,
            &mut self.school_pickup,
            &mut self.work_plans,
            &mut self.shelter_spot_home,
            &mut self.shelter_spot_work,
            &mut self.where_we_would_go,
            &mut self.neighbours_who_check,
            &mut self.who_takes_animals,
            &mut self.shutoff_gas,
            &mut self.shutoff_water,
            &mut self.shutoff_electric,
        ] {
            tidy_opt(note, FAMILY_PLAN_TEXT_MAX);
        }
        tidy_opt(&mut self.roadside_assistance, FAMILY_PLAN_SHORT_MAX);
        for contact in [&mut self.out_of_area_contact, &mut self.lawyer] {
            if let Some(c) = contact.as_mut() {
                c.tidy();
            }
            if contact.as_ref().is_some_and(Contact::is_empty) {
                *contact = None;
            }
        }
        tidy_list(&mut self.routes, FAMILY_PLAN_TEXT_MAX, ROUTES_MAX);
        tidy_list(
            &mut self.numbers_by_heart,
            FAMILY_PLAN_SHORT_MAX,
            NUMBERS_BY_HEART_MAX,
        );
        for person in &mut self.trusted_circle {
            person.tidy();
        }
        self.trusted_circle.retain(|p| !p.is_empty());
        self.trusted_circle.truncate(TRUSTED_CIRCLE_MAX);
    }

    /// True when nothing in the plan is filled in.
    pub fn is_empty(&self) -> bool {
        *self == FamilyPlan::default()
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
                below_grade_bedroom: false,
                cooking: None,
                raw_water_source: None,
                water_system_record: None,
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
                access_needs: Vec::new(),
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
                benefits: Vec::new(),
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
                rare_opt_in: Vec::new(),
                minimum_kit: false,
                long_horizon: false,
            },
            stage: None,
            confidence_1to5: None,
            family_plan: None,
        }
    }

    /// Tidies the free text the engine only echoes: the family plan ([`FamilyPlan::tidy`]), which
    /// becomes absent when nothing is left in it. [`PlanInput::from_json`] calls this before
    /// validating, so the CLI and the web app store and print the same text.
    pub fn tidy(&mut self) {
        if let Some(plan) = self.family_plan.as_mut() {
            plan.tidy();
        }
        if self.family_plan.as_ref().is_some_and(FamilyPlan::is_empty) {
            self.family_plan = None;
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
        assert_eq!(v["dials"]["rare_opt_in"], serde_json::json!([]));
        assert_eq!(v["dials"]["minimum_kit"], false);
        assert_eq!(v["dials"]["long_horizon"], false);
        assert_eq!(v["housing"]["below_grade_bedroom"], false);
        assert_eq!(v["finances"]["benefits"], serde_json::json!([]));
        assert_eq!(v["people"][0]["access_needs"], serde_json::json!([]));
        assert!(v.get("family_plan").is_none() && v["housing"].get("cooking").is_none());
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
    fn rare_families_maps_the_v1_switch_to_all() {
        let every: Vec<&str> = rare_family_ids().collect();
        assert_eq!(every.len(), 9);
        let mut d = PlanInput::defaults().dials;
        assert!(d.rare_families().is_empty(), "off by default");
        assert!(!d.allows_rare(HazardId::NuclearAttack));

        // The v1 boolean means every family (DESIGN-DELTA §1.1: it maps to ["all"]).
        d.rare_catastrophic_opt_in = true;
        assert_eq!(d.rare_families(), every);
        let mut all = PlanInput::defaults().dials;
        all.rare_opt_in = vec!["all".into()];
        assert_eq!(all.rare_families(), d.rare_families());

        // A list picks families; order follows HazardId::RARE, repeats collapse, and unknown ids
        // are left out (validation reports them).
        let mut some = PlanInput::defaults().dials;
        some.rare_opt_in = vec![
            "mass_violence".into(),
            "nuclear_attack".into(),
            "nuclear_attack".into(),
            "not_a_family".into(),
            "house_fire".into(),
        ];
        assert_eq!(some.rare_families(), ["nuclear_attack", "mass_violence"]);
        assert!(some.allows_rare(HazardId::NuclearAttack));
        assert!(!some.allows_rare(HazardId::GeomagneticStorm));
        assert!(
            !some.allows_rare(HazardId::HouseFire),
            "ranked hazards head no family"
        );
    }

    #[test]
    fn v1_inputs_default_every_v2_field() {
        // A v1 household, as the web app saved it before contract v2 (no v2 key anywhere).
        let v1 = serde_json::json!({
            "planning_date": "2026-10-01",
            "location": { "country": "US", "zip": "19147", "setting": "urban" },
            "housing": { "kind": "rowhouse", "tenure": "rent", "floor": 1, "basement": true,
                         "water": "municipal", "sewer": "sewer", "heating": "gas",
                         "cooling": "window", "backup_power": "none",
                         "alarms": { "smoke": true, "co": false, "extinguisher": false } },
            "people": [{ "age_band": "adult", "pregnant_or_nursing": false, "earner": true,
                         "medical": { "daily_rx": false, "refrigerated_rx": false,
                                      "powered_device": "none", "mobility": "none",
                                      "dietary": [], "epinephrine": false } }],
            "pets": { "dogs": 0, "cats": 0, "small": 0, "large_animals": 0 },
            "mobility": { "vehicles": [] },
            "finances": { "monthly_budget_usd": 20, "one_off_budget_usd": 0,
                          "emergency_fund_months": 0,
                          "income": { "earners": 1, "stability": "stable" },
                          "insurance": { "home_or_renters": false, "flood": false,
                                         "earthquake": false } },
            "existing": [{ "item_id": "water_stored", "qty": 4 }],
            "dials": { "return_period": "one_in_100", "climate": "today", "horizon_years": 10,
                       "rare_catastrophic_opt_in": true }
        });
        let p: PlanInput = serde_json::from_value(v1).unwrap();
        assert!(p.people[0].access_needs.is_empty());
        let h = &p.housing;
        assert!(!h.below_grade_bedroom);
        assert!(h.cooking.is_none() && h.raw_water_source.is_none());
        assert!(h.water_system_record.is_none());
        assert!(p.finances.benefits.is_empty());
        let ins = p.finances.insurance;
        assert!(ins.sewer_backup.is_none() && ins.life_or_disability.is_none());
        assert!(p.existing[0].tested_on.is_none());
        assert!(p.dials.rare_opt_in.is_empty() && !p.dials.minimum_kit && !p.dials.long_horizon);
        assert!(p.family_plan.is_none());
        assert_eq!(
            p.dials.rare_families().len(),
            9,
            "the v1 switch still opts in"
        );
        assert_eq!(p.validate(), vec![]);
    }

    #[test]
    fn family_plan_tidy_trims_caps_and_never_requires() {
        let long = "x".repeat(FAMILY_PLAN_TEXT_MAX + 50);
        let mut plan = FamilyPlan {
            meeting_place_near: Some("  the corner mailbox \n".into()),
            meeting_place_far: Some("   ".into()),
            out_of_area_contact: Some(Contact {
                name: Some(" ".into()),
                phone: None,
            }),
            where_we_would_go: Some(long.clone()),
            routes: vec![" north ".into(), "".into(), "west".into(), "south".into()],
            trusted_circle: vec![
                TrustedPerson::default(),
                TrustedPerson {
                    name: Some(" Rosa ".into()),
                    phone: Some("555-0100".into()),
                    holds: vec![Holds::SpareKey],
                },
                TrustedPerson {
                    name: None,
                    phone: None,
                    holds: vec![Holds::Documents],
                },
                TrustedPerson {
                    name: Some("B".into()),
                    ..TrustedPerson::default()
                },
                TrustedPerson {
                    name: Some("C".into()),
                    ..TrustedPerson::default()
                },
                TrustedPerson {
                    name: Some("D".into()),
                    ..TrustedPerson::default()
                },
            ],
            lawyer: Some(Contact {
                name: Some("J. Ortiz".into()),
                phone: Some(format!(" {} ", "5".repeat(100))),
            }),
            numbers_by_heart: (0..8).map(|i| format!("555-010{i}")).collect(),
            ..FamilyPlan::default()
        };
        plan.tidy();
        assert_eq!(
            plan.meeting_place_near.as_deref(),
            Some("the corner mailbox")
        );
        assert_eq!(plan.meeting_place_far, None, "blank becomes absent");
        assert_eq!(
            plan.out_of_area_contact, None,
            "an empty contact is dropped"
        );
        assert_eq!(
            plan.where_we_would_go.as_deref().map(|t| t.chars().count()),
            Some(FAMILY_PLAN_TEXT_MAX)
        );
        assert_eq!(
            plan.routes,
            ["north", "west"],
            "blank dropped, then capped at two"
        );
        // The empty person is dropped; the four kept are the first four with something in them.
        assert_eq!(plan.trusted_circle.len(), TRUSTED_CIRCLE_MAX);
        assert_eq!(plan.trusted_circle[0].name.as_deref(), Some("Rosa"));
        assert_eq!(plan.trusted_circle[1].holds, [Holds::Documents]);
        assert_eq!(plan.trusted_circle[3].name.as_deref(), Some("C"));
        let phone = plan.lawyer.as_ref().unwrap().phone.as_deref().unwrap();
        assert_eq!(phone.chars().count(), FAMILY_PLAN_SHORT_MAX);
        assert_eq!(plan.numbers_by_heart.len(), NUMBERS_BY_HEART_MAX);

        // Multi-byte text is cut on character boundaries.
        let mut accents = FamilyPlan {
            school_pickup: Some("é".repeat(FAMILY_PLAN_TEXT_MAX + 1)),
            ..FamilyPlan::default()
        };
        accents.tidy();
        assert_eq!(
            accents.school_pickup.unwrap().chars().count(),
            FAMILY_PLAN_TEXT_MAX
        );

        // A plan with nothing left in it disappears from the input; nothing is ever a problem.
        let mut input = PlanInput::defaults();
        input.family_plan = Some(FamilyPlan {
            work_plans: Some("  ".into()),
            ..FamilyPlan::default()
        });
        input.tidy();
        assert_eq!(input.family_plan, None);
        input.family_plan = Some(plan.clone());
        input.tidy();
        assert_eq!(
            input.family_plan.as_ref(),
            Some(&plan),
            "tidying twice changes nothing"
        );
        assert_eq!(input.validate(), vec![]);
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
