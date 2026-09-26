//! Area arithmetic on a regular latitude-longitude lattice: which lattice cells a polygon covers
//! (scanline fill, even-odd rule), and which county each cell or point falls in.
//!
//! Used where a job needs the share of a county's land that some polygon layer covers (karst
//! rock, leveed areas) or needs to split a polygon's attribute among the counties it overlaps.
//! Cells are counted at their centres and weighted by the cosine of their latitude, so a share is
//! a share of area, not of cells. At 0.01 degrees (about 1 km) the error of a county share is a
//! fraction of a percent for all but the smallest counties (independent cities of a few square
//! miles), which the jobs say in their notes.
//!
//! Longitudes: every positive longitude is shifted by -360 degrees ([`norm_lon`]), so the
//! Aleutians (which cross the 180th meridian) and Guam stay contiguous. No US land lies east of
//! Greenwich and west of 180 E in the ordinary sense, so the shift is safe for US data; queries go
//! through the same function.

use crate::geo::Poly;
use crate::http::zip_entry;
use crate::manifest::SourceRecord;
use crate::shp::{Shape, read_dbf, read_shp};
use crate::{Result, data_err};
use std::collections::{BTreeSet, HashMap};

/// Census 2024 cartographic county boundaries, 1:500,000 (also read by `geography` and
/// `facilities`).
pub const CB500: &str = "https://www2.census.gov/geo/tiger/GENZ2024/shp/cb_2024_us_county_500k.zip";

/// Shift positive longitudes by -360 (see the module notes).
pub fn norm_lon(lon: f64) -> f64 {
    if lon > 0.0 { lon - 360.0 } else { lon }
}

/// Apply [`norm_lon`] to every vertex of a set of rings.
pub fn norm_rings(rings: Vec<Vec<[f64; 2]>>) -> Vec<Vec<[f64; 2]>> {
    rings
        .into_iter()
        .map(|r| r.into_iter().map(|p| [norm_lon(p[0]), p[1]]).collect())
        .collect()
}

/// A regular lattice: cell `(i, j)` has its centre at `(lon0 + (i + 0.5) step, lat0 + (j + 0.5)
/// step)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lattice {
    /// West edge.
    pub lon0: f64,
    /// South edge.
    pub lat0: f64,
    /// Cell size in degrees.
    pub step: f64,
    /// Columns.
    pub nx: usize,
    /// Rows.
    pub ny: usize,
}

impl Lattice {
    /// The lattice with cell size `step` whose cells cover the box `[min_lon, min_lat, max_lon,
    /// max_lat]`, snapped to multiples of `step` so that lattices built for different boxes line up.
    pub fn covering(bbox: [f64; 4], step: f64) -> Self {
        // A tiny tolerance keeps an edge that is already a multiple of `step` from slipping a
        // cell to the south-west through rounding (39.87 / 0.01 = 3986.9999999999995).
        let lon0 = (bbox[0] / step + 1e-9).floor() * step;
        let lat0 = (bbox[1] / step + 1e-9).floor() * step;
        let nx = (((bbox[2] - lon0) / step).ceil() as usize).max(1);
        let ny = (((bbox[3] - lat0) / step).ceil() as usize).max(1);
        Self {
            lon0,
            lat0,
            step,
            nx,
            ny,
        }
    }

    /// Longitude of a column's centre.
    pub fn lon(&self, i: usize) -> f64 {
        self.lon0 + (i as f64 + 0.5) * self.step
    }

    /// Latitude of a row's centre.
    pub fn lat(&self, j: usize) -> f64 {
        self.lat0 + (j as f64 + 0.5) * self.step
    }

    /// The cell holding a point, if inside the lattice.
    pub fn cell(&self, lon: f64, lat: f64) -> Option<(usize, usize)> {
        let i = ((lon - self.lon0) / self.step).floor();
        let j = ((lat - self.lat0) / self.step).floor();
        if i < 0.0 || j < 0.0 || i >= self.nx as f64 || j >= self.ny as f64 {
            return None;
        }
        Some((i as usize, j as usize))
    }

