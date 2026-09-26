//! Job — strategic exposure: which counties sit at or downwind of the places treated as likely
//! targets in a large nuclear war, and each county's share of FEMA's Urban Area Security
//! Initiative money (a terrorism-risk proxy). Inputs:
//!
//! - the curated site list `crates/rr-etl/data/strategic_sites.toml` (compiled from public sources,
//!   one primary source per site where one exists; see its header): sites, the class rules as data,
//!   metropolitan areas, refineries, ports and the FY2026 UASI table;
//! - Census 2020 county centres of population (where people live, which is what the class rules
//!   measure from); Connecticut's planning regions and the island areas have none and use the
//!   Gazetteer internal point from `core/counties.csv`;
//! - `core/counties.csv` (internal points) and `core/nri_counties.csv` (population, to split an
//!   urban area's UASI share among its counties).
//!
//! Classes, highest first (a county takes the first it qualifies for):
//! - **A** counterforce and command sites: the county holds a missile field's launch facilities,
//!   or contains / has its population centre within 30 km of an A point site;
//! - **C1** the ten largest metros, the National Capital Region, and NNSA weapons-complex sites
//!   (county list or the 30 km rule);
//! - **B** the missile-field fallout corridor: population centre within 800 km of a
//!   launch-facility county at a bearing of 45 to 135 degrees from it, or within 75 km;
//! - **C2** other metros of a million or more, refinery counties of 200,000 b/cd or more, the
//!   principal counties of the ten busiest ports, and the other listed installations;
//! - **D** downwind of a target: within 150 km of an A point site, an NNSA site or a C1 county at a
//!   bearing of 45 to 135 degrees from it;
//! - **E** everything else.
//!
//! The f_S factors in the class table are expert priors (hazard-expansion B1.4) and are carried
//! into the pack labelled `prior = true`; this job does not use them.

use super::{Ctx, JobOutput, load_counties};
use crate::csvout::{Table, col, parse_delimited, read_table};
use crate::geo::{bearing_deg, haversine_km, in_sector};
use crate::manifest::{Attribution, SourceRecord};
use crate::num::{fixed, sig4};
use crate::raster::{CountyGeo, norm_lon};
use crate::{Result, data_err};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// County classes and UASI shares.
pub const STRATEGIC: &str = "core/strategic.csv";
/// The engine-facing site table (classes, templates, sites, areas, sources).
pub const STRATEGIC_SITES: &str = "core/strategic_sites.toml";

/// The curated input, compiled into the binary so a refresh is reproducible.
pub const CURATED: &str = include_str!("../../data/strategic_sites.toml");
/// Where the curated input lives in the repository.
pub const CURATED_PATH: &str = "crates/rr-etl/data/strategic_sites.toml";
const CURATED_URL: &str = "https://github.com/holdTheDoorHoid/ready-reckoner/blob/main/crates/rr-etl/data/strategic_sites.toml";
const CENPOP: &str =
    "https://www2.census.gov/geo/docs/reference/cenpop2020/county/CenPop2020_Mean_CO.txt";

/// The classes in precedence order.
pub const PRECEDENCE: [&str; 6] = ["A", "C1", "B", "C2", "D", "E"];

/// One class definition.
#[derive(Debug, Clone, Deserialize)]
pub struct ClassDef {
    /// `A`, `B`, `C1`, `C2`, `D`, `E`.
    pub id: String,
    /// Plain label.
    pub label: String,
    /// Central factor (expert prior).
    pub f_s: f64,
    /// Low end.
    pub f_s_low: f64,
    /// High end.
    pub f_s_high: f64,
    /// Always true: the factors are priors.
    #[serde(default)]
    pub prior: bool,
    /// The rule in words.
    pub rule: String,
    /// The "Why here" sentence template.
    pub why_here: String,
}

/// A sector rule (`[geometry.B]`, `[geometry.D]`) or a radius rule (`[geometry.A]`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Rule {
    /// Radius for the A and C1 point-site rule, km.
    #[serde(default)]
    pub radius_km: Option<f64>,
    /// Start of the downwind sector, degrees clockwise from north.
    #[serde(default)]
    pub azimuth_from_deg: Option<f64>,
    /// End of the downwind sector.
    #[serde(default)]
    pub azimuth_to_deg: Option<f64>,
    /// Reach of the sector, km.
    #[serde(default)]
    pub distance_km: Option<f64>,
    /// Ring in every direction, km.
    #[serde(default)]
    pub ring_km: Option<f64>,
    /// What distances are measured from.
    #[serde(default)]
    pub measured_from: String,
    /// Citation ids.
    #[serde(default)]
    pub sources: Vec<String>,
}

/// `[geometry]`.
#[derive(Debug, Clone, Deserialize)]
pub struct Geometry {
    /// Point-site radius.
    #[serde(rename = "A")]
    pub a: Rule,
    /// Missile-field corridor.
    #[serde(rename = "B")]
    pub b: Rule,
    /// Downwind of a target.
    #[serde(rename = "D")]
    pub d: Rule,
}

/// A strategic site.
#[derive(Debug, Clone, Deserialize)]
pub struct Site {
    /// Stable id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Main kind (`icbm_field`, `nc3_node`, ...).
    pub kind: String,
    /// Other kinds.
    #[serde(default)]
    pub also: Vec<String>,
    /// `included` or `excluded`.
    pub status: String,
    /// `A`, `C1`, `C2` or `none`.
    pub proposed_class: String,
    /// State(s) in words.
    #[serde(default)]
    pub state: String,
    /// Latitude of the site point.
    #[serde(default)]
    pub lat: Option<f64>,
    /// Longitude.
    #[serde(default)]
    pub lon: Option<f64>,
    /// Where the coordinates come from.
    #[serde(default)]
    pub coord_source: String,
    /// What the site does.
    #[serde(default)]
    pub role: String,
    /// Counties that hold the site (launch-facility counties for a missile field).
    #[serde(default)]
    pub counties: Vec<String>,
    /// How well the sources support it.
    #[serde(default)]
    pub support: String,
    /// Citation ids.
    #[serde(default)]
    pub sources: Vec<String>,
    /// Notes.
    #[serde(default)]
    pub note: String,
}

