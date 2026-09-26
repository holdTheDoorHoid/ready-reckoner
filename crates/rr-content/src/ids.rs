//! Ids content may name, and the rules that go with each kind of guidance block.
//!
//! Hazards, buckets, rare families, access needs and benefits come from engine contract v2 in
//! `rr-types`; this module turns them into the checks the validator and the conditional spans use.
//! A rare family is named by its lead hazard id ([`rr_types::HazardId::family`]), so a family
//! block applies to `family:<hazard id>`.

use std::str::FromStr;

pub use rr_types::GuidanceKind;
use rr_types::{AccessNeed, Benefit, BucketId, HazardId};

/// A person's access and functional needs, for `{if:need:<id>}` spans (`Person.access_needs`).
pub const ACCESS_NEEDS: &[&str] = AccessNeed::STRS;

/// Benefits a household may rely on, for `{if:benefit:<id>}` spans (`Finances.benefits`).
pub const BENEFITS: &[&str] = Benefit::STRS;

/// A hazard id content may name: any id `rr-types` parses, the retired `terrorism` included (a
/// block may keep explaining it for saved plans).
pub fn is_hazard(id: &str) -> bool {
    HazardId::from_str(id).is_ok()
}

/// A bucket id content may name.
pub fn is_bucket(id: &str) -> bool {
    BucketId::from_str(id).is_ok()
}

/// The lead hazard id of a rare family ([`HazardId::family`]).
pub fn is_rare_family(id: &str) -> bool {
    HazardId::from_str(id).is_ok_and(|h| h.family() == Some(id))
}

/// The rare families' ids, in `rr-types` order.
pub fn rare_families() -> Vec<&'static str> {
    rr_types::rare_family_ids().collect()
}

/// The rules each kind of guidance block follows (the `kind` front-matter field). The validator
/// checks that a block's id starts with its kind ([`KindRules::prefix`]) and that it applies to at
/// least one target of the same kind.
pub trait KindRules {
    /// The prefix every id of this kind starts with, for example `plan_`.
    fn prefix(self) -> String;
    /// Whether the block must open with the household's own `{frequency}` sentence (bucket,
    /// hazard and family blocks, `docs/CONTENT_STANDARDS.md` §4).
    fn opens_with_frequency(self) -> bool;
    /// Whether a hazard condition in the block may name any hazard (plan, after-disaster and
    /// topic blocks, which are about the household rather than one hazard) or only the hazards
    /// the block applies to (the others).
    fn any_hazard_condition(self) -> bool;
}

impl KindRules for GuidanceKind {
    fn prefix(self) -> String {
        format!("{}_", self.as_str())
    }

    fn opens_with_frequency(self) -> bool {
        matches!(
            self,
            GuidanceKind::Bucket | GuidanceKind::Hazard | GuidanceKind::Family
        )
    }

    fn any_hazard_condition(self) -> bool {
        matches!(
            self,
            GuidanceKind::Plan | GuidanceKind::After | GuidanceKind::Topic
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_name_their_prefix_and_rules() {
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
    fn ids_come_from_the_contract() {
        assert!(is_hazard("tornado"));
        assert!(is_hazard("water_damage"));
        assert!(is_hazard("terrorism"));
        assert!(!is_hazard("zombies"));
        assert!(is_bucket("clean_air"));
        assert!(!is_bucket("weather"));
        assert!(is_rare_family("nuclear_attack"));
        assert!(is_rare_family("geomagnetic_storm"));
        assert!(!is_rare_family("tornado"));
        assert_eq!(rare_families().len(), 9);
        assert!(ACCESS_NEEDS.contains(&"dialysis"));
        assert!(BENEFITS.contains(&"snap_wic"));
    }
}