    /// Call `f(i, j)` for every cell whose centre lies inside the rings (even-odd rule across all
    /// rings, so holes and islands are handled). Cells outside the lattice are skipped.
    pub fn fill(&self, rings: &[Vec<[f64; 2]>], mut f: impl FnMut(usize, usize)) {
        // Rows whose centre latitude each edge crosses, half-open in y so a vertex on a row is
        // counted once.
        let mut crossings: HashMap<usize, Vec<f64>> = HashMap::new();
        for ring in rings {
            let n = ring.len();
            if n < 3 {
                continue;
            }
            for k in 0..n {
                let a = ring[k];
                let b = ring[(k + 1) % n];
                if a[1] == b[1] {
                    continue;
                }
                let (lo, hi) = if a[1] < b[1] { (a, b) } else { (b, a) };
                // Rows j with lo.y <= lat(j) < hi.y.
                let j0 = ((lo[1] - self.lat0) / self.step - 0.5).ceil();
                let j1 = ((hi[1] - self.lat0) / self.step - 0.5).ceil();
                let j0 = j0.max(0.0) as i64;
                let j1 = (j1 as i64).min(self.ny as i64);
                let mut j = j0;
                while j < j1 {
                    let y = self.lat(j as usize);
                    if y >= lo[1] && y < hi[1] {
                        let t = (y - lo[1]) / (hi[1] - lo[1]);
                        crossings
                            .entry(j as usize)
                            .or_default()
                            .push(lo[0] + t * (hi[0] - lo[0]));
                    }
                    j += 1;
                }
            }
        }
        let mut rows: Vec<(usize, Vec<f64>)> = crossings.into_iter().collect();
        rows.sort_by_key(|(j, _)| *j);
        for (j, mut xs) in rows {
            xs.sort_by(|a, b| a.total_cmp(b));
            for pair in xs.chunks(2) {
                let [xa, xb] = pair else { continue };
                // Columns i with xa <= lon(i) < xb.
                let i0 = ((xa - self.lon0) / self.step - 0.5).ceil().max(0.0) as usize;
                let i1 =
                    (((xb - self.lon0) / self.step - 0.5).ceil().max(0.0) as usize).min(self.nx);
                for i in i0..i1 {
                    f(i, j);
                }
            }
        }
    }
}

/// Bounding box `[min_lon, min_lat, max_lon, max_lat]` of a set of rings.
pub fn bbox_of(rings: &[Vec<[f64; 2]>]) -> [f64; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for r in rings {
        for p in r {
            b = [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ];
        }
    }
    b
}

/// Weight of a cell at latitude `lat`, proportional to its area.
pub fn cell_weight(lat: f64) -> f64 {
    lat.to_radians().cos().max(0.0)
}

/// County polygons from the Census 1:500k file, with a point locator.
pub struct CountyGeo {
    /// `(fips, polygon)` with normalised longitudes, sorted by FIPS.
    pub polys: Vec<(String, Poly)>,
    grid: HashMap<(i32, i32), Vec<usize>>,
}

impl CountyGeo {
    /// Download the Census 2024 1:500k county file and keep the counties in `canon`.
    pub fn download(
        ctx: &crate::jobs::Ctx,
        canon: &BTreeSet<String>,
        raw_name: &str,
    ) -> Result<(Self, SourceRecord)> {
        let cb = ctx.http.get(CB500, Some(raw_name))?;
        let source = crate::jobs::source_from(
            "Census cartographic boundary counties 2024, 1:500,000 (for point-in-polygon and areas)",
            &cb,
            "GENZ2024",
            crate::jobs::PUBLIC_DOMAIN,
            "",
        );
        Ok((Self::from_zip(&cb.bytes, canon)?, source))
    }

    /// Build from the zipped shapefile.
    pub fn from_zip(zip: &[u8], canon: &BTreeSet<String>) -> Result<Self> {
        let shp = read_shp(&zip_entry(zip, ".shp")?)?;
        let dbf = read_dbf(&zip_entry(zip, ".dbf")?)?;
        if shp.len() != dbf.records.len() {
            return Err(data_err(
                "county shapefile: .shp and .dbf record counts differ",
            ));
        }
        let ig = dbf.field("GEOID")?;
        let mut polys = Vec::new();
        for (s, r) in shp.into_iter().zip(dbf.records.iter()) {
            if let Shape::Polygon(rings) = s
                && canon.contains(&r[ig])
            {
                polys.push((r[ig].clone(), Poly::new(norm_rings(rings))));
            }
        }
        Ok(Self::new(polys))
    }

