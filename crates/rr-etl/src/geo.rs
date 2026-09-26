//! Small geometry toolkit: great-circle distance, point-in-polygon, polygon centroids, distance
//! from a point to a polygon, and ring grouping for GeoJSON output.
//!
//! Coordinates are `[longitude, latitude]` in degrees (NAD83/WGS84; the difference, under two
//! metres, is irrelevant at county scale).

/// Mean Earth radius in kilometres (IUGG).
pub const EARTH_RADIUS_KM: f64 = 6371.0088;

/// Kilometres per statute mile.
pub const KM_PER_MILE: f64 = 1.609344;

/// Kilometres per nautical mile.
pub const KM_PER_NMI: f64 = 1.852;

/// Great-circle distance between two points in kilometres (haversine).
///
/// ```
/// // Philadelphia City Hall to the Salem nuclear site, about 63 km.
/// let d = rr_etl::geo::haversine_km(39.9526, -75.1635, 39.4625, -75.5358);
/// assert!((d - 63.0).abs() < 2.0, "{d}");
/// ```
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = p2 - p1;
    let dl = (lon2 - lon1).to_radians();
    let (sp, sl) = ((dp / 2.0).sin(), (dl / 2.0).sin());
    let a = sp * sp + p1.cos() * p2.cos() * sl * sl;
    2.0 * EARTH_RADIUS_KM * a.sqrt().min(1.0).asin()
}

/// Initial great-circle bearing (forward azimuth) from point 1 to point 2, in degrees clockwise
/// from true north, in `[0, 360)`.
///
/// ```
/// // Due east along the equator is 90 degrees; due north is 0.
/// assert!((rr_etl::geo::bearing_deg(0.0, 0.0, 0.0, 1.0) - 90.0).abs() < 1e-9);
/// assert!(rr_etl::geo::bearing_deg(40.0, -100.0, 41.0, -100.0).abs() < 1e-9);
/// ```
pub fn bearing_deg(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    let b = y.atan2(x).to_degrees();
    let b = if b < 0.0 { b + 360.0 } else { b };
    if b >= 360.0 { 0.0 } else { b }
}

/// True when `bearing` (degrees) lies in the clockwise sector from `from` to `to` (degrees),
/// both ends included. Handles sectors that wrap through north (for example 315 to 45).
pub fn in_sector(bearing: f64, from: f64, to: f64) -> bool {
    let norm = |x: f64| x.rem_euclid(360.0);
    let (b, f, t) = (norm(bearing), norm(from), norm(to));
    if f <= t {
        b >= f && b <= t
    } else {
        b >= f || b <= t
    }
}

/// The 16-point compass name of a bearing in degrees ("north", "north-northeast", ...).
///
/// ```
/// assert_eq!(rr_etl::geo::compass16(0.0), "north");
/// assert_eq!(rr_etl::geo::compass16(292.0), "west-northwest");
/// assert_eq!(rr_etl::geo::compass16(359.0), "north");
/// ```
pub fn compass16(deg: f64) -> &'static str {
    const NAMES: [&str; 16] = [
        "north",
        "north-northeast",
        "northeast",
        "east-northeast",
        "east",
        "east-southeast",
        "southeast",
        "south-southeast",
        "south",
        "south-southwest",
        "southwest",
        "west-southwest",
        "west",
        "west-northwest",
        "northwest",
        "north-northwest",
    ];
    let i = ((deg.rem_euclid(360.0) + 11.25) / 22.5).floor() as usize % 16;
    NAMES[i]
}

/// An Albers equal-area conic projection on an ellipsoid (Snyder 1987, "Map Projections: A
/// Working Manual", USGS Professional Paper 1395, equations 14-3 to 14-12). Used to measure areas
/// in the native plane of layers published in Albers (the USGS karst map), where every lattice
/// cell has the same area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Albers {
    a: f64,
    e: f64,
    lon0: f64,
    n: f64,
    c: f64,
    rho0: f64,
}

impl Albers {
    /// Build from the ellipsoid (semi-major axis in metres, inverse flattening) and the
    /// projection parameters in degrees.
    pub fn new(a: f64, inv_f: f64, lat0: f64, lon0: f64, lat1: f64, lat2: f64) -> Self {
        let f = 1.0 / inv_f;
        let e2 = 2.0 * f - f * f;
        let e = e2.sqrt();
        let m = |phi: f64| {
            let s = phi.to_radians().sin();
            phi.to_radians().cos() / (1.0 - e2 * s * s).sqrt()
        };
        let (m1, m2) = (m(lat1), m(lat2));
        let (q0, q1, q2) = (Self::q(e, lat0), Self::q(e, lat1), Self::q(e, lat2));
        let n = if (lat1 - lat2).abs() < 1e-12 {
            lat1.to_radians().sin()
        } else {
            (m1 * m1 - m2 * m2) / (q2 - q1)
        };
        let c = m1 * m1 + n * q1;
        let rho0 = a * (c - n * q0).sqrt() / n;
        Self {
            a,
            e,
            lon0,
            n,
            c,
            rho0,
        }
    }

