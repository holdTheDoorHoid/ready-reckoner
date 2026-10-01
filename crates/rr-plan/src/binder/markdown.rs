//! The binder as Markdown (DESIGN-DELTA-v3 §4.1; brief 3), for the CLI, the goldens and people
//! reading a plan in a text editor. The web app renders the same tree to HTML and PDF.
//!
//! - The binder's title as `#`, then the contents (every tab and its pages), then each part as
//!   `# Tab 3: Home and places` and each page as `## Home`; a page's own headings are `###` to
//!   `#####`.
//! - `fields` as a two-column table, `__________` for each line to write on; `steps` as a
//!   numbered list, the "do first" (memory) steps all bold; `decision` as its question in bold and
//!   a bulleted list, each branch with the page to turn to; `map_slot` as a block quote ("Map: your
//!   neighborhood. Add it in the app, or paste a printed map here."); `cards` as one block quote
//!   per card; `log` as a table with empty rows; a callout as a block quote.
//! - Inline: `link` as "(Tab 3, Home)", `cite` as "[3]" or "[3, 7]", `blank` as `__________`.
//!   Text is escaped (`*`, `_`, `` ` ``, `[`, `]`, `|`, `<`, `>`, `#`, `\`), so the household's
//!   words can never start emphasis, a link, a table cell or a heading; web addresses are printed
//!   exactly as they are (only a `|`, which would end a table cell, is escaped in one).

use std::collections::BTreeMap;

use rr_types::binder::{Binder, Block, CalloutKind, Inline};

use super::BLANK_MD;

/// The binder as Markdown, ending with one newline.
pub fn render(b: &Binder) -> String {
    let dir: BTreeMap<&str, (u8, &str)> = b
        .parts
        .iter()
        .flat_map(|p| {
            p.pages
                .iter()
                .map(move |pg| (pg.id.as_str(), (p.tab, pg.title.as_str())))
        })
        .collect();
    let r = R { dir };
    let mut out: Vec<String> = vec![format!("# {}", esc(&b.title)), String::new()];
    out.push("**Contents**".to_owned());
    out.push(String::new());
    for part in &b.parts {
        let pages: Vec<String> = part.pages.iter().map(|p| esc(&p.title)).collect();
        out.push(format!(
            "- **Tab {}, {}:** {}",
            part.tab,
            esc(&part.title),
            pages.join(" · ")
        ));
    }
    out.push(String::new());
    for part in &b.parts {
        out.push(format!("# Tab {}: {}", part.tab, esc(&part.title)));
        out.push(String::new());
        for page in &part.pages {
            out.push(format!("## {}", esc(&page.title)));
            out.push(String::new());
            for block in &page.blocks {
                let lines = r.block(block);
                if lines.is_empty() {
                    continue;
                }
                out.extend(lines);
                out.push(String::new());
            }
        }
    }
    let text = out.join("\n");
    format!("{}\n", text.trim_end())
}

/// What separates the binder from the Prepare sheet in `rr plan` and the goldens.
pub const PREPARE_SEPARATOR: &str = "\n\n---\n\n# Prepare sheet\n\n";

/// The binder's Markdown, a rule, then the Prepare sheet: what `rr plan` prints and what
/// `fixtures/golden/<name>.md` holds (brief 3).
pub fn with_prepare(b: &Binder, prepare_markdown: &str) -> String {
    format!(
        "{}{PREPARE_SEPARATOR}{}",
        render(b).trim_end(),
        prepare_markdown
    )
}

/// The renderer: the page directory links resolve against.
struct R<'a> {
    dir: BTreeMap<&'a str, (u8, &'a str)>,
}