    /// Build from polygons (longitudes already normalised).
    pub fn new(mut polys: Vec<(String, Poly)>) -> Self {
        polys.sort_by(|a, b| a.0.cmp(&b.0));
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, (_, p)) in polys.iter().enumerate() {
            for x in p.bbox[0].floor() as i32..=p.bbox[2].floor() as i32 {
                for y in p.bbox[1].floor() as i32..=p.bbox[3].floor() as i32 {
                    grid.entry((x, y)).or_default().push(i);
                }
            }
        }
        Self { polys, grid }
    }

    /// Index of the county containing a point (exact point-in-polygon), if any.
    pub fn locate_index(&self, lat: f64, lon: f64) -> Option<usize> {
        let lon = norm_lon(lon);
        let cands = self.grid.get(&(lon.floor() as i32, lat.floor() as i32))?;
        cands
            .iter()
            .copied()
            .find(|i| self.polys[*i].1.contains(lon, lat))
    }

    /// FIPS of the county containing a point, if any.
    pub fn locate(&self, lat: f64, lon: f64) -> Option<&str> {
        self.locate_index(lat, lon)
            .map(|i| self.polys[i].0.as_str())
    }

    /// Rasterise every county at `step` degrees: one lattice per group of counties (by the box
    /// their polygons span), each cell holding the index into [`Self::polys`] of the county whose
    /// polygon covers the cell's centre.
    pub fn raster(&self, step: f64) -> CountyRaster {
        // Group by state code (first two FIPS digits) into regions: the contiguous US and each
        // outlying state or territory separately, so no lattice spans an ocean.
        let mut groups: HashMap<&'static str, Vec<usize>> = HashMap::new();
        for (i, (fips, _)) in self.polys.iter().enumerate() {
            let key = match &fips[..2] {
                "02" => "AK",
                "15" => "HI",
                "72" | "78" => "PRVI",
                "66" | "69" => "GUMP",
                "60" => "AS",
                _ => "CONUS",
            };
            groups.entry(key).or_default().push(i);
        }
        let mut keys: Vec<&'static str> = groups.keys().copied().collect();
        keys.sort();
        let mut regions = Vec::new();
        for key in keys {
            let members = &groups[key];
            let mut bbox = [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ];
            for &i in members {
                let b = self.polys[i].1.bbox;
                bbox = [
                    bbox[0].min(b[0]),
                    bbox[1].min(b[1]),
                    bbox[2].max(b[2]),
                    bbox[3].max(b[3]),
                ];
            }
            let lattice = Lattice::covering(bbox, step);
            let mut cells = vec![u16::MAX; lattice.nx * lattice.ny];
            for &i in members {
                let idx = i as u16;
                lattice.fill(&self.polys[i].1.rings, |x, y| {
                    cells[y * lattice.nx + x] = idx;
                });
            }
            regions.push(RasterRegion { lattice, cells });
        }
        CountyRaster { regions }
    }
}

/// One lattice of county indices.
pub struct RasterRegion {
    /// The lattice.
    pub lattice: Lattice,
    /// `ny * nx` county indices (row-major from the south-west corner); `u16::MAX` = no county.
    pub cells: Vec<u16>,
}

/// County indices on lattices (see [`CountyGeo::raster`]).
pub struct CountyRaster {
    /// One lattice per region.
    pub regions: Vec<RasterRegion>,
}

impl CountyRaster {
    /// County index at a point (the cell holding it), if any.
    pub fn county_at(&self, lat: f64, lon: f64) -> Option<usize> {
        let lon = norm_lon(lon);
        for r in &self.regions {
            if let Some((i, j)) = r.lattice.cell(lon, lat) {
                let c = r.cells[j * r.lattice.nx + i];
                if c != u16::MAX {
                    return Some(c as usize);
                }
            }
        }
        None
    }

    /// Area weight of every county's cells (index into [`CountyGeo::polys`]).
    pub fn county_weights(&self, n_counties: usize) -> Vec<f64> {
        let mut w = vec![0.0; n_counties];
        for r in &self.regions {
            for j in 0..r.lattice.ny {
                let cw = cell_weight(r.lattice.lat(j));
                for i in 0..r.lattice.nx {
                    let c = r.cells[j * r.lattice.nx + i];
                    if c != u16::MAX {
                        w[c as usize] += cw;
                    }
                }
            }
        }
        w
    }

