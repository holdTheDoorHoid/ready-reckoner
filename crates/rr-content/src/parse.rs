//! Parsing content files into the shared types, and looking things up.
//!
//! File layout (all paths relative to `content/`):
//!
//! - `citations.toml`: `[[citation]]` tables, each a [`Citation`].
//! - `items/<category>.toml`: `[[item]]` tables, each an [`Item`]; one file per category.
//! - `guidance/<id>.md`: a front-matter block between `---` lines (`id`, `title`, `applies_to`,
//!   `citations`, exactly the fields of [`GuidanceMeta`]) followed by Markdown.
//! - `glossary.toml`: `[[term]]` tables, each a [`GlossaryEntry`].
//! - `VERSION`: the human part of [`crate::CONTENT_VERSION`].
//!
//! Unknown fields anywhere are errors (the shared types deny them), so a typo in a content file
//! fails the build instead of being silently ignored.

use std::collections::BTreeMap;

use rr_types::{
    BucketId, BucketInfo, Catalogue, Citation, CitationId, GuidanceMeta, HazardInfo, Item, TierId,
    TierInfo,
};
use serde::{Deserialize, Serialize};

/// A content file could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{file}: {message}")]
pub struct LoadError {
    /// The file, relative to `content/`.
    pub file: String,
    /// What was wrong, in plain words.
    pub message: String,
}

impl LoadError {
    fn new(file: &str, message: impl Into<String>) -> Self {
        Self {
            file: file.to_owned(),
            message: message.into(),
        }
    }
}

/// A glossary entry: the plain phrase first, the jargon second (`docs/PRINCIPLES.md` §12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlossaryEntry {
    /// The plain phrase a reader would use, for example "How rare an event to be ready for".
    pub plain: String,
    /// The technical term it explains, for example "Return period".
    pub term: String,
    /// One or two plain sentences.
    pub definition: String,
    /// Sources for any number or rule in the definition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<CitationId>,
}

/// A guidance block: its metadata and its Markdown body.
#[derive(Debug, Clone, PartialEq)]
pub struct Guidance {
    /// The front matter.
    pub meta: GuidanceMeta,
    /// The Markdown after the front matter, including the closing `## Sources` section.
    pub body: String,
    /// The file it came from, relative to `content/`.
    pub file: String,
}

impl Guidance {
    /// The body with the household's frequency sentence in place of `{frequency}`. With `None`
    /// the placeholder is removed (for the generic, household-free view).
    ///
    /// Conditional spans (`{if:<hazard>}…{/if}`, `{if:home:<kind>}…{/if}`,
    /// [`crate::policy::CONDITION_OPEN`]) are all kept here, without their markers; the packet
    /// keeps only those about hazards likely enough for the household and about its kind of home.
    pub fn render(&self, frequency: Option<&str>) -> String {
        let body = crate::policy::apply_conditions(&self.body, |_| true);
        match frequency {
            Some(sentence) => body.replace(crate::policy::FREQUENCY_PLACEHOLDER, sentence),
            None => body
                .replace(&format!("{} ", crate::policy::FREQUENCY_PLACEHOLDER), "")
                .replace(crate::policy::FREQUENCY_PLACEHOLDER, ""),
        }
    }

    /// The prose part of the body: everything before the `## Sources` heading.
    pub fn prose(&self) -> &str {
        match find_sources_heading(&self.body) {
            Some(i) => &self.body[..i],
            None => &self.body,
        }
    }

    /// The footnote definitions in the `## Sources` section, as `(citation id, text)`.
    pub fn footnote_definitions(&self) -> Vec<(String, String)> {
        let Some(i) = find_sources_heading(&self.body) else {
            return Vec::new();
        };
        self.body[i..]
            .lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix("[^")?;
                let (id, text) = rest.split_once("]:")?;
                Some((id.trim().to_owned(), text.trim().to_owned()))
            })
            .collect()
    }

    /// The citation ids referenced inline in the prose as `[^id]`, in order of first use.
    pub fn footnote_references(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let prose = self.prose();
        let mut rest = prose;
        while let Some(start) = rest.find("[^") {
            let after = &rest[start + 2..];
            match after.find(']') {
                Some(end) => {
                    let id = after[..end].trim().to_owned();
                    if !out.contains(&id) {
                        out.push(id);
                    }
                    rest = &after[end + 1..];
                }
                None => break,
            }
        }
        out
    }
}