impl Site {
    fn included(&self) -> bool {
        self.status == "included"
    }
    fn is_field(&self) -> bool {
        self.kind == "icbm_field"
    }
    fn is_nnsa(&self) -> bool {
        self.kind == "doe_weapons_complex" || self.also.iter().any(|k| k == "doe_weapons_complex")
    }
    /// A site whose class reaches beyond its own counties by the 30 km rule: an A or C1 site with
    /// a real point (not a missile field, not the National Capital Region, not a C2 installation).
    fn is_point_site(&self) -> bool {
        self.included()
            && !self.is_field()
            && self.kind != "capital_region"
            && (self.proposed_class == "A" || (self.proposed_class == "C1" && self.is_nnsa()))
            && self.lat.is_some()
            && self.lon.is_some()
    }
    fn point(&self) -> Option<(f64, f64)> {
        Some((self.lat?, self.lon?))
    }
}

/// A metropolitan area of a million or more.
#[derive(Debug, Clone, Deserialize)]
pub struct Metro {
    /// Rank by 2024 population.
    pub rank: u32,
    /// CBSA code.
    pub cbsa: String,
    /// Title.
    pub title: String,
    /// July 2024 population.
    pub pop_2024: u64,
    /// `C1` (top ten) or `C2`.
    pub class: String,
    /// Member counties (2023 delineation).
    pub counties: Vec<String>,
}

/// A refinery.
#[derive(Debug, Clone, Deserialize)]
pub struct Refinery {
    /// Operator.
    pub company: String,
    /// Site name.
    pub site: String,
    /// State in words.
    #[serde(default)]
    pub state: String,
    /// County FIPS.
    pub county: String,
    /// Operating capacity, barrels per calendar day.
    pub bcd_operating: u64,
    /// Idle capacity.
    #[serde(default)]
    pub bcd_idle: u64,
}

/// A port.
#[derive(Debug, Clone, Deserialize)]
pub struct Port {
    /// Rank by 2022 tonnage.
    pub rank: u32,
    /// Name.
    pub name: String,
    /// Million short tons, 2022.
    pub million_short_tons_2022: f64,
    /// Principal counties.
    pub counties: Vec<String>,
}

/// An urban area funded under UASI.
#[derive(Debug, Clone, Deserialize)]
pub struct Uasi {
    /// Rank by FY2026 allocation.
    pub rank: u32,
    /// FEMA's name for the urban area.
    pub urban_area: String,
    /// State in words.
    #[serde(default)]
    pub state: String,
    /// FY2026 allocation, dollars.
    pub usd_fy2026: u64,
    /// Share of the national UASI total.
    pub share: f64,
    /// CBSA mapping in words.
    #[serde(default)]
    pub cbsa_2023: String,
    /// Proposed county footprint.
    pub counties: Vec<String>,
    /// Notes on the mapping.
    #[serde(default)]
    pub mapping_note: String,
}

/// `[uasi_check]`: how the UASI amounts were verified.
#[derive(Debug, Clone, Deserialize)]
pub struct UasiCheck {
    /// Every amount checked against FEMA's PDF.
    pub amounts_verified: bool,
    /// When.
    pub checked: String,
    /// Against what.
    pub against: String,
    /// Whether the county footprints are verified (they are not: FEMA publishes none).
    pub footprint_verified: bool,
    /// Why.
    #[serde(default)]
    pub footprint_note: String,
}

/// A citation.
#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    /// Citation id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Publisher.
    pub publisher: String,
    /// Year.
    pub year: u32,
    /// URL.
    pub url: String,
    /// `fetched`, `search_summary` or `not_retrieved`.
    pub access: String,
    /// Agency, statute or EIS.
    #[serde(default)]
    pub primary: bool,
}

/// The curated file.
#[derive(Debug, Clone, Deserialize)]
pub struct SitesFile {
    /// Schema label.
    pub schema: String,
    /// `reviewed`.
    pub status: String,
    /// Review date.
    #[serde(default)]
    pub reviewed: String,
    /// Class precedence.
    pub precedence: Vec<String>,
    /// Classes.
    pub class: Vec<ClassDef>,
    /// Plain role phrases by kind (or by site id for overrides).
    pub role_plain: BTreeMap<String, String>,
    /// How each template placeholder is filled.
    pub template_rules: BTreeMap<String, String>,
    /// The class rules as data.
    pub geometry: Geometry,
    /// Sites.
    pub site: Vec<Site>,
    /// Metros.
    pub metro: Vec<Metro>,
    /// Refineries.
    pub refinery: Vec<Refinery>,
    /// Ports.
    pub port: Vec<Port>,
    /// UASI verification.
    pub uasi_check: UasiCheck,
    /// UASI areas.
    pub uasi: Vec<Uasi>,
    /// Citations.
    pub source: Vec<Source>,
}

/// Parse and check the curated file: known classes, every cited source present, every UASI share
/// consistent with its dollars.
pub fn parse_sites(text: &str) -> Result<SitesFile> {
    let f: SitesFile = toml::from_str(text)
        .map_err(|e| data_err(format!("{CURATED_PATH} does not parse: {e}")))?;
    if f.precedence != PRECEDENCE {
        return Err(data_err(format!(
            "{CURATED_PATH}: precedence {:?} is not the one this job implements ({PRECEDENCE:?})",
            f.precedence
        )));
    }
    let ids: BTreeSet<&str> = f.source.iter().map(|s| s.id.as_str()).collect();
    for s in &f.site {
        for c in &s.sources {
            if !ids.contains(c.as_str()) {
                return Err(data_err(format!("site {} cites unknown source {c}", s.id)));
            }
        }
        if s.included() && !["A", "C1", "C2"].contains(&s.proposed_class.as_str()) {
            return Err(data_err(format!(
                "site {} has class {}",
                s.id, s.proposed_class
            )));
        }
    }
    let total: u64 = f.uasi.iter().map(|u| u.usd_fy2026).sum();
    for u in &f.uasi {
        let share = u.usd_fy2026 as f64 / total as f64;
        if (share - u.share).abs() > 1e-5 {
            return Err(data_err(format!(
                "UASI {}: share {} but dollars give {share:.5}",
                u.urban_area, u.share
            )));
        }
    }
    Ok(f)
}

/// Why a county qualifies for a class: the site or area id, and the distance and bearing from the
/// county's population centre to the triggering point (`None` for membership: the county is in
/// the metro, holds the launch facilities, and so on).
#[derive(Debug, Clone, PartialEq)]
pub struct Reason {
    /// Site id, `metro_<cbsa>`, `port_<rank>` or `refinery_<fips>`.
    pub id: String,
    /// Distance, km.
    pub km: Option<f64>,
    /// Bearing from the county to the triggering point, degrees.
    pub bearing: Option<f64>,
}

