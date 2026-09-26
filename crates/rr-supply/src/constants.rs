//! The constants registry: every number the quantity rules use, with its sources.
//!
//! The data lives in `constants.toml` next to this file (embedded at build time, parsed once).
//! The rules read values only through [`constants()`], so nothing numeric from the research is
//! hard-coded anywhere else, and each requirement line can cite exactly the sources of the
//! numbers it used. The web app's expert view shows the same registry: every constant has a plain
//! label, a unit, a default, an optional range and alternatives, its sources, and the note from
//! `docs/research/supply-standards.md` §13 where authorities disagree.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use rr_types::{AgeBand, CitationId};
use serde::{Deserialize, Serialize};

/// The embedded registry source.
pub const CONSTANTS_TOML: &str = include_str!("constants.toml");

/// The citation id of the planning-estimate source (`prior = true`): every number tagged `Prior`
/// cites it.
pub const PRIOR_SOURCE: &str = "rr_expert_prior";

/// The parsed registry. Parsed once per process (and once per WebAssembly instance).
///
/// # Panics
///
/// If the embedded `constants.toml` does not parse. The crate's tests guarantee it does.
pub fn constants() -> &'static Constants {
    static REGISTRY: OnceLock<Constants> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        Constants::parse(CONSTANTS_TOML)
            .unwrap_or_else(|e| panic!("rr-supply constants.toml is invalid: {e}"))
    })
}

/// One number the rules use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constant {
    /// Stable snake_case id (see [`keys`]).
    pub key: String,
    /// What it is, in plain words.
    pub label: String,
    /// The unit of `default`, `low` and `high`.
    pub unit: String,
    /// The value the engine uses.
    pub default: f64,
    /// Low end of the plausible range, where one is published or estimated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<f64>,
    /// High end of the plausible range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<f64>,
    /// A planning estimate rather than a published figure (shown as an estimate).
    #[serde(default)]
    pub prior: bool,
    /// Citation ids for the default value.
    pub sources: Vec<CitationId>,
    /// Where authorities disagree, in plain words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Citation ids for numbers in the note.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub note_sources: Vec<CitationId>,
    /// Other published values.
    #[serde(
        default,
        rename(deserialize = "alternative", serialize = "alternatives"),
        skip_serializing_if = "Vec::is_empty"
    )]
    pub alternatives: Vec<Alternative>,
}

/// Another published value for a constant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alternative {
    /// What it is and who says it.
    pub label: String,
    /// The value, in the constant's unit.
    pub value: f64,
    /// Where it comes from.
    pub sources: Vec<CitationId>,
}

/// Dietary Guidelines for Americans 2020–2025, Appendix 2 (Tables A2-1 and A2-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DgaTable {
    /// Where the table comes from.
    pub sources: Vec<CitationId>,
    /// The activity column the plan uses (`moderate`).
    pub activity: String,
    /// How the plan reads the table.
    pub note: String,
    /// Table A2-2 rows, ages 2 and over.
    #[serde(rename(deserialize = "row", serialize = "rows"))]
    pub rows: Vec<DgaRow>,
    /// Table A2-1 rows, toddlers 12 to 23 months.
    #[serde(rename(deserialize = "month_row", serialize = "month_rows"))]
    pub month_rows: Vec<DgaMonthRow>,
    /// The ages each household age band covers.
    #[serde(rename(deserialize = "band", serialize = "bands"))]
    pub bands: Vec<DgaBand>,
}

/// One row of DGA Table A2-2: kcal a day for `[sedentary, moderately active, active]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DgaRow {
    /// First and last age in years, inclusive.
    pub ages: [u8; 2],
    /// Men and boys.
    pub male: [f64; 3],
    /// Women and girls.
    pub female: [f64; 3],
}

/// One row of DGA Table A2-1 (toddlers; one activity column).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DgaMonthRow {
    /// First and last age in months, inclusive.
    pub months: [u8; 2],
    /// Boys.
    pub male: f64,
    /// Girls.
    pub female: f64,
}

/// The whole-year ages a household age band covers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DgaBand {
    /// The band.
    pub band: AgeBand,
    /// First and last age in years, inclusive.
    pub ages: [u8; 2],
}

