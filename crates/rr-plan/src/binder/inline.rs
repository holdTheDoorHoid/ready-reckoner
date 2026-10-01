//! Running text for the binder: content Markdown and the engine's own sentences turned into
//! [`Inline`]s, and content blocks turned into [`Block`]s.
//!
//! The text the assembly hands in carries markers, characters that never occur in content:
//!
//! - `\u{1}id\u{2}`: a citation (the packet's marker, [`crate::packet::cite`]); a run of them is
//!   one [`Inline::Cite`];
//! - `\u{3}blank:n\u{3}`: a ruled blank of about `n` characters
//!   ([`rr_content::checklist::blank_marker`]);
//! - `\u{5}text\u{6}`: the household's own words ([`user`]), taken exactly as written and never
//!   read for Markdown;
//! - `**bold**`: bold, in content prose only ([`Mode::Markdown`]);
//! - `{ref:page}`: a cross-reference to a page, which becomes an [`Inline::Link`].
//!
//! Plain text keeps its characters as they are: the Markdown renderer escapes it, the web
//! renderer does not need to.

use rr_types::binder::{Block, Heading, Inline, Link};

use super::Bx;

/// Opens the household's own words in marked text (see the module docs).
pub(crate) const USER_OPEN: char = '\u{5}';
/// Closes the household's own words.
pub(crate) const USER_CLOSE: char = '\u{6}';

/// The household's own words, marked so the parser never reads them as Markdown or markers.
/// Any marker character inside them is dropped (it cannot be typed, but a pasted one must not
/// break the page).
pub(crate) fn user(text: &str) -> String {
    let clean: String = text
        .chars()
        .filter(|c| !matches!(c, '\u{1}' | '\u{2}' | '\u{3}' | '\u{5}' | '\u{6}'))
        .collect();
    format!("{USER_OPEN}{clean}{USER_CLOSE}")
}

/// How to read marked text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Content prose: `**` opens and closes bold.
    Markdown,
    /// An engine sentence or a name: every character is text (markers still count).
    Plain,
}

