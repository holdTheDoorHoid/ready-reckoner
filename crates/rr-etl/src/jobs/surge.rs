//! Optional job `surge` — share of each ZIP code's land inside NOAA/NHC's storm-surge areas
//! (National Storm Surge Risk Maps, SLOSH Maximum of Maximums at high tide), for the optional
//! `surge` pack. Not in the quarterly refresh (`default: false`): the contiguous-US archive is
//! 1.65 GB. Run it with `rr-etl refresh --out data --only surge`.
//!
//! For each map region (Texas to Maine v4, Southern California, Hawaii, Puerto Rico, the US
//! Virgin Islands, Guam, American Samoa) the archive goes to `data/raw/surge/`, the Category 1 and
//! Category 3 GeoTIFFs are extracted one at a time, and every ZIP code (Census 2020 cartographic
//! ZCTA outlines) whose box meets the raster is sampled on a 3 arc-second lattice (about 90 m).
//! A lattice point is inundated when its cell holds a depth class (1 to 21: feet of water above
//! ground in one-foot bins, 21 = more than 20 feet). The share is inundated points / all points in
//! the ZIP. The Category 3 area contains the Category 1 and 2 areas, so `surge_cat3_share` is the
//! "Category 1 to 3 zone". Every file is deleted as soon as it has been read.
//!
//! NHC's terms: the maps are "a tool for general education/awareness of the storm surge hazard at
//! a city/community level (not for a parcel level/grid cell assessment)" and "should not be used
//! to replace the maps used for hurricane evacuation zones". A ZIP share is community level; the
//! app must still send people to their official evacuation zone. Areas behind levees are hatched
//! in NHC's viewer rather than mapped, so leveed ZIPs (central New Orleans) read low.

use super::{Ctx, JobOutput};
use crate::csvout::Table;
use crate::http::zip_entry;
use crate::manifest::{Attribution, SourceRecord};
use crate::num::sig;
use crate::raster::{Lattice, Rings, bbox_of, norm_rings};
use crate::shp::{Shape, read_dbf, read_shp};
use crate::{Result, data_err};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// ZIP surge shares (optional pack `surge`).
pub const ZIP_SURGE: &str = "opt/surge/zip_surge.csv";

const BASE: &str = "https://www.nhc.noaa.gov/gis/hazardmaps/";
const ZCTA_OUTLINES: &str =
    "https://www2.census.gov/geo/tiger/GENZ2020/shp/cb_2020_us_zcta520_500k.zip";
const PAGE: &str = "https://www.nhc.noaa.gov/nationalsurge/";

/// Map archives: (file, region label).
pub const REGIONS: &[(&str, &str)] = &[
    (
        "US_SLOSH_MOM_Inundation_v4.zip",
        "Texas to Maine (v4, 2026)",
    ),
    (
        "Southern_California_SLOSH_MOM_Inundation_v3.zip",
        "Southern California (v3)",
    ),
    ("Hawaii_SLOSH_MOM_Inundation.zip", "Hawaii"),
    ("PR_SLOSH_MOM_Inundation.zip", "Puerto Rico"),
    ("USVI_SLOSH_MOM_Inundation.zip", "US Virgin Islands"),
    ("Guam_SLOSH_MOM_Inundation_v3.zip", "Guam (v3)"),
    (
        "American_Samoa_SLOSH_MOM_Inundation_v3.zip",
        "American Samoa (v3)",
    ),
];

/// Lattice step, degrees (3 arc-seconds).
pub const STEP_DEG: f64 = 1.0 / 1200.0;

/// Whether a raster value is a depth class (inundated).
pub fn inundated(v: u8) -> bool {
    (1..=21).contains(&v)
}

/// A world file (`.tfw`): pixel size and the centre of the upper-left pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldFile {
    /// Pixel width (degrees, positive).
    pub dx: f64,
    /// Pixel height (degrees, negative: rows run south).
    pub dy: f64,
    /// Longitude of the upper-left pixel's centre.
    pub x0: f64,
    /// Latitude of the upper-left pixel's centre.
    pub y0: f64,
}

