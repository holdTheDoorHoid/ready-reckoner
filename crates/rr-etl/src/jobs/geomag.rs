//! Job — geomagnetic latitude and the NERC geomagnetic scaling factor per county (for the
//! geomagnetic-storm family).
//!
//! - **IGRF-14** (International Geomagnetic Reference Field, 14th generation; NOAA NCEI copy of
//!   the IAGA coefficient file): the degree-1 (dipole) coefficients of the latest main-field epoch
//!   give the geomagnetic north pole. A county's geomagnetic latitude is the latitude of its
//!   internal point measured from that dipole axis.
//! - **NERC TPL-007 benchmark GMD event** (Benchmark GMD Event Description, Appendix II, eq. II.1):
//!   the reference geoelectric field (8 V/km for a 1-in-100-year storm at geomagnetic latitude 60°)
//!   scales with latitude by alpha = 0.001 * exp(0.115 L), bounded to 0.1..1.0.
//!
//! The earth-conductivity factor beta (NERC Table II-2, by USGS earth-conductivity region) is not
//! included: NERC publishes the regions only as a map image (Figure II-3), and no digitised
//! version was found. The engine must say that local ground can raise or lower the field by a
//! factor of several.

use super::{Ctx, JobOutput, load_counties};
use crate::csvout::Table;
use crate::manifest::{Attribution, SourceRecord};
use crate::num::{fixed, sig};
use crate::{Result, data_err};
use rr_types::math::exp;

/// Geomagnetic latitude and NERC alpha per county.
pub const GEOMAG: &str = "core/geomag.csv";

const IGRF: &str = "https://www.ngdc.noaa.gov/IAGA/vmod/coeffs/igrf14coeffs.txt";
const NERC: &str = "https://www.nerc.com/globalassets/standards/projects/2013-03/benchmark_gmd_event_aug27_clean.pdf";

/// The dipole terms of one IGRF epoch, nT.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dipole {
    /// Epoch (decimal year).
    pub epoch: f64,
    /// g(1,0).
    pub g10: f64,
    /// g(1,1).
    pub g11: f64,
    /// h(1,1).
    pub h11: f64,
}

/// Read the dipole terms of the latest main-field epoch from an IGRF coefficient file (the
/// secular-variation column, labelled like `2025-30`, is skipped).
pub fn parse_igrf(text: &str) -> Result<Dipole> {
    let mut epochs: Vec<(usize, f64)> = Vec::new();
    let (mut g10, mut g11, mut h11) = (None, None, None);
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.first() == Some(&"g/h") {
            epochs = cols
                .iter()
                .enumerate()
                .skip(3)
                .filter_map(|(i, c)| c.parse::<f64>().ok().map(|y| (i, y)))
                .collect();
            continue;
        }
        if cols.len() < 4 || epochs.is_empty() {
            continue;
        }
        let Some(&(col, _)) = epochs.last() else {
            continue;
        };
        let value = |s: &str| s.parse::<f64>().ok();
        match (cols[0], cols[1], cols[2]) {
            ("g", "1", "0") => g10 = cols.get(col).and_then(|s| value(s)),
            ("g", "1", "1") => g11 = cols.get(col).and_then(|s| value(s)),
            ("h", "1", "1") => h11 = cols.get(col).and_then(|s| value(s)),
            _ => {}
        }
    }
    let epoch = epochs
        .last()
        .map(|(_, y)| *y)
        .ok_or_else(|| data_err("IGRF: no epoch header line (g/h n m ...)"))?;
    match (g10, g11, h11) {
        (Some(g10), Some(g11), Some(h11)) => Ok(Dipole {
            epoch,
            g10,
            g11,
            h11,
        }),
        _ => Err(data_err(
            "IGRF: dipole coefficients g10, g11, h11 not found",
        )),
    }
}