/// The class a county ends up in, with the reasons for that class (membership first, then by
/// distance).
#[derive(Debug, Clone, PartialEq)]
pub struct Assigned {
    /// `A` .. `E`.
    pub class: &'static str,
    /// Reasons for that class.
    pub reasons: Vec<Reason>,
}

/// Points the rules measure from and to.
pub struct Points {
    /// County population centre (or internal point where there is none).
    pub centre: BTreeMap<String, (f64, f64)>,
    /// County internal point (Gazetteer).
    pub internal: BTreeMap<String, (f64, f64)>,
}

fn push(v: &mut Vec<Reason>, id: impl Into<String>, km: Option<f64>, bearing: Option<f64>) {
    v.push(Reason {
        id: id.into(),
        km,
        bearing,
    });
}

fn tidy(mut v: Vec<Reason>) -> Vec<Reason> {
    // Keep each id once (its nearest qualification), membership first, then by distance and id.
    v.sort_by(|a, b| {
        let ka = a.km.unwrap_or(-1.0);
        let kb = b.km.unwrap_or(-1.0);
        ka.total_cmp(&kb).then_with(|| a.id.cmp(&b.id))
    });
    let mut seen = BTreeSet::new();
    v.retain(|r| seen.insert(r.id.clone()));
    v
}

/// Apply the class rules to every county in `points.centre`.
pub fn classify(f: &SitesFile, points: &Points) -> Result<BTreeMap<String, Assigned>> {
    let radius = f.geometry.a.radius_km.unwrap_or(30.0);
    let b_rule = &f.geometry.b;
    let (b_from, b_to) = (
        b_rule.azimuth_from_deg.unwrap_or(45.0),
        b_rule.azimuth_to_deg.unwrap_or(135.0),
    );
    let (b_reach, b_ring) = (
        b_rule.distance_km.unwrap_or(800.0),
        b_rule.ring_km.unwrap_or(75.0),
    );
    let d_rule = &f.geometry.d;
    let (d_from, d_to) = (
        d_rule.azimuth_from_deg.unwrap_or(45.0),
        d_rule.azimuth_to_deg.unwrap_or(135.0),
    );
    let (d_reach, d_ring) = (
        d_rule.distance_km.unwrap_or(150.0),
        d_rule.ring_km.unwrap_or(0.0),
    );
    let included: Vec<&Site> = f.site.iter().filter(|s| s.included()).collect();
    let dist = |a: (f64, f64), b: (f64, f64)| haversine_km(a.0, a.1, b.0, b.1);
    let bear = |a: (f64, f64), b: (f64, f64)| bearing_deg(a.0, a.1, b.0, b.1);

    // Pass 1: A and C1 (D needs the C1 counties).
    let mut qa: BTreeMap<&str, Vec<Reason>> = BTreeMap::new();
    let mut qc1: BTreeMap<&str, Vec<Reason>> = BTreeMap::new();
    for (fips, &pc) in &points.centre {
        let mut a = Vec::new();
        let mut c1 = Vec::new();
        for s in &included {
            let member = s.counties.iter().any(|c| c == fips);
            let target = match s.proposed_class.as_str() {
                "A" => &mut a,
                "C1" => &mut c1,
                _ => continue,
            };
            if member {
                // The county holds the site (or the launch facilities): membership, no distance.
                push(target, &s.id, None, None);
            } else if s.is_point_site()
                && let Some(p) = s.point()
            {
                let d = dist(pc, p);
                if d <= radius {
                    push(target, &s.id, Some(d), Some(bear(pc, p)));
                }
            }
        }
        for m in &f.metro {
            if m.class == "C1" && m.counties.iter().any(|c| c == fips) {
                push(&mut c1, format!("metro_{}", m.cbsa), None, None);
            }
        }
        if !a.is_empty() {
            qa.insert(fips.as_str(), tidy(a));
        }
        if !c1.is_empty() {
            qc1.insert(fips.as_str(), tidy(c1));
        }
    }

    // Sources for the D rule: A point sites, NNSA sites, and every C1 county's centre (labelled
    // with the C1 county's own first reason, so the sentence can name the metro or site).
    let mut d_sources: Vec<(String, (f64, f64))> = Vec::new();
    for s in &included {
        if s.is_point_site()
            && let Some(p) = s.point()
        {
            d_sources.push((s.id.clone(), p));
        }
    }
    for (fips, reasons) in &qc1 {
        if let (Some(p), Some(r)) = (points.centre.get(*fips), reasons.first()) {
            d_sources.push((r.id.clone(), *p));
        }
    }
    // Sources for the B rule: each launch-facility county's internal point, labelled by field.
    let mut b_sources: Vec<(String, (f64, f64))> = Vec::new();
    for s in included.iter().filter(|s| s.is_field()) {
        for c in &s.counties {
            let p = points.internal.get(c).ok_or_else(|| {
                data_err(format!("missile field {} lists unknown county {c}", s.id))
            })?;
            b_sources.push((s.id.clone(), *p));
        }
    }

    let mut out = BTreeMap::new();
    for (fips, &pc) in &points.centre {
        let f_str = fips.as_str();
        if let Some(r) = qa.get(f_str) {
            out.insert(
                fips.clone(),
                Assigned {
                    class: "A",
                    reasons: r.clone(),
                },
            );
            continue;
        }
        if let Some(r) = qc1.get(f_str) {
            out.insert(
                fips.clone(),
                Assigned {
                    class: "C1",
                    reasons: r.clone(),
                },
            );
            continue;
        }
        let mut b = Vec::new();
        for (id, p) in &b_sources {
            let d = dist(*p, pc);
            let from_field = bear(*p, pc);
            if (d <= b_reach && in_sector(from_field, b_from, b_to)) || d <= b_ring {
                push(&mut b, id, Some(d), Some(bear(pc, *p)));
            }
        }
        if !b.is_empty() {
            out.insert(
                fips.clone(),
                Assigned {
                    class: "B",
                    reasons: tidy(b),
                },
            );
            continue;
        }
        let mut c2 = Vec::new();
        for m in &f.metro {
            if m.class == "C2" && m.counties.iter().any(|c| c == fips) {
                push(&mut c2, format!("metro_{}", m.cbsa), None, None);
            }
        }
        let cap: u64 = f
            .refinery
            .iter()
            .filter(|r| &r.county == fips && r.bcd_operating + r.bcd_idle >= 200_000)
            .map(|r| r.bcd_operating + r.bcd_idle)
            .sum();
        if cap > 0 {
            push(&mut c2, format!("refinery_{fips}"), None, None);
        }
        for p in &f.port {
            if p.counties.iter().any(|c| c == fips) {
                push(&mut c2, format!("port_{}", p.rank), None, None);
            }
        }
        for s in included.iter().filter(|s| s.proposed_class == "C2") {
            if s.counties.iter().any(|c| c == fips) {
                push(&mut c2, &s.id, None, None);
            }
        }
        if !c2.is_empty() {
            out.insert(
                fips.clone(),
                Assigned {
                    class: "C2",
                    reasons: tidy(c2),
                },
            );
            continue;
        }
        let mut d = Vec::new();
        for (id, p) in &d_sources {
            let km = dist(*p, pc);
            if (km <= d_reach && in_sector(bear(*p, pc), d_from, d_to)) || km <= d_ring {
                push(&mut d, id, Some(km), Some(bear(pc, *p)));
            }
        }
        if !d.is_empty() {
            out.insert(
                fips.clone(),
                Assigned {
                    class: "D",
                    reasons: tidy(d),
                },
            );
            continue;
        }
        out.insert(
            fips.clone(),
            Assigned {
                class: "E",
                reasons: Vec::new(),
            },
        );
    }
    Ok(out)
}

