//! Collects the sources, and the `Prior` tag, behind every number a rule reads.
//!
//! A rule reads constants only through a [`Basis`], so the requirement line it produces cites
//! exactly the sources of the numbers it used, and is tagged as an estimate exactly when one of
//! them is a planning estimate.

use rr_types::CitationId;

use crate::constants::{
    ChildShareTable, Constant, DgaTable, PRIOR_SOURCE, ShelfLifeTable, SolarTable, StaplesTable,
    TfpSizeTable, constants,
};

/// The sources and the estimate tag gathered while a rule runs.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Basis {
    cites: Vec<CitationId>,
    prior: bool,
}

impl Basis {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, ids: &[CitationId]) {
        for id in ids {
            if id == PRIOR_SOURCE {
                self.prior = true;
            }
            if !self.cites.contains(id) {
                self.cites.push(id.clone());
            }
        }
    }

    /// The constant with this key; records its sources.
    pub(crate) fn constant(&mut self, key: &str) -> &'static Constant {
        let c = constants().constant(key);
        self.add(&c.sources);
        if c.prior {
            self.prior = true;
        }
        c
    }

    /// The default value of a constant; records its sources.
    pub(crate) fn k(&mut self, key: &str) -> f64 {
        self.constant(key).default
    }

    /// The low and high of a constant (each falls back to the default); records its sources.
    pub(crate) fn range(&mut self, key: &str) -> (f64, f64) {
        let c = self.constant(key);
        (c.low.unwrap_or(c.default), c.high.unwrap_or(c.default))
    }

    pub(crate) fn dga(&mut self) -> &'static DgaTable {
        let t = &constants().dga;
        self.add(&t.sources);
        t
    }

    pub(crate) fn tfp_size(&mut self) -> &'static TfpSizeTable {
        let t = &constants().tfp_size;
        self.add(&t.sources);
        t
    }

    pub(crate) fn staples(&mut self) -> &'static StaplesTable {
        let t = &constants().staples;
        self.add(&t.sources);
        t
    }

    pub(crate) fn child_share(&mut self) -> &'static ChildShareTable {
        let t = &constants().child_share;
        self.add(&t.sources);
        t
    }

    pub(crate) fn shelf_life(&mut self) -> &'static ShelfLifeTable {
        let t = &constants().shelf_life;
        self.add(&t.sources);
        t
    }

    pub(crate) fn solar(&mut self) -> &'static SolarTable {
        let t = &constants().solar;
        self.add(&t.sources);
        if t.prior {
            self.prior = true;
        }
        t
    }

    /// Records one citation directly (for guidance in the text that has no number).
    pub(crate) fn cite(&mut self, id: &str) {
        self.add(&[CitationId::from(id)]);
    }

    /// Records several citations (for example a bucket target's own sources).
    pub(crate) fn cite_all(&mut self, ids: &[CitationId]) {
        self.add(ids);
    }

    /// Whether any number read so far is a planning estimate.
    pub(crate) fn is_prior(&self) -> bool {
        self.prior
    }

    /// The citations, in the order first used.
    pub(crate) fn cites(&self) -> &[CitationId] {
        &self.cites
    }

    /// The short attribution appended to a plain-language line: "(Ready.gov, CDC)", with a note
    /// when some amounts are estimates.
    pub(crate) fn attribution(&self) -> String {
        let reg = constants();
        let mut names: Vec<&str> = Vec::new();
        for id in &self.cites {
            if id == PRIOR_SOURCE {
                continue;
            }
            // Ids outside the supply source table (a bucket target's hazard data) are cited in
            // the line's citations but not named in its sentence.
            let Some(short) = reg.source(id.as_str()).map(|s| s.short.as_str()) else {
                continue;
            };
            if !names.contains(&short) {
                names.push(short);
            }
        }
        names.truncate(4);
        match (names.is_empty(), self.prior) {
            (true, true) => "(an estimate)".to_owned(),
            (true, false) => String::new(),
            (false, false) => format!("({})", names.join(", ")),
            (false, true) => format!("({}; some amounts are estimates)", names.join(", ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::keys;

    #[test]
    fn reading_a_constant_records_its_sources_once() {
        let mut b = Basis::new();
        assert_eq!(b.k(keys::WATER_BASIC_GAL), 1.0);
        assert_eq!(b.k(keys::WATER_BASIC_GAL), 1.0);
        assert_eq!(b.cites(), ["ready_gov_water", "cdc_water_storage"]);
        assert!(!b.is_prior());
        assert_eq!(b.attribution(), "(Ready.gov, CDC)");
    }

    #[test]
    fn a_prior_constant_tags_the_basis() {
        let mut b = Basis::new();
        b.k(keys::TOILET_BAGS_PER_PERSON_DAY);
        assert!(b.is_prior());
        assert!(b.cites().iter().any(|c| c == PRIOR_SOURCE));
        assert_eq!(
            b.attribution(),
            "(RDPO, Oregon OEM; some amounts are estimates)"
        );
        let mut only = Basis::new();
        only.k(keys::DOG_WEIGHT_LB);
        assert_eq!(only.attribution(), "(an estimate)");
    }

    #[test]
    fn ranges_fall_back_to_the_default() {
        let mut b = Basis::new();
        assert_eq!(b.range(keys::RX_DAYS_ON_HAND), (7.0, 30.0));
        assert_eq!(b.range(keys::WATER_BASIC_GAL), (1.0, 1.0));
    }
}