    /// The GRS 1980 ellipsoid (NAD 83) with the given parameters in degrees.
    pub fn grs80(lat0: f64, lon0: f64, lat1: f64, lat2: f64) -> Self {
        Self::new(6_378_137.0, 298.257_222_101, lat0, lon0, lat1, lat2)
    }

    fn q(e: f64, phi_deg: f64) -> f64 {
        let s = phi_deg.to_radians().sin();
        let e2 = e * e;
        (1.0 - e2)
            * (s / (1.0 - e2 * s * s)
                - (1.0 / (2.0 * e)) * rr_types::math::ln((1.0 - e * s) / (1.0 + e * s)))
    }

    /// Project a point (degrees) to plane coordinates in metres.
    pub fn forward(&self, lat: f64, lon: f64) -> (f64, f64) {
        let q = Self::q(self.e, lat);
        let rho = self.a * (self.c - self.n * q).max(0.0).sqrt() / self.n;
        let theta = self.n * (lon - self.lon0).to_radians();
        (rho * theta.sin(), self.rho0 - rho * theta.cos())
    }
}

/// Signed planar area of a ring (positive when counter-clockwise).
pub fn ring_signed_area(ring: &[[f64; 2]]) -> f64 {
    let n = ring.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        s += a[0] * b[1] - b[0] * a[1];
    }
    s / 2.0
}

