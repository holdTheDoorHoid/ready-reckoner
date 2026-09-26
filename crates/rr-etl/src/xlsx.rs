//! A minimal reader for Office Open XML workbooks (`.xlsx`): one sheet by name, as rows of cell
//! strings. Enough for the plain data tables federal agencies publish (shared strings, numbers,
//! inline strings); no formulas are evaluated (a cell's cached value is read), no dates are
//! converted. Built on the `zip` crate already in use, so no spreadsheet dependency is needed.

use crate::{Result, data_err};
use std::collections::BTreeMap;
use std::io::Read;

/// Decode the XML entities used in cell text (`&amp;` and friends, numeric references).
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
        let decoded = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            e if e.starts_with("#x") => u32::from_str_radix(&e[2..], 16)
                .ok()
                .and_then(char::from_u32),
            e if e.starts_with('#') => e[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match decoded {
            Some(c) => out.push(c),
            None => out.push_str(&rest[..=j]),
        }
        rest = &rest[j + 1..];
    }
    out.push_str(rest);
    out
}

/// The value of an attribute in an XML start tag's text (`name="value"`).
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let i = tag.find(&key)? + key.len();
    let j = tag[i..].find('"')?;
    Some(&tag[i..i + j])
}

/// Text of every `<t>` element inside `s`, concatenated (rich text runs included).
fn text_runs(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("<t") {
        let after = &rest[i + 2..];
        // `<t>` or `<t xml:space="preserve">`, not `<tabColor>` and the like.
        if !(after.starts_with('>') || after.starts_with(' ')) {
            rest = after;
            continue;
        }
        let Some(open_end) = after.find('>') else {
            break;
        };
        if after[..open_end].ends_with('/') {
            rest = &after[open_end + 1..];
            continue;
        }
        let body = &after[open_end + 1..];
        let Some(close) = body.find("</t>") else {
            break;
        };
        out.push_str(&unescape(&body[..close]));
        rest = &body[close + 4..];
    }
    out
}

/// Parse the shared-strings part.
pub fn shared_strings(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<si") {
        let after = &rest[i + 3..];
        let Some(end) = after.find("</si>") else {
            break;
        };
        out.push(text_runs(&after[..end]));
        rest = &after[end + 5..];
    }
    out
}

/// Column letters of a cell reference ("AB12" -> 27, zero-based).
fn column_index(r: &str) -> Option<usize> {
    let mut n = 0usize;
    let mut any = false;
    for c in r.chars() {
        if c.is_ascii_uppercase() {
            n = n * 26 + (c as usize - 'A' as usize + 1);
            any = true;
        } else {
            break;
        }
    }
    any.then(|| n - 1)
}

/// Parse a worksheet part into rows of cell strings (empty cells as empty strings).
pub fn sheet_rows(xml: &str, strings: &[String]) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<row") {
        let after = &rest[i + 4..];
        let Some(open_end) = after.find('>') else {
            break;
        };
        if after[..open_end].ends_with('/') {
            rows.push(Vec::new());
            rest = &after[open_end + 1..];
            continue;
        }
        let body_start = &after[open_end + 1..];
        let Some(end) = body_start.find("</row>") else {
            break;
        };
        let body = &body_start[..end];
        let mut row: Vec<String> = Vec::new();
        let mut cells = body;
        while let Some(ci) = cells.find("<c") {
            let c_after = &cells[ci + 2..];
            if !(c_after.starts_with(' ') || c_after.starts_with('>')) {
                cells = c_after;
                continue;
            }
            let Some(tag_end) = c_after.find('>') else {
                break;
            };
            let tag = &c_after[..tag_end];
            let (inner, next) = if tag.ends_with('/') {
                ("", &c_after[tag_end + 1..])
            } else {
                let content = &c_after[tag_end + 1..];
                let Some(close) = content.find("</c>") else {
                    break;
                };
                (&content[..close], &content[close + 4..])
            };
            let col = attr(tag, "r").and_then(column_index).unwrap_or(row.len());
            let kind = attr(tag, "t").unwrap_or("n");
            let value = match kind {
                "s" => {
                    let v = inner
                        .split_once("<v>")
                        .and_then(|(_, r)| r.split_once("</v>"))
                        .map(|(v, _)| v)
                        .unwrap_or("");
                    v.trim()
                        .parse::<usize>()
                        .ok()
                        .and_then(|k| strings.get(k).cloned())
                        .unwrap_or_default()
                }
                "inlineStr" => text_runs(inner),
                _ => inner
                    .split_once("<v>")
                    .and_then(|(_, r)| r.split_once("</v>"))
                    .map(|(v, _)| unescape(v))
                    .unwrap_or_default(),
            };
            if row.len() <= col {
                row.resize(col + 1, String::new());
            }
            row[col] = value;
            cells = next;
        }
        rows.push(row);
        rest = &body_start[end + 6..];
    }
    rows
}