/// Byte offset of the `## Sources` heading, if the body has one.
fn find_sources_heading(body: &str) -> Option<usize> {
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        if line.trim().eq_ignore_ascii_case("## sources") {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CitationFile {
    citation: Vec<Citation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemFile {
    item: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GlossaryFile {
    term: Vec<GlossaryEntry>,
}

/// All content, parsed, in a deterministic order (files sorted by path, entries in file order).
#[derive(Debug, Clone, PartialEq)]
pub struct Content {
    /// Every citation.
    pub citations: Vec<Citation>,
    /// Every catalogue item.
    pub items: Vec<Item>,
    /// The file each item came from (same order as `items`), relative to `content/`.
    pub item_files: Vec<String>,
    /// Every guidance block.
    pub guidance: Vec<Guidance>,
    /// Every glossary entry.
    pub glossary: Vec<GlossaryEntry>,
    /// The human part of the version (`content/VERSION`), if present.
    pub version: Option<String>,
    citation_index: BTreeMap<String, usize>,
    item_index: BTreeMap<String, usize>,
    guidance_index: BTreeMap<String, usize>,
}

impl Content {
    /// Parses a set of content files given as `(path relative to content/, text)`. Used for the
    /// embedded files and, in tests, for small synthetic content sets.
    pub fn from_files(files: &[(&str, &str)]) -> Result<Content, LoadError> {
        let mut citations = Vec::new();
        let mut items = Vec::new();
        let mut item_files = Vec::new();
        let mut guidance = Vec::new();
        let mut glossary = Vec::new();
        let mut version = None;
        let mut sorted: Vec<&(&str, &str)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        for &&(path, text) in &sorted {
            let text = text.replace("\r\n", "\n");
            if path == "citations.toml" {
                let f: CitationFile =
                    toml::from_str(&text).map_err(|e| LoadError::new(path, e.to_string()))?;
                citations.extend(f.citation);
            } else if path == "glossary.toml" {
                let f: GlossaryFile =
                    toml::from_str(&text).map_err(|e| LoadError::new(path, e.to_string()))?;
                glossary.extend(f.term);
            } else if path == "VERSION" {
                version = Some(text.trim().to_owned());
            } else if let Some(rest) = path.strip_prefix("items/") {
                if !rest.ends_with(".toml") || rest.contains('/') {
                    return Err(LoadError::new(path, "item files are items/<category>.toml"));
                }
                let f: ItemFile =
                    toml::from_str(&text).map_err(|e| LoadError::new(path, e.to_string()))?;
                for item in f.item {
                    item_files.push(path.to_owned());
                    items.push(item);
                }
            } else if let Some(rest) = path.strip_prefix("guidance/") {
                if !rest.ends_with(".md") || rest.contains('/') {
                    return Err(LoadError::new(path, "guidance files are guidance/<id>.md"));
                }
                guidance.push(parse_guidance(path, &text)?);
            } else {
                return Err(LoadError::new(
                    path,
                    "unexpected file in content/ (expected citations.toml, glossary.toml, VERSION, items/*.toml or guidance/*.md)",
                ));
            }
        }
        let citation_index = first_index(citations.iter().map(|c| c.id.as_str()));
        let item_index = first_index(items.iter().map(|i| i.id.as_str()));
        let guidance_index = first_index(guidance.iter().map(|g| g.meta.id.as_str()));
        Ok(Content {
            citations,
            items,
            item_files,
            guidance,
            glossary,
            version,
            citation_index,
            item_index,
            guidance_index,
        })
    }

    /// The citation with this id.
    pub fn citation(&self, id: &str) -> Option<&Citation> {
        self.citation_index.get(id).map(|&i| &self.citations[i])
    }

    /// The item with this id.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.item_index.get(id).map(|&i| &self.items[i])
    }

    /// The guidance block with this id.
    pub fn guidance(&self, id: &str) -> Option<&Guidance> {
        self.guidance_index.get(id).map(|&i| &self.guidance[i])
    }

    /// Guidance blocks whose `applies_to` names this target, for example `"bucket:power"`,
    /// `"hazard:tornado"`, `"tier:w2"` or `"topic:renters"`.
    pub fn guidance_for<'a>(&'a self, target: &'a str) -> impl Iterator<Item = &'a Guidance> + 'a {
        self.guidance
            .iter()
            .filter(move |g| g.meta.applies_to.iter().any(|a| a == target))
    }

    /// Items that help with this bucket.
    pub fn items_for_bucket(&self, bucket: BucketId) -> impl Iterator<Item = &Item> + '_ {
        self.items
            .iter()
            .filter(move |i| i.buckets.contains(&bucket))
    }

    /// Items in this tier.
    pub fn items_in_tier(&self, tier: TierId) -> impl Iterator<Item = &Item> + '_ {
        self.items.iter().filter(move |i| i.tier == tier)
    }

    /// Items in this category (`"water"`, `"food"`, ...).
    pub fn items_in_category<'a>(
        &'a self,
        category: &'a str,
    ) -> impl Iterator<Item = &'a Item> + 'a {
        self.items.iter().filter(move |i| i.category == category)
    }

    /// The citations for a list of ids, in the given order, skipping unknown ids and duplicates.
    /// Useful for building an output's provenance list.
    pub fn citations_for<'a, I>(&self, ids: I) -> Vec<&Citation>
    where
        I: IntoIterator<Item = &'a CitationId>,
    {
        let mut seen = std::collections::BTreeSet::new();
        ids.into_iter()
            .filter(|id| seen.insert(id.as_str().to_owned()))
            .filter_map(|id| self.citation(id.as_str()))
            .collect()
    }

    /// The glossary entry for a technical term (case-insensitive).
    pub fn glossary_term(&self, term: &str) -> Option<&GlossaryEntry> {
        self.glossary
            .iter()
            .find(|g| g.term.eq_ignore_ascii_case(term))
    }

    /// What the engine's `catalogue()` function returns.
    pub fn catalogue(&self) -> Catalogue {
        Catalogue {
            items: self.items.clone(),
            citations: self.citations.clone(),
            guidance: self.guidance.iter().map(|g| g.meta.clone()).collect(),
            hazards: HazardInfo::all(),
            buckets: BucketInfo::all(),
            tiers: TierInfo::all(),
        }
    }
}

