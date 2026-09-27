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
    /// What the household wrote about this person for their binder page and wallet card: name,
    /// phone, medical details, insurance, where they spend the day (contract v3; DESIGN-DELTA-v3
    /// §2.1, §3.1). Echo-only, like [`FamilyPlan`]: trimmed, capped and printed, never computed
    /// with and never required. Omitted when nothing in it is filled in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<PersonProfile>,
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
    /// Show a legal-emergency amount (bail and a lawyer) beside the savings goal, apart from the
    /// months of income it protects. The household turns it on from the arrest row of the risk
    /// register; off by default, because the copy assumes nothing about the household (contract
    /// v2; owner decision 2026-09-26). Left out of the JSON when false.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub legal_opt_in: bool,
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
    /// The home: address, utilities, insurer, landlord, where things are kept (contract v3;
    /// DESIGN-DELTA-v3 §2.2, §3.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<HomeInfo>,
    /// The neighbourhood: hospital, urgent care, pharmacy, shelter, the county emergency office,
    /// how local alerts arrive (contract v3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neighbourhood: Option<Neighbourhood>,
    /// One entry per animal, at most [`PETS_MAX`] (contract v3; DESIGN-DELTA-v3 §2.3). Who takes
    /// the animals stays in `who_takes_animals`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pets: Vec<PetInfo>,
    /// One entry per vehicle, at most [`VEHICLES_MAX`] (contract v3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vehicles: Vec<VehicleInfo>,
    /// Accounts, policies and where the documents are (contract v3). Account numbers are never
    /// asked: only their last four digits ([`AccountInfo::last4`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documents: Option<DocumentsInfo>,
}

/// Someone to call: a name, a phone number and an address, all free text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    /// The person's or the organisation's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Their phone number, as the household writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// Their address, at most [`LONG_TEXT_MAX`] characters (contract v3: a hospital, a
    /// pharmacy, a doctor's office).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
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

// Contract v3 (DESIGN-DELTA-v3 §3.1, §3.2): the answers to the interview's optional steps 6–8.
// Like the family plan they are echo-only: `tidy` trims each string and cuts it to its cap (in
// characters, after trimming), drops blank strings, empty entries and empty groups, and keeps
// lists to their length. Nothing here is ever required or reported as a problem.

/// The last four digits of an account number: [`AccountInfo::last4`] keeps at most this many,
/// and only digits.
pub const LAST4_LEN: usize = 4;

/// Longest blood type ([`PersonProfile::blood_type`]).
pub const BLOOD_TYPE_MAX: usize = 8;

/// Longest licence plate ([`VehicleInfo::plate`]).
pub const PLATE_MAX: usize = 20;

/// Longest short answer: a date of birth, a phone number in the v3 groups, the kind of a pet,
/// an account or a policy.
pub const SHORT_TEXT_MAX: usize = 40;

/// Longest name of a person or an animal, dose, member ID, group, policy or microchip number.
pub const MEDIUM_TEXT_MAX: usize = 60;

/// Longest email address, institution, insurance carrier or plan name, insurer, or a
/// medication's name, schedule or purpose.
pub const LABEL_TEXT_MAX: usize = 80;

/// Longest description: a place's name, an animal or a vehicle described.
pub const DESCRIPTION_MAX: usize = 120;

/// Longest address or "where it is" answer: addresses, where things are kept, allergies,
/// pick-up rules, safe spots, ID notes, how alerts arrive, an animal's medicines, what stays in
/// the car.
pub const LONG_TEXT_MAX: usize = 200;

/// Longest note: medical conditions, anything else a helper should know, a place's own emergency
/// plan.
pub const NOTE_MAX: usize = 400;

/// Most medications per person.
pub const MEDICATIONS_MAX: usize = 12;

/// Most animals in [`FamilyPlan::pets`].
pub const PETS_MAX: usize = 8;

/// Most vehicles in [`FamilyPlan::vehicles`].
pub const VEHICLES_MAX: usize = 4;

/// Most accounts in [`DocumentsInfo::accounts`].
pub const ACCOUNTS_MAX: usize = 12;

/// Most insurance policies in [`DocumentsInfo::policies`].
pub const POLICIES_MAX: usize = 8;