/// A county's part of the FY2026 UASI allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UasiCounty {
    /// The county's share of the national total: its urban area's share split among the area's
    /// counties by population.
    pub share: f64,
    /// The urban area's own share of the national total (the same for every county in it).
    pub area_share: f64,
    /// Rank of the urban area in `strategic_sites.toml`.
    pub rank: u32,
}

/// Each county's share of the national UASI total (area share split by population) and the rank
/// of the urban area it falls in.
pub fn uasi_shares(
    f: &SitesFile,
    population: &BTreeMap<String, f64>,
) -> Result<BTreeMap<String, UasiCounty>> {
    let mut out: BTreeMap<String, UasiCounty> = BTreeMap::new();
    // Shares from the dollars (the file's `share` column is rounded to five decimals).
    let national: f64 = f.uasi.iter().map(|u| u.usd_fy2026 as f64).sum();
    for u in &f.uasi {
        let area_share = u.usd_fy2026 as f64 / national;
        let pops: Vec<(&String, f64)> = u
            .counties
            .iter()
            .map(|c| {
                population.get(c).map(|p| (c, *p)).ok_or_else(|| {
                    data_err(format!("UASI {}: no population for {c}", u.urban_area))
                })
            })
            .collect::<Result<_>>()?;
        let total: f64 = pops.iter().map(|(_, p)| *p).sum();
        if total <= 0.0 {
            return Err(data_err(format!("UASI {}: zero population", u.urban_area)));
        }
        for (c, p) in pops {
            // A county funded under two urban areas would make its area share ambiguous.
            if let Some(prev) = out.get(c) {
                return Err(data_err(format!(
                    "UASI: county {c} is in urban areas ranked {} and {}",
                    prev.rank, u.rank
                )));
            }
            out.insert(
                c.clone(),
                UasiCounty {
                    share: area_share * p / total,
                    area_share,
                    rank: u.rank,
                },
            );
        }
    }
    Ok(out)
}