impl Dipole {
    /// Geographic (latitude, longitude) of the geomagnetic north pole, degrees.
    pub fn north_pole(&self) -> (f64, f64) {
        let b0 = (self.g10 * self.g10 + self.g11 * self.g11 + self.h11 * self.h11).sqrt();
        let colat = (-self.g10 / b0).acos();
        let lon = (-self.h11).atan2(-self.g11);
        (90.0 - colat.to_degrees(), lon.to_degrees())
    }

    /// Geomagnetic latitude of a point, degrees (negative south of the geomagnetic equator).
    pub fn geomag_lat(&self, lat: f64, lon: f64) -> f64 {
        let (plat, plon) = self.north_pole();
        let (p, pp) = (lat.to_radians(), plat.to_radians());
        let dl = (lon - plon).to_radians();
        let s = p.sin() * pp.sin() + p.cos() * pp.cos() * dl.cos();
        s.clamp(-1.0, 1.0).asin().to_degrees()
    }
}

/// NERC's geomagnetic scaling factor for a geomagnetic latitude (degrees; the magnitude is used,
/// so the southern hemisphere is treated like the northern).
pub fn nerc_alpha(geomag_lat: f64) -> f64 {
    (0.001 * exp(0.115 * geomag_lat.abs())).clamp(0.1, 1.0)
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let f = ctx.http.get(IGRF, Some("geomag/igrf14coeffs.txt"))?;
    let dip = parse_igrf(&f.text())?;
    out.source(super::source_from(
        "IGRF-14 spherical harmonic coefficients (IAGA; NOAA NCEI copy)",
        &f,
        format!("IGRF-14, main-field epoch {:.1} (dipole terms)", dip.epoch),
        "Open coefficients published by IAGA for any use; NOAA NCEI distribution is a US Government work",
        "",
    ));
    out.source(SourceRecord {
        name: "NERC Benchmark Geomagnetic Disturbance Event Description (TPL-007), Appendix II"
            .into(),
        url: NERC.into(),
        version: "draft of 2014-08-21; formula II.1 and Table II-1 transcribed (read 2026-09-26)"
            .into(),
        retrieved: "2026-09-26T00:00:00Z".into(),
        sha256: "7fb2757bde8ac84d109399fc56de29ebe01dcd17237f25b7266478d83224ad2d".into(),
        bytes: 0,
        license: "NERC publication; the formula is cited, not copied data".into(),
        obligations: "Cite NERC as the source of the scaling formula.".into(),
    });
    let (plat, plon) = dip.north_pole();
    if !(80.0..=81.5).contains(&plat) {
        return Err(data_err(format!(
            "IGRF geomagnetic north pole at {plat:.2} N: expected 80-81.5 N"
        )));
    }
    let mut table = Table::new(&["fips", "geomag_lat", "geomag_factor"], 1);
    let check = |fips: &str, lo: f64, hi: f64, what: &str| -> Result<()> {
        let c = counties
            .iter()
            .find(|c| c.fips == fips)
            .ok_or_else(|| data_err(format!("no county {fips}")))?;
        let a = nerc_alpha(dip.geomag_lat(c.lat, c.lon));
        if !(lo..=hi).contains(&a) {
            return Err(data_err(format!(
                "NERC alpha for {what} is {a:.3}, expected {lo}-{hi}"
            )));
        }
        Ok(())
    };
    check("12086", 0.1, 0.1, "Miami-Dade (floor)")?;
    check("02020", 1.0, 1.0, "Anchorage (cap)")?;
    check("38101", 0.6, 0.7, "Ward County, ND (Minot)")?;
    check("42101", 0.25, 0.33, "Philadelphia")?;
    for c in &counties {
        let gl = dip.geomag_lat(c.lat, c.lon);
        // One decimal of latitude and two significant figures of alpha: more would be false
        // precision next to the omitted conductivity factor.
        table.push(vec![c.fips.clone(), fixed(gl, 1), sig(nerc_alpha(gl), 2)]);
    }
    out.rows_in += 3;
    out.table(ctx, GEOMAG, &mut table)?;
    out.notes.push(format!(
        "Geomagnetic latitude: angle of each county's internal point from the IGRF-14 dipole equator (epoch {:.1}: g10 {}, g11 {}, h11 {} nT; geomagnetic north pole {plat:.2} N, {:.2} W). geomag_factor: NERC's alpha = 0.001 x exp(0.115 x |latitude|), bounded 0.1 to 1 (TPL-007 benchmark, Appendix II, eq. II.1); the benchmark field is 8 V/km x alpha x beta.",
        dip.epoch, dip.g10, dip.g11, dip.h11, -plon
    ));
    out.notes
        .push("Rounded to 0.1 degree of latitude and two significant figures of alpha.".into());
    out.notes.push("Earth conductivity (NERC's beta, 0.2 to 1.2 across US earth models) is not included: NERC publishes its regions only as a map image. Values are the dipole geomagnetic latitude, not corrected geomagnetic coordinates, which differ by a degree or two over the US.".into());
    out.definitions.insert("geomag_factor".into(), "NERC TPL-007 benchmark scaling factor alpha for geomagnetic latitude: 0.001 x exp(0.115 x L), 0.1 <= alpha <= 1.0 (1.0 at 60 degrees and above).".into());
    out.attributions.push(Attribution {
        source: "IGRF-14 and NERC TPL-007".into(),
        text: "Geomagnetic latitude from the International Geomagnetic Reference Field, 14th generation (IAGA, distributed by NOAA NCEI); scaling factor from NERC's Benchmark Geomagnetic Disturbance Event Description (TPL-007).".into(),
        license: "Open (IAGA coefficients); formula cited from NERC".into(),
        url: IGRF.into(),
        version: Some(format!("IGRF-14 epoch {:.1}", dip.epoch)),
        accessed: f.retrieved[..10].to_string(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# IGRF sample\n\
c/s deg ord IGRF DGRF IGRF SV\n\
g/h n m 1900.0 2020.0 2025.0 2025-30\n\
g  1  0 -31543 -29403.41 -29350.0 12.6\n\
g  1  1  -2298 -1451.37 -1410.3 10.0\n\
h  1  1   5922 4653.35 4545.5 -21.5\n";

    #[test]
    fn dipole_pole_and_alpha_match_the_audit() {
        let d = parse_igrf(SAMPLE).unwrap();
        assert_eq!(
            (d.epoch, d.g10, d.g11, d.h11),
            (2025.0, -29350.0, -1410.3, 4545.5)
        );
        let (lat, lon) = d.north_pole();
        assert!(
            (lat - 80.79).abs() < 0.02 && (lon - -72.76).abs() < 0.02,
            "{lat} {lon}"
        );
        // Audit §8.5: Miami 34.9 (alpha floored at 0.1), Philadelphia 49.2 (0.29), Seattle
        // 53.0 (0.45), Minot 56.1 (0.63), Anchorage 61.9 (1.0).
        for (la, lo, gl, alpha) in [
            (25.77, -80.19, 34.9, 0.1),
            (39.95, -75.16, 49.2, 0.29),
            (47.61, -122.33, 53.0, 0.45),
            (48.23, -101.30, 56.1, 0.63),
            (61.22, -149.90, 61.9, 1.0),
        ] {
            let g = d.geomag_lat(la, lo);
            assert!((g - gl).abs() < 0.3, "{la},{lo}: {g}");
            assert!(
                (nerc_alpha(g) - alpha).abs() < 0.02,
                "{la},{lo}: {}",
                nerc_alpha(g)
            );
        }
        // NERC Table II-1: 0.2 at 45 degrees, 0.3 at 50, 0.5 at 54, 1.0 at 60.
        assert!((nerc_alpha(45.0) - 0.177).abs() < 0.01);
        assert!((nerc_alpha(50.0) - 0.314).abs() < 0.01);
        assert!((nerc_alpha(54.0) - 0.498).abs() < 0.01);
        assert_eq!(nerc_alpha(60.5), 1.0);
        assert_eq!(nerc_alpha(-20.0), 0.1);
    }
}