/// USDA Thrifty Food Plan household-size adjustment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TfpSizeTable {
    /// Where it comes from.
    pub sources: Vec<CitationId>,
    /// One factor per household-size range.
    #[serde(rename(deserialize = "row", serialize = "rows"))]
    pub rows: Vec<TfpSizeRow>,
}

/// A household-size range and its cost factor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TfpSizeRow {
    /// Smallest and largest household size, inclusive.
    pub people: [u16; 2],
    /// Multiply the per-person cost by this.
    pub factor: f64,
}

/// Long-term staples per adult per year.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaplesTable {
    /// Where the default list comes from.
    pub sources: Vec<CitationId>,
    /// The list the plan uses (`byu_2019`).
    pub default_list: String,
    /// Caveats (nutrient gaps, packaging).
    pub note: String,
    /// Citation ids for the note.
    pub note_sources: Vec<CitationId>,
    /// One row per staple.
    #[serde(rename(deserialize = "row", serialize = "rows"))]
    pub rows: Vec<StapleRow>,
}

/// One staple and how much an adult needs in a year.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StapleRow {
    /// Stable id.
    pub key: String,
    /// Plain name.
    pub label: String,
    /// Group the plan reports it under (`grains`, `legumes`, ...).
    pub group: String,
    /// Which list it belongs to (`byu_2019` or `ensign_2006`).
    pub list: String,
    /// Amount per adult per year, in `unit`.
    pub per_adult_year: f64,
    /// `lb`, `gallon` or `tablet`.
    pub unit: String,
}

/// Children's share of the adult staples amount.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildShareTable {
    /// Where it comes from.
    pub sources: Vec<CitationId>,
    /// Years added to a child's age before looking up the share.
    pub age_offset_years: u8,
    /// One share per age range.
    #[serde(rename(deserialize = "row", serialize = "rows"))]
    pub rows: Vec<ChildShareRow>,
}

/// An age range and its share of the adult amount.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildShareRow {
    /// First and last age in years, inclusive.
    pub ages: [u8; 2],
    /// Share of the adult amount, 0 to 1.
    pub share: f64,
}

/// Storage life of staples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShelfLifeTable {
    /// Where it comes from.
    pub sources: Vec<CitationId>,
    /// Where sources disagree.
    pub note: String,
    /// Citation ids for the note.
    pub note_sources: Vec<CitationId>,
    /// One row per food.
    #[serde(rename(deserialize = "row", serialize = "rows"))]
    pub rows: Vec<ShelfLifeRow>,
}

/// How long a food keeps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShelfLifeRow {
    /// Stable id (matches a staple key where there is one).
    pub key: String,
    /// Years.
    pub years: f64,
}

/// December solar output by latitude band (a proxy for the county).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolarTable {
    /// Where it comes from.
    pub sources: Vec<CitationId>,
    /// The band mapping is an estimate.
    pub prior: bool,
    /// Caveats.
    pub note: String,
    /// One row per latitude band.
    #[serde(rename(deserialize = "row", serialize = "rows"))]
    pub rows: Vec<SolarRow>,
}

/// A latitude band and what a 100 W panel makes there on a December day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolarRow {
    /// Southern and northern edge in degrees; the band includes the southern edge.
    pub lat: [f64; 2],
    /// Watt-hours a day from a 100 W panel in December.
    pub dec_wh_per_100w: f64,
    /// The measured city behind the value.
    pub city: String,
}

/// A disagreement between authorities that is not a single number the rules use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Disagreement {
    /// What it is about.
    pub topic: String,
    /// Who says what.
    pub positions: String,
    /// What the plan does.
    pub handling: String,
    /// Where the positions come from.
    pub sources: Vec<CitationId>,
}

/// A source the supply lines cite. `rr-content` owns the full citation; this is the list of what
/// rr-supply needs, with the short name used in plain-language lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Citation id (a key into `content/citations.toml`).
    pub id: CitationId,
    /// Short name for plain-language lines ("Ready.gov").
    pub short: String,
    /// Title of the page or document.
    pub title: String,
    /// Who published it.
    pub publisher: String,
    /// Where to read it.
    pub url: String,
    /// An expert estimate rather than data.
    #[serde(default)]
    pub prior: bool,
}