    /// For a polygon layer (rings with normalised longitudes), add each covered cell's weight to
    /// the county it falls in. Cells are marked once even when several polygons overlap them, so
    /// the result is the area of the union. Returns weights by county index.
    pub fn covered_weights(&self, polys: &[Vec<Vec<[f64; 2]>>], n_counties: usize) -> Vec<f64> {
        let mut w = vec![0.0; n_counties];
        for r in &self.regions {
            let mut hit = vec![false; r.cells.len()];
            let lb = [
                r.lattice.lon0,
                r.lattice.lat0,
                r.lattice.lon0 + r.lattice.nx as f64 * r.lattice.step,
                r.lattice.lat0 + r.lattice.ny as f64 * r.lattice.step,
            ];
            for rings in polys {
                let b = bbox_of(rings);
                if b[2] < lb[0] || b[0] > lb[2] || b[3] < lb[1] || b[1] > lb[3] {
                    continue;
                }
                r.lattice
                    .fill(rings, |i, j| hit[j * r.lattice.nx + i] = true);
            }
            for j in 0..r.lattice.ny {
                let cw = cell_weight(r.lattice.lat(j));
                for i in 0..r.lattice.nx {
                    let k = j * r.lattice.nx + i;
                    if hit[k] && r.cells[k] != u16::MAX {
                        w[r.cells[k] as usize] += cw;
                    }
                }
            }
        }
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f64, y0: f64, s: f64) -> Vec<[f64; 2]> {
        vec![
            [x0, y0],
            [x0 + s, y0],
            [x0 + s, y0 + s],
            [x0, y0 + s],
            [x0, y0],
        ]
    }

    #[test]
    fn fill_counts_cells_of_a_square_with_a_hole() {
        let l = Lattice::covering([0.0, 0.0, 1.0, 1.0], 0.1);
        assert_eq!((l.nx, l.ny), (10, 10));
        let mut n = 0;
        l.fill(&[square(0.0, 0.0, 1.0)], |_, _| n += 1);
        assert_eq!(n, 100);
        let mut n = 0;
        l.fill(&[square(0.0, 0.0, 1.0), square(0.2, 0.2, 0.4)], |_, _| {
            n += 1
        });
        assert_eq!(n, 100 - 16);
        // A triangle covers about half the cells.
        let mut n = 0;
        l.fill(
            &[vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [0.0, 0.0]]],
            |_, _| n += 1,
        );
        assert!((40..=55).contains(&n), "{n}");
    }

    #[test]
    fn lattices_snap_to_the_step() {
        let a = Lattice::covering([-75.135, 39.875, -74.96, 40.14], 0.01);
        assert!((a.lon0 - -75.14).abs() < 1e-9 && (a.lat0 - 39.87).abs() < 1e-9);
        assert_eq!(a.cell(-75.135, 39.875), Some((0, 0)));
        assert_eq!(a.cell(-80.0, 39.9), None);
        // An edge on a multiple of the step stays put.
        let b = Lattice::covering([-75.13, 39.87, -74.96, 40.14], 0.01);
        assert!(
            (b.lon0 - -75.13).abs() < 1e-9 && (b.lat0 - 39.87).abs() < 1e-9,
            "{b:?}"
        );
    }

    #[test]
    fn county_raster_shares() {
        // Two unit-square "counties" side by side; a layer covering the east half of the first.
        let geo = CountyGeo::new(vec![
            ("01001".into(), Poly::new(vec![square(-90.0, 30.0, 1.0)])),
            ("01003".into(), Poly::new(vec![square(-89.0, 30.0, 1.0)])),
        ]);
        let r = geo.raster(0.05);
        let total = r.county_weights(2);
        let east_half = vec![
            [-89.5, 30.0],
            [-89.0, 30.0],
            [-89.0, 31.0],
            [-89.5, 31.0],
            [-89.5, 30.0],
        ];
        // Overlapping polygons count once (the union).
        let layer = vec![vec![east_half], vec![square(-89.5, 30.2, 0.2)]];
        let hit = r.covered_weights(&layer, 2);
        assert!(
            (hit[0] / total[0] - 0.5).abs() < 0.02,
            "{}",
            hit[0] / total[0]
        );
        assert_eq!(hit[1], 0.0);
        assert_eq!(geo.locate(30.5, -88.5), Some("01003"));
        assert_eq!(
            r.county_at(30.5, -89.7).map(|i| geo.polys[i].0.as_str()),
            Some("01001")
        );
        assert_eq!(norm_lon(172.5), -187.5);
    }
}