/// One person's page in the binder (DESIGN-DELTA-v3 §2.1): everything a helper would need to know
/// about them, all optional free text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonProfile {
    /// Name or nickname ([`MEDIUM_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Date of birth, as the household writes it ([`SHORT_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<String>,
    /// Phone number ([`SHORT_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// Email address ([`LABEL_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Where they spend the day: work, school, child care.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<Place>,
    /// Their doctor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doctor: Option<Contact>,
    /// Their pharmacy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pharmacy: Option<Contact>,
    /// Medical conditions ([`NOTE_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<String>,
    /// Medications, at most [`MEDICATIONS_MAX`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub medications: Vec<Medication>,
    /// Allergies ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allergies: Option<String>,
    /// Blood type ([`BLOOD_TYPE_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blood_type: Option<String>,
    /// Health insurance (JSON field `insurance`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insurance: Option<HealthInsurance>,
    /// Identity documents: "passport number, or where it is kept" ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_notes: Option<String>,
    /// Anything else a helper should know ([`NOTE_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

string_enum! {
    /// What kind of place a person spends the day at ([`Place::kind`]).
    pub enum PlaceKind: "place kind" {
        /// A workplace.
        Work = "work",
        /// A school or college.
        School = "school",
        /// Child care: a nursery, day care, a sitter.
        Childcare = "childcare",
        /// Anywhere else (a day programme, a volunteer post).
        Other = "other",
    }
}

/// Where a person spends the day, with its own emergency arrangements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    /// Work, school, child care or other.
    pub kind: PlaceKind,
    /// The place's name ([`DESCRIPTION_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its address ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Its phone number ([`SHORT_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// The place's own emergency plan ([`NOTE_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    /// Pick-up rules: who may collect a child, what the school needs to see ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pickup: Option<String>,
    /// The safest spot there ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safest_spot: Option<String>,
}

/// One medication a person takes, as the household writes it. Echoed, never checked: the app
/// gives no doses of its own.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Medication {
    /// Its name ([`LABEL_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The dose, as prescribed ([`MEDIUM_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dose: Option<String>,
    /// When it is taken ([`LABEL_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    /// What it is for ([`LABEL_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// A person's health insurance ([`PersonProfile::insurance`]). Not to be confused with
/// [`Insurance`], the household's insurance answers in [`Finances`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthInsurance {
    /// The insurance company ([`LABEL_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier: Option<String>,
    /// The plan's name ([`LABEL_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_name: Option<String>,
    /// The member ID ([`MEDIUM_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
    /// The group number ([`MEDIUM_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_number: Option<String>,
    /// The insurer's phone number ([`SHORT_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

/// The home (DESIGN-DELTA-v3 §2.2): address, the companies to call, and where things are kept.
/// The shut-offs, the safest spot and the neighbours stay in their v2 [`FamilyPlan`] fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomeInfo {
    /// The street address ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// The electric company, with its outage number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electric_utility: Option<Contact>,
    /// The gas company, with its outage number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_utility: Option<Contact>,
    /// The water company, with its outage number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_utility: Option<Contact>,
    /// The home or renters insurer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insurer: Option<Contact>,
    /// The policy number ([`MEDIUM_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_number: Option<String>,
    /// The landlord or mortgage company.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landlord_or_mortgage: Option<Contact>,
    /// Where the emergency kit is ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_kit: Option<String>,
    /// Where the documents are ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_documents: Option<String>,
    /// Where the cash is ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_cash: Option<String>,
    /// Where the spare keys are ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_keys: Option<String>,
}

/// The neighbourhood (DESIGN-DELTA-v3 §2.2): where help is, and how warnings arrive.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Neighbourhood {
    /// The nearest hospital with an emergency room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hospital: Option<Contact>,
    /// Urgent care.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urgent_care: Option<Contact>,
    /// The pharmacy the household uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pharmacy: Option<Contact>,
    /// The shelter or place the community opens in an emergency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shelter: Option<Contact>,
    /// The county emergency management office.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub county_emergency_office: Option<Contact>,
    /// How the household gets local alerts: the county's alert service, a radio station
    /// ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alerts: Option<String>,
}