fn part(zip: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str) -> Result<String> {
    let mut s = String::new();
    zip.by_name(name)
        .map_err(|e| data_err(format!("xlsx: no part {name}: {e}")))?
        .read_to_string(&mut s)?;
    Ok(s)
}

/// Read one sheet of a workbook (by its tab name) as rows of strings.
pub fn read_sheet(bytes: &[u8], sheet: &str) -> Result<Vec<Vec<String>>> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let workbook = part(&mut zip, "xl/workbook.xml")?;
    let rels = part(&mut zip, "xl/_rels/workbook.xml.rels")?;
    let mut targets: BTreeMap<String, String> = BTreeMap::new();
    let mut rest = rels.as_str();
    while let Some(i) = rest.find("<Relationship ") {
        let after = &rest[i..];
        let end = after.find('>').unwrap_or(after.len());
        let tag = &after[..end];
        if let (Some(id), Some(t)) = (attr(tag, "Id"), attr(tag, "Target")) {
            targets.insert(id.to_string(), t.trim_start_matches('/').to_string());
        }
        rest = &after[end.min(after.len())..];
        if end == after.len() {
            break;
        }
    }
    let mut target = None;
    let mut rest = workbook.as_str();
    while let Some(i) = rest.find("<sheet ") {
        let after = &rest[i..];
        let end = after.find('>').unwrap_or(after.len());
        let tag = &after[..end];
        if attr(tag, "name").map(unescape).as_deref() == Some(sheet) {
            target = attr(tag, "r:id").and_then(|id| targets.get(id)).cloned();
            break;
        }
        rest = &after[end.min(after.len())..];
        if end == after.len() {
            break;
        }
    }
    let target = target.ok_or_else(|| data_err(format!("xlsx: no sheet named {sheet}")))?;
    let path = if target.starts_with("xl/") {
        target
    } else {
        format!("xl/{target}")
    };
    let strings = match zip.by_name("xl/sharedStrings.xml") {
        Ok(mut f) => {
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            shared_strings(&s)
        }
        Err(_) => Vec::new(),
    };
    let xml = part(&mut zip, &path)?;
    Ok(sheet_rows(&xml, &strings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entities_and_strings() {
        assert_eq!(unescape("A &amp; B &lt;3 &#233;t&#xE9;"), "A & B <3 été");
        let ss = shared_strings(
            r#"<sst><si><t>GEOID</t></si><si><r><t>Pa</t></r><r><t xml:space="preserve">radise </t></r></si><si><t/></si></sst>"#,
        );
        assert_eq!(ss, vec!["GEOID", "Paradise ", ""]);
    }

    #[test]
    fn sheet_cells_by_reference() {
        let strings = vec![
            "GEOID".to_string(),
            "NAME".to_string(),
            "Paradise, CA".to_string(),
        ];
        let xml = r#"<sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row><row r="2"><c r="A2"><v>655520</v></c><c r="C2" t="inlineStr"><is><t>x</t></is></c><c r="B2" t="s"><v>2</v></c></row></sheetData>"#;
        let rows = sheet_rows(xml, &strings);
        assert_eq!(rows[0], vec!["GEOID", "NAME"]);
        assert_eq!(rows[1], vec!["655520", "Paradise, CA", "x"]);
        assert_eq!(column_index("AB12"), Some(27));
    }
}
