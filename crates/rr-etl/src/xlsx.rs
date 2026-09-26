//! A minimal reader for the machine-written Excel workbooks some federal sources publish (EIA-861,
//! the FBI's Crime in the United States tables): shared strings, inline strings, numbers and
//! booleans, by cell reference. No formulas are evaluated (the cached value is read), no styles,
//! no dates (these sources store years and counts as plain numbers).
//!
//! An `.xlsx` file is a ZIP of XML parts: `xl/workbook.xml` names the sheets,
//! `xl/_rels/workbook.xml.rels` maps them to `xl/worksheets/sheetN.xml`, and
//! `xl/sharedStrings.xml` holds the strings cells refer to by index.

use crate::{Result, data_err};
use std::io::Read;

/// A worksheet as rows of cell text (empty string for empty cells), row 1 first.
pub type Sheet = Vec<Vec<String>>;

fn part(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<String>> {
    let mut f = match archive.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(Some(s))
}

/// Replace the five XML entities and numeric character references.
pub fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(j) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        let ent = &rest[1..j];
        let rep = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if ent.starts_with("#x") => u32::from_str_radix(&ent[2..], 16)
                .ok()
                .and_then(char::from_u32),
            _ if ent.starts_with('#') => ent[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match rep {
            Some(c) => {
                out.push(c);
                rest = &rest[j + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Value of attribute `name` in a start tag's text (`<c r="A1" t="s">` -> `r` = `A1`).
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    loop {
        let i = rest.find(name)?;
        let before = rest[..i].chars().last();
        let after = &rest[i + name.len()..];
        if before.is_some_and(|c| c.is_whitespace()) && after.starts_with("=\"") {
            let v = &after[2..];
            return v.find('"').map(|j| &v[..j]);
        }
        rest = &rest[i + name.len()..];
    }
}

/// The text of every `<t>` element inside `xml`, concatenated (rich-text runs join up).
fn texts(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<t") {
        let after = &rest[i + 2..];
        // `<t>` or `<t xml:space="preserve">`, not `<tr...>` or similar.
        if !(after.starts_with('>') || after.starts_with(' ')) {
            rest = after;
            continue;
        }
        let Some(gt) = after.find('>') else { break };
        if after[..gt].ends_with('/') {
            rest = &after[gt + 1..];
            continue;
        }
        let body = &after[gt + 1..];
        let Some(end) = body.find("</t>") else { break };
        out.push_str(&unescape(&body[..end]));
        rest = &body[end + 4..];
    }
    out
}

/// Shared strings, in order.
pub fn shared_strings(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<si") {
        let after = &rest[i + 3..];
        let Some(gt) = after.find('>') else { break };
        if after[..gt].ends_with('/') {
            out.push(String::new());
            rest = &after[gt + 1..];
            continue;
        }
        let body = &after[gt + 1..];
        let Some(end) = body.find("</si>") else { break };
        out.push(texts(&body[..end]));
        rest = &body[end + 5..];
    }
    out
}

/// Zero-based (row, column) of a cell reference like `AB12`.
pub fn cell_ref(r: &str) -> Option<(usize, usize)> {
    let letters: String = r.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let digits = &r[letters.len()..];
    if letters.is_empty() || digits.is_empty() {
        return None;
    }
    let mut col = 0usize;
    for c in letters.chars() {
        col = col * 26 + (c.to_ascii_uppercase() as usize - 'A' as usize + 1);
    }
    let row: usize = digits.parse().ok()?;
    Some((row.checked_sub(1)?, col - 1))
}

/// Parse a worksheet part into rows of text.
pub fn parse_sheet(xml: &str, strings: &[String]) -> Sheet {
    let mut rows: Sheet = Vec::new();
    let mut rest = xml;
    let mut auto_row = 0usize;
    while let Some(i) = rest.find("<c") {
        let after = &rest[i + 2..];
        if !(after.starts_with(' ') || after.starts_with('>') || after.starts_with('/')) {
            rest = after;
            continue;
        }
        let Some(gt) = after.find('>') else { break };
        let tag = &after[..gt];
        let self_closing = tag.ends_with('/');
        let (body, next) = if self_closing {
            ("", &after[gt + 1..])
        } else {
            let b = &after[gt + 1..];
            match b.find("</c>") {
                Some(e) => (&b[..e], &b[e + 4..]),
                None => break,
            }
        };
        rest = next;
        let (r, c) = match attr(tag, "r").and_then(cell_ref) {
            Some(rc) => rc,
            None => (auto_row, rows.get(auto_row).map_or(0, |x| x.len())),
        };
        auto_row = r;
        let kind = attr(tag, "t").unwrap_or("n");
        let value = if kind == "inlineStr" {
            texts(body)
        } else {
            let v = body
                .find("<v>")
                .and_then(|a| body[a + 3..].find("</v>").map(|b| &body[a + 3..a + 3 + b]))
                .map(unescape)
                .unwrap_or_default();
            match kind {
                "s" => v
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .and_then(|k| strings.get(k).cloned())
                    .unwrap_or_default(),
                "b" => (if v.trim() == "1" { "TRUE" } else { "FALSE" }).to_string(),
                _ => v,
            }
        };
        if rows.len() <= r {
            rows.resize(r + 1, Vec::new());
        }
        let row = &mut rows[r];
        if row.len() <= c {
            row.resize(c + 1, String::new());
        }
        row[c] = value;
    }
    rows
}

/// Read every sheet of a workbook: (sheet name, rows), in workbook order.
pub fn read_workbook(bytes: &[u8]) -> Result<Vec<(String, Sheet)>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let workbook = part(&mut archive, "xl/workbook.xml")?
        .ok_or_else(|| data_err("xlsx: no xl/workbook.xml"))?;
    let rels = part(&mut archive, "xl/_rels/workbook.xml.rels")?.unwrap_or_default();
    let strings = part(&mut archive, "xl/sharedStrings.xml")?
        .map(|s| shared_strings(&s))
        .unwrap_or_default();
    let mut out = Vec::new();
    let mut rest = workbook.as_str();
    while let Some(i) = rest.find("<sheet ") {
        let after = &rest[i..];
        let Some(gt) = after.find('>') else { break };
        let tag = &after[..gt];
        rest = &after[gt..];
        let name = attr(tag, "name").map(unescape).unwrap_or_default();
        let Some(rid) = attr(tag, "r:id") else {
            continue;
        };
        // Find the relationship with this id.
        let target = rels
            .split("<Relationship ")
            .skip(1)
            .find(|r| attr(&format!(" {r}"), "Id") == Some(rid))
            .and_then(|r| attr(&format!(" {r}"), "Target").map(|t| t.to_string()))
            .ok_or_else(|| data_err(format!("xlsx: sheet {name} has no relationship {rid}")))?;
        let path = if let Some(abs) = target.strip_prefix('/') {
            abs.to_string()
        } else {
            format!("xl/{target}")
        };
        let xml = part(&mut archive, &path)?
            .ok_or_else(|| data_err(format!("xlsx: missing part {path}")))?;
        out.push((name, parse_sheet(&xml, &strings)));
    }
    Ok(out)
}

/// Collapse runs of whitespace (header cells often hold line breaks and doubled spaces).
pub fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn cell_references() {
        assert_eq!(cell_ref("A1"), Some((0, 0)));
        assert_eq!(cell_ref("AB12"), Some((11, 27)));
        assert_eq!(cell_ref("Z3"), Some((2, 25)));
        assert_eq!(cell_ref("12"), None);
    }

    #[test]
    fn entities_and_rich_text() {
        assert_eq!(
            unescape("A &amp; N Electric &#x26; Co &lt;1&gt; &#233;"),
            "A & N Electric & Co <1> é"
        );
        let sst = r#"<sst><si><t>Data Year</t></si><si><t/></si><si><r><t>Ages</t></r><r><t xml:space="preserve"> under 18</t></r></si></sst>"#;
        assert_eq!(shared_strings(sst), vec!["Data Year", "", "Ages under 18"]);
    }

    #[test]
    fn a_workbook_round_trips() {
        // Build a tiny workbook by hand.
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("xl/workbook.xml", o).unwrap();
            z.write_all(br#"<workbook><sheets><sheet name="Reliability_States" sheetId="1" r:id="rId1"/></sheets></workbook>"#).unwrap();
            z.start_file("xl/_rels/workbook.xml.rels", o).unwrap();
            z.write_all(br#"<Relationships><Relationship Id="rId1" Type="x" Target="worksheets/sheet1.xml"/></Relationships>"#).unwrap();
            z.start_file("xl/sharedStrings.xml", o).unwrap();
            z.write_all(br#"<sst><si><t>Utility Name</t></si><si><t>A &amp; N Electric Coop</t></si></sst>"#).unwrap();
            z.start_file("xl/worksheets/sheet1.xml", o).unwrap();
            z.write_all(br#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="C1" s="2"/></row><row r="2"><c r="A2" t="s"><v>1</v></c><c r="B2"><v>123.5</v></c><c r="C2" t="inlineStr"><is><t>MD</t></is></c></row></sheetData></worksheet>"#).unwrap();
            z.finish().unwrap();
        }
        let wb = read_workbook(buf.get_ref()).unwrap();
        assert_eq!(wb.len(), 1);
        assert_eq!(wb[0].0, "Reliability_States");
        assert_eq!(wb[0].1[0][0], "Utility Name");
        assert_eq!(wb[0].1[1], vec!["A & N Electric Coop", "123.5", "MD"]);
        assert_eq!(norm(" Ages\n  under 18 "), "Ages under 18");
    }
}
