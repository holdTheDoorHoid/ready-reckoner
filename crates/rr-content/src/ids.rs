//! Ids content may name: the `rr-types` enums, plus what engine contract v2 adds
//! (`docs/DESIGN.md` §4, round-2 design delta §1) before `rr-types` knows it.
//!
//! Content for v0.2.0 is written in parallel with the contract. A block may apply to
//! `hazard:water_damage` or `bucket:clean_air`, and a conditional span may say
//! `{if:need:hearing}`, before the types workstream has added those values. The lists here let the
//! validator accept them now; once `rr-types` parses them, each check below passes through the
//! enum and the list is redundant (harmless) until it is removed.
//!
//! The guidance kinds ([`GuidanceKind`]) belong to `rr-content`: they classify blocks for the
//! packet and the Learn screen and are not part of the engine contract.

use std::fmt;
use std::str::FromStr;

use rr_types::{BucketId, HazardId};

/// Hazard ids that contract v2 adds: the ten new ranked hazards, the owner's personal hazard
/// (`arrest_or_detention`, 2026-09-26) and the eight new rare families (`nuclear_attack` is kept).
// awaiting: rr-types (contract v2 `HazardId`); remove once `HazardId` parses every entry.
pub const V2_HAZARDS: &[&str] = &[
    "water_damage",
    "wildfire_smoke",
    "dam_failure",
    "network_outage",
    "drug_shortage",
    "benefit_interruption",
    "eviction",
    "attack_disruption",
    "dust_storm",
    "sinkhole",
    "arrest_or_detention",
    "geomagnetic_storm",
    "multi_month_blackout",
    "war_infrastructure",
    "cbrn_attack",
    "severe_pandemic",
    "vei7_eruption",
    "financial_crisis",
    "mass_violence",
];

/// Bucket ids that contract v2 adds (the 15th bucket, "Unhealthy air indoors").
// awaiting: rr-types (contract v2 `BucketId::CleanAir`).
pub const V2_BUCKETS: &[&str] = &["clean_air"];

/// The nine rare families, each named by its lead hazard id (the round-2 review §2.4). A family
/// block applies to `family:<id>` with one of these.
// awaiting: rr-types (contract v2 `HazardId::family()`), which may name families differently;
// the family block for a rare hazard is found by that hazard's id either way.
pub const RARE_FAMILIES: &[&str] = &[
    "nuclear_attack",
    "geomagnetic_storm",
    "multi_month_blackout",
    "war_infrastructure",
    "cbrn_attack",
    "severe_pandemic",
    "vei7_eruption",
    "financial_crisis",
    "mass_violence",
];

/// A person's access and functional needs (contract v2 `Person.access_needs`), for
/// `{if:need:<id>}` spans.
// awaiting: rr-types (contract v2 `AccessNeed`); replace with its `STRS`.
pub const ACCESS_NEEDS: &[&str] = &[
    "hearing",
    "vision",
    "limited_english",
    "cognitive",
    "supervision",
    "service_animal",
    "dialysis",
    "home_health",
];

/// Benefits a household may rely on (contract v2 `Finances.benefits`), for `{if:benefit:<id>}`
/// spans.
// awaiting: rr-types (contract v2 `Benefit`); replace with its `STRS`.
pub const BENEFITS: &[&str] = &["federal_pay", "snap_wic", "ssi_ssdi", "va", "unemployment"];

/// A hazard id content may name: one `rr-types` knows, or a contract-v2 addition.
pub fn is_hazard(id: &str) -> bool {
    HazardId::from_str(id).is_ok() || V2_HAZARDS.contains(&id)
}

/// A bucket id content may name: one `rr-types` knows, or a contract-v2 addition.
pub fn is_bucket(id: &str) -> bool {
    BucketId::from_str(id).is_ok() || V2_BUCKETS.contains(&id)
}

/// The lead hazard id of a rare family.
pub fn is_rare_family(id: &str) -> bool {
    RARE_FAMILIES.contains(&id)
}

