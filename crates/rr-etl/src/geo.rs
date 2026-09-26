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
        let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
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
    let t = if len2 == 0.0 { 0.0 } else { (-(ax * dx + ay * dy) / len2).clamp(0.0, 1.0) };
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
    seg_dist_origin(unwrap(a[0]) * kx, (a[1] - lat) * ky, unwrap(b[0]) * kx, (b[1] - lat) * ky)
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
        outers = holes.drain(..).map(|i| (i, ring_signed_area(&rings[i]).abs())).collect();
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
        let q = [crate::num::round_places(p[0], places), crate::num::round_places(p[1], places)];
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
        let mut r = vec![[x0, y0], [x0 + s, y0], [x0 + s, y0 + s], [x0, y0 + s], [x0, y0]];
        if cw {
            r.reverse();
        }
        r
    }

    #[test]
    fn containment_with_hole() {
        let p = Poly::new(vec![square(0.0, 0.0, 10.0, true), square(4.0, 4.0, 2.0, false)]);
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
        let polys = group_rings_rfc7946(&[square(0.0, 0.0, 10.0, true), square(4.0, 4.0, 2.0, false)]);
        assert_eq!(polys.len(), 1);
        assert!(ring_signed_area(&polys[0][0]) > 0.0);
        assert!(ring_signed_area(&polys[0][1]) < 0.0);
    }

    #[test]
    fn rounding_drops_tiny_rings() {
        assert!(round_ring(&square(0.0, 0.0, 0.0001, true), 3).is_none());
        assert_eq!(round_ring(&square(0.0, 0.0, 1.0, true), 3).unwrap().len(), 5);
    }
}
