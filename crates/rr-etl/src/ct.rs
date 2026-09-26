//! Connecticut: old counties (`09001`–`09015`) versus planning regions (`09110`–`09190`).
//!
//! Since 2022 the Census Bureau uses Connecticut's nine planning regions as county equivalents.
//! NRI v1.20, the Census 2024 files, CRE 2024, SVI 2022 and OpenFEMA use the regions; the 2020
//! ZCTA relationship file, EAGLE-I, Storm Events, SPC, the NWS zone file, the NCA5 Atlas and
//! CMRA use the old counties. Towns (county subdivisions) nest in both, so the Census town-level
//! crosswalk gives exact old-county/region overlaps; we weight them by town land area.
//!
//! Two conversions are offered:
//! - [`Crosswalk::intensive`] for rates, shares and averages: a region's value is the
//!   land-area-weighted mean of the old counties that overlap it.
//! - [`Crosswalk::extensive`] for counts and totals: an old county's total is split across
//!   regions in proportion to land area.

use crate::csvout::{Table, col, read_table};
use crate::num::sig4;
use crate::{Result, data_err};
use std::collections::BTreeMap;
use std::path::Path;

/// Relative path of the crosswalk pack file.
pub const PATH: &str = "core/ct_crosswalk.csv";

/// One old-county × region overlap.
#[derive(Debug, Clone, PartialEq)]
pub struct Overlap {
    /// Old county FIPS (`090xx`).
    pub old: String,
    /// Planning region FIPS (`091x0`).
    pub region: String,
    /// Share of the old county's land inside the region.
    pub share_of_old: f64,
    /// Share of the region's land that comes from the old county.
    pub share_of_region: f64,
    /// Number of towns in the overlap.
    pub towns: u32,
}

/// The crosswalk.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Crosswalk {
    /// Overlaps sorted by (old, region).
    pub overlaps: Vec<Overlap>,
}

/// True for an old Connecticut county code.
pub fn is_old_ct(fips: &str) -> bool {
    fips.len() == 5 && fips.starts_with("090")
}

/// Retired county codes that older records still use, with their 2024 successors. Counts are
/// split evenly among successors; rates are copied to each.
pub const RETIRED: &[(&str, &[&str])] = &[
    ("02261", &["02063", "02066"]), // Valdez-Cordova Census Area, split in 2019
    ("02270", &["02158"]),          // Wade Hampton Census Area, renamed Kusilvak in 2015
    ("02280", &["02195", "02275"]), // Wrangell-Petersburg Census Area, split in 2008
    ("12025", &["12086"]),          // Dade County, renamed Miami-Dade in 1997
    ("46113", &["46102"]),          // Shannon County, renamed Oglala Lakota in 2015
    ("51515", &["51019"]),          // Bedford city, merged into Bedford County in 2013
];

/// Successors of a retired county code, if it is one.
pub fn successors(code: &str) -> Option<&'static [&'static str]> {
    RETIRED
        .iter()
        .find(|(old, _)| *old == code)
        .map(|(_, new)| *new)
}

impl Crosswalk {
    /// Build from town rows `(old county fips, region fips, town land area m²)`.
    pub fn from_towns(towns: &[(String, String, f64)]) -> Result<Self> {
        let mut pair: BTreeMap<(String, String), (f64, u32)> = BTreeMap::new();
        let mut old_tot: BTreeMap<String, f64> = BTreeMap::new();
        let mut reg_tot: BTreeMap<String, f64> = BTreeMap::new();
        for (old, reg, land) in towns {
            let e = pair.entry((old.clone(), reg.clone())).or_insert((0.0, 0));
            e.0 += land;
            e.1 += 1;
            *old_tot.entry(old.clone()).or_default() += land;
            *reg_tot.entry(reg.clone()).or_default() += land;
        }
        if old_tot.len() != 8 || reg_tot.len() != 9 {
            return Err(data_err(format!(
                "Connecticut crosswalk should link 8 old counties to 9 planning regions, found {} and {}",
                old_tot.len(),
                reg_tot.len()
            )));
        }
        let overlaps = pair
            .into_iter()
            .map(|((old, region), (land, towns))| Overlap {
                share_of_old: land / old_tot[&old],
                share_of_region: land / reg_tot[&region],
                old,
                region,
                towns,
            })
            .collect();
        Ok(Self { overlaps })
    }

    /// As a pack table.
    pub fn to_table(&self) -> Table {
        let mut t = Table::new(
            &[
                "old_fips",
                "region_fips",
                "land_share_of_old",
                "land_share_of_region",
                "towns",
            ],
            2,
        );
        for o in &self.overlaps {
            t.push(vec![
                o.old.clone(),
                o.region.clone(),
                sig4(o.share_of_old),
                sig4(o.share_of_region),
                o.towns.to_string(),
            ]);
        }
        t
    }