/// Parse a world file; only north-up grids in degrees are accepted.
pub fn parse_world_file(text: &str) -> Result<WorldFile> {
    let v: Vec<f64> = text
        .split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    if v.len() < 6 {
        return Err(data_err("world file: expected six numbers"));
    }
    let w = WorldFile {
        dx: v[0],
        dy: v[3],
        x0: v[4],
        y0: v[5],
    };
    if v[1] != 0.0 || v[2] != 0.0 || w.dx <= 0.0 || w.dy >= 0.0 || w.dx > 1.0 || w.x0.abs() > 360.0
    {
        return Err(data_err(format!(
            "world file: not a north-up grid in degrees ({v:?}); the job reads geographic rasters only"
        )));
    }
    Ok(w)
}

/// A single-band 8-bit GeoTIFF read chunk by chunk (tiles or strips), with a small cache.
pub struct Raster<R: Read + std::io::Seek> {
    dec: tiff::decoder::Decoder<R>,
    /// Width and height in pixels.
    pub size: (u32, u32),
    chunk: (u32, u32),
    across: u32,
    tiled: bool,
    empty: Vec<bool>,
    world: WorldFile,
    cache: HashMap<u32, Option<(u32, Vec<u8>)>>,
}

/// Cells outside the raster or in empty chunks.
pub const NODATA: u8 = 255;

impl<R: Read + std::io::Seek> Raster<R> {
    /// Open a TIFF.
    pub fn open(reader: R, world: WorldFile) -> Result<Self> {
        let mut limits = tiff::decoder::Limits::default();
        limits.ifd_value_size = 1 << 30;
        let mut dec = tiff::decoder::Decoder::new(reader)
            .map_err(|e| data_err(format!("GeoTIFF: {e}")))?
            .with_limits(limits);
        let size = dec
            .dimensions()
            .map_err(|e| data_err(format!("GeoTIFF: {e}")))?;
        let chunk = dec.chunk_dimensions();
        let tiled = matches!(dec.get_chunk_type(), tiff::decoder::ChunkType::Tile);
        let across = if tiled { size.0.div_ceil(chunk.0) } else { 1 };
        let tag = if tiled {
            tiff::tags::Tag::TileByteCounts
        } else {
            tiff::tags::Tag::StripByteCounts
        };
        let counts: Vec<u64> = dec
            .find_tag_unsigned_vec::<u64>(tag)
            .map_err(|e| data_err(format!("GeoTIFF byte counts: {e}")))?
            .unwrap_or_default();
        Ok(Self {
            dec,
            size,
            chunk,
            across,
            tiled,
            empty: counts.iter().map(|c| *c == 0).collect(),
            world,
            cache: HashMap::new(),
        })
    }

    /// The raster's box `[min_lon, min_lat, max_lon, max_lat]` (cell edges).
    pub fn bbox(&self) -> [f64; 4] {
        let w = &self.world;
        [
            w.x0 - w.dx / 2.0,
            w.y0 + w.dy * (self.size.1 as f64 - 0.5),
            w.x0 + w.dx * (self.size.0 as f64 - 0.5),
            w.y0 - w.dy / 2.0,
        ]
    }