/// One animal (DESIGN-DELTA-v3 §2.3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PetInfo {
    /// Its name ([`MEDIUM_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// What kind of animal ([`SHORT_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// What it looks like ([`DESCRIPTION_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its medicines, as the household writes them ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub medications: Option<String>,
    /// Its vet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vet: Option<Contact>,
    /// Microchip or tag number ([`MEDIUM_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub microchip: Option<String>,
    /// Where its records are ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub records_where: Option<String>,
}

/// One vehicle as the household describes it (DESIGN-DELTA-v3 §2.3). Its fuel, which the plan
/// computes with, stays in [`HouseholdMobility`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleInfo {
    /// "blue 2016 hatchback" ([`DESCRIPTION_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The licence plate ([`PLATE_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plate: Option<String>,
    /// The vehicle's insurer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insurer: Option<Contact>,
    /// The policy number ([`MEDIUM_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_number: Option<String>,
    /// What stays in the car ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept_in_car: Option<String>,
}

/// Documents and money (DESIGN-DELTA-v3 §2.3): the accounts and policies to call about, and
/// where the papers are.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentsInfo {
    /// Bank and other accounts, at most [`ACCOUNTS_MAX`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<AccountInfo>,
    /// Insurance policies not given elsewhere, at most [`POLICIES_MAX`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policies: Vec<PolicyInfo>,
    /// Where the original documents are ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_originals: Option<String>,
    /// Where the copies are ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_copies: Option<String>,
    /// Where the digital backup is ([`LONG_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digital_backup: Option<String>,
}

/// An account to call about (DESIGN-DELTA-v3 §2.3). The app never asks for an account number.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountInfo {
    /// The bank or institution ([`LABEL_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub institution: Option<String>,
    /// What kind of account ([`SHORT_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The institution's phone number ([`SHORT_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// The last four digits of the account number, and nothing more: `tidy` keeps only the
    /// last [`LAST4_LEN`] digits of whatever was typed, so a pasted full number never reaches the
    /// file, and drops the field when it holds no digit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last4: Option<String>,
}

/// An insurance policy (DESIGN-DELTA-v3 §2.3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyInfo {
    /// The insurer ([`LABEL_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insurer: Option<String>,
    /// What kind of policy ([`SHORT_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The policy number ([`MEDIUM_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_number: Option<String>,
    /// The insurer's phone number ([`SHORT_TEXT_MAX`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
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

/// The echo-only groups, for the helpers below: tidy in place, then say whether anything is left.
trait Echo {
    fn tidy_echo(&mut self);
    fn is_blank(&self) -> bool;
}

macro_rules! echo_groups {
    ($($t:ty),+ $(,)?) => {
        $(impl Echo for $t {
            fn tidy_echo(&mut self) {
                self.tidy();
            }
            fn is_blank(&self) -> bool {
                self.is_empty()
            }
        })+
    };
}

echo_groups!(
    Contact,
    PersonProfile,
    Place,
    Medication,
    HealthInsurance,
    HomeInfo,
    Neighbourhood,
    PetInfo,
    VehicleInfo,
    DocumentsInfo,
    AccountInfo,
    PolicyInfo,
);

/// Tidies an optional group and drops it when nothing is left in it.
fn tidy_group<T: Echo>(group: &mut Option<T>) {
    if let Some(g) = group.as_mut() {
        g.tidy_echo();
    }
    if group.as_ref().is_some_and(Echo::is_blank) {
        *group = None;
    }
}

/// Tidies every row, drops the rows left empty, then keeps the first `max`.
fn tidy_rows<T: Echo>(rows: &mut Vec<T>, max: usize) {
    for row in rows.iter_mut() {
        row.tidy_echo();
    }
    rows.retain(|row| !row.is_blank());
    rows.truncate(max);
}

/// The last [`LAST4_LEN`] ASCII digits in `text`, in order (fewer when it has fewer); `None` when
/// it holds no digit.
fn last_four_digits(text: &str) -> Option<String> {
    let digits: Vec<char> = text.chars().filter(char::is_ascii_digit).collect();
    let keep = &digits[digits.len().saturating_sub(LAST4_LEN)..];
    (!keep.is_empty()).then(|| keep.iter().collect())
}

