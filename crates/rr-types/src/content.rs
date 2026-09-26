//! Content types: catalogue items, citations and guidance metadata (DESIGN §4.6).
//!
//! `rr-content` parses `content/items/*.toml`, `content/citations.toml` and the front matter of
//! `content/guidance/*.md` into these, so unknown fields in content files fail the build.

use serde::{Deserialize, Serialize};

use crate::{BucketId, CitationId, Date, HazardId, ItemId, TierId};

/// A source. Every user-visible number cites at least one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    /// Stable id, referenced from items, effects and profiles.
    pub id: CitationId,
    /// Title of the page or document.
    pub title: String,
    /// Who published it.
    pub publisher: String,
    /// Year of publication, where known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    /// Where to read it.
    pub url: String,
    /// When it was retrieved.
    pub retrieved: Date,
    /// A short quotation supporting the number, where useful.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
    /// Licence of the source, for example "US Government Work (public domain)".
    pub license: String,
    /// This "source" is an expert judgement (a prior), not data. Every number that cites it is
    /// shown as an estimate. Optional in content files; defaults to false.
    #[serde(default)]
    pub prior: bool,
}

/// A catalogue entry: something to do or buy (DESIGN §4.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// Stable id, for example `water_stored`.
    pub id: ItemId,
    /// Plain name.
    pub name: String,
    /// Grouping for lists and the packet, for example `"water"`.
    pub category: String,
    /// Unit the item is counted in, for example `"gallon"`.
    pub unit: String,
    /// Buckets it helps with.
    pub buckets: Vec<BucketId>,
    /// The tier it belongs to.
    pub tier: TierId,
    /// Costs nothing (a free action).
    pub free: bool,
    /// Life-safety item: the allocator orders it first within its tier (smoke and carbon
    /// monoxide alarms, water, a dependent's medication, backup power for a medical device).
    /// Optional in content files; defaults to false.
    #[serde(default)]
    pub life_safety: bool,
    /// Specialised item for rare catastrophes (a radiation meter, potassium iodide, Faraday
    /// storage): the allocator gives it $0 by default and at most 10% of the monthly budget when
    /// the user opts in. Optional in content files; defaults to false.
    #[serde(default)]
    pub rare_catastrophic: bool,
    /// What it is, in a sentence or two.
    pub spec: String,
    /// What to look for when buying.
    pub look_for: Vec<String>,
    /// What to avoid.
    pub avoid: Vec<String>,
    /// Typical price.
    pub price_band_usd: PriceBand,
    /// When the price band was observed ("prices as of ...").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieved: Option<Date>,
    /// Name of the quantity rule in `rr-supply` that sizes it.
    pub quantity_rule: String,
    /// Rotation and check intervals, if it needs any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<Maintenance>,
    /// Sources for the spec, quantity and price.
    pub citations: Vec<CitationId>,
    /// Hazards for which this is a hazard-specific extra rather than a general supply.
    pub hazard_extras: Vec<HazardId>,
    /// Food energy in one `unit`, in kilocalories, for food items (so the app can show cost per
    /// 2,000 kcal).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub energy_kcal_per_unit: Option<f32>,
    /// Volume of one `unit` in litres, for water and other liquids (so the app can show cost per
    /// litre or gallon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_l_per_unit: Option<f32>,
}

/// A typical price range for one unit, in US dollars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriceBand {
    /// Low end.
    pub low: f32,
    /// High end.
    pub high: f32,
    /// What the price is for, for example `"gallon"`.
    pub per: String,
    /// A note on the range, for example "reused bottles are free".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// How often an item needs attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Maintenance {
    /// Replace or use and restock every this many months.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotate_months: Option<u16>,
    /// Check (test, inspect, charge) every this many months.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_months: Option<u16>,
}

/// Front matter of a guidance block (`content/guidance/*.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidanceMeta {
    /// Stable id (the file name without `.md`).
    pub id: String,
    /// Heading shown to the user.
    pub title: String,
    /// Bucket, hazard, tier or topic ids the block applies to.
    pub applies_to: Vec<String>,
    /// Sources for the block.
    pub citations: Vec<CitationId>,
}