/// The whole registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constants {
    /// Every scalar constant, in file order.
    #[serde(rename(deserialize = "constant", serialize = "constants"))]
    pub constants: Vec<Constant>,
    /// Food energy by age.
    pub dga: DgaTable,
    /// Food cost by household size.
    pub tfp_size: TfpSizeTable,
    /// Long-term staples.
    pub staples: StaplesTable,
    /// Children's share of staples.
    pub child_share: ChildShareTable,
    /// Storage life of staples.
    pub shelf_life: ShelfLifeTable,
    /// Winter solar output by latitude.
    pub solar: SolarTable,
    /// Disagreements that are not a single number.
    #[serde(rename(deserialize = "disagreement", serialize = "disagreements"))]
    pub disagreements: Vec<Disagreement>,
    /// Sources the supply lines cite.
    #[serde(rename(deserialize = "source", serialize = "sources"))]
    pub sources: Vec<Source>,
    #[serde(skip)]
    index: BTreeMap<String, usize>,
    #[serde(skip)]
    source_index: BTreeMap<String, usize>,
}

impl Constants {
    /// Parses a registry from TOML (the embedded file, or a test variant).
    ///
    /// # Errors
    ///
    /// If the TOML does not match the schema, or a key or source id appears twice.
    pub fn parse(text: &str) -> Result<Constants, String> {
        let mut c: Constants = toml::from_str(text).map_err(|e| e.to_string())?;
        for (i, k) in c.constants.iter().enumerate() {
            if c.index.insert(k.key.clone(), i).is_some() {
                return Err(format!("constant `{}` appears twice", k.key));
            }
        }
        for (i, s) in c.sources.iter().enumerate() {
            if c.source_index.insert(s.id.as_str().to_owned(), i).is_some() {
                return Err(format!("source `{}` appears twice", s.id));
            }
        }
        Ok(c)
    }

    /// The constant with this key, if there is one.
    pub fn get(&self, key: &str) -> Option<&Constant> {
        self.index.get(key).map(|&i| &self.constants[i])
    }

    /// The constant with this key.
    ///
    /// # Panics
    ///
    /// If there is no such key. Every key the rules use is in [`keys::ALL`], and a test checks
    /// that each one exists.
    pub fn constant(&self, key: &str) -> &Constant {
        self.get(key)
            .unwrap_or_else(|| panic!("rr-supply constant `{key}` is missing from constants.toml"))
    }

    /// The default value of the constant with this key.
    ///
    /// # Panics
    ///
    /// As [`Constants::constant`].
    pub fn value(&self, key: &str) -> f64 {
        self.constant(key).default
    }

    /// The source with this citation id, if rr-supply lists it.
    pub fn source(&self, id: &str) -> Option<&Source> {
        self.source_index.get(id).map(|&i| &self.sources[i])
    }

    /// Every citation id the registry refers to (constants, notes, alternatives and tables).
    /// Each one must resolve to `content/citations.toml`.
    pub fn citations_used(&self) -> BTreeSet<CitationId> {
        let mut out = BTreeSet::new();
        for k in &self.constants {
            out.extend(k.sources.iter().cloned());
            out.extend(k.note_sources.iter().cloned());
            for a in &k.alternatives {
                out.extend(a.sources.iter().cloned());
            }
        }
        out.extend(self.dga.sources.iter().cloned());
        out.extend(self.tfp_size.sources.iter().cloned());
        out.extend(self.staples.sources.iter().cloned());
        out.extend(self.staples.note_sources.iter().cloned());
        out.extend(self.child_share.sources.iter().cloned());
        out.extend(self.shelf_life.sources.iter().cloned());
        out.extend(self.shelf_life.note_sources.iter().cloned());
        out.extend(self.solar.sources.iter().cloned());
        for d in &self.disagreements {
            out.extend(d.sources.iter().cloned());
        }
        out
    }