impl Contact {
    /// Trims every field and caps the name and phone number at [`FAMILY_PLAN_SHORT_MAX`]
    /// characters and the address at [`LONG_TEXT_MAX`].
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.name, FAMILY_PLAN_SHORT_MAX);
        tidy_opt(&mut self.phone, FAMILY_PLAN_SHORT_MAX);
        tidy_opt(&mut self.address, LONG_TEXT_MAX);
    }

    /// True when no name, phone number or address is given.
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.phone.is_none() && self.address.is_none()
    }
}

impl PersonProfile {
    /// Trims every answer and caps it (the cap each field names), drops empty medications and
    /// keeps at most [`MEDICATIONS_MAX`], and drops the place, doctor, pharmacy and insurance when
    /// nothing is left in them.
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.name, MEDIUM_TEXT_MAX);
        tidy_opt(&mut self.date_of_birth, SHORT_TEXT_MAX);
        tidy_opt(&mut self.phone, SHORT_TEXT_MAX);
        tidy_opt(&mut self.email, LABEL_TEXT_MAX);
        tidy_group(&mut self.place);
        tidy_group(&mut self.doctor);
        tidy_group(&mut self.pharmacy);
        tidy_opt(&mut self.conditions, NOTE_MAX);
        tidy_rows(&mut self.medications, MEDICATIONS_MAX);
        tidy_opt(&mut self.allergies, LONG_TEXT_MAX);
        tidy_opt(&mut self.blood_type, BLOOD_TYPE_MAX);
        tidy_group(&mut self.insurance);
        tidy_opt(&mut self.id_notes, LONG_TEXT_MAX);
        tidy_opt(&mut self.notes, NOTE_MAX);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == PersonProfile::default()
    }
}

impl Place {
    /// Trims every answer and caps it (the cap each field names).
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.name, DESCRIPTION_MAX);
        tidy_opt(&mut self.address, LONG_TEXT_MAX);
        tidy_opt(&mut self.phone, SHORT_TEXT_MAX);
        tidy_opt(&mut self.plan, NOTE_MAX);
        tidy_opt(&mut self.pickup, LONG_TEXT_MAX);
        tidy_opt(&mut self.safest_spot, LONG_TEXT_MAX);
    }

    /// True when nothing but the kind is given. A kind alone tells a helper nothing (the form
    /// preselects one), so [`PersonProfile::tidy`] drops such a place.
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.address.is_none()
            && self.phone.is_none()
            && self.plan.is_none()
            && self.pickup.is_none()
            && self.safest_spot.is_none()
    }
}

impl Medication {
    /// Trims every answer and caps it (the cap each field names).
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.name, LABEL_TEXT_MAX);
        tidy_opt(&mut self.dose, MEDIUM_TEXT_MAX);
        tidy_opt(&mut self.schedule, LABEL_TEXT_MAX);
        tidy_opt(&mut self.purpose, LABEL_TEXT_MAX);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == Medication::default()
    }
}

impl HealthInsurance {
    /// Trims every answer and caps it (the cap each field names).
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.carrier, LABEL_TEXT_MAX);
        tidy_opt(&mut self.plan_name, LABEL_TEXT_MAX);
        tidy_opt(&mut self.member_id, MEDIUM_TEXT_MAX);
        tidy_opt(&mut self.group_number, MEDIUM_TEXT_MAX);
        tidy_opt(&mut self.phone, SHORT_TEXT_MAX);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == HealthInsurance::default()
    }
}

impl HomeInfo {
    /// Trims every answer and caps it (the cap each field names); drops a company or contact
    /// with nothing left in it.
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.address, LONG_TEXT_MAX);
        tidy_group(&mut self.electric_utility);
        tidy_group(&mut self.gas_utility);
        tidy_group(&mut self.water_utility);
        tidy_group(&mut self.insurer);
        tidy_opt(&mut self.policy_number, MEDIUM_TEXT_MAX);
        tidy_group(&mut self.landlord_or_mortgage);
        tidy_opt(&mut self.where_kit, LONG_TEXT_MAX);
        tidy_opt(&mut self.where_documents, LONG_TEXT_MAX);
        tidy_opt(&mut self.where_cash, LONG_TEXT_MAX);
        tidy_opt(&mut self.where_keys, LONG_TEXT_MAX);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == HomeInfo::default()
    }
}

