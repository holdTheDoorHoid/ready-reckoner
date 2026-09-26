//! Reading-level arithmetic: Markdown to plain prose, word, sentence and syllable counts, and the
//! Flesch-Kincaid grade level
//!
//! ```text
//! grade = 0.39 × (words ÷ sentences) + 11.8 × (syllables ÷ words) − 15.59
//! ```
//!
//! (Kincaid, Fishburne, Rogers and Chissom 1975). Syllables are counted with the usual
//! vowel-group rule, which is within about one syllable per word of a dictionary for English prose;
//! the grade is a guide for writers, which is why the validator warns rather than fails above
//! grade 9 (`docs/CONTENT_STANDARDS.md` §4).

/// A sentence the engine substitutes for `{frequency}`; used when measuring reading level so the
/// placeholder is scored like the text a reader actually sees.
pub const TYPICAL_FREQUENCY_SENTENCE: &str =
    "About 20 of 100 households like yours will face this in the next ten years.";

/// Converts Markdown prose to plain text for counting: drops headings' `#` marks, list bullets,
/// emphasis, footnote references (`[^id]`), link targets (keeps link text), table pipes and the
/// `{frequency}` placeholder (replaced by [`TYPICAL_FREQUENCY_SENTENCE`]). Each non-empty line is
/// kept as its own line.
pub fn plain_text(markdown: &str) -> String {
    let mut out = String::new();
    for raw in markdown.lines() {
        let mut line = raw.trim().to_owned();
        if line.is_empty() || line.starts_with("|--") || line.starts_with("| --") {
            continue;
        }
        line = line.trim_start_matches('#').trim().to_owned();
        for bullet in ["- ", "* ", "+ ", "> "] {
            if let Some(rest) = line.strip_prefix(bullet) {
                line = rest.to_owned();
            }
        }
        // numbered list "1. "
        if let Some((n, rest)) = line.split_once(". ")
            && !n.is_empty()
            && n.chars().all(|c| c.is_ascii_digit())
        {
            line = rest.to_owned();
        }
        line = line.replace(
            crate::policy::FREQUENCY_PLACEHOLDER,
            TYPICAL_FREQUENCY_SENTENCE,
        );
        line = strip_footnotes_and_links(&line);
        line = line.replace(['*', '_', '`', '|'], " ");
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if !line.is_empty() {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

fn strip_footnotes_and_links(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(i) = rest.find('[') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let Some(close) = after.find(']') else {
            out.push_str(&rest[i..]);
            return out;
        };
        let inner = &after[..close];
        let tail = &after[close + 1..];
        if inner.starts_with('^') {
            // footnote reference: drop it
            rest = tail;
        } else if let Some(t) = tail.strip_prefix('(') {
            out.push_str(inner);
            rest = match t.find(')') {
                Some(p) => &t[p + 1..],
                None => "",
            };
        } else {
            out.push('[');
            out.push_str(inner);
            out.push(']');
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// The words in plain text: whitespace-separated pieces that contain a letter or digit.
pub fn words(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .collect()
}

/// Sentences in plain text: each `.`, `!` or `?` that ends a word, plus each line that ends
/// without one (headings and list items read as sentences).
pub fn sentence_count(text: &str) -> usize {
    let mut n = 0;
    for line in text.lines() {
        let pieces: Vec<&str> = line.split_whitespace().collect();
        if pieces.is_empty() {
            continue;
        }
        let mut ended = false;
        for w in &pieces {
            let t = w.trim_end_matches(['"', '\'', ')', '’', '”']);
            ended = t.ends_with('.') || t.ends_with('!') || t.ends_with('?');
            if ended && !is_abbreviation(t) {
                n += 1;
            }
        }
        if !ended {
            n += 1;
        }
    }
    n
}

fn is_abbreviation(word: &str) -> bool {
    let lower = word.to_lowercase();
    matches!(
        lower.as_str(),
        "e.g." | "i.e." | "etc." | "vs." | "u.s." | "st." | "dr." | "mr." | "mrs." | "ms." | "no."
    )
}

/// Syllables in one word, by the vowel-group rule with the common silent-e adjustments. Numbers
/// count as two syllables.
pub fn syllables(word: &str) -> usize {
    if word.chars().any(|c| c.is_ascii_digit()) {
        return 2;
    }
    let w: String = word
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if w.is_empty() {
        return 0;
    }
    if w.len() <= 3 {
        return 1;
    }
    let mut s = w.as_str();
    if (s.ends_with("es") || s.ends_with("ed"))
        && !s.ends_with("ted")
        && !s.ends_with("ded")
        && !s.ends_with("les")
    {
        s = &s[..s.len() - 2];
    } else if s.ends_with('e') && !s.ends_with("le") {
        s = &s[..s.len() - 1];
    }
    let mut count = 0;
    let mut prev_vowel = false;
    for (i, c) in s.chars().enumerate() {
        let vowel = matches!(c, 'a' | 'e' | 'i' | 'o' | 'u') || (c == 'y' && i > 0);
        if vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = vowel;
    }
    count.max(1)
}

/// Counts for a piece of plain text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Words.
    pub words: usize,
    /// Sentences.
    pub sentences: usize,
    /// Syllables.
    pub syllables: usize,
}

/// Word, sentence and syllable counts for plain text.
pub fn counts(text: &str) -> Counts {
    let ws = words(text);
    Counts {
        words: ws.len(),
        sentences: sentence_count(text),
        syllables: ws.iter().map(|w| syllables(w)).sum(),
    }
}

/// The Flesch-Kincaid grade level of plain text, or `None` for text with no words.
pub fn flesch_kincaid_grade(text: &str) -> Option<f64> {
    let c = counts(text);
    if c.words == 0 || c.sentences == 0 {
        return None;
    }
    let w = c.words as f64;
    Some(0.39 * (w / c.sentences as f64) + 11.8 * (c.syllables as f64 / w) - 15.59)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_counts_match_a_dictionary_for_common_words() {
        for (w, n) in [
            ("water", 2),
            ("gallon", 2),
            ("the", 1),
            ("store", 1),
            ("emergency", 4),
            ("family", 3),
            ("candle", 2),
            ("boiled", 1),
            ("treated", 2),
            ("supplies", 2),
            ("battery", 3),
            ("medicine", 3),
        ] {
            assert_eq!(syllables(w), n, "{w}");
        }
    }

    #[test]
    fn simple_text_scores_low_and_dense_text_scores_high() {
        let easy = "Store water. Keep it cool. Change it every six months.";
        let hard = "Comprehensive organizational preparedness necessitates considerable \
                    administrative coordination between municipal authorities and residential \
                    communities.";
        let e = flesch_kincaid_grade(easy).unwrap();
        let h = flesch_kincaid_grade(hard).unwrap();
        assert!(e < 3.0, "easy text scored {e}");
        assert!(h > 16.0, "hard text scored {h}");
    }

    #[test]
    fn a_known_passage_scores_near_its_published_grade() {
        // "The cat sat on the mat." style text: 6 words, 1 sentence, 6 syllables -> grade -1.45.
        let g = flesch_kincaid_grade("The cat sat on the mat.").unwrap();
        assert!((g - (0.39 * 6.0 + 11.8 * 1.0 - 15.59)).abs() < 1e-9);
    }

    #[test]
    fn markdown_is_reduced_to_prose() {
        let md = "## Heading\n- A list item with a note.[^cdc_water_storage]\n\
                  See [the guide](https://example.org) now.\n{frequency} More.";
        let p = plain_text(md);
        assert!(p.contains("Heading\n"));
        assert!(p.contains("A list item with a note.\n"));
        assert!(p.contains("See the guide now."));
        assert!(!p.contains("[^"));
        assert!(p.contains(TYPICAL_FREQUENCY_SENTENCE));
        assert_eq!(sentence_count("Heading\nOne. Two.\nNo stop here\n"), 4);
    }
}