fn q(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

fn q_list(v: &[String]) -> String {
    format!(
        "[{}]",
        v.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")
    )
}

/// Whether a site's `support` note says its role rests (at least partly) on a secondary or
/// unchecked source. Clauses about coordinates ("coordinates secondary") do not count: those are
/// flagged from `coord_source`.
pub fn role_rests_on_secondary(support: &str) -> bool {
    support
        .split(';')
        .map(str::trim)
        .filter(|c| !c.starts_with("coordinates"))
        .any(|c| c.contains("secondary") || c.contains("not checked"))
}

/// Write the engine-facing TOML: classes, templates, the rules as numbers, included sites,
/// metros, ports, refineries, UASI areas and the sources they cite. Deterministic.
pub fn pack_toml(f: &SitesFile, sha: &str) -> (String, u64) {
    let mut s = String::new();
    // Rows are the entries of every top-level array, as `verify` counts them.
    let mut rows = f.precedence.len() as u64;
    s.push_str("# Strategic sites for the nuclear family and the terrorism tier (Ready Reckoner data pack).\n");
    s.push_str(&format!(
        "# Written by the rr-etl `strategic` job from {CURATED_PATH} (sha256 {sha}); do not edit here.\n"
    ));
    s.push_str("# The f_s factors are expert priors (prior = true): show them only as ranges.\n\n");
    s.push_str(&format!("schema = {}\n", q(&f.schema)));
    s.push_str(&format!(
        "status = {}\nreviewed = {}\n",
        q(&f.status),
        q(&f.reviewed)
    ));
    s.push_str(&format!("precedence = {}\n", q_list(&f.precedence)));
    s.push_str("county_point = \"Census 2020 county centre of population (Gazetteer 2024 internal point where the Census publishes none)\"\n\n");
    for c in &f.class {
        rows += 1;
        s.push_str("[[class]]\n");
        s.push_str(&format!("id = {}\nlabel = {}\n", q(&c.id), q(&c.label)));
        s.push_str(&format!(
            "f_s = {}\nf_s_low = {}\nf_s_high = {}\nprior = {}\n",
            sig4(c.f_s),
            sig4(c.f_s_low),
            sig4(c.f_s_high),
            c.prior
        ));
        s.push_str(&format!(
            "rule = {}\nwhy_here = {}\n\n",
            q(&c.rule),
            q(&c.why_here)
        ));
    }
    s.push_str("[role_plain]\n");
    for (k, v) in &f.role_plain {
        s.push_str(&format!("{k} = {}\n", q(v)));
    }
    s.push_str("\n[template_rules]\n");
    for (k, v) in &f.template_rules {
        s.push_str(&format!("{k} = {}\n", q(v)));
    }
    for (name, r) in [
        ("A", &f.geometry.a),
        ("B", &f.geometry.b),
        ("D", &f.geometry.d),
    ] {
        s.push_str(&format!("\n[geometry.{name}]\n"));
        if let Some(v) = r.radius_km {
            s.push_str(&format!("radius_km = {}\n", sig4(v)));
        }
        if let (Some(a), Some(b)) = (r.azimuth_from_deg, r.azimuth_to_deg) {
            s.push_str(&format!(
                "azimuth_from_deg = {}\nazimuth_to_deg = {}\n",
                sig4(a),
                sig4(b)
            ));
        }
        if let Some(v) = r.distance_km {
            s.push_str(&format!("distance_km = {}\n", sig4(v)));
        }
        if let Some(v) = r.ring_km {
            s.push_str(&format!("ring_km = {}\n", sig4(v)));
        }
        s.push_str(&format!(
            "measured_from = {}\nsources = {}\n",
            q(&r.measured_from),
            q_list(&r.sources)
        ));
    }
    let access: BTreeMap<&str, &Source> = f.source.iter().map(|x| (x.id.as_str(), x)).collect();
    let mut cited: BTreeSet<String> = BTreeSet::new();
    for r in [&f.geometry.a, &f.geometry.b, &f.geometry.d] {
        cited.extend(r.sources.iter().cloned());
    }
    for site in f.site.iter().filter(|x| x.included()) {
        rows += 1;
        s.push_str("\n[[site]]\n");
        s.push_str(&format!(
            "id = {}\nname = {}\nkind = {}\n",
            q(&site.id),
            q(&site.name),
            q(&site.kind)
        ));
        if !site.also.is_empty() {
            s.push_str(&format!("also = {}\n", q_list(&site.also)));
        }
        s.push_str(&format!(
            "class = {}\nstate = {}\n",
            q(&site.proposed_class),
            q(&site.state)
        ));
        if let (Some(lat), Some(lon)) = (site.lat, site.lon) {
            s.push_str(&format!(
                "lat = {}\nlon = {}\n",
                fixed(lat, 4),
                fixed(lon, 4)
            ));
        }
        let rule = if site.is_point_site() {
            "point_30km"
        } else {
            "county_list"
        };
        s.push_str(&format!("rule = {}\n", q(rule)));
        s.push_str(&format!("counties = {}\n", q_list(&site.counties)));
        let plain = f
            .role_plain
            .get(&site.id)
            .or_else(|| f.role_plain.get(&site.kind))
            .cloned()
            .unwrap_or_default();
        s.push_str(&format!(
            "role_plain = {}\nrole = {}\n",
            q(&plain),
            q(&site.role)
        ));
        s.push_str(&format!(
            "support = {}\nsources = {}\n",
            q(&site.support),
            q_list(&site.sources)
        ));
        let mut unverified: Vec<String> = site
            .sources
            .iter()
            .filter_map(|id| access.get(id.as_str()))
            .filter(|x| x.access != "fetched")
            .map(|x| format!("{}: {}", x.id, x.access))
            .collect();
        if site.coord_source.contains("secondary") {
            unverified.push("coordinates: secondary (not an agency point)".to_string());
        }
        if role_rests_on_secondary(&site.support) {
            unverified.push("role: rests on a secondary source".to_string());
        }
        s.push_str(&format!("unverified = {}\n", q_list(&unverified)));
        if !site.note.is_empty() {
            s.push_str(&format!("note = {}\n", q(&site.note)));
        }
        cited.extend(site.sources.iter().cloned());
    }
    for m in &f.metro {
        rows += 1;
        s.push_str("\n[[metro]]\n");
        s.push_str(&format!(
            "id = {}\ncbsa = {}\nrank = {}\ntitle = {}\npop_2024 = {}\nclass = {}\n",
            q(&format!("metro_{}", m.cbsa)),
            q(&m.cbsa),
            m.rank,
            q(&m.title),
            m.pop_2024,
            q(&m.class)
        ));
    }
    cited.insert("census_cbsa_pop_2024".into());
    cited.insert("census_delineation_2023".into());
    for p in &f.port {
        rows += 1;
        s.push_str("\n[[port]]\n");
        s.push_str(&format!(
            "id = {}\nrank = {}\nname = {}\nmillion_short_tons_2022 = {}\ncounties = {}\n",
            q(&format!("port_{}", p.rank)),
            p.rank,
            q(&p.name),
            sig4(p.million_short_tons_2022),
            q_list(&p.counties)
        ));
    }
    cited.insert("bts_ports_2025".into());
    let mut by_county: BTreeMap<&str, Vec<&Refinery>> = BTreeMap::new();
    for r in &f.refinery {
        if r.bcd_operating + r.bcd_idle >= 200_000 {
            by_county.entry(r.county.as_str()).or_default().push(r);
        }
    }
    for (county, list) in &by_county {
        rows += 1;
        let total: u64 = list.iter().map(|r| r.bcd_operating + r.bcd_idle).sum();
        let sites: Vec<String> = list
            .iter()
            .map(|r| {
                format!(
                    "{} ({}), {} b/cd",
                    r.company,
                    r.site,
                    r.bcd_operating + r.bcd_idle
                )
            })
            .collect();
        s.push_str("\n[[refinery]]\n");
        s.push_str(&format!(
            "id = {}\ncounty = {}\nbarrels_per_calendar_day = {}\nsites = {}\n",
            q(&format!("refinery_{county}")),
            q(county),
            total,
            q_list(&sites)
        ));
    }
    cited.insert("eia_refcap_2026".into());
    s.push_str("\n[uasi_check]\n");
    s.push_str(&format!(
        "amounts_verified = {}\nchecked = {}\nagainst = {}\nfootprint_verified = {}\nfootprint_note = {}\n",
        f.uasi_check.amounts_verified,
        q(&f.uasi_check.checked),
        q(&f.uasi_check.against),
        f.uasi_check.footprint_verified,
        q(&f.uasi_check.footprint_note)
    ));
    for u in &f.uasi {
        rows += 1;
        s.push_str("\n[[uasi]]\n");
        s.push_str(&format!(
            "rank = {}\nurban_area = {}\nstate = {}\nusd_fy2026 = {}\nshare = {}\ncounties = {}\n",
            u.rank,
            q(&u.urban_area),
            q(&u.state),
            u.usd_fy2026,
            sig4(u.share),
            q_list(&u.counties)
        ));
        if !u.mapping_note.is_empty() {
            s.push_str(&format!("mapping_note = {}\n", q(&u.mapping_note)));
        }
    }
    cited.insert("fema_hsgp_fy2026".into());
    for src in f.source.iter().filter(|x| cited.contains(&x.id)) {
        rows += 1;
        s.push_str("\n[[source]]\n");
        s.push_str(&format!(
            "id = {}\ntitle = {}\npublisher = {}\nyear = {}\nurl = {}\naccess = {}\nprimary = {}\n",
            q(&src.id),
            q(&src.title),
            q(&src.publisher),
            src.year,
            q(&src.url),
            q(&src.access),
            src.primary
        ));
    }
    (s, rows)
}

/// Census 2020 county centres of population, keyed by five-digit FIPS.
pub fn parse_cenpop(text: &str) -> Result<BTreeMap<String, (f64, f64)>> {
    let (h, rows) = parse_delimited(text, b',')?;
    let (is, ic, ila, ilo) = (
        col(&h, "STATEFP")?,
        col(&h, "COUNTYFP")?,
        col(&h, "LATITUDE")?,
        col(&h, "LONGITUDE")?,
    );
    let mut out = BTreeMap::new();
    for r in &rows {
        let (Ok(lat), Ok(lon)) = (r[ila].parse::<f64>(), r[ilo].parse::<f64>()) else {
            continue;
        };
        out.insert(format!("{}{}", r[is], r[ic]), (lat, lon));
    }
    Ok(out)
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let sites = parse_sites(CURATED)?;
    let sha = crate::http::sha256_hex(CURATED.as_bytes());
    out.source(SourceRecord {
        name: "Ready Reckoner curated strategic sites (sites, class rules, metros, refineries, ports, FY2026 UASI table)".into(),
        url: CURATED_URL.into(),
        version: format!("{} ({}), reviewed {}", sites.schema, sites.status, sites.reviewed),
        retrieved: crate::timefmt::now_utc(),
        sha256: sha.clone(),
        bytes: CURATED.len() as u64,
        license: "Compiled by Ready Reckoner from the public sources listed in the file".into(),
        obligations: String::new(),
    });
    out.source(SourceRecord {
        name: "FEMA FY2026 Homeland Security Grant Program NOFO, Appendix I: UASI allocations".into(),
        url: "https://www.fema.gov/sites/default/files/documents/fema_gpd_hsgp-nofo-fy2026.pdf".into(),
        version: "figures transcribed into the curated file and checked row by row (fema.gov refuses scripted clients); pp. 71-73".into(),
        retrieved: "2026-09-26T00:00:00Z".into(),
        sha256: "b94875508f001bf3ff5d37b6fb3a5934941177bc4f41b7f7a7f24f15161cd9bd".into(),
        bytes: 0,
        license: super::PUBLIC_DOMAIN.into(),
        obligations: String::new(),
    });
    out.source(SourceRecord {
        name: "Sentinel (GBSD) Final EIS, Volume I: missile-field county lists".into(),
        url: "https://minutemanmissile.com/documents/SentinelFinalEISVolumeIFinalEIS.pdf".into(),
        version: "county lists checked against sections 2.1.6.1 (p. 2-13), 2.1.7.1 (p. 2-33), 2.1.8.1 (p. 2-40); mirror of the AFGSC document".into(),
        retrieved: "2026-09-26T00:00:00Z".into(),
        sha256: String::new(),
        bytes: 101_019_598,
        license: super::PUBLIC_DOMAIN.into(),
        obligations: String::new(),
    });

    let cen = ctx
        .http
        .get(CENPOP, Some("strategic/CenPop2020_Mean_CO.txt"))?;
    out.source(super::source_from(
        "Census 2020 centers of population by county",
        &cen,
        "CenPop2020_Mean_CO",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let cenpop = parse_cenpop(&cen.text())?;
    out.rows_in += cenpop.len() as u64;
    let mut points = Points {
        centre: BTreeMap::new(),
        internal: BTreeMap::new(),
    };
    let mut fallback = Vec::new();
    for c in &counties {
        points.internal.insert(c.fips.clone(), (c.lat, c.lon));
        match cenpop.get(&c.fips) {
            Some(p) => {
                points.centre.insert(c.fips.clone(), *p);
            }
            None => {
                points.centre.insert(c.fips.clone(), (c.lat, c.lon));
                fallback.push(c.fips.clone());
            }
        }
    }
    for s in sites.site.iter().filter(|s| s.included()) {
        for c in &s.counties {
            if !canon.contains(c) {
                return Err(data_err(format!(
                    "site {} lists county {c}, not in the county list",
                    s.id
                )));
            }
        }
    }
    let listed = sites
        .metro
        .iter()
        .flat_map(|m| {
            m.counties
                .iter()
                .map(move |c| (format!("metro {}", m.cbsa), c))
        })
        .chain(sites.port.iter().flat_map(|p| {
            p.counties
                .iter()
                .map(move |c| (format!("port {}", p.name), c))
        }))
        .chain(
            sites
                .refinery
                .iter()
                .map(|r| (format!("refinery {}", r.site), &r.county)),
        )
        .chain(sites.uasi.iter().flat_map(|u| {
            u.counties
                .iter()
                .map(move |c| (format!("UASI {}", u.urban_area), c))
        }));
    for (what, c) in listed {
        if !canon.contains(c) {
            return Err(data_err(format!(
                "{what} lists county {c}, not in the county list"
            )));
        }
    }

    let assigned = classify(&sites, &points)?;

    // Population, for the UASI split and the class summary.
    let (h, rows) = read_table(&ctx.data, super::nri::NRI_COUNTIES)?;
    let (i_f, i_p) = (col(&h, "fips")?, col(&h, "population")?);
    let population: BTreeMap<String, f64> = rows
        .iter()
        .filter_map(|r| Some((r[i_f].clone(), r[i_p].parse::<f64>().ok()?)))
        .collect();
    let uasi = uasi_shares(&sites, &population)?;

    let mut table = Table::new(
        &[
            "fips",
            "strategic_class",
            "strategic_site_ids",
            "strategic_km",
            "strategic_bearing",
            "uasi_share",
            "uasi_area_share",
            "uasi_area",
        ],
        1,
    );
    let mut counts: BTreeMap<&str, (u32, f64)> = BTreeMap::new();
    let total_pop: f64 = population.values().sum();
    for c in &counties {
        let a = &assigned[&c.fips];
        let first = a.reasons.first();
        let (km, bearing) = match first {
            Some(Reason {
                km: Some(k),
                bearing: Some(b),
                ..
            }) => (fixed(*k, 1), format!("{:.0}", b.round().rem_euclid(360.0))),
            _ => (String::new(), String::new()),
        };
        let (share, area_share, area) = uasi
            .get(&c.fips)
            .map(|u| (sig4(u.share), sig4(u.area_share), u.rank.to_string()))
            .unwrap_or_else(|| ("0".to_string(), "0".to_string(), String::new()));
        table.push(vec![
            c.fips.clone(),
            a.class.to_string(),
            a.reasons
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>()
                .join(";"),
            km,
            bearing,
            share,
            area_share,
            area,
        ]);
        let e = counts.entry(a.class).or_default();
        e.0 += 1;
        e.1 += population.get(&c.fips).copied().unwrap_or(0.0);
    }

    // Sanity anchors (the brief's fixtures): a wrong rule or a broken list stops the job.
    for (fips, want, place) in [
        ("38101", "A", "Ward County, ND (Minot)"),
        ("20051", "B", "Ellis County, KS (Hays)"),
        ("41011", "E", "Coos County, OR (Coos Bay)"),
        ("12086", "C1", "Miami-Dade County, FL"),
        ("04013", "C1", "Maricopa County, AZ (Phoenix)"),
        ("42101", "C1", "Philadelphia"),
        ("17031", "C1", "Cook County, IL (Chicago)"),
        ("48157", "C1", "Fort Bend County, TX (Sugar Land)"),
        ("53035", "A", "Kitsap County, WA (Bangor)"),
    ] {
        let got = assigned.get(fips).map(|a| a.class).unwrap_or("none");
        if got != want {
            return Err(data_err(format!(
                "strategic class for {place} is {got}, expected {want}: check the rules or the site list"
            )));
        }
    }
    let n_a = counts.get("A").map(|x| x.0).unwrap_or(0);
    if !(40..=120).contains(&n_a) {
        return Err(data_err(format!(
            "{n_a} class A counties; expected about 60"
        )));
    }
    out.table(ctx, STRATEGIC, &mut table)?;
    let (text, rows_toml) = pack_toml(&sites, &sha);
    out.text(ctx, STRATEGIC_SITES, &text, rows_toml)?;

    // Counties whose land reaches within 30 km of an A point site while their centre does not:
    // listed for review (the ZIP-level distance in zip_facilities.csv refines them).
    let (geo, cb_source) =
        CountyGeo::download(ctx, &canon, "strategic/cb_2024_us_county_500k.zip")?;
    out.source(cb_source);
    let mut touch: Vec<String> = Vec::new();
    for s in sites
        .site
        .iter()
        .filter(|s| s.is_point_site() && s.proposed_class == "A")
    {
        let Some((lat, lon)) = s.point() else {
            continue;
        };
        for (fips, poly) in &geo.polys {
            if assigned[fips].class == "A" {
                continue;
            }
            let lonn = norm_lon(lon);
            if poly.bbox_distance_km(lonn, lat) > 30.0 {
                continue;
            }
            if poly.distance_km(lonn, lat) <= 30.0 {
                touch.push(format!("{fips} ({}, now {})", s.id, assigned[fips].class));
            }
        }
    }
    touch.sort();
    touch.dedup();

    let summary: Vec<String> = PRECEDENCE
        .iter()
        .map(|k| {
            let (n, p) = counts.get(k).copied().unwrap_or_default();
            format!("{k} {n} ({:.1}% of people)", 100.0 * p / total_pop.max(1.0))
        })
        .collect();
    out.notes.push(format!(
        "Classes by county (population from NRI 2020): {}. The research first cut (internal points) gave A 60, B 537, C1 129, C2 332, D 245, E 1929.",
        summary.join(", ")
    ));
    out.notes.push(format!(
        "Distances are from each county's Census 2020 centre of population (the rules measure where people live). {} counties have no 2020 centre in the Census file and use their Gazetteer 2024 internal point: {}.",
        fallback.len(),
        fallback.join(", ")
    ));
    out.notes.push("Class B distances are measured from the Gazetteer internal point of each county holding launch facilities (23 counties; a stand-in for the 450 launch facilities, whose coordinates no public source reached here provides). The missile-field county lists match the Sentinel Final EIS Volume I (sections 2.1.6.1, 2.1.7.1, 2.1.8.1).".into());
    out.notes.push(format!(
        "Counties whose land comes within 30 km of an A point site while their centre of population does not (for review; zip_facilities.csv gives each ZIP's own distance): {}.",
        if touch.is_empty() { "none".to_string() } else { touch.join("; ") }
    ));
    out.notes.push("strategic_site_ids: the sites or areas that put the county in its class, membership first (the county is in the metro, holds the launch facilities, and so on), then by distance; ids resolve in strategic_sites.toml (site ids, metro_<cbsa>, port_<rank>, refinery_<county>). strategic_km and strategic_bearing: distance (km, 1 decimal) and compass bearing (degrees clockwise from north) from the county's centre of population to the first distance-based reason, empty for membership and for class E.".into());
    out.notes.push(format!(
        "uasi_share: the county's share of the national FY2026 UASI total ($584,250,000, 44 urban areas): the urban area's share split among its counties by 2020 population; 0 outside every funded urban area. uasi_area_share: the urban area's own share of that total, the same for every county in the area (0 outside). uasi_area: rank of that urban area in strategic_sites.toml. Amounts checked against FEMA's PDF ({}); county footprints are a proposal (UNVERIFIED: FEMA publishes none).",
        sites.uasi_check.checked
    ));
    out.definitions.insert("uasi_area_share".into(), "The FEMA urban area's share of the national FY2026 Urban Area Security Initiative allocation (its dollars / $584,250,000), the same for every county in the urban area; 0 outside every funded urban area. The county footprints are Ready Reckoner's proposal, not a FEMA list.".into());
    out.definitions.insert("strategic_class".into(), "A: counterforce and command sites; C1: largest cities, the capital region and weapons plants; B: missile-field fallout corridor; C2: other large cities, ports, refineries and bases; D: downwind of a target; E: remote from targets and fallout paths. Rules and f_S priors in strategic_sites.toml.".into());
    out.definitions.insert("precedent".into(), "FEMA, Protection in the Nuclear Age (H-20, 1985), p. 12: \"Designating a place as a 'risk' area does not mean that it will be attacked; it does indicate a greater potential for attack.\" NAPB-90 (1987, released 2005) is a method precedent, not a public one.".into());
    out.attributions.push(Attribution {
        source: "Strategic sites".into(),
        text: "Strategic-site classes compiled by Ready Reckoner from public sources: DoD MIRTA installation points, the Sentinel Final EIS (Department of the Air Force), NNSA, the Missile Defense Agency, Census metropolitan areas and centres of population, EIA refinery capacity, BTS port statistics and FEMA's FY2026 UASI allocations. A higher class does not mean an attack is likely.".into(),
        license: "Compiled from public sources (US Government works are public domain)".into(),
        url: CURATED_URL.into(),
        version: Some(format!("reviewed {}", sites.reviewed)),
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curated_file_parses_and_checks() {
        let f = parse_sites(CURATED).unwrap();
        assert_eq!(f.class.len(), 6);
        let included = f.site.iter().filter(|s| s.included()).count();
        assert_eq!(included, 39);
        let fields: Vec<usize> = f
            .site
            .iter()
            .filter(|s| s.is_field())
            .map(|s| s.counties.len())
            .collect();
        assert_eq!(
            fields,
            vec![8, 8, 7],
            "Sentinel FEIS Vol I: 8 + 8 + 7 launch-facility counties"
        );
        assert_eq!(f.uasi.len(), 44);
        assert_eq!(
            f.uasi.iter().map(|u| u.usd_fy2026).sum::<u64>(),
            584_250_000
        );
        assert!(f.uasi_check.amounts_verified && !f.uasi_check.footprint_verified);
        // Point sites: A sites with a point plus the NNSA C1 sites; not fields or the capital region.
        let points: BTreeSet<&str> = f
            .site
            .iter()
            .filter(|s| s.is_point_site())
            .map(|s| s.id.as_str())
            .collect();
        assert!(
            points.contains("kitsap_bangor") && points.contains("y12") && points.contains("pantex")
        );
        assert!(!points.contains("minot_field") && !points.contains("national_capital_region"));
        assert!(!points.contains("dyess_afb"));
    }

    fn toy_points() -> Points {
        let mut centre = BTreeMap::new();
        let mut internal = BTreeMap::new();
        for (f, lat, lon) in [
            ("38101", 48.23, -101.54), // Ward, ND (holds Minot field and base)
            ("20051", 38.91, -99.32),  // Ellis, KS (Hays)
            ("41011", 43.2, -124.1),   // Coos, OR
            ("12086", 25.61, -80.50),  // Miami-Dade
            ("56021", 41.31, -104.69), // Laramie, WY (Warren field)
        ] {
            centre.insert(f.to_string(), (lat, lon));
            internal.insert(f.to_string(), (lat, lon));
        }
        Points { centre, internal }
    }

    #[test]
    fn classes_follow_the_rules_on_toy_points() {
        let mut f = parse_sites(CURATED).unwrap();
        // Keep only the counties the toy knows about in the field lists.
        for s in &mut f.site {
            if s.is_field() {
                s.counties.retain(|c| c == "38101" || c == "56021");
            }
        }
        let a = classify(&f, &toy_points()).unwrap();
        assert_eq!(a["38101"].class, "A");
        assert!(
            a["38101"]
                .reasons
                .iter()
                .any(|r| r.id == "minot_field" && r.km.is_none())
        );
        assert_eq!(a["20051"].class, "B");
        let r = &a["20051"].reasons[0];
        assert_eq!(r.id, "warren_field");
        // In this toy the only Warren field county is Laramie, WY: about 530 km to the
        // west-northwest (the real run measures to the nearest of the seven field counties).
        assert!((480.0..580.0).contains(&r.km.unwrap()), "{r:?}");
        assert_eq!(crate::geo::compass16(r.bearing.unwrap()), "west-northwest");
        assert_eq!(a["41011"].class, "E");
        assert_eq!(a["12086"].class, "C1");
        assert_eq!(a["12086"].reasons[0].id, "metro_33100");
    }

    #[test]
    fn uasi_shares_split_by_population() {
        let f = parse_sites(CURATED).unwrap();
        let mut pop = BTreeMap::new();
        for u in &f.uasi {
            for c in &u.counties {
                pop.insert(c.clone(), 1000.0);
            }
        }
        let s = uasi_shares(&f, &pop).unwrap();
        let total: f64 = s.values().map(|x| x.share).sum();
        assert!((total - 1.0).abs() < 1e-9, "{total}");
        // San Diego is one county: it carries its area's whole share.
        let sd = s["06073"];
        assert!((sd.share - 16_266_915.0 / 584_250_000.0).abs() < 1e-9);
        assert_eq!((sd.area_share, sd.rank), (sd.share, 9));
        // New York-White Plains: every county carries the area's whole share as `area_share`
        // and a population part of it as `share`.
        let nyc = 142_481_143.0 / 584_250_000.0;
        let ny: Vec<&UasiCounty> = s.values().filter(|u| u.rank == 1).collect();
        assert_eq!(ny.len(), 10);
        assert!(ny.iter().all(|u| (u.area_share - nyc).abs() < 1e-12));
        let parts: f64 = ny.iter().map(|u| u.share).sum();
        assert!((parts - nyc).abs() < 1e-12, "{parts}");
    }

    #[test]
    fn pack_toml_parses_back() {
        let f = parse_sites(CURATED).unwrap();
        let (text, rows) = pack_toml(&f, "abc");
        let v: toml::Table = text.parse().unwrap();
        assert_eq!(v["class"].as_array().unwrap().len(), 6);
        assert_eq!(v["site"].as_array().unwrap().len(), 39);
        let bangor = v["site"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"].as_str() == Some("kitsap_bangor"))
            .unwrap();
        assert_eq!(bangor["rule"].as_str(), Some("point_30km"));
        assert!(
            bangor["role_plain"]
                .as_str()
                .unwrap()
                .contains("submarines")
        );
        let counted: u64 = v
            .values()
            .map(|x| x.as_array().map_or(0, |a| a.len() as u64))
            .sum();
        assert_eq!(counted, rows);
        // Y-12's role rests on the NNSA page (read in full); only its coordinates are secondary.
        let y12 = v["site"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"].as_str() == Some("y12"))
            .unwrap();
        let flags: Vec<&str> = y12["unverified"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        assert_eq!(flags, ["coordinates: secondary (not an agency point)"]);
    }

    #[test]
    fn coordinate_clauses_do_not_mark_a_role_secondary() {
        assert!(!role_rests_on_secondary(
            "primary (NNSA locations page, read in full 2026-09-26); coordinates secondary"
        ));
        assert!(role_rests_on_secondary(
            "primary (NNSA); storage secondary (FAS 2025); coordinates secondary"
        ));
        assert!(role_rests_on_secondary(
            "primary location (MIRTA); role secondary (FAS 2025)"
        ));
        assert!(!role_rests_on_secondary(
            "primary location (MIRTA); role primary (MDA, search summary)"
        ));
    }
}
