//! Resolving a ZIP code or county code into a [`LocationResolved`], and county search, following
//! `docs/ENGINE-API.md`: a county code wins over a ZIP code (that is how the app records the
//! county a user picked for an ambiguous ZIP code); a ZIP code resolves only when one county holds
//! at least 80 % of it, otherwise `ambiguous_zip` lists every county, largest share first.

use rr_types::{CountyRecord, EngineError, ErrorCode, LocationInput, LocationResolved};

use crate::source::CountySource;

/// A ZIP code resolves to one county only when that county holds at least this share of it
/// (`docs/DESIGN.md` §6: about 30 % of ZIP codes span more than one county).
pub const AMBIGUOUS_ZIP_SHARE: f32 = 0.8;

/// Most results a county search returns.
pub const MAX_SEARCH_RESULTS: usize = 10;

/// Resolves `input` with the lookups of `source` (the default of [`CountySource::resolve`]).
pub fn resolve_with<S: CountySource + ?Sized>(
    source: &S,
    input: &LocationInput,
) -> Result<LocationResolved, EngineError> {
    if !source.has_counties() {
        return Err(EngineError::new(
            ErrorCode::PackMissing,
            "The county data has not loaded yet. Wait a moment and try again.",
        ));
    }
    let zip = input
        .zip
        .as_deref()
        .map(str::trim)
        .filter(|z| !z.is_empty());
    if let Some(fips) = input
        .county_fips
        .as_deref()
        .map(str::trim)
        .filter(|f| !f.is_empty())
    {
        return source
            .location(fips, zip)
            .ok_or_else(|| EngineError::unknown_county(fips, suggest_for_county(source, fips)));
    }
    let Some(zip) = zip else {
        return Err(EngineError::new(
            ErrorCode::BadInput,
            "Enter a ZIP code or choose your county.",
        ));
    };
    let shares = source.resolve_zip(zip);
    match shares.first() {
        None => Err(EngineError::unknown_zip(zip, Vec::new())),
        Some((fips, share)) if *share >= AMBIGUOUS_ZIP_SHARE => source
            .location(fips, Some(zip))
            .ok_or_else(|| EngineError::unknown_zip(zip, Vec::new())),
        Some(_) => Err(EngineError::ambiguous_zip(
            zip,
            shares
                .iter()
                .filter_map(|(f, _)| source.location(f, Some(zip)))
                .collect(),
        )),
    }
}

/// Suggestions for an unknown county code: counties in the same state (same first two digits).
fn suggest_for_county<S: CountySource + ?Sized>(source: &S, fips: &str) -> Vec<LocationResolved> {
    let state: String = fips.chars().take(2).collect();
    if state.len() < 2 {
        return Vec::new();
    }
    source.search(&state)
}

/// Lower-case, accents folded, punctuation dropped, "County" and similar words removed.
fn normalise(s: &str) -> String {
    const SUFFIXES: [&str; 6] = [
        "county",
        "parish",
        "borough",
        "census area",
        "municipality",
        "planning region",
    ];
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
    for suffix in SUFFIXES {
        if let Some(stripped) = t.strip_suffix(&format!(" {suffix}")) {
            t = stripped.to_owned();
        }
    }
    t
}

/// FIPS codes of the records matching a free-text query, best first (at most
/// [`MAX_SEARCH_RESULTS`]). Digits match FIPS codes exactly or as a prefix; text matches county
/// names ignoring case, accents and words like "County"; "Name, ST" or "Name, State" narrows to a
/// state. Ties go to the more populous county, then the lower FIPS code. The same rules as
/// `rr-data`'s search, so results do not change when the data pack replaces the fixtures.
pub fn search_records<'a>(
    records: impl Iterator<Item = &'a CountyRecord>,
    query: &str,
) -> Vec<String> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(u32, u32, &CountyRecord)> = Vec::new();
    let digits = q.chars().all(|c| c.is_ascii_digit());
    let (name_part, state_part) = match q.split_once(',') {
        Some((n, s)) => (n, Some(s.trim().to_lowercase())),
        None => (q, None),
    };
    let nq = normalise(name_part);
    for c in records {
        let score = if digits {
            if c.fips == q {
                100
            } else if q.len() < 5 && c.fips.starts_with(q) {
                50
            } else {
                continue;
            }
        } else {
            if nq.is_empty() {
                return Vec::new();
            }
            if let Some(st) = state_part.as_deref().filter(|s| !s.is_empty()) {
                let abbr_ok = c.state_abbr.to_lowercase() == st;
                let name_ok = c.state_name.to_lowercase().starts_with(st);
                if !abbr_ok && !name_ok {
                    continue;
                }
            }
            let n = normalise(&c.name);
            if n == nq {
                90
            } else if n.starts_with(&nq) {
                70
            } else if n.split(' ').any(|w| w.starts_with(&nq)) {
                60
            } else if n.contains(&nq) {
                40
            } else if normalise(&c.state_name).starts_with(&nq) {
                20
            } else {
                continue;
            }
        };
        scored.push((score, c.population.unwrap_or(0), c));
    }
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.2.fips.cmp(&b.2.fips))
    });
    scored
        .into_iter()
        .take(MAX_SEARCH_RESULTS)
        .map(|(_, _, c)| c.fips.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalise;

    #[test]
    fn names_normalise() {
        assert_eq!(normalise("Philadelphia County"), "philadelphia");
        assert_eq!(normalise("  Miami-Dade "), "miami dade");
        assert_eq!(normalise("Doña Ana"), "dona ana");
    }
}