impl Bx<'_> {
    /// Marked content prose as inlines ([`Mode::Markdown`]).
    pub(crate) fn inl(&self, marked: &str) -> Vec<Inline> {
        self.parse(marked, Mode::Markdown)
    }

    /// A marked engine sentence as inlines ([`Mode::Plain`]).
    pub(crate) fn plain(&self, marked: &str) -> Vec<Inline> {
        self.parse(marked, Mode::Plain)
    }

    /// Marked text as inlines.
    pub(crate) fn parse(&self, marked: &str, mode: Mode) -> Vec<Inline> {
        let mut out = Out::default();
        let mut bold = false;
        let mut rest = marked;
        while let Some(c) = rest.chars().next() {
            if c == crate::packet::OPEN {
                // A run of citation markers, however many, is one citation.
                let mut keys: Vec<u32> = Vec::new();
                while let Some(after) = rest.strip_prefix(crate::packet::OPEN) {
                    let Some(end) = after.find(crate::packet::CLOSE) else {
                        rest = "";
                        break;
                    };
                    keys.push(self.cite_key(after[..end].trim()));
                    rest = &after[end + crate::packet::CLOSE.len_utf8()..];
                }
                out.push(Inline::Cite(keys));
                continue;
            }
            if c == rr_content::checklist::BLANK_MARK {
                if let Some((width, len)) = rr_content::checklist::parse_blank_marker(rest) {
                    out.push(Inline::Blank(width));
                    rest = &rest[len..];
                    continue;
                }
            }
            if c == USER_OPEN {
                let after = &rest[USER_OPEN.len_utf8()..];
                let end = after.find(USER_CLOSE).unwrap_or(after.len());
                out.text(&after[..end], bold);
                rest = after.get(end + USER_CLOSE.len_utf8()..).unwrap_or_default();
                continue;
            }
            if mode == Mode::Markdown && rest.starts_with("**") {
                bold = !bold;
                rest = &rest[2..];
                continue;
            }
            // A Markdown escape (`text::md` wrote it): the character itself.
            if mode == Mode::Markdown
                && c == '\\'
                && let Some(next) = rest[1..].chars().next()
                && ESCAPED.contains(&next)
            {
                out.text(&rest[1..1 + next.len_utf8()], bold);
                rest = &rest[1 + next.len_utf8()..];
                continue;
            }
            if let Some(after) = rest.strip_prefix(rr_content::checklist::REF_OPEN)
                && let Some(end) = after.find('}')
            {
                if let Some(link) = self.link(after[..end].trim()) {
                    out.push(Inline::Link(link));
                }
                rest = &after[end + 1..];
                continue;
            }
            out.text(&rest[..c.len_utf8()], bold);
            rest = &rest[c.len_utf8()..];
        }
        out.finish()
    }

    /// A link to a page in the binder, with the words a reader sees ("Tab 3, Home").
    pub(crate) fn link(&self, page: &str) -> Option<Link> {
        self.directory.get(page).map(|(tab, title)| Link {
            to: page.to_owned(),
            text: format!("Tab {tab}, {title}"),
        })
    }

    /// A link inline to a page, or nothing when the page is not in this binder.
    pub(crate) fn link_inline(&self, page: &str) -> Option<Inline> {
        self.link(page).map(Inline::Link)
    }

    /// Content prose (a rendered guidance block, markers in) as blocks: one per paragraph, a
    /// paragraph of `- ` lines as bullets, of `1. ` lines as a numbered list, a paragraph that is
    /// only bold words as a heading.
    pub(crate) fn prose_blocks(&self, rendered: &str) -> Vec<Block> {
        crate::packet::paragraphs(rendered)
            .iter()
            .map(|p| self.prose_block(p))
            .collect()
    }

    /// One paragraph of content prose as a block (see [`Bx::prose_blocks`]).
    pub(crate) fn prose_block(&self, p: &str) -> Block {
        let lines: Vec<&str> = p.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        if !lines.is_empty() && lines.iter().all(|l| l.starts_with("- ")) {
            return Block::Bullets(lines.iter().map(|l| self.inl(&l[2..])).collect());
        }
        if !lines.is_empty() && lines.iter().all(|l| numbered(l).is_some()) {
            return Block::Numbered(
                lines
                    .iter()
                    .filter_map(|l| numbered(l))
                    .map(|l| self.inl(l))
                    .collect(),
            );
        }
        let joined = lines.join(" ");
        if let Some(inner) = joined
            .strip_prefix("**")
            .and_then(|t| t.strip_suffix("**"))
            .filter(|t| !t.contains("**") && !t.contains(crate::packet::OPEN))
        {
            return Block::Heading(Heading {
                level: 2,
                text: inner.trim_end_matches('.').to_owned(),
            });
        }
        Block::Para(self.inl(&joined))
    }
}

/// The characters `crate::packet::text::md` escapes with a backslash.
const ESCAPED: [char; 10] = ['\\', '*', '_', '`', '[', ']', '|', '<', '>', '#'];