/// Even-odd point-in-ring test.
pub fn point_in_ring(x: f64, y: f64, ring: &[[f64; 2]]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (ring[i][0], ring[i][1]);
        let (xj, yj) = (ring[j][0], ring[j][1]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// A polygon or multipolygon as a flat list of rings (outer rings and holes), with its bounding
/// box. Containment uses the even-odd rule across all rings, which handles holes and islands.
#[derive(Debug, Clone)]
pub struct Poly {
    /// All rings.
    pub rings: Vec<Vec<[f64; 2]>>,
    /// `[min_lon, min_lat, max_lon, max_lat]`.
    pub bbox: [f64; 4],
}

impl Poly {
    /// Build from rings.
    pub fn new(rings: Vec<Vec<[f64; 2]>>) -> Self {
        let mut bbox = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for r in &rings {
            for p in r {
                bbox[0] = bbox[0].min(p[0]);
                bbox[1] = bbox[1].min(p[1]);
                bbox[2] = bbox[2].max(p[0]);
                bbox[3] = bbox[3].max(p[1]);
            }
        }
        Self { rings, bbox }
    }

    /// True if the point is inside the bounding box.
    pub fn bbox_contains(&self, lon: f64, lat: f64) -> bool {
        lon >= self.bbox[0] && lon <= self.bbox[2] && lat >= self.bbox[1] && lat <= self.bbox[3]
    }

    /// Even-odd containment.
    pub fn contains(&self, lon: f64, lat: f64) -> bool {
        if !self.bbox_contains(lon, lat) {
            return false;
        }
        let mut inside = false;
        for r in &self.rings {
            if point_in_ring(lon, lat, r) {
                inside = !inside;
            }
        }
        inside
    }

    /// Distance in km from a point to the polygon (0 when inside), using a local equirectangular
    /// projection centred on the point. Accurate to well under 1% within a few hundred km.
    pub fn distance_km(&self, lon: f64, lat: f64) -> f64 {
        if self.contains(lon, lat) {
            return 0.0;
        }
        let kx = lat.to_radians().cos() * EARTH_RADIUS_KM.to_radians();
        let ky = EARTH_RADIUS_KM.to_radians();
        let mut best = f64::INFINITY;
        for r in &self.rings {
            let n = r.len();
            for i in 0..n {
                let a = r[i];
                let b = r[(i + 1) % n];
                let ax = (a[0] - lon) * kx;
                let ay = (a[1] - lat) * ky;
                let bx = (b[0] - lon) * kx;
                let by = (b[1] - lat) * ky;
                let d = seg_dist_origin(ax, ay, bx, by);
                if d < best {
                    best = d;
                }
            }
        }
        best
    }

    /// Lower bound on the distance (km) from a point to the bounding box; cheap pre-filter.
    pub fn bbox_distance_km(&self, lon: f64, lat: f64) -> f64 {
        let clon = lon.clamp(self.bbox[0], self.bbox[2]);
        let clat = lat.clamp(self.bbox[1], self.bbox[3]);
        haversine_km(lat, lon, clat, clon)
    }

    /// Area-weighted centroid `[lon, lat]` (planar in degrees; fine for counties).
    pub fn centroid(&self) -> Option<[f64; 2]> {
        let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
        for r in &self.rings {
            let n = r.len();
            for i in 0..n {
                let p = r[i];
                let q = r[(i + 1) % n];
                let cross = p[0] * q[1] - q[0] * p[1];
                a += cross;
                cx += (p[0] + q[0]) * cross;
                cy += (p[1] + q[1]) * cross;
            }
        }
        if a.abs() < 1e-12 {
            return None;
        }
        Some([cx / (3.0 * a), cy / (3.0 * a)])
    }
}

/// Distance from the origin to segment AB.
fn seg_dist_origin(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (-(ax * dx + ay * dy) / len2).clamp(0.0, 1.0)
    };
    let (px, py) = (ax + t * dx, ay + t * dy);
    (px * px + py * py).sqrt()
}

/// Distance in km from a point to the segment between two points (local projection).
pub fn point_segment_km(lat: f64, lon: f64, a: [f64; 2], b: [f64; 2]) -> f64 {
    let kx = lat.to_radians().cos() * EARTH_RADIUS_KM.to_radians();
    let ky = EARTH_RADIUS_KM.to_radians();
    // Handle segments that straddle the antimeridian by unwrapping longitudes near the point.
    let unwrap = |x: f64| {
        let mut d = x - lon;
        while d > 180.0 {
            d -= 360.0;
        }
        while d < -180.0 {
            d += 360.0;
        }
        d
    };
    seg_dist_origin(
        unwrap(a[0]) * kx,
        (a[1] - lat) * ky,
        unwrap(b[0]) * kx,
        (b[1] - lat) * ky,
    )
}

/// Group shapefile rings (outer clockwise, holes counter-clockwise) into GeoJSON polygons that
/// follow RFC 7946 winding: exterior counter-clockwise, holes clockwise. Each output polygon is
/// `[exterior, hole, hole, ...]`. Holes are attached to the smallest exterior containing them.
pub fn group_rings_rfc7946(rings: &[Vec<[f64; 2]>]) -> Vec<Vec<Vec<[f64; 2]>>> {
    let mut outers: Vec<(usize, f64)> = Vec::new();
    let mut holes: Vec<usize> = Vec::new();
    for (i, r) in rings.iter().enumerate() {
        let a = ring_signed_area(r);
        if a < 0.0 {
            outers.push((i, -a));
        } else if a > 0.0 {
            holes.push(i);
        }
    }
    // If a file used the opposite convention (no clockwise rings), treat every ring as outer.
    if outers.is_empty() {
        outers = holes
            .drain(..)
            .map(|i| (i, ring_signed_area(&rings[i]).abs()))
            .collect();
    }
    let mut polys: Vec<Vec<Vec<[f64; 2]>>> = outers
        .iter()
        .map(|(i, _)| {
            let mut r = rings[*i].clone();
            if ring_signed_area(&r) < 0.0 {
                r.reverse();
            }
            vec![r]
        })
        .collect();
    for h in holes {
        let p = rings[h][0];
        let mut best: Option<(usize, f64)> = None;
        for (k, (i, area)) in outers.iter().enumerate() {
            if point_in_ring(p[0], p[1], &rings[*i]) && best.is_none_or(|(_, a)| *area < a) {
                best = Some((k, *area));
            }
        }
        if let Some((k, _)) = best {
            let mut r = rings[h].clone();
            if ring_signed_area(&r) > 0.0 {
                r.reverse();
            }
            polys[k].push(r);
        }
    }
    polys
}

/// Round a ring's coordinates to `places` decimals, drop consecutive duplicates, and keep it
/// closed. Returns `None` if fewer than four positions remain or the ring collapses to zero area.
pub fn round_ring(ring: &[[f64; 2]], places: usize) -> Option<Vec<[f64; 2]>> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(ring.len());
    for p in ring {
        let q = [
            crate::num::round_places(p[0], places),
            crate::num::round_places(p[1], places),
        ];
        if out.last() != Some(&q) {
            out.push(q);
        }
    }
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    // Remove immediate back-tracks (a, b, a) that rounding can create.
    let mut changed = true;
    while changed && out.len() >= 3 {
        changed = false;
        let n = out.len();
        for i in 0..n {
            let prev = out[(i + n - 1) % n];
            let next = out[(i + 1) % n];
            if prev == next {
                out.remove(i);
                changed = true;
                break;
            }
        }
    }
    if out.len() < 3 || ring_signed_area(&out).abs() == 0.0 {
        return None;
    }
    out.push(out[0]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f64, y0: f64, s: f64, cw: bool) -> Vec<[f64; 2]> {
        let mut r = vec![
            [x0, y0],
            [x0 + s, y0],
            [x0 + s, y0 + s],
            [x0, y0 + s],
            [x0, y0],
        ];
        if cw {
            r.reverse();
        }
        r
    }

    #[test]
    fn containment_with_hole() {
        let p = Poly::new(vec![
            square(0.0, 0.0, 10.0, true),
            square(4.0, 4.0, 2.0, false),
        ]);
        assert!(p.contains(1.0, 1.0));
        assert!(!p.contains(5.0, 5.0));
        assert!(!p.contains(11.0, 5.0));
    }

    #[test]
    fn centroid_of_square() {
        let p = Poly::new(vec![square(0.0, 0.0, 2.0, true)]);
        let c = p.centroid().unwrap();
        assert!((c[0] - 1.0).abs() < 1e-12 && (c[1] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn distance_to_square() {
        // A 0.1-degree square at the equator; a point 0.1 degrees east of its edge is ~11.1 km away.
        let p = Poly::new(vec![square(0.0, 0.0, 0.1, true)]);
        let d = p.distance_km(0.2, 0.05);
        assert!((d - 11.12).abs() < 0.05, "{d}");
        assert_eq!(p.distance_km(0.05, 0.05), 0.0);
    }

    #[test]
    fn grouping_orients_rings() {
        let polys =
            group_rings_rfc7946(&[square(0.0, 0.0, 10.0, true), square(4.0, 4.0, 2.0, false)]);
        assert_eq!(polys.len(), 1);
        assert!(ring_signed_area(&polys[0][0]) > 0.0);
        assert!(ring_signed_area(&polys[0][1]) < 0.0);
    }

    #[test]
    fn bearings_and_sectors() {
        // Hays, KS lies east-southeast of the F.E. Warren missile field.
        let b = bearing_deg(41.23, -103.85, 38.91, -99.32);
        assert!(in_sector(b, 45.0, 135.0), "{b}");
        // Seen from Hays the field lies to the north-west (305 degrees).
        let back = bearing_deg(38.91, -99.32, 41.23, -103.85);
        assert!((back - 305.2).abs() < 0.5, "{back}");
        assert_eq!(compass16(back), "northwest");
        assert_eq!(
            compass16(bearing_deg(0.0, 0.0, 0.4, -1.0)),
            "west-northwest"
        );
        assert!(in_sector(350.0, 315.0, 45.0));
        assert!(in_sector(10.0, 315.0, 45.0));
        assert!(!in_sector(90.0, 315.0, 45.0));
        assert_eq!(compass16(180.0), "south");
        assert_eq!(compass16(-90.0), "west");
    }

    #[test]
    fn albers_matches_snyders_worked_example() {
        // Snyder (1987) p. 292: Clarke 1866, standard parallels 29.5 and 45.5 N, origin 23 N 96 W;
        // 35 N 75 W projects to x = 1,885,472.7 m, y = 1,535,925.0 m.
        let a = Albers::new(6_378_206.4, 294.978_698_2, 23.0, -96.0, 29.5, 45.5);
        let (x, y) = a.forward(35.0, -75.0);
        assert!(
            (x - 1_885_472.7).abs() < 1.0 && (y - 1_535_925.0).abs() < 1.0,
            "{x} {y}"
        );
        // Equal area: a 1-degree cell at 40 N is about 9,480 km^2 (cos 40 x 111.2^2 x ~1.0).
        let g = Albers::grs80(37.0, -96.0, 25.0, 50.0);
        let p = [(40.0, -100.0), (40.0, -99.0), (41.0, -99.0), (41.0, -100.0)];
        let xy: Vec<(f64, f64)> = p.iter().map(|(la, lo)| g.forward(*la, *lo)).collect();
        let mut area = 0.0;
        for i in 0..4 {
            let (x1, y1) = xy[i];
            let (x2, y2) = xy[(i + 1) % 4];
            area += x1 * y2 - x2 * y1;
        }
        let km2 = area.abs() / 2.0 / 1e6;
        assert!((km2 - 9_380.0).abs() < 150.0, "{km2}");
    }

    #[test]
    fn rounding_drops_tiny_rings() {
        assert!(round_ring(&square(0.0, 0.0, 0.0001, true), 3).is_none());
        assert_eq!(
            round_ring(&square(0.0, 0.0, 1.0, true), 3).unwrap().len(),
            5
        );
    }
}
