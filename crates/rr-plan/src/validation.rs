//! The frozen backtest, bundled with the engine (`docs/VALIDATION.md`, model review Part 3.1):
//! 22 real events, the household used for each, what happened, and the verdicts recorded for
//! four runs of the model. `fixtures/backtest/events.json` holds the set; it is embedded here so
//! `EngineInfo::validation` (the public `#/validation` page's tally) and the packet's "How well
//! do these numbers hold up?" line need no file, and `rr validate` re-scores every run against
//! the recorded verdicts.
//!
//! The scoring rule, defined in the round-2 model review before the events were scored: a
//! target is **covered** when it is at least the duration of about 9 in 10 affected households,
//! **partial** when it covers the median household but not the tail, **short** below the median,
//! **over** above three times the event (counted as covered); an event with several needs takes
//! the worst of them; an evacuation is covered when the household is told to keep a go-bag (a
//! ten-year chance of 2 in 100 or more), the warning it plans for from the event's own hazard is
//! no longer than the warning people had, and the time away it plans for reaches the median
//! evacuee's, partial when that time away is within a factor of three, short otherwise.

use serde::Deserialize;

use rr_types::{BucketId, HazardId, ValidationSummary};

/// Where the full table is published.
pub const DOC_URL: &str =
    "https://github.com/holdTheDoorHoid/ready-reckoner/blob/main/docs/VALIDATION.md";

/// The frozen set, as committed.
pub const EVENTS_JSON: &str = include_str!("../../../fixtures/backtest/events.json");

/// A verdict under the scoring rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Below the median affected household.
    Short,
    /// Covers the median affected household, not the tail.
    Partial,
    /// At least the duration of about nine in ten affected households.
    Covered,
    /// More than three times the event (counted as covered).
    Over,
    /// The model has no consequence for it.
    NotModelled,
}

impl Verdict {
    /// The word the table prints.
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Short => "short",
            Verdict::Partial => "partial",
            Verdict::Covered => "covered",
            Verdict::Over => "over (covered)",
            Verdict::NotModelled => "not modelled",
        }
    }

    /// How good it is, with over counted as covered: not modelled 0, short 1, partial 2,
    /// covered 3.
    pub fn rank(self) -> u8 {
        match self {
            Verdict::NotModelled => 0,
            Verdict::Short => 1,
            Verdict::Partial => 2,
            Verdict::Covered | Verdict::Over => 3,
        }
    }
}

/// One need that mattered in an event, with what happened.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scored {
    /// The duration bucket.
    pub bucket: BucketId,
    /// Days until about half of the affected households had service back.
    pub median: f64,
    /// Days until about nine in ten had it back.
    pub p90: f64,
}

/// An evacuation: the hazard that forced people out, the warning they had and how long they
/// were away.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Leave {
    /// The hazard.
    pub hazard: HazardId,
    /// Hours of warning people had.
    pub warning_hours: f64,
    /// Days the median evacuee was away.
    pub days_away: f64,
}

/// The answers to the contract v2 questions a household there would have given before the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answers {
    /// None.
    None,
    /// Someone sleeps below street level.
    Basement,
    /// The household relies on SNAP or WIC.
    Snap,
    /// The water system has frequent problems.
    Frequent,
}

/// One event of the frozen set.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// Stable id.
    pub id: String,
    /// The event, in words.
    pub name: String,
    /// The household file in `fixtures/backtest/`.
    pub household: String,
    /// The needs scored.
    #[serde(default)]
    pub scored: Vec<Scored>,
    /// The evacuation scored, if any.
    #[serde(default)]
    pub leave: Option<Leave>,
    /// The model has no consequence for it.
    pub not_modelled: bool,
    /// The v2 answers for the third and fourth runs.
    pub answers: Answers,
    /// Why the event is inside the records the model learned from, if it is.
    pub in_sample: String,
    /// The recorded verdict of each run.
    pub expected: [Verdict; 4],
}

/// The whole file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frozen {
    /// What the file is.
    pub about: String,
    /// The four runs, in words.
    pub runs: [String; 4],
    /// The run whose verdicts are this version's headline tally.
    pub headline_run: usize,
    /// The data pack the verdicts were recorded on.
    pub data_pack: String,
    /// Where the app shows the table.
    pub url_anchor: String,
    /// The events.
    pub events: Vec<Event>,
}

/// The frozen set, parsed (`None` only if the embedded file is broken, which a test prevents).
pub fn frozen() -> Option<Frozen> {
    serde_json::from_str(EVENTS_JSON).ok()
}

/// The tally of the headline run (`EngineInfo::validation`): over counts as covered.
pub fn summary() -> ValidationSummary {
    let Some(f) = frozen() else {
        return ValidationSummary::default();
    };
    let run = f.headline_run.min(3);
    let mut s = ValidationSummary {
        events_tested: f.events.len() as u16,
        data_pack: f.data_pack.clone(),
        url_anchor: f.url_anchor.clone(),
        ..ValidationSummary::default()
    };
    for e in &f.events {
        match e.expected[run] {
            Verdict::Short => s.short += 1,
            Verdict::Partial => s.partial += 1,
            Verdict::Covered | Verdict::Over => s.covered += 1,
            Verdict::NotModelled => s.not_modelled += 1,
        }
    }
    s
}

/// The rule for one need: the target against the median and nine-in-ten durations.
pub fn score_days(target: f64, median: f64, p90: f64) -> Verdict {
    if target > 3.0 * p90 {
        Verdict::Over
    } else if target >= p90 {
        Verdict::Covered
    } else if target >= median {
        Verdict::Partial
    } else {
        Verdict::Short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_table_parses_and_tallies_as_the_document_says() {
        let f = frozen().expect("fixtures/backtest/events.json parses");
        assert_eq!(f.events.len(), 22);
        let s = summary();
        // docs/VALIDATION.md, "This version, with the tables and the v2 answers".
        assert_eq!(
            (
                s.events_tested,
                s.covered,
                s.partial,
                s.short,
                s.not_modelled
            ),
            (22, 6, 9, 6, 1)
        );
        assert_eq!(s.url_anchor, "#/validation");
        assert_eq!(score_days(10.0, 2.0, 3.0), Verdict::Over);
        assert_eq!(score_days(3.0, 2.0, 3.0), Verdict::Covered);
        assert_eq!(score_days(2.0, 2.0, 3.0), Verdict::Partial);
        assert_eq!(score_days(1.0, 2.0, 3.0), Verdict::Short);
    }
}