    /// Load from `data/core/ct_crosswalk.csv` (written by the geography job).
    pub fn load(data_dir: &Path) -> Result<Self> {
        let (h, rows) = read_table(data_dir, PATH)?;
        let (io, ir, iso, isr, it) = (
            col(&h, "old_fips")?,
            col(&h, "region_fips")?,
            col(&h, "land_share_of_old")?,
            col(&h, "land_share_of_region")?,
            col(&h, "towns")?,
        );
        let parse = |s: &str| {
            s.parse::<f64>()
                .map_err(|_| data_err(format!("bad number {s} in {PATH}")))
        };
        let mut overlaps = Vec::new();
        for r in rows {
            overlaps.push(Overlap {
                old: r[io].clone(),
                region: r[ir].clone(),
                share_of_old: parse(&r[iso])?,
                share_of_region: parse(&r[isr])?,
                towns: r[it].parse().unwrap_or(0),
            });
        }
        Ok(Self { overlaps })
    }

    /// Planning regions.
    pub fn regions(&self) -> Vec<String> {
        let mut v: Vec<String> = self.overlaps.iter().map(|o| o.region.clone()).collect();
        v.sort();
        v.dedup();
        v
    }

    /// Area-weighted mean for each region of the old-county values it overlaps. Old counties
    /// absent from `values` are skipped and the remaining weights renormalised; a region with no
    /// overlapping values is omitted.
    pub fn intensive(&self, values: &BTreeMap<String, f64>) -> BTreeMap<String, f64> {
        let mut acc: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for o in &self.overlaps {
            if let Some(v) = values.get(&o.old) {
                let e = acc.entry(o.region.clone()).or_insert((0.0, 0.0));
                e.0 += o.share_of_region * v;
                e.1 += o.share_of_region;
            }
        }
        acc.into_iter()
            .filter(|(_, (_, w))| *w > 0.0)
            .map(|(k, (s, w))| (k, s / w))
            .collect()
    }

    /// Split old-county totals across regions by land area.
    pub fn extensive(&self, values: &BTreeMap<String, f64>) -> BTreeMap<String, f64> {
        let mut acc: BTreeMap<String, f64> = BTreeMap::new();
        for o in &self.overlaps {
            if let Some(v) = values.get(&o.old) {
                *acc.entry(o.region.clone()).or_default() += o.share_of_old * v;
            }
        }
        acc
    }

    /// Replace old-county keys in a per-county map by region keys using [`Self::intensive`].
    /// Non-Connecticut keys pass through unchanged; region keys already present win.
    pub fn apply_intensive(&self, values: BTreeMap<String, f64>) -> BTreeMap<String, f64> {
        let (old, mut rest): (BTreeMap<_, _>, BTreeMap<_, _>) =
            values.into_iter().partition(|(k, _)| is_old_ct(k));
        for (k, v) in self.intensive(&old) {
            rest.entry(k).or_insert(v);
        }
        rest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toy() -> Crosswalk {
        // Minimal valid crosswalk: 8 old counties, 9 regions; old 09001 split 30/70 by land
        // between regions 09120 and 09190.
        let mut towns = vec![
            ("09001".to_string(), "09120".to_string(), 30.0),
            ("09001".to_string(), "09190".to_string(), 70.0),
        ];
        let rest = [
            ("09003", "09110"),
            ("09005", "09160"),
            ("09007", "09130"),
            ("09009", "09170"),
            ("09011", "09180"),
            ("09013", "09150"),
            ("09015", "09140"),
        ];
        for (o, r) in rest {
            towns.push((o.to_string(), r.to_string(), 50.0));
        }
        Crosswalk::from_towns(&towns).unwrap()
    }

    #[test]
    fn intensive_and_extensive() {
        let cw = toy();
        let mut v = BTreeMap::new();
        v.insert("09001".to_string(), 10.0);
        let ext = cw.extensive(&v);
        assert!((ext["09120"] - 3.0).abs() < 1e-12);
        assert!((ext["09190"] - 7.0).abs() < 1e-12);
        let int = cw.intensive(&v);
        assert!((int["09120"] - 10.0).abs() < 1e-12);
        assert!((int["09190"] - 10.0).abs() < 1e-12);
        assert!(!int.contains_key("09110"));
    }

    #[test]
    fn apply_keeps_other_states() {
        let cw = toy();
        let mut v = BTreeMap::new();
        v.insert("09003".to_string(), 2.0);
        v.insert("42101".to_string(), 5.0);
        let out = cw.apply_intensive(v);
        assert_eq!(out.get("42101"), Some(&5.0));
        assert_eq!(out.get("09110"), Some(&2.0));
        assert!(!out.contains_key("09003"));
    }
}