impl R<'_> {
    fn block(&self, b: &Block) -> Vec<String> {
        match b {
            Block::Heading(h) => vec![format!(
                "{} {}",
                "#".repeat(usize::from(h.level.clamp(1, 3)) + 2),
                esc(&h.text)
            )],
            Block::Para(v) => vec![inl(v)],
            Block::Bullets(items) => items.iter().map(|i| format!("- {}", inl(i))).collect(),
            Block::Numbered(items) => items
                .iter()
                .enumerate()
                .map(|(n, i)| format!("{}. {}", n + 1, inl(i)))
                .collect(),
            Block::Steps(steps) => steps
                .iter()
                .enumerate()
                .map(|(n, s)| {
                    if s.memory {
                        format!("{}. {}", n + 1, bold_all(&s.text))
                    } else {
                        format!("{}. {}", n + 1, inl(&s.text))
                    }
                })
                .collect(),
            Block::Fields(rows) => {
                let mut v = vec!["| | |".to_owned(), "| --- | --- |".to_owned()];
                for r in rows {
                    let value = match &r.value {
                        Some(s) => esc(s),
                        None => vec![BLANK_MD; usize::from(r.lines.max(1))].join(" "),
                    };
                    v.push(format!("| {} | {value} |", esc(&r.label)));
                }
                v
            }
            Block::Table(t) => {
                let mut v = vec![
                    format!(
                        "| {} |",
                        t.header
                            .iter()
                            .map(|h| esc(h))
                            .collect::<Vec<_>>()
                            .join(" | ")
                    ),
                    format!("|{}", " --- |".repeat(t.header.len().max(1))),
                ];
                for row in &t.rows {
                    let cells: Vec<String> = row.iter().map(|c| inl(c)).collect();
                    v.push(format!("| {} |", cells.join(" | ")));
                }
                v
            }
            Block::Callout(c) => {
                let mut inner: Vec<String> = Vec::new();
                let lead = match (&c.title, c.kind) {
                    (Some(t), _) => Some(format!("**{}.**", esc(t.trim_end_matches('.')))),
                    (None, CalloutKind::Stop) => Some("**Stop.**".to_owned()),
                    (None, CalloutKind::Warning) => Some("**Warning.**".to_owned()),
                    (None, _) => None,
                };
                if let Some(l) = lead {
                    inner.push(l);
                    inner.push(String::new());
                }
                for (i, blk) in c.blocks.iter().enumerate() {
                    if i > 0 {
                        inner.push(String::new());
                    }
                    inner.extend(self.block(blk));
                }
                quote(&inner)
            }
            Block::Decision(d) => {
                let mut v = vec![format!("**{}**", esc(&d.question)), String::new()];
                for br in &d.branches {
                    let mut line = inl(&br.when);
                    let then = inl(&br.then);
                    if !then.is_empty() {
                        line.push(' ');
                        line.push_str(&then);
                    }
                    if let Some(to) = &br.go_to {
                        line.push(' ');
                        line.push_str(&self.link_text(to));
                    }
                    v.push(format!("- {}", line.trim()));
                }
                v
            }
            Block::MapSlot(m) => vec![format!(
                "> {}. Add it in the app, or paste a printed map here.",
                esc(m.caption.trim_end_matches('.'))
            )],
            Block::Cards(cards) => {
                let mut v: Vec<String> = Vec::new();
                for (i, c) in cards.iter().enumerate() {
                    if i > 0 {
                        v.push(String::new());
                    }
                    let mut inner = vec![format!("**{}**", esc(&c.title)), String::new()];
                    inner.extend(c.lines.iter().map(|l| format!("- {}", inl(l))));
                    v.extend(quote(&inner));
                }
                v
            }
            Block::Log(l) => {
                let mut v = vec![
                    format!(
                        "| {} |",
                        l.columns
                            .iter()
                            .map(|c| esc(c))
                            .collect::<Vec<_>>()
                            .join(" | ")
                    ),
                    format!("|{}", " --- |".repeat(l.columns.len().max(1))),
                ];
                let empty = format!("|{}", "  |".repeat(l.columns.len().max(1)));
                for _ in 0..l.rows {
                    v.push(empty.clone());
                }
                v
            }
            Block::PageBreak(_) => Vec::new(),
        }
    }

    /// "(Tab 3, Getting out)" for a page id.
    fn link_text(&self, to: &str) -> String {
        match self.dir.get(to) {
            Some((tab, title)) => format!("(Tab {tab}, {})", esc(title)),
            None => String::new(),
        }
    }
}

/// Lines as a block quote ("> line", ">" for an empty one).
fn quote(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            if l.is_empty() {
                ">".to_owned()
            } else {
                format!("> {l}")
            }
        })
        .collect()
}