impl Neighbourhood {
    /// Trims every answer and caps it (the cap each field names); drops a contact with nothing
    /// left in it.
    pub fn tidy(&mut self) {
        tidy_group(&mut self.hospital);
        tidy_group(&mut self.urgent_care);
        tidy_group(&mut self.pharmacy);
        tidy_group(&mut self.shelter);
        tidy_group(&mut self.county_emergency_office);
        tidy_opt(&mut self.alerts, LONG_TEXT_MAX);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == Neighbourhood::default()
    }
}

impl PetInfo {
    /// Trims every answer and caps it (the cap each field names).
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.name, MEDIUM_TEXT_MAX);
        tidy_opt(&mut self.kind, SHORT_TEXT_MAX);
        tidy_opt(&mut self.description, DESCRIPTION_MAX);
        tidy_opt(&mut self.medications, LONG_TEXT_MAX);
        tidy_group(&mut self.vet);
        tidy_opt(&mut self.microchip, MEDIUM_TEXT_MAX);
        tidy_opt(&mut self.records_where, LONG_TEXT_MAX);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == PetInfo::default()
    }
}

impl VehicleInfo {
    /// Trims every answer and caps it (the cap each field names).
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.description, DESCRIPTION_MAX);
        tidy_opt(&mut self.plate, PLATE_MAX);
        tidy_group(&mut self.insurer);
        tidy_opt(&mut self.policy_number, MEDIUM_TEXT_MAX);
        tidy_opt(&mut self.kept_in_car, LONG_TEXT_MAX);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == VehicleInfo::default()
    }
}

impl DocumentsInfo {
    /// Trims every answer and caps it (the cap each field names); drops empty accounts and
    /// policies and keeps at most [`ACCOUNTS_MAX`] and [`POLICIES_MAX`].
    pub fn tidy(&mut self) {
        tidy_rows(&mut self.accounts, ACCOUNTS_MAX);
        tidy_rows(&mut self.policies, POLICIES_MAX);
        tidy_opt(&mut self.where_originals, LONG_TEXT_MAX);
        tidy_opt(&mut self.where_copies, LONG_TEXT_MAX);
        tidy_opt(&mut self.digital_backup, LONG_TEXT_MAX);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == DocumentsInfo::default()
    }
}

impl AccountInfo {
    /// Trims every answer and caps it (the cap each field names). `last4` keeps only the last
    /// [`LAST4_LEN`] digits of whatever was typed ("1234 5678 9012" becomes "9012") and is dropped
    /// when it holds no digit.
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.institution, LABEL_TEXT_MAX);
        tidy_opt(&mut self.kind, SHORT_TEXT_MAX);
        tidy_opt(&mut self.phone, SHORT_TEXT_MAX);
        self.last4 = self.last4.as_deref().and_then(last_four_digits);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == AccountInfo::default()
    }
}

impl PolicyInfo {
    /// Trims every answer and caps it (the cap each field names).
    pub fn tidy(&mut self) {
        tidy_opt(&mut self.insurer, LABEL_TEXT_MAX);
        tidy_opt(&mut self.kind, SHORT_TEXT_MAX);
        tidy_opt(&mut self.policy_number, MEDIUM_TEXT_MAX);
        tidy_opt(&mut self.phone, SHORT_TEXT_MAX);
    }