/// What a guidance block is for (the `kind` front-matter field). The packet and the Learn screen
/// choose blocks by kind; the validator checks that a block's id starts with its kind
/// ([`GuidanceKind::prefix`]) and that it applies to at least one target of the same kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuidanceKind {
    /// The days after a disaster (`after_<slug>`, applies to `after:<slug>`).
    After,
    /// A household plan section: shelter, the 48 hours before a forecast storm, communication
    /// (`plan_<slug>`, applies to `plan:<slug>`).
    Plan,
    /// A ranked hazard or a family of them (`hazard_<name>`, applies to `hazard:<id>`).
    Hazard,
    /// A consequence bucket (`bucket_<id>`, applies to `bucket:<id>`).
    Bucket,
    /// A tier of the plan (`tier_<id>`, applies to `tier:<id>`).
    Tier,
    /// A topic for the Learn screen or a packet section (`topic_<slug>`, applies to
    /// `topic:<slug>`).
    Topic,
    /// A rare family (`family_<name>`, applies to `family:<lead hazard id>`), with "what it
    /// changes in your plan".
    Family,
}

impl GuidanceKind {
    /// Every kind, in the order the front matter documents them.
    pub const ALL: &'static [GuidanceKind] = &[
        GuidanceKind::After,
        GuidanceKind::Plan,
        GuidanceKind::Hazard,
        GuidanceKind::Bucket,
        GuidanceKind::Tier,
        GuidanceKind::Topic,
        GuidanceKind::Family,
    ];

    /// The front-matter value, which is also the target kind in `applies_to`.
    pub const fn as_str(self) -> &'static str {
        match self {
            GuidanceKind::After => "after",
            GuidanceKind::Plan => "plan",
            GuidanceKind::Hazard => "hazard",
            GuidanceKind::Bucket => "bucket",
            GuidanceKind::Tier => "tier",
            GuidanceKind::Topic => "topic",
            GuidanceKind::Family => "family",
        }
    }

    /// The prefix every id of this kind starts with, for example `plan_`.
    pub fn prefix(self) -> String {
        format!("{}_", self.as_str())
    }

    /// Whether the block must open with the household's own `{frequency}` sentence (bucket,
    /// hazard and family blocks, `docs/CONTENT_STANDARDS.md` §4).
    pub const fn opens_with_frequency(self) -> bool {
        matches!(
            self,
            GuidanceKind::Bucket | GuidanceKind::Hazard | GuidanceKind::Family
        )
    }

    /// Whether a hazard condition in the block may name any hazard (plan, after-disaster and
    /// topic blocks, which are about the household rather than one hazard) or only the hazards
    /// the block applies to (the others).
    pub const fn any_hazard_condition(self) -> bool {
        matches!(
            self,
            GuidanceKind::Plan | GuidanceKind::After | GuidanceKind::Topic
        )
    }
}

impl fmt::Display for GuidanceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GuidanceKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GuidanceKind::ALL
            .iter()
            .copied()
            .find(|k| k.as_str() == s)
            .ok_or_else(|| {
                let all: Vec<&str> = GuidanceKind::ALL.iter().map(|k| k.as_str()).collect();
                format!("`{s}` is not a kind of guidance block (one of {all:?})")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip_and_name_their_prefix() {
        for k in GuidanceKind::ALL {
            assert_eq!(k.as_str().parse::<GuidanceKind>(), Ok(*k));
            assert_eq!(k.prefix(), format!("{}_", k.as_str()));
        }
        assert!("recipe".parse::<GuidanceKind>().is_err());
        assert!(GuidanceKind::Family.opens_with_frequency());
        assert!(!GuidanceKind::Plan.opens_with_frequency());
        assert!(GuidanceKind::Plan.any_hazard_condition());
        assert!(!GuidanceKind::Hazard.any_hazard_condition());
    }

    #[test]
    fn known_ids_come_from_rr_types_or_the_v2_lists() {
        assert!(is_hazard("tornado"));
        assert!(is_hazard("water_damage"));
        assert!(!is_hazard("zombies"));
        assert!(is_bucket("power"));
        assert!(is_bucket("clean_air"));
        assert!(!is_bucket("weather"));
        assert!(is_rare_family("nuclear_attack"));
        assert!(!is_rare_family("tornado"));
        for f in RARE_FAMILIES {
            assert!(is_hazard(f), "{f} is a hazard id");
        }
    }

    #[test]
    fn v2_lists_have_no_duplicates() {
        for list in [
            V2_HAZARDS,
            V2_BUCKETS,
            RARE_FAMILIES,
            ACCESS_NEEDS,
            BENEFITS,
        ] {
            let mut sorted = list.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), list.len());
            for id in list {
                assert!(rr_types::is_well_formed_id(id), "{id}");
            }
        }
    }
}