/// Running text as Markdown.
pub fn inl(v: &[Inline]) -> String {
    let mut s = String::new();
    for i in v {
        match i {
            Inline::T(t) => s.push_str(&esc(t)),
            Inline::B(t) => s.push_str(&bold(&esc(t))),
            Inline::Cite(n) => {
                if !n.is_empty() {
                    let list: Vec<String> = n.iter().map(u32::to_string).collect();
                    s.push_str(&format!("[{}]", list.join(", ")));
                }
            }
            Inline::Link(l) => s.push_str(&format!("({})", esc(&l.text))),
            Inline::Blank(_) => s.push_str(BLANK_MD),
        }
    }
    s
}

/// Bold, with any space at either end kept outside the markers (`** x**` is not bold).
fn bold(t: &str) -> String {
    let lead = t.len() - t.trim_start().len();
    let trail = t.len() - t.trim_end().len();
    let core = t.trim();
    if core.is_empty() {
        return t.to_owned();
    }
    format!("{}**{core}**{}", &t[..lead], &t[t.len() - trail..])
}

/// A memory step: all of its words bold.
fn bold_all(v: &[Inline]) -> String {
    let plain: Vec<Inline> = v
        .iter()
        .map(|i| match i {
            Inline::B(t) => Inline::T(t.clone()),
            other => other.clone(),
        })
        .collect();
    bold(&inl(&plain))
}

/// Characters escaped in text.
const SPECIAL: [char; 10] = ['\\', '*', '_', '`', '[', ']', '|', '<', '>', '#'];

/// Escapes text for Markdown, leaving web addresses as they are (only `|`, which ends a table
/// cell, is escaped in one).
pub fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    let mut rest = text;
    while let Some(start) = url_start(rest) {
        push_escaped(&mut out, &rest[..start]);
        let url = &rest[start..];
        let mut end = url
            .char_indices()
            .find(|(_, c)| c.is_whitespace())
            .map_or(url.len(), |(i, _)| i);
        // A sentence's punctuation after the address is not part of it.
        while end > 0 && url[..end].ends_with(['.', ',', ';', ':', ')']) {
            end -= 1;
        }
        for c in url[..end].chars() {
            if c == '|' {
                out.push('\\');
            }
            out.push(c);
        }
        rest = &url[end..];
    }
    push_escaped(&mut out, rest);
    out
}

/// Where the next web address starts: `http://` or `https://` at the start of a word.
fn url_start(s: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(i) = s[from..].find("http") {
        let at = from + i;
        let tail = &s[at..];
        let starts_word = at == 0
            || s[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace() || c == '(');
        if starts_word && (tail.starts_with("https://") || tail.starts_with("http://")) {
            return Some(at);
        }
        from = at + 4;
    }
    None
}

fn push_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        if SPECIAL.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_escaped_and_addresses_are_not() {
        assert_eq!(esc("a|b*c # [x]"), "a\\|b\\*c \\# \\[x\\]");
        assert_eq!(
            esc("See https://data-downloads.evictionlab.org/#estimating-eviction/. Done"),
            "See https://data-downloads.evictionlab.org/#estimating-eviction/. Done"
        );
        assert_eq!(esc("(https://x.org/a_b), and"), "(https://x.org/a_b), and");
        assert_eq!(esc("https://x.org/a|b"), "https://x.org/a\\|b");
        assert_eq!(esc("rehttps://no"), "rehttps://no");
        assert_eq!(bold(" Get out. "), " **Get out.** ");
        assert_eq!(
            inl(&[
                Inline::B("Get out.".into()),
                Inline::T(" Now to ".into()),
                Inline::Blank(30),
                Inline::T(".".into()),
                Inline::Cite(vec![3, 7]),
                Inline::T(" ".into()),
                Inline::Link(rr_types::binder::Link {
                    to: "home".into(),
                    text: "Tab 3, Home".into()
                }),
            ]),
            "**Get out.** Now to __________.[3, 7] (Tab 3, Home)"
        );
        assert_eq!(
            bold_all(&[Inline::B("Go.".into()), Inline::T(" Now.".into())]),
            "**Go. Now.**"
        );
    }
}
