//! County search for the location picker: "phila", "42101", "Cook, IL", "Cook County, Illinois".

use crate::DataStore;
use rr_types::{CountyRecord, LocationResolved};

/// Most results returned by a search.
pub const MAX_RESULTS: usize = 10;

/// Words dropped from county names and queries before matching.
const SUFFIXES: &[&str] = &[
    "county",
    "parish",
    "borough",
    "census area",
    "city and borough",
    "municipality",
    "municipio",
    "planning region",
    "island",
    "district",
];

pub(crate) fn normalise(s: &str) -> String {
    let lower: String = s
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            c if c.is_alphanumeric() || c == ' ' => c,
            _ => ' ',
        })
        .collect();
    let mut t = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    for suf in SUFFIXES {
        if let Some(stripped) = t.strip_suffix(&format!(" {suf}")) {
            t = stripped.to_string();
        }
    }
    let t = t
        .strip_prefix("saint ")
        .map(|r| format!("st {r}"))
        .unwrap_or(t);
    t.replace("st. ", "st ")
}

impl DataStore {
    /// Counties matching a free-text query, best first (at most [`MAX_RESULTS`]). Digits match
    /// FIPS codes (exact, or as a prefix); text matches county names ignoring case, accents and
    /// words like "County"; "Name, ST" or "Name, State" narrows to a state. Ties go to the more
    /// populous county.
    pub fn search(&self, query: &str) -> Vec<&CountyRecord> {
        let q = query.trim();
        if q.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(u32, &CountyRecord)> = Vec::new();
        if q.chars().all(|c| c.is_ascii_digit()) {
            for c in self.counties() {
                if c.fips == q {
                    scored.push((100, c));
                } else if q.len() < 5 && c.fips.starts_with(q) {
                    scored.push((50, c));
                }
            }
        } else {
            let (name_part, state_part) = match q.split_once(',') {
                Some((n, s)) => (n, Some(s.trim().to_lowercase())),
                None => (q, None),
            };
            let nq = normalise(name_part);
            if nq.is_empty() {
                return Vec::new();
            }
            for c in self.counties() {
                if let Some(st) = &state_part
                    && !st.is_empty()
                    && c.state_abbr.to_lowercase() != *st
                    && !c.state_name.to_lowercase().starts_with(st.as_str())
                {
                    continue;
                }
                let n = normalise(&c.name);
                let score = if n == nq {
                    90
                } else if n.starts_with(&nq) {
                    70
                } else if n.split(' ').any(|w| w.starts_with(&nq)) {
                    60
                } else if n.contains(&nq) {
                    40
                } else {
                    continue;
                };
                scored.push((score, c));
            }
        }
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| {
                    b.1.population
                        .unwrap_or(0)
                        .cmp(&a.1.population.unwrap_or(0))
                })
                .then_with(|| a.1.fips.cmp(&b.1.fips))
        });
        scored
            .into_iter()
            .take(MAX_RESULTS)
            .map(|(_, c)| c)
            .collect()
    }

    /// [`Self::search`] as resolved locations (what `county_search` returns).
    pub fn search_locations(&self, query: &str) -> Vec<LocationResolved> {
        self.search(query)
            .into_iter()
            .filter_map(|c| self.location(&c.fips, None))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::normalise;

    #[test]
    fn normalises_names() {
        assert_eq!(normalise("Philadelphia County"), "philadelphia");
        assert_eq!(normalise("Doña Ana"), "dona ana");
        assert_eq!(normalise("St. Louis city"), "st louis city");
        assert_eq!(normalise("Saint Louis"), "st louis");
        assert_eq!(normalise("  Cook  "), "cook");
    }
}