    /// True when nothing is filled in.
    pub fn is_empty(&self) -> bool {
        *self == PolicyInfo::default()
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
    /// [`NUMBERS_BY_HEART_MAX`] entries; the v3 groups at the caps their fields name, with at most
    /// [`PETS_MAX`] animals and [`VEHICLES_MAX`] vehicles. Blank text becomes absent, empty entries
    /// and groups are dropped; nothing is ever required, and nothing is reported as a problem.
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
        tidy_group(&mut self.out_of_area_contact);
        tidy_group(&mut self.lawyer);
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
        tidy_group(&mut self.home);
        tidy_group(&mut self.neighbourhood);
        tidy_rows(&mut self.pets, PETS_MAX);
        tidy_rows(&mut self.vehicles, VEHICLES_MAX);
        tidy_group(&mut self.documents);
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
                profile: None,
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
                legal_opt_in: false,
            },
            stage: None,
            confidence_1to5: None,
            family_plan: None,
        }
    }

    /// Tidies the free text the engine only echoes: each person's profile
    /// ([`PersonProfile::tidy`]) and the family plan ([`FamilyPlan::tidy`]), each of which becomes
    /// absent when nothing is left in it. [`PlanInput::from_json`] calls this before validating,
    /// so the CLI and the web app store and print the same text.
    pub fn tidy(&mut self) {
        for person in &mut self.people {
            tidy_group(&mut person.profile);
        }
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
                ..Contact::default()
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
                address: None,
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

    /// `n` characters of `c`, with spaces around it.
    fn padded(c: char, n: usize) -> Option<String> {
        Some(format!("  {}  ", c.to_string().repeat(n)))
    }

    fn len(text: &Option<String>) -> Option<usize> {
        text.as_deref().map(|t| t.chars().count())
    }

    #[test]
    fn last4_keeps_only_the_last_four_digits() {
        let tidy = |typed: &str| {
            let mut a = AccountInfo {
                last4: Some(typed.to_owned()),
                ..AccountInfo::default()
            };
            a.tidy();
            a.last4
        };
        assert_eq!(tidy("1234 5678 9012 4821").as_deref(), Some("4821"));
        assert_eq!(tidy("4821").as_deref(), Some("4821"));
        assert_eq!(tidy(" •••• 0042 ").as_deref(), Some("0042"));
        assert_eq!(tidy("acct-99").as_deref(), Some("99"), "fewer digits stay");
        assert_eq!(tidy("12-34-56-78-90").as_deref(), Some("7890"));
        assert_eq!(tidy("none"), None, "no digit, no field");
        assert_eq!(tidy("   "), None);
        // An account left with nothing else is dropped from the documents.
        let mut docs = DocumentsInfo {
            accounts: vec![AccountInfo {
                last4: Some("n/a".into()),
                ..AccountInfo::default()
            }],
            ..DocumentsInfo::default()
        };
        docs.tidy();
        assert!(docs.is_empty());
    }

    #[test]
    fn v3_answers_are_trimmed_capped_and_emptied() {
        let long = |c| padded(c, 1000);
        let mut profile = PersonProfile {
            name: long('n'),
            date_of_birth: long('d'),
            phone: long('p'),
            email: long('e'),
            place: Some(Place {
                kind: PlaceKind::School,
                name: long('a'),
                address: long('b'),
                phone: long('c'),
                plan: long('d'),
                pickup: long('e'),
                safest_spot: long('f'),
            }),
            doctor: Some(Contact {
                name: long('g'),
                phone: long('h'),
                address: long('i'),
            }),
            pharmacy: Some(Contact {
                name: Some("   ".into()),
                ..Contact::default()
            }),
            conditions: long('j'),
            medications: (0..20)
                .map(|i| Medication {
                    name: Some(format!(" med {i} ")),
                    dose: long('k'),
                    ..Medication::default()
                })
                .collect(),
            allergies: long('l'),
            blood_type: long('m'),
            insurance: Some(HealthInsurance {
                carrier: long('o'),
                plan_name: long('q'),
                member_id: long('r'),
                group_number: long('s'),
                phone: long('t'),
            }),
            id_notes: long('u'),
            notes: long('v'),
        };
        profile.tidy();
        assert_eq!(len(&profile.name), Some(MEDIUM_TEXT_MAX));
        assert_eq!(len(&profile.date_of_birth), Some(SHORT_TEXT_MAX));
        assert_eq!(len(&profile.phone), Some(SHORT_TEXT_MAX));
        assert_eq!(len(&profile.email), Some(LABEL_TEXT_MAX));
        let place = profile.place.as_ref().unwrap();
        assert_eq!(len(&place.name), Some(DESCRIPTION_MAX));
        assert_eq!(len(&place.address), Some(LONG_TEXT_MAX));
        assert_eq!(len(&place.phone), Some(SHORT_TEXT_MAX));
        assert_eq!(len(&place.plan), Some(NOTE_MAX));
        assert_eq!(len(&place.pickup), Some(LONG_TEXT_MAX));
        assert_eq!(len(&place.safest_spot), Some(LONG_TEXT_MAX));
        let doctor = profile.doctor.as_ref().unwrap();
        assert_eq!(len(&doctor.name), Some(FAMILY_PLAN_SHORT_MAX));
        assert_eq!(len(&doctor.phone), Some(FAMILY_PLAN_SHORT_MAX));
        assert_eq!(len(&doctor.address), Some(LONG_TEXT_MAX));
        assert_eq!(profile.pharmacy, None, "a blank contact is dropped");
        assert_eq!(len(&profile.conditions), Some(NOTE_MAX));
        assert_eq!(profile.medications.len(), MEDICATIONS_MAX);
        assert_eq!(profile.medications[0].name.as_deref(), Some("med 0"));
        assert_eq!(len(&profile.medications[0].dose), Some(MEDIUM_TEXT_MAX));
        assert_eq!(len(&profile.allergies), Some(LONG_TEXT_MAX));
        assert_eq!(len(&profile.blood_type), Some(BLOOD_TYPE_MAX));
        let ins = profile.insurance.as_ref().unwrap();
        assert_eq!(len(&ins.carrier), Some(LABEL_TEXT_MAX));
        assert_eq!(len(&ins.plan_name), Some(LABEL_TEXT_MAX));
        assert_eq!(len(&ins.member_id), Some(MEDIUM_TEXT_MAX));
        assert_eq!(len(&ins.group_number), Some(MEDIUM_TEXT_MAX));
        assert_eq!(len(&ins.phone), Some(SHORT_TEXT_MAX));
        assert_eq!(len(&profile.id_notes), Some(LONG_TEXT_MAX));
        assert_eq!(len(&profile.notes), Some(NOTE_MAX));
        let again = profile.clone();
        profile.tidy();
        assert_eq!(profile, again, "tidying twice changes nothing");

        let mut plan = FamilyPlan {
            home: Some(HomeInfo {
                address: long('a'),
                electric_utility: Some(Contact {
                    phone: long('b'),
                    ..Contact::default()
                }),
                gas_utility: Some(Contact::default()),
                policy_number: long('c'),
                where_kit: long('d'),
                where_keys: Some("  with Rosa ".into()),
                ..HomeInfo::default()
            }),
            neighbourhood: Some(Neighbourhood {
                hospital: Some(Contact::default()),
                alerts: Some(" ".into()),
                ..Neighbourhood::default()
            }),
            pets: (0..12)
                .map(|i| PetInfo {
                    name: (i != 0).then(|| format!("Pet {i}")),
                    kind: long('k'),
                    description: long('e'),
                    medications: long('m'),
                    microchip: long('c'),
                    records_where: long('r'),
                    vet: None,
                })
                .collect(),
            vehicles: (0..6)
                .map(|i| VehicleInfo {
                    description: long('v'),
                    plate: Some(format!(" PLATE-{i}-{} ", "X".repeat(40))),
                    policy_number: long('p'),
                    kept_in_car: long('k'),
                    insurer: None,
                })
                .collect(),
            documents: Some(DocumentsInfo {
                accounts: (0..15)
                    .map(|i| AccountInfo {
                        institution: long('i'),
                        kind: long('k'),
                        phone: long('p'),
                        last4: Some(format!("0000 1111 2222 {:04}", i)),
                    })
                    .collect(),
                policies: (0..10)
                    .map(|_| PolicyInfo {
                        insurer: long('i'),
                        kind: long('k'),
                        policy_number: long('n'),
                        phone: long('p'),
                    })
                    .collect(),
                where_originals: long('o'),
                where_copies: long('c'),
                digital_backup: long('d'),
            }),
            ..FamilyPlan::default()
        };
        plan.tidy();
        let home = plan.home.as_ref().unwrap();
        assert_eq!(len(&home.address), Some(LONG_TEXT_MAX));
        assert_eq!(
            len(&home.electric_utility.as_ref().unwrap().phone),
            Some(FAMILY_PLAN_SHORT_MAX)
        );
        assert_eq!(home.gas_utility, None, "an empty company is dropped");
        assert_eq!(len(&home.policy_number), Some(MEDIUM_TEXT_MAX));
        assert_eq!(len(&home.where_kit), Some(LONG_TEXT_MAX));
        assert_eq!(home.where_keys.as_deref(), Some("with Rosa"));
        assert_eq!(
            plan.neighbourhood, None,
            "a group with nothing left is dropped"
        );
        assert_eq!(plan.pets.len(), PETS_MAX);
        let pet = &plan.pets[0];
        assert_eq!(pet.name, None);
        assert_eq!(len(&pet.kind), Some(SHORT_TEXT_MAX));
        assert_eq!(len(&pet.description), Some(DESCRIPTION_MAX));
        assert_eq!(len(&pet.medications), Some(LONG_TEXT_MAX));
        assert_eq!(len(&pet.microchip), Some(MEDIUM_TEXT_MAX));
        assert_eq!(len(&pet.records_where), Some(LONG_TEXT_MAX));
        assert_eq!(plan.vehicles.len(), VEHICLES_MAX);
        assert_eq!(len(&plan.vehicles[0].plate), Some(PLATE_MAX));
        assert_eq!(len(&plan.vehicles[0].description), Some(DESCRIPTION_MAX));
        assert_eq!(len(&plan.vehicles[0].policy_number), Some(MEDIUM_TEXT_MAX));
        assert_eq!(len(&plan.vehicles[0].kept_in_car), Some(LONG_TEXT_MAX));
        let docs = plan.documents.as_ref().unwrap();
        assert_eq!(docs.accounts.len(), ACCOUNTS_MAX);
        assert_eq!(docs.accounts[3].last4.as_deref(), Some("0003"));
        assert_eq!(len(&docs.accounts[0].institution), Some(LABEL_TEXT_MAX));
        assert_eq!(len(&docs.accounts[0].kind), Some(SHORT_TEXT_MAX));
        assert_eq!(len(&docs.accounts[0].phone), Some(SHORT_TEXT_MAX));
        assert_eq!(docs.policies.len(), POLICIES_MAX);
        assert_eq!(len(&docs.policies[0].insurer), Some(LABEL_TEXT_MAX));
        assert_eq!(len(&docs.policies[0].policy_number), Some(MEDIUM_TEXT_MAX));
        assert_eq!(len(&docs.where_originals), Some(LONG_TEXT_MAX));
        assert_eq!(len(&docs.digital_backup), Some(LONG_TEXT_MAX));
    }

    #[test]
    fn a_place_with_only_its_kind_and_an_empty_profile_disappear() {
        let mut input = PlanInput::defaults();
        input.people[0].profile = Some(PersonProfile {
            place: Some(Place {
                kind: PlaceKind::Work,
                name: Some("  ".into()),
                address: None,
                phone: None,
                plan: None,
                pickup: None,
                safest_spot: None,
            }),
            medications: vec![Medication::default(), Medication::default()],
            notes: Some("\n\t".into()),
            ..PersonProfile::default()
        });
        input.family_plan = Some(FamilyPlan {
            pets: vec![PetInfo::default()],
            documents: Some(DocumentsInfo::default()),
            ..FamilyPlan::default()
        });
        input.tidy();
        assert_eq!(input.people[0].profile, None);
        assert_eq!(input.family_plan, None);
        // Nothing in the new groups is ever a problem, and absent groups stay out of the JSON.
        assert_eq!(input.validate(), vec![]);
        let v = serde_json::to_value(&input).unwrap();
        assert!(v["people"][0].get("profile").is_none());
        assert!(v.get("family_plan").is_none());
        // A place with a name stays, whatever its kind.
        input.people[0].profile = Some(PersonProfile {
            place: Some(Place {
                kind: PlaceKind::Childcare,
                name: Some("Little Oaks".into()),
                address: None,
                phone: None,
                plan: None,
                pickup: None,
                safest_spot: None,
            }),
            ..PersonProfile::default()
        });
        input.tidy();
        let v = serde_json::to_value(&input).unwrap();
        assert_eq!(
            v["people"][0]["profile"],
            serde_json::json!({ "place": { "kind": "childcare", "name": "Little Oaks" } })
        );
    }

    #[test]
    fn place_kinds_are_the_four_json_values() {
        assert_eq!(PlaceKind::STRS, ["work", "school", "childcare", "other"]);
        assert!(serde_json::from_str::<Place>(r#"{"kind":"office"}"#).is_err());
        assert!(
            serde_json::from_str::<Place>(r#"{"name":"x"}"#).is_err(),
            "kind is required"
        );
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