    /// Consistency problems in the registry, in a fixed order (empty when it is sound). The crate's
    /// tests require it to be empty.
    pub fn problems(&self) -> Vec<String> {
        let mut p = Vec::new();
        for k in &self.constants {
            if !rr_types::is_well_formed_id(&k.key) {
                p.push(format!("constant key `{}` is not snake_case", k.key));
            }
            if k.label.trim().is_empty() || k.unit.trim().is_empty() {
                p.push(format!("constant `{}` needs a label and a unit", k.key));
            }
            if k.sources.is_empty() {
                p.push(format!("constant `{}` has no source", k.key));
            }
            if !k.default.is_finite() {
                p.push(format!("constant `{}` is not finite", k.key));
            }
            if let Some(lo) = k.low {
                if lo > k.default {
                    p.push(format!(
                        "constant `{}`: low {lo} is above the default",
                        k.key
                    ));
                }
            }
            if let Some(hi) = k.high {
                if hi < k.default {
                    p.push(format!(
                        "constant `{}`: high {hi} is below the default",
                        k.key
                    ));
                }
            }
            let cites_prior = k.sources.iter().any(|s| s == PRIOR_SOURCE);
            if k.prior != cites_prior {
                p.push(format!(
                    "constant `{}`: `prior = {}` but it {} cite `{PRIOR_SOURCE}`",
                    k.key,
                    k.prior,
                    if cites_prior { "does" } else { "does not" }
                ));
            }
            if k.note.is_none() && !k.note_sources.is_empty() {
                p.push(format!("constant `{}` has note sources but no note", k.key));
            }
            for a in &k.alternatives {
                if a.sources.is_empty() {
                    p.push(format!(
                        "constant `{}`: alternative without a source",
                        k.key
                    ));
                }
            }
        }
        for id in self.citations_used() {
            if self.source(id.as_str()).is_none() {
                p.push(format!(
                    "citation `{id}` is used but not listed in [[source]]"
                ));
            }
            if !id.is_well_formed() {
                p.push(format!("citation id `{id}` is not snake_case"));
            }
        }
        for s in &self.sources {
            if s.prior != (s.id == PRIOR_SOURCE) {
                p.push(format!(
                    "source `{}`: only `{PRIOR_SOURCE}` is a prior",
                    s.id
                ));
            }
            if !s.url.starts_with("https://") {
                p.push(format!("source `{}` needs an https URL", s.id));
            }
        }
        for b in AgeBand::ALL {
            let n = self.dga.bands.iter().filter(|x| x.band == *b).count();
            let want = usize::from(*b != AgeBand::Infant);
            if n != want {
                p.push(format!(
                    "DGA band table lists `{b}` {n} times (want {want})"
                ));
            }
        }
        for band in &self.dga.bands {
            for age in band.ages[0]..=band.ages[1] {
                let covered = age == 1
                    || self
                        .dga
                        .rows
                        .iter()
                        .any(|r| (r.ages[0]..=r.ages[1]).contains(&age));
                if !covered {
                    p.push(format!("DGA table has no row for age {age}"));
                }
            }
        }
        if !self
            .staples
            .rows
            .iter()
            .any(|r| r.list == self.staples.default_list)
        {
            p.push("staples default list has no rows".to_owned());
        }
        let solar = &self.solar.rows;
        if solar.is_empty() || solar[0].lat[0] > -90.0 || solar[solar.len() - 1].lat[1] < 90.0 {
            p.push("solar bands must cover -90 to 90 degrees".to_owned());
        }
        for w in solar.windows(2) {
            if w[0].lat[1] != w[1].lat[0] {
                p.push("solar bands must be contiguous".to_owned());
            }
        }
        p
    }
}