impl Bx<'_> {
    /// Markdown as the v2 packet's writers produce it (markers in) as blocks: `###` and `####`
    /// headings, lists (a task box `[ ]` dropped), numbered lists, block quotes as notes, tables
    /// and paragraphs, split at blank lines.
    pub(crate) fn md_blocks(&self, md: &str) -> Vec<Block> {
        let mut out: Vec<Block> = Vec::new();
        for para in md.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
            let lines: Vec<&str> = para
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            // A heading line may be followed directly by its paragraph.
            let mut lines = &lines[..];
            while let Some(first) = lines.first() {
                let level = if first.starts_with("#### ") {
                    3
                } else if first.starts_with("### ") {
                    2
                } else {
                    break;
                };
                let text: String = first.trim_start_matches('#').trim().to_owned();
                out.push(Block::Heading(Heading {
                    level,
                    text: unescape(&text),
                }));
                lines = &lines[1..];
            }
            if lines.is_empty() {
                continue;
            }
            if lines.iter().all(|l| l.starts_with("> ") || *l == ">") {
                let inner: Vec<&str> = lines
                    .iter()
                    .map(|l| l.trim_start_matches('>').trim())
                    .collect();
                out.push(Block::Callout(rr_types::binder::Callout {
                    kind: rr_types::binder::CalloutKind::Note,
                    title: None,
                    blocks: self.md_blocks(&inner.join("\n")),
                }));
                continue;
            }
            if lines.len() >= 2 && lines.iter().all(|l| l.starts_with('|')) {
                let cells = |l: &str| -> Vec<String> {
                    split_cells(l.trim().trim_start_matches('|').trim_end_matches('|'))
                };
                let header: Vec<String> = cells(lines[0]).iter().map(|c| unescape(c)).collect();
                let rows = lines[2..]
                    .iter()
                    .map(|l| cells(l).iter().map(|c| self.inl(c)).collect())
                    .collect();
                out.push(Block::Table(rr_types::binder::Table { header, rows }));
                continue;
            }
            if lines.iter().all(|l| l.starts_with("- ")) {
                out.push(Block::Bullets(
                    lines
                        .iter()
                        .map(|l| {
                            let item = &l[2..];
                            let item = item
                                .strip_prefix("[ ] ")
                                .or_else(|| item.strip_prefix("[x] "))
                                .unwrap_or(item);
                            self.inl(item)
                        })
                        .collect(),
                ));
                continue;
            }
            if lines.iter().all(|l| numbered(l).is_some()) {
                out.push(Block::Numbered(
                    lines
                        .iter()
                        .filter_map(|l| numbered(l))
                        .map(|l| self.inl(l))
                        .collect(),
                ));
                continue;
            }
            out.push(Block::Para(self.inl(&lines.join(" "))));
        }
        out
    }
}

/// Markdown escapes undone, for text that becomes a heading or a table header.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&n) = chars.peek()
            && ESCAPED.contains(&n)
        {
            out.push(n);
            chars.next();
            continue;
        }
        out.push(c);
    }
    out
}

/// A table row's cells, split at pipes that are not escaped.
fn split_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut escaped = false;
    for c in row.chars() {
        if escaped {
            cur.push(c);
            escaped = false;
            continue;
        }
        match c {
            '\\' => {
                cur.push(c);
                escaped = true;
            }
            '|' => cells.push(std::mem::take(&mut cur).trim().to_owned()),
            _ => cur.push(c),
        }
    }
    cells.push(cur.trim().to_owned());
    cells
}

/// The text after a list number (`1. `), if the line opens with one.
pub(crate) fn numbered(line: &str) -> Option<&str> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    (digits > 0)
        .then(|| line[digits..].strip_prefix(". "))
        .flatten()
}

/// Inlines as they are built: neighbouring text of one weight joins, neighbouring citations join.
#[derive(Default)]
struct Out {
    inlines: Vec<Inline>,
}

impl Out {
    fn text(&mut self, s: &str, bold: bool) {
        if s.is_empty() {
            return;
        }
        match (self.inlines.last_mut(), bold) {
            (Some(Inline::T(t)), false) | (Some(Inline::B(t)), true) => t.push_str(s),
            _ => self.inlines.push(if bold {
                Inline::B(s.to_owned())
            } else {
                Inline::T(s.to_owned())
            }),
        }
    }

    fn push(&mut self, inline: Inline) {
        match (self.inlines.last_mut(), inline) {
            (Some(Inline::Cite(a)), Inline::Cite(b)) => a.extend(b),
            (_, Inline::Cite(b)) if b.is_empty() => {}
            (_, other) => self.inlines.push(other),
        }
    }

    fn finish(self) -> Vec<Inline> {
        self.inlines
            .into_iter()
            .filter(|i| !matches!(i, Inline::T(t) | Inline::B(t) if t.is_empty()))
            .collect()
    }
}

/// Plain text: the inline's words as a reader sees them, citations left out (for tests, titles
/// and the fit proxy).
pub fn text_of(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for i in inlines {
        match i {
            Inline::T(t) | Inline::B(t) => s.push_str(t),
            Inline::Link(l) => {
                s.push('(');
                s.push_str(&l.text);
                s.push(')');
            }
            Inline::Blank(_) => s.push_str("__________"),
            Inline::Cite(_) => {}
        }
    }
    s
}