fn first_index<'a>(ids: impl Iterator<Item = &'a str>) -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    for (i, id) in ids.enumerate() {
        map.entry(id.to_owned()).or_insert(i);
    }
    map
}

/// Parses one guidance file: `---`, `key: value` lines, `---`, then Markdown.
fn parse_guidance(path: &str, text: &str) -> Result<Guidance, LoadError> {
    let rest = text.strip_prefix("---\n").ok_or_else(|| {
        LoadError::new(path, "guidance must start with a `---` front-matter line")
    })?;
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| LoadError::new(path, "front matter is not closed with a `---` line"))?;
    let front = &rest[..end];
    let body = rest[end + 5..].to_owned();

    let mut id = None;
    let mut title = None;
    let mut applies_to = None;
    let mut citations = None;
    for (n, line) in front.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once(':').ok_or_else(|| {
            LoadError::new(
                path,
                format!("front matter line {} is not `key: value`", n + 2),
            )
        })?;
        let value = value.trim();
        let slot = match key.trim() {
            "id" => &mut id,
            "title" => &mut title,
            "applies_to" => &mut applies_to,
            "citations" => &mut citations,
            other => {
                return Err(LoadError::new(
                    path,
                    format!(
                        "unknown front-matter field `{other}` (allowed: id, title, applies_to, citations)"
                    ),
                ));
            }
        };
        if slot.is_some() {
            return Err(LoadError::new(
                path,
                format!("`{}` is given twice", key.trim()),
            ));
        }
        *slot = Some(value.to_owned());
    }
    let need = |v: Option<String>, name: &str| {
        v.ok_or_else(|| LoadError::new(path, format!("front matter is missing `{name}`")))
    };
    let id = unquote(&need(id, "id")?);
    let title = unquote(&need(title, "title")?);
    let applies_to = parse_list(path, "applies_to", &need(applies_to, "applies_to")?)?;
    let citations = parse_list(path, "citations", &need(citations, "citations")?)?
        .into_iter()
        .map(CitationId::new)
        .collect();
    Ok(Guidance {
        meta: GuidanceMeta {
            id,
            title,
            applies_to,
            citations,
        },
        body,
        file: path.to_owned(),
    })
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')))
    {
        t[1..t.len() - 1].to_owned()
    } else {
        t.to_owned()
    }
}