    /// The value of the cell holding a point.
    pub fn value(&mut self, lon: f64, lat: f64) -> Result<u8> {
        let w = self.world;
        let col = ((lon - w.x0) / w.dx).round();
        let row = ((lat - w.y0) / w.dy).round();
        if col < 0.0 || row < 0.0 || col >= self.size.0 as f64 || row >= self.size.1 as f64 {
            return Ok(NODATA);
        }
        let (col, row) = (col as u32, row as u32);
        let (idx, cx, cy) = if self.tiled {
            let (tx, ty) = (col / self.chunk.0, row / self.chunk.1);
            (ty * self.across + tx, tx * self.chunk.0, ty * self.chunk.1)
        } else {
            let s = row / self.chunk.1;
            (s, 0, s * self.chunk.1)
        };
        if self.empty.get(idx as usize).copied().unwrap_or(true) {
            return Ok(NODATA);
        }
        if !self.cache.contains_key(&idx) {
            if self.cache.len() >= 4096 {
                self.cache.clear();
            }
            let (data_w, _) = self.dec.chunk_data_dimensions(idx);
            let buf = match self
                .dec
                .read_chunk(idx)
                .map_err(|e| data_err(format!("GeoTIFF chunk {idx}: {e}")))?
            {
                tiff::decoder::DecodingResult::U8(v) => Some((data_w, v)),
                _ => return Err(data_err("GeoTIFF: expected 8-bit samples")),
            };
            self.cache.insert(idx, buf);
        }
        Ok(match self.cache.get(&idx).and_then(|c| c.as_ref()) {
            Some((data_w, v)) => v
                .get(((row - cy) * data_w + (col - cx)) as usize)
                .copied()
                .unwrap_or(NODATA),
            None => NODATA,
        })
    }
}

/// Share of a ZIP's lattice points inundated in a raster: (inundated, total).
pub fn sample_zip<R: Read + std::io::Seek>(
    raster: &mut Raster<R>,
    rings: &Rings,
    step: f64,
) -> Result<(u64, u64)> {
    let lat = Lattice::covering(bbox_of(rings), step);
    let mut cells: Vec<(usize, usize)> = Vec::new();
    lat.fill(rings, |i, j| cells.push((i, j)));
    let mut wet = 0u64;
    for &(i, j) in &cells {
        if inundated(raster.value(lat.lon(i), lat.lat(j))?) {
            wet += 1;
        }
    }
    Ok((wet, cells.len() as u64))
}

/// Extract one entry of an archive on disk to a file (streamed), returning its path.
fn extract(zip_path: &Path, entry_name: &str, to: &Path) -> Result<PathBuf> {
    let mut archive = zip::ZipArchive::new(BufReader::new(File::open(zip_path)?))?;
    let mut entry = archive.by_name(entry_name)?;
    let mut f = std::io::BufWriter::with_capacity(1 << 20, File::create(to)?);
    std::io::copy(&mut entry, &mut f)?;
    f.flush()?;
    Ok(to.to_path_buf())
}