macro_rules! keys {
    ($( $(#[$m:meta])* $name:ident = $s:literal ),+ $(,)?) => {
        /// The key of every constant the rules read, so a typo is a compile error. A test checks
        /// that `ALL` and `constants.toml` list the same keys.
        pub mod keys {
            $( $(#[$m])* #[doc = concat!("`", $s, "`")] pub const $name: &str = $s; )+
            /// Every key, in `constants.toml` order.
            pub const ALL: &[&str] = &[$($s),+];
        }
    };
}

keys! {
    WATER_SURVIVAL_GAL = "water_survival_gal",
    WATER_BASIC_GAL = "water_basic_gal",
    WATER_COMFORTABLE_GAL = "water_comfortable_gal",
    WATER_DRINKING_SHARE_BASIC_GAL = "water_drinking_share_basic_gal",
    WATER_HEAT_DRINKING_MULTIPLIER = "water_heat_drinking_multiplier",
    HOT_CLIMATE_DAYS_95F = "hot_climate_days_95f",
    WATER_NURSING_ADD_GAL = "water_nursing_add_gal",
    WATER_PREGNANCY_ADD_GAL = "water_pregnancy_add_gal",
    WATER_FORMULA_INFANT_GAL = "water_formula_infant_gal",
    WATER_DOG_OZ_PER_LB_DAY = "water_dog_oz_per_lb_day",
    WATER_CAT_OZ_PER_LB_DAY = "water_cat_oz_per_lb_day",
    DOG_WEIGHT_LB = "dog_weight_lb",
    CAT_WEIGHT_LB = "cat_weight_lb",
    WATER_SMALL_PET_OZ_DAY = "water_small_pet_oz_day",
    WATER_LIVESTOCK_L_DAY = "water_livestock_l_day",
    WATER_REHYDRATION_GAL_PER_PERSON_DAY = "water_rehydration_gal_per_person_day",
    WATER_STORED_CAP_DAYS = "water_stored_cap_days",
    WATER_ROTATION_MONTHS = "water_rotation_months",
    BLEACH_DROPS_PER_GAL_6PCT = "bleach_drops_per_gal_6pct",
    BLEACH_DROPS_PER_GAL_8PCT = "bleach_drops_per_gal_8pct",
    BLEACH_CLOUDY_MULTIPLIER = "bleach_cloudy_multiplier",
    DISINFECT_WAIT_MINUTES = "disinfect_wait_minutes",
    BOIL_MINUTES = "boil_minutes",
    BOIL_MINUTES_HIGH_ALTITUDE = "boil_minutes_high_altitude",
    BOIL_ALTITUDE_FT = "boil_altitude_ft",
    PROPANE_L_BOILED_PER_LB = "propane_l_boiled_per_lb",
    KCAL_PREGNANT_OR_NURSING_ADD = "kcal_pregnant_or_nursing_add",
    KCAL_PREGNANCY_T2_ADD = "kcal_pregnancy_t2_add",
    KCAL_PREGNANCY_T3_ADD = "kcal_pregnancy_t3_add",
    KCAL_LACTATION_0_6_ADD = "kcal_lactation_0_6_add",
    KCAL_LACTATION_7_12_ADD = "kcal_lactation_7_12_add",
    SPHERE_KCAL_AVERAGE = "sphere_kcal_average",
    FOOD_COST_PANTRY_USD_PER_PERSON_DAY = "food_cost_pantry_usd_per_person_day",
    FOOD_COST_STAPLES_USD_PER_PERSON_DAY = "food_cost_staples_usd_per_person_day",
    FOOD_COST_FREEZE_DRIED_USD_PER_2000KCAL = "food_cost_freeze_dried_usd_per_2000kcal",
    RETAIL_KIT_KCAL_PER_DAY = "retail_kit_kcal_per_day",
    RETAIL_KIT_LABEL_DAYS = "retail_kit_label_days",
    STAPLES_AFTER_DAYS = "staples_after_days",
    INFANT_FORMULA_OZ_DAY = "infant_formula_oz_day",
    NURSING_PAD_BOXES = "nursing_pad_boxes",
    MANUAL_PUMPS_PER_NURSING_INFANT = "manual_pumps_per_nursing_infant",
    RX_DAYS_ON_HAND = "rx_days_on_hand",
    RX_EMERGENCY_REFILL_DAYS = "rx_emergency_refill_days",
    REFRIGERATED_RX_HOLD_DAYS = "refrigerated_rx_hold_days",
    ANTIBIOTIC_QUANTITY = "antibiotic_quantity",
    CPAP_WH_PER_NIGHT = "cpap_wh_per_night",
    CPAP_WH_PER_NIGHT_DRY = "cpap_wh_per_night_dry",
    OXYGEN_CONCENTRATOR_W = "oxygen_concentrator_w",
    DEVICE_HOURS_PER_DAY = "device_hours_per_day",
    FRIDGE_WH_PER_DAY = "fridge_wh_per_day",
    FRIDGE_COLD_HOURS = "fridge_cold_hours",
    FREEZER_COLD_HOURS = "freezer_cold_hours",
    PHONE_WH_PER_DAY = "phone_wh_per_day",
    LIGHTS_PER_PERSON = "lights_per_person",
    POWER_STATION_WH = "power_station_wh",
    POWER_STATION_USABLE_SHARE = "power_station_usable_share",
    GENERATOR_GAL_PER_DAY = "generator_gal_per_day",
    FUEL_STORAGE_MAX_GAL = "fuel_storage_max_gal",
    FUEL_STORAGE_ATTACHED_GARAGE_GAL = "fuel_storage_attached_garage_gal",
    FUEL_STORAGE_INDOOR_GAL = "fuel_storage_indoor_gal",
    PROPANE_SMALL_CYLINDERS_INDOOR_MAX = "propane_small_cylinders_indoor_max",
    FUEL_ROTATION_MONTHS = "fuel_rotation_months",
    GENERATOR_CLEARANCE_FT = "generator_clearance_ft",
    WELL_PUMP_W = "well_pump_w",
    WELL_PUMP_START_MULTIPLIER = "well_pump_start_multiplier",
    WELL_PUMP_HOURS_PER_DAY = "well_pump_hours_per_day",
    WHEELCHAIR_SPARE_BATTERIES = "wheelchair_spare_batteries",
    NOAA_RADIOS_PER_HOUSEHOLD = "noaa_radios_per_household",
    GMRS_LICENSE_USD = "gmrs_license_usd",
    GMRS_LICENSE_YEARS = "gmrs_license_years",
    TWO_WAY_RADIO_MIN_PEOPLE = "two_way_radio_min_people",
    CONTACT_CARDS_PER_PERSON = "contact_cards_per_person",
    LOCAL_MAPS_PER_HOUSEHOLD = "local_maps_per_household",
    CASH_DAYS_MIN = "cash_days_min",
    CASH_DAYS_MAX = "cash_days_max",
    TOILET_BUCKETS_PER_HOUSEHOLD = "toilet_buckets_per_household",
    TOILET_BAGS_PER_PERSON_DAY = "toilet_bags_per_person_day",
    TOILET_COVER_CUPS_PER_PERSON_DAY = "toilet_cover_cups_per_person_day",
    SOAP_BATH_G_PER_PERSON_MONTH = "soap_bath_g_per_person_month",
    SOAP_LAUNDRY_G_PER_PERSON_MONTH = "soap_laundry_g_per_person_month",
    MENSTRUAL_CYCLES_KIT = "menstrual_cycles_kit",
    MENSTRUAL_PRODUCTS_PER_CYCLE = "menstrual_products_per_cycle",
    MENSTRUAL_CYCLE_DAYS = "menstrual_cycle_days",
    MENSTRUATING_SHARE = "menstruating_share",
    DIAPERS_PER_DAY_INFANT = "diapers_per_day_infant",
    DIAPERS_PER_DAY_TODDLER = "diapers_per_day_toddler",
    WIPES_PACKS_PER_CHILD_2WK = "wipes_packs_per_child_2wk",
    FIRST_AID_PEOPLE_PER_KIT = "first_aid_people_per_kit",
    OTC_KINDS = "otc_kinds",
    OTC_DAYS_PER_PACKAGE = "otc_days_per_package",
    N95_PER_PERSON_DAY = "n95_per_person_day",
    N95_DAYS = "n95_days",
    ORS_PACKETS_PER_PERSON_2WK = "ors_packets_per_person_2wk",
    ORS_L_PER_PACKET = "ors_l_per_packet",
    THERMOMETERS_PER_HOUSEHOLD = "thermometers_per_household",
    FAN_MAX_INDOOR_F = "fan_max_indoor_f",
    BATTERY_FANS_BASE = "battery_fans_base",
    BATTERY_FANS_PER_VULNERABLE = "battery_fans_per_vulnerable",
    COOLING_TOWELS_PER_PERSON = "cooling_towels_per_person",
    SLEEPING_BAGS_PER_PERSON = "sleeping_bags_per_person",
    WARM_LAYER_SETS_PER_PERSON = "warm_layer_sets_per_person",
    HYPOTHERMIA_F = "hypothermia_f",
    ALARM_LEVELS_APARTMENT = "alarm_levels_apartment",
    ALARM_LEVELS_HOUSE = "alarm_levels_house",
    ALARM_LEVELS_BASEMENT = "alarm_levels_basement",
    CO_ALARM_REPLACE_YEARS = "co_alarm_replace_years",
    EXTINGUISHERS_PER_HOUSEHOLD = "extinguishers_per_household",
    NEIGHBOUR_CONTACTS = "neighbour_contacts",
    EFFAK_PARTS = "effak_parts",
    NFIP_WAIT_DAYS = "nfip_wait_days",
    NFIP_BUILDING_LIMIT_USD = "nfip_building_limit_usd",
    NFIP_CONTENTS_LIMIT_USD = "nfip_contents_limit_usd",
    EMERGENCY_FUND_MONTHS = "emergency_fund_months",
    WALK_MPH = "walk_mph",
    WALK_WATER_L_PER_HOUR = "walk_water_l_per_hour",
    WALK_WATER_L_PER_HOUR_HOT = "walk_water_l_per_hour_hot",
    WALK_WATER_CARRY_MAX_L = "walk_water_carry_max_l",
    WALK_KCAL_PER_12H = "walk_kcal_per_12h",
    WORK_SHELTER_HOURS = "work_shelter_hours",
    CAR_KITS_PER_VEHICLE = "car_kits_per_vehicle",
    GO_BAG_DAYS = "go_bag_days",
    GO_BAGS_PER_PERSON = "go_bags_per_person",
    PET_EVAC_FOOD_DAYS = "pet_evac_food_days",
    PET_EVAC_WATER_DAYS = "pet_evac_water_days",
    PET_KIT_ROTATION_MONTHS = "pet_kit_rotation_months",
    GAS_TANK_MIN_SHARE = "gas_tank_min_share",
    NOTICE_MINUTES_BAND_HOURS = "notice_minutes_band_hours",
    NOTICE_HOURS_BAND_HOURS = "notice_hours_band_hours",
    READINESS_P_NEED_THRESHOLD = "readiness_p_need_threshold",
    EVACUATION_PLANS_PER_PERSON = "evacuation_plans_per_person",
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_registry_parses_and_is_sound() {
        let c = constants();
        let problems = c.problems();
        assert!(
            problems.is_empty(),
            "constants.toml problems:\n{}",
            problems.join("\n")
        );
        assert!(
            c.constants.len() >= 100,
            "only {} constants",
            c.constants.len()
        );
    }

    #[test]
    fn keys_module_and_file_list_the_same_constants_in_the_same_order() {
        let file: Vec<&str> = constants()
            .constants
            .iter()
            .map(|k| k.key.as_str())
            .collect();
        assert_eq!(file, keys::ALL);
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        let one = r#"
[[constant]]
key = "a"
label = "A"
unit = "u"
default = 1.0
sources = ["x"]
"#;
        let tail = CONSTANTS_TOML
            .split_once("# Tables")
            .map(|(_, t)| t)
            .expect("constants.toml has a Tables section");
        let text = format!("{one}{one}\n{tail}");
        let err = Constants::parse(&text).unwrap_err();
        assert!(err.contains("appears twice"), "{err}");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let text = CONSTANTS_TOML.replacen("default = 0.8\n", "default = 0.8\ndefalt = 1.0\n", 1);
        assert!(Constants::parse(&text).is_err());
    }

    #[test]
    fn headline_numbers_match_the_research() {
        // docs/research/supply-standards.md §14, spot checks against the cited sources.
        let c = constants();
        for (key, want) in [
            (keys::WATER_BASIC_GAL, 1.0),
            (keys::WATER_SURVIVAL_GAL, 0.8),
            (keys::WATER_COMFORTABLE_GAL, 4.0),
            (keys::WATER_DRINKING_SHARE_BASIC_GAL, 0.75),
            (keys::WATER_NURSING_ADD_GAL, 0.29),
            (keys::WATER_FORMULA_INFANT_GAL, 0.25),
            (keys::WATER_LIVESTOCK_L_DAY, 25.0),
            (keys::WATER_REHYDRATION_GAL_PER_PERSON_DAY, 0.6),
            (keys::FOOD_COST_PANTRY_USD_PER_PERSON_DAY, 8.44),
            (keys::RX_DAYS_ON_HAND, 14.0),
            (keys::ANTIBIOTIC_QUANTITY, 0.0),
            (keys::FRIDGE_WH_PER_DAY, 995.0),
            (keys::CPAP_WH_PER_NIGHT, 170.0),
            (keys::GENERATOR_GAL_PER_DAY, 2.8),
            (keys::FUEL_STORAGE_MAX_GAL, 25.0),
            (keys::SOAP_BATH_G_PER_PERSON_MONTH, 250.0),
            (keys::SOAP_LAUNDRY_G_PER_PERSON_MONTH, 200.0),
            (keys::GMRS_LICENSE_USD, 35.0),
            (keys::NFIP_WAIT_DAYS, 30.0),
        ] {
            assert_eq!(c.value(key), want, "{key}");
        }
        let rx = c.constant(keys::RX_DAYS_ON_HAND);
        assert_eq!((rx.low, rx.high), (Some(7.0), Some(30.0)));
        let staples = c.constant(keys::FOOD_COST_STAPLES_USD_PER_PERSON_DAY);
        assert_eq!((staples.low, staples.high), (Some(2.15), Some(2.85)));
        let fd = c.constant(keys::FOOD_COST_FREEZE_DRIED_USD_PER_2000KCAL);
        assert_eq!((fd.low, fd.high), (Some(9.0), Some(39.0)));
        let kit = c.constant(keys::RETAIL_KIT_KCAL_PER_DAY);
        assert_eq!((kit.low, kit.high), (Some(1290.0), Some(1730.0)));
        let generator = c.constant(keys::GENERATOR_GAL_PER_DAY);
        assert_eq!(generator.high, Some(7.1));
    }

    #[test]
    fn the_four_named_disagreements_carry_notes() {
        // The brief names drinking share, boil time/altitude, bleach concentration and rotation.
        let c = constants();
        for key in [
            keys::WATER_DRINKING_SHARE_BASIC_GAL,
            keys::BOIL_ALTITUDE_FT,
            keys::BLEACH_DROPS_PER_GAL_8PCT,
            keys::WATER_ROTATION_MONTHS,
            keys::RX_DAYS_ON_HAND,
            keys::FAN_MAX_INDOOR_F,
            keys::ANTIBIOTIC_QUANTITY,
            keys::RETAIL_KIT_KCAL_PER_DAY,
        ] {
            let k = c.constant(key);
            assert!(
                k.note.as_deref().is_some_and(|n| n.len() > 40),
                "{key} lacks a note"
            );
            assert!(!k.note_sources.is_empty() || !k.sources.is_empty(), "{key}");
        }
        assert_eq!(
            c.constant(keys::BOIL_ALTITUDE_FT).alternatives[0].value,
            6500.0
        );
        assert!(c.disagreements.len() >= 4);
    }

    #[test]
    fn prior_constants_cite_the_prior_source() {
        for k in &constants().constants {
            if k.prior {
                assert!(k.sources.iter().any(|s| s == PRIOR_SOURCE), "{}", k.key);
            }
        }
        assert!(constants().source(PRIOR_SOURCE).unwrap().prior);
    }

    #[test]
    fn registry_serialises_for_the_expert_view() {
        let json = serde_json::to_value(constants()).unwrap();
        assert!(json["constants"].as_array().unwrap().len() >= 100);
        assert!(json["dga"]["rows"].is_array());
        assert!(json["sources"].is_array());
        let water = &json["constants"][1];
        assert_eq!(water["key"], "water_basic_gal");
        assert_eq!(water["alternatives"][0]["value"], 3.96);
    }
}