fn parse_list(path: &str, key: &str, value: &str) -> Result<Vec<String>, LoadError> {
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or_else(|| LoadError::new(path, format!("`{key}` must be a list like [a, b]")))?;
    Ok(inner
        .split(',')
        .map(unquote)
        .filter(|s| !s.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CITES: &str = r#"
[[citation]]
id = "ready_gov_water"
title = "Water"
publisher = "FEMA / Ready.gov"
year = 2021
url = "https://www.ready.gov/water"
retrieved = "2026-09-25"
license = "US Government Work (public domain)"
"#;

    const GUIDE: &str = "---\nid: bucket_water_out\ntitle: No tap water at all\napplies_to: [bucket:water_out, hazard:earthquake]\ncitations: [ready_gov_water]\n---\n{frequency} Store water.[^ready_gov_water]\n\n## Sources\n\n[^ready_gov_water]: FEMA, Water (2021).\n";

    #[test]
    fn parses_front_matter_body_and_footnotes() {
        let c = Content::from_files(&[
            ("citations.toml", CITES),
            ("guidance/bucket_water_out.md", GUIDE),
        ])
        .unwrap();
        let g = c.guidance("bucket_water_out").unwrap();
        assert_eq!(g.meta.title, "No tap water at all");
        assert_eq!(
            g.meta.applies_to,
            vec!["bucket:water_out", "hazard:earthquake"]
        );
        assert_eq!(g.meta.citations, vec![CitationId::new("ready_gov_water")]);
        assert_eq!(g.footnote_references(), vec!["ready_gov_water"]);
        assert_eq!(g.footnote_definitions()[0].0, "ready_gov_water");
        assert!(!g.prose().contains("## Sources"));
        assert_eq!(c.guidance_for("hazard:earthquake").count(), 1);
    }

    #[test]
    fn render_substitutes_or_drops_the_frequency_placeholder() {
        let c = Content::from_files(&[("guidance/bucket_water_out.md", GUIDE)]).unwrap();
        let g = c.guidance("bucket_water_out").unwrap();
        let with = g.render(Some("About 10 of 100 households like yours."));
        assert!(with.starts_with("About 10 of 100 households like yours. Store water."));
        let without = g.render(None);
        assert!(without.starts_with("Store water."));
    }

    #[test]
    fn unknown_front_matter_field_is_an_error() {
        let bad = GUIDE.replace("title:", "reading_level: 8\ntitle:");
        let e = Content::from_files(&[("guidance/bucket_water_out.md", &bad)]).unwrap_err();
        assert!(e.message.contains("reading_level"), "{e}");
    }

    #[test]
    fn unknown_item_field_is_an_error() {
        let item = r#"
[[item]]
id = "x"
name = "X"
category = "water"
unit = "each"
buckets = ["water_out"]
tier = "h72"
free = false
spec = "A thing."
look_for = ["a", "b"]
avoid = ["c", "d"]
price_band_usd = { low = 1, high = 2, per = "each", note = "n" }
quantity_rule = "once"
citations = ["ready_gov_water"]
hazard_extras = []
colour = "blue"
"#;
        let e = Content::from_files(&[("items/water.toml", item)]).unwrap_err();
        assert!(e.message.contains("colour"), "{e}");
    }

    #[test]
    fn stray_files_are_rejected() {
        let e = Content::from_files(&[("notes.txt", "hello")]).unwrap_err();
        assert_eq!(e.file, "notes.txt");
    }
}