/// Names in an archive on disk.
fn entry_names(zip_path: &Path) -> Result<Vec<String>> {
    let mut archive = zip::ZipArchive::new(BufReader::new(File::open(zip_path)?))?;
    let mut out = Vec::new();
    for i in 0..archive.len() {
        out.push(archive.by_index(i)?.name().to_string());
    }
    Ok(out)
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();

    // ZIP outlines.
    let cb = ctx
        .http
        .get(ZCTA_OUTLINES, Some("surge/cb_2020_us_zcta520_500k.zip"))?;
    out.source(super::source_from(
        "Census cartographic boundary ZCTAs 2020, 1:500,000",
        &cb,
        "GENZ2020 cb_2020_us_zcta520_500k",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let shp = read_shp(&zip_entry(&cb.bytes, ".shp")?)?;
    let dbf = read_dbf(&zip_entry(&cb.bytes, ".dbf")?)?;
    let iz = dbf.field("ZCTA5CE20").or_else(|_| dbf.field("GEOID20"))?;
    let mut zips: Vec<(String, Rings, [f64; 4])> = Vec::new();
    for (s, r) in shp.into_iter().zip(dbf.records.iter()) {
        if let Shape::Polygon(rings) = s {
            let rings = norm_rings(rings);
            let b = bbox_of(&rings);
            zips.push((r[iz].clone(), rings, b));
        }
    }
    drop(cb);
    out.rows_in += zips.len() as u64;

    let dir = ctx.http.raw_dir.join("surge");
    std::fs::create_dir_all(&dir)?;
    // (zip) -> [cat1 (wet, total), cat3 (wet, total)]; a ZIP met by two regions keeps the
    // larger share.
    let mut shares: BTreeMap<String, [Option<f64>; 2]> = BTreeMap::new();
    let mut region_notes = Vec::new();
    for (file, label) in REGIONS {
        let url = format!("{BASE}{file}");
        let zip_path = dir.join(file);
        let streamed = ctx.http.download_to(&url, &zip_path)?;
        out.source(SourceRecord {
            name: format!("NOAA NHC National Storm Surge Risk Maps (SLOSH MOM), {label}"),
            url: streamed.final_url.clone(),
            version: (*file).to_string(),
            retrieved: streamed.retrieved.clone(),
            sha256: streamed.sha256.clone(),
            bytes: streamed.bytes,
            license: super::PUBLIC_DOMAIN.into(),
            obligations: "General education and awareness at a city or community level; not a replacement for official evacuation zones (NHC disclaimer).".into(),
        });
        let result = (|| -> Result<String> {
            let names = entry_names(&zip_path)?;
            let mut met = 0usize;
            for (k, cat) in ["Category1", "Category3"].iter().enumerate() {
                let tif = names
                    .iter()
                    .find(|n| n.contains(cat) && n.to_ascii_lowercase().ends_with(".tif"))
                    .ok_or_else(|| data_err(format!("{file}: no {cat} GeoTIFF")))?;
                let tfw_name = format!("{}.tfw", tif.trim_end_matches(".tif"));
                let tfw = names
                    .iter()
                    .find(|n| n.eq_ignore_ascii_case(&tfw_name))
                    .ok_or_else(|| data_err(format!("{file}: no world file for {tif}")))?;
                let world = {
                    let mut archive = zip::ZipArchive::new(BufReader::new(File::open(&zip_path)?))?;
                    let mut s = String::new();
                    archive.by_name(tfw)?.read_to_string(&mut s)?;
                    parse_world_file(&s)?
                };
                let tif_path = extract(&zip_path, tif, &dir.join(format!("{cat}.tif")))?;
                let sampled = (|| -> Result<usize> {
                    let mut raster = Raster::open(
                        BufReader::with_capacity(1 << 16, File::open(&tif_path)?),
                        world,
                    )?;
                    let rb = raster.bbox();
                    let mut n = 0usize;
                    for (z, rings, b) in &zips {
                        if b[2] < rb[0] || b[0] > rb[2] || b[3] < rb[1] || b[1] > rb[3] {
                            continue;
                        }
                        let (wet, total) = sample_zip(&mut raster, rings, STEP_DEG)?;
                        if total == 0 {
                            continue;
                        }
                        n += 1;
                        let share = wet as f64 / total as f64;
                        let slot = &mut shares.entry(z.clone()).or_insert([None, None])[k];
                        *slot = Some(slot.map_or(share, |s: f64| s.max(share)));
                    }
                    Ok(n)
                })();
                crate::http::remove_raw(&tif_path)?;
                met = met.max(sampled?);
            }
            Ok(format!("{label}: {met} ZIPs sampled"))
        })();
        if !ctx.http.keep_raw {
            crate::http::remove_raw(&zip_path)?;
        }
        region_notes.push(result?);
    }

    let mut table = Table::new(&["zip", "surge_cat1_share", "surge_cat3_share"], 1);
    let mut nonzero = 0usize;
    for (z, [c1, c3]) in &shares {
        let (c1, c3) = (c1.unwrap_or(0.0), c3.unwrap_or(0.0));
        if c3 > 0.0 {
            nonzero += 1;
        }
        table.push(vec![z.clone(), sig(c1, 3), sig(c3, 3)]);
    }
    let get = |z: &str| shares.get(z).and_then(|s| s[1]).unwrap_or(-1.0);
    for (z, lo, place) in [
        ("33139", 0.8, "Miami Beach, FL"),
        ("33040", 0.5, "Key West, FL"),
        ("77550", 0.3, "Galveston, TX"),
    ] {
        if get(z) < lo {
            return Err(data_err(format!(
                "Category 3 surge share for {place} ({z}) is {:.3}, expected more than {lo}",
                get(z)
            )));
        }
    }
    if shares.contains_key("80202") || nonzero < 3000 {
        return Err(data_err(format!(
            "surge: Denver 80202 sampled ({}) or only {nonzero} ZIPs with any Category 3 surge (expected 3,000+)",
            shares.contains_key("80202")
        )));
    }
    out.table(ctx, ZIP_SURGE, &mut table)?;
    out.notes.push(format!(
        "{}. {nonzero} ZIPs have some Category 3 surge area. Lattice every 3 arc-seconds (about 90 m) inside each Census 2020 1:500k ZCTA outline; a point counts when its raster cell holds a depth class (1-21 feet bins). ZIPs whose outline meets no raster have no row: outside the mapped area, which is not the same as no risk.",
        region_notes.join("; ")
    ));
    out.definitions.insert("surge_cat3_share".into(), "Share of the ZIP code's area inside NOAA/NHC's Category 3 storm-surge area (SLOSH Maximum of Maximums at high tide; it contains the Category 1 and 2 areas). Community-level awareness only; official evacuation zones decide who leaves.".into());
    out.definitions.insert(
        "surge_cat1_share".into(),
        "Share of the ZIP code's area inside NOAA/NHC's Category 1 storm-surge area.".into(),
    );
    out.attributions.push(Attribution {
        source: "NOAA NHC storm surge maps".into(),
        text: "Storm-surge areas from NOAA's National Hurricane Center National Storm Surge Risk Maps (Zachry, B. C., Booth, W. J., Rhome, J. R., and Sharon, T. M., 2015: A National View of Storm Surge Risk and Inundation. Weather, Climate, and Society, 7(2), 109-117). For awareness at a community level; check your official evacuation zone.".into(),
        license: "US Government work (public domain)".into(),
        url: PAGE.into(),
        version: Some("v4 (2026) Texas to Maine; v3 and earlier elsewhere".into()),
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn tiny_tiff() -> Vec<u8> {
        // 8 x 4 pixels, one strip per row; the left half inundated (class 3), the right dry (0).
        let mut img = vec![0u8; 32];
        for row in 0..4 {
            for col in 0..4 {
                img[row * 8 + col] = 3;
            }
        }
        let mut buf = Cursor::new(Vec::new());
        let mut enc = tiff::encoder::TiffEncoder::new(&mut buf).unwrap();
        enc.write_image::<tiff::encoder::colortype::Gray8>(8, 4, &img)
            .unwrap();
        buf.into_inner()
    }

    #[test]
    fn world_files_and_pixels() {
        let w = parse_world_file("0.1\n0\n0\n-0.1\n-80.05\n25.35\n").unwrap();
        assert_eq!((w.dx, w.dy, w.x0, w.y0), (0.1, -0.1, -80.05, 25.35));
        assert!(parse_world_file("30\n0\n0\n-30\n500000\n4000000\n").is_err());
        let mut r = Raster::open(Cursor::new(tiny_tiff()), w).unwrap();
        assert_eq!(r.size, (8, 4));
        let b = r.bbox();
        assert!(
            (b[0] - -80.1).abs() < 1e-9 && (b[3] - 25.4).abs() < 1e-9,
            "{b:?}"
        );
        assert_eq!(r.value(-80.05, 25.35).unwrap(), 3); // upper-left cell
        assert_eq!(r.value(-79.35, 25.05).unwrap(), 0); // lower-right cell
        assert_eq!(r.value(-81.0, 25.2).unwrap(), NODATA); // outside
        // A "ZIP" covering the whole raster: half its lattice points are wet.
        let rings = vec![vec![
            [-80.1, 25.0],
            [-79.3, 25.0],
            [-79.3, 25.4],
            [-80.1, 25.4],
            [-80.1, 25.0],
        ]];
        let (wet, total) = sample_zip(&mut r, &rings, 0.05).unwrap();
        assert_eq!(total, 16 * 8);
        assert_eq!(wet, total / 2);
    }

    #[test]
    fn depth_classes() {
        assert!(inundated(1) && inundated(21));
        assert!(!inundated(0) && !inundated(NODATA) && !inundated(22));
    }
}
