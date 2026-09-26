//! A minimal reader for ESRI shapefiles (`.shp` + `.dbf`): points and polygons only, which is all
//! the Census cartographic boundaries and the USGS intensity grid use.

use crate::{Result, data_err};

/// One shape record.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// No geometry.
    Null,
    /// A single point (x = longitude, y = latitude).
    Point([f64; 2]),
    /// A polygon record: every part is a closed ring. Outer rings run clockwise, holes
    /// counter-clockwise (shapefile convention).
    Polygon(Vec<Vec<[f64; 2]>>),
}

fn le_i32(b: &[u8], at: usize) -> Result<i32> {
    b.get(at..at + 4)
        .map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| data_err("shapefile truncated"))
}

fn be_i32(b: &[u8], at: usize) -> Result<i32> {
    b.get(at..at + 4)
        .map(|s| i32::from_be_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| data_err("shapefile truncated"))
}

fn le_f64(b: &[u8], at: usize) -> Result<f64> {
    b.get(at..at + 8)
        .map(|s| f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]))
        .ok_or_else(|| data_err("shapefile truncated"))
}

/// Parse a `.shp` file's records in order.
pub fn read_shp(bytes: &[u8]) -> Result<Vec<Shape>> {
    if bytes.len() < 100 || be_i32(bytes, 0)? != 9994 {
        return Err(data_err("not a shapefile (bad file code)"));
    }
    let mut out = Vec::new();
    let mut pos = 100;
    while pos + 8 <= bytes.len() {
        let len_words = be_i32(bytes, pos + 4)? as usize;
        let start = pos + 8;
        let end = start + len_words * 2;
        if end > bytes.len() {
            return Err(data_err("shapefile record runs past end of file"));
        }
        let rec = &bytes[start..end];
        let kind = le_i32(rec, 0)?;
        let shape = match kind {
            0 => Shape::Null,
            1 | 11 | 21 => Shape::Point([le_f64(rec, 4)?, le_f64(rec, 12)?]),
            5 | 15 | 25 => {
                let nparts = le_i32(rec, 36)? as usize;
                let npoints = le_i32(rec, 40)? as usize;
                let parts_at = 44;
                let pts_at = parts_at + 4 * nparts;
                let mut starts = Vec::with_capacity(nparts);
                for i in 0..nparts {
                    starts.push(le_i32(rec, parts_at + 4 * i)? as usize);
                }
                let mut rings = Vec::with_capacity(nparts);
                for i in 0..nparts {
                    let a = starts[i];
                    let b = if i + 1 < nparts { starts[i + 1] } else { npoints };
                    if a > b || b > npoints {
                        return Err(data_err("shapefile polygon has bad part offsets"));
                    }
                    let mut ring = Vec::with_capacity(b - a);
                    for j in a..b {
                        let at = pts_at + 16 * j;
                        ring.push([le_f64(rec, at)?, le_f64(rec, at + 8)?]);
                    }
                    rings.push(ring);
                }
                Shape::Polygon(rings)
            }
            other => return Err(data_err(format!("unsupported shape type {other}"))),
        };
        out.push(shape);
        pos = end;
    }
    Ok(out)
}

/// A parsed `.dbf` attribute table.
#[derive(Debug, Clone)]
pub struct Dbf {
    /// Field names in order.
    pub fields: Vec<String>,
    /// Records (not including deleted ones), each with one trimmed string per field.
    pub records: Vec<Vec<String>>,
}

impl Dbf {
    /// Index of a field by name (case-insensitive).
    pub fn field(&self, name: &str) -> Result<usize> {
        self.fields
            .iter()
            .position(|f| f.eq_ignore_ascii_case(name))
            .ok_or_else(|| data_err(format!("DBF has no field {name}; fields are {:?}", self.fields)))
    }
}

/// Parse a dBase III `.dbf` file. Text is decoded as UTF-8 (the Census files declare UTF-8 in
/// their `.cpg`), with invalid bytes replaced.
pub fn read_dbf(bytes: &[u8]) -> Result<Dbf> {
    if bytes.len() < 32 {
        return Err(data_err("DBF truncated"));
    }
    let nrec = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let rec_len = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;
    let mut fields = Vec::new();
    let mut widths = Vec::new();
    let mut pos = 32;
    while pos + 32 <= bytes.len() && bytes[pos] != 0x0D {
        let d = &bytes[pos..pos + 32];
        let name_end = d[..11].iter().position(|&c| c == 0).unwrap_or(11);
        fields.push(String::from_utf8_lossy(&d[..name_end]).trim().to_string());
        widths.push(d[16] as usize);
        pos += 32;
    }
    let mut records = Vec::with_capacity(nrec);
    for i in 0..nrec {
        let start = header_len + i * rec_len;
        let rec = bytes.get(start..start + rec_len).ok_or_else(|| data_err("DBF record truncated"))?;
        if rec[0] == b'*' {
            continue;
        }
        let mut at = 1;
        let mut row = Vec::with_capacity(widths.len());
        for w in &widths {
            let cell = rec.get(at..at + w).ok_or_else(|| data_err("DBF field truncated"))?;
            row.push(String::from_utf8_lossy(cell).trim().to_string());
            at += w;
        }
        records.push(row);
    }
    Ok(Dbf { fields, records })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shp_with_square() -> Vec<u8> {
        // One polygon record: unit square, clockwise.
        let pts: [[f64; 2]; 5] = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        let mut content = Vec::new();
        content.extend_from_slice(&5i32.to_le_bytes());
        for v in [0.0f64, 0.0, 1.0, 1.0] {
            content.extend_from_slice(&v.to_le_bytes());
        }
        content.extend_from_slice(&1i32.to_le_bytes());
        content.extend_from_slice(&(pts.len() as i32).to_le_bytes());
        content.extend_from_slice(&0i32.to_le_bytes());
        for p in pts {
            content.extend_from_slice(&p[0].to_le_bytes());
            content.extend_from_slice(&p[1].to_le_bytes());
        }
        let mut f = vec![0u8; 100];
        f[0..4].copy_from_slice(&9994i32.to_be_bytes());
        f.extend_from_slice(&1i32.to_be_bytes());
        f.extend_from_slice(&((content.len() / 2) as i32).to_be_bytes());
        f.extend_from_slice(&content);
        f
    }

    #[test]
    fn reads_polygon() {
        let shapes = read_shp(&shp_with_square()).unwrap();
        assert_eq!(shapes.len(), 1);
        match &shapes[0] {
            Shape::Polygon(r) => assert_eq!(r[0].len(), 5),
            s => panic!("unexpected {s:?}"),
        }
    }

    #[test]
    fn reads_dbf() {
        let mut b = vec![0u8; 32];
        b[0] = 3;
        b[4..8].copy_from_slice(&1u32.to_le_bytes());
        let header_len = 32 + 32 + 1;
        b[8..10].copy_from_slice(&(header_len as u16).to_le_bytes());
        b[10..12].copy_from_slice(&6u16.to_le_bytes());
        let mut fd = [0u8; 32];
        fd[..5].copy_from_slice(b"GEOID");
        fd[11] = b'C';
        fd[16] = 5;
        b.extend_from_slice(&fd);
        b.push(0x0D);
        b.extend_from_slice(b" 42101");
        let d = read_dbf(&b).unwrap();
        assert_eq!(d.fields, vec!["GEOID"]);
        assert_eq!(d.records[0][d.field("geoid").unwrap()], "42101");
    }
}
