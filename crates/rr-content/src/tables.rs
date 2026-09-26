//! Tables of local pointers: `content/tables/state_registries.toml`.
//!
//! One row per state, the District of Columbia and Puerto Rico: where a household finds its
//! evacuation-zone lookup, the registry for people who may need help in a disaster, an alert
//! sign-up, and the state's emergency prescription-refill rule. The packet prints the household's
//! own row ([`StateRow::lines`]); nothing about the household leaves the device.
//!
//! A state-level tool is listed only where it was confirmed on the page itself, with the date it
//! was checked. Everywhere else the row points to the county (or parish, borough or municipio)
//! emergency management office through the state agency's site, which is the source for that
//! line. The web address printed in each line is that line's source; the refill rule, which
//! carries numbers, cites registry entries as well.

use serde::{Deserialize, Serialize};

use rr_types::{CitationId, Date};

/// The file, relative to `content/`.
pub const STATE_REGISTRIES_FILE: &str = "tables/state_registries.toml";

/// Registry citations behind the line that sends a household to its local office: Ready.gov says
/// registries are kept by cities and counties and zones are set locally.
pub const LOCAL_OFFICE_SOURCES: &[&str] = &["ready_gov_disability", "ready_gov_evacuation"];

/// Registry citations behind the alerts line when a state has no confirmed sign-up: phone alerts
/// need no sign-up.
pub const ALERT_SOURCES: &[&str] = &["ready_gov_alerts"];

/// A confirmed state-level tool: its name, its address and, if useful, one plain sentence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateEntry {
    /// The tool's own name, for example "Know Your Zone, Know Your Home".
    pub name: String,
    /// Its web address, checked on the row's `checked` date.
    pub url: String,
    /// One plain sentence a household should know (who it is for; that registering does not
    /// guarantee a service), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One row of the state table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateRow {
    /// Two-letter postal code: `FL`, `DC`, `PR`.
    pub code: String,
    /// The name: "Florida".
    pub name: String,
    /// When every address in the row was checked.
    pub checked: Date,
    /// The state's emergency management agency.
    pub em_agency: String,
    /// Its web address.
    pub em_url: String,
    /// Who runs local services, in words that fit "your …": "county emergency management
    /// office" unless the state has parishes, boroughs, towns or municipios. Empty when the state
    /// agency is itself the local office (the District of Columbia).
    #[serde(default = "default_local_office")]
    pub local_office: String,
    /// A confirmed state-level evacuation-zone lookup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<StateEntry>,
    /// A confirmed state-level registry for people who may need help in a disaster.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<StateEntry>,
    /// A confirmed state-level alert sign-up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alerts: Option<StateEntry>,
    /// Say plainly that no jurisdiction-wide zone lookup, registry or alert sign-up was found
    /// (Puerto Rico), instead of saying the local office runs them.
    #[serde(default)]
    pub none_found: bool,
    /// The emergency prescription-refill rule in plain words.
    pub refill: String,
    /// Registry citations for the refill rule (two where sources disagree).
    pub refill_sources: Vec<CitationId>,
}

fn default_local_office() -> String {
    "county emergency management office".to_owned()
}

/// One printed line of a state row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateLine {
    /// What the line is about: "Evacuation zones", "Registry", "Alerts", "Prescription refills".
    pub topic: &'static str,
    /// The sentence, with the web address in it.
    pub text: String,
    /// Registry citations behind the sentence (the address in the text is its own source).
    pub sources: Vec<CitationId>,
}

fn cite(ids: &[&str]) -> Vec<CitationId> {
    ids.iter().map(|s| CitationId::new(*s)).collect()
}

impl StateEntry {
    fn sentence(&self) -> String {
        match &self.note {
            Some(n) => format!("{} ({}). {}", self.name, self.url, n),
            None => format!("{} ({}).", self.name, self.url),
        }
    }
}

impl StateRow {
    /// "Florida Division of Emergency Management (https://www.floridadisaster.org/)".
    fn agency(&self) -> String {
        format!("{} ({})", self.em_agency, self.em_url)
    }

    /// The row as printed: evacuation zones, registry, alerts, prescription refills, in that
    /// order. A confirmed tool is named with its address; anything else sends the household to
    /// its local office through the state agency, or, where nothing was found (Puerto Rico), says
    /// so.
    pub fn lines(&self) -> Vec<StateLine> {
        let mut out = Vec::new();
        // (what the office handles, what was not found)
        let local = |handles: &str, missing: &str| {
            if self.none_found {
                format!(
                    "No {missing} was found for all of {}. Ask your {} or the agency for all of {}: {}.",
                    self.name,
                    self.local_office,
                    self.name,
                    self.agency()
                )
            } else if self.local_office.is_empty() {
                format!(
                    "The {} handles {handles} ({}).",
                    self.em_agency, self.em_url
                )
            } else {
                format!(
                    "Your {} handles {handles}. Find yours through the state agency: {}.",
                    self.local_office,
                    self.agency()
                )
            }
        };
        let local_sources = || {
            if self.none_found {
                Vec::new()
            } else {
                cite(LOCAL_OFFICE_SOURCES)
            }
        };
        const ZONES: (&str, &str) = ("evacuation zones", "evacuation-zone lookup");
        const REGISTRY: (&str, &str) = (
            "any registry for people who may need help in a disaster",
            "registry for people who may need help in a disaster",
        );
        match (&self.zone, &self.registry) {
            (None, None) => out.push(StateLine {
                topic: "Evacuation zones and registry",
                text: local(
                    "evacuation zones and any registry for people who may need help in a disaster",
                    "evacuation-zone lookup or registry for people who may need help in a disaster",
                ),
                sources: local_sources(),
            }),
            (zone, registry) => {
                out.push(StateLine {
                    topic: "Evacuation zones",
                    text: match zone {
                        Some(z) => z.sentence(),
                        None => local(ZONES.0, ZONES.1),
                    },
                    sources: if zone.is_some() {
                        Vec::new()
                    } else {
                        local_sources()
                    },
                });
                out.push(StateLine {
                    topic: "Registry",
                    text: match registry {
                        Some(r) => r.sentence(),
                        None => local(REGISTRY.0, REGISTRY.1),
                    },
                    sources: if registry.is_some() {
                        Vec::new()
                    } else {
                        local_sources()
                    },
                });
            }
        }
        out.push(match &self.alerts {
            Some(a) => StateLine {
                topic: "Alerts",
                text: format!(
                    "{} Emergency alerts also reach phones with no sign-up.",
                    a.sentence()
                ),
                sources: cite(ALERT_SOURCES),
            },
            None if self.none_found => StateLine {
                topic: "Alerts",
                text: format!(
                    "No alert sign-up was found for all of {}. Emergency alerts still reach phones with no sign-up.",
                    self.name
                ),
                sources: cite(ALERT_SOURCES),
            },
            None => StateLine {
                topic: "Alerts",
                text: if self.local_office.is_empty() {
                    format!(
                        "Emergency alerts reach phones with no sign-up. Ask the {} about its own text or email alerts.",
                        self.em_agency
                    )
                } else {
                    format!(
                        "Emergency alerts reach phones with no sign-up. Ask your {} about its own text or email alerts.",
                        self.local_office
                    )
                },
                sources: cite(ALERT_SOURCES),
            },
        });
        out.push(StateLine {
            topic: "Prescription refills",
            text: self.refill.clone(),
            sources: self.refill_sources.clone(),
        });
        out
    }

    /// Every web address in the row.
    pub fn urls(&self) -> Vec<&str> {
        let mut v = vec![self.em_url.as_str()];
        for e in [&self.zone, &self.registry, &self.alerts]
            .into_iter()
            .flatten()
        {
            v.push(e.url.as_str());
        }
        v
    }
}

/// The parsed `state_registries.toml`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateTable {
    /// One row per state, DC and Puerto Rico, in file order.
    #[serde(default)]
    pub state: Vec<StateRow>,
}

impl StateTable {
    /// The row for a two-letter postal code (case-insensitive).
    pub fn row(&self, code: &str) -> Option<&StateRow> {
        self.state
            .iter()
            .find(|r| r.code.eq_ignore_ascii_case(code.trim()))
    }
}

/// The 50 states, the District of Columbia and Puerto Rico: the rows the table must have.
pub const JURISDICTIONS: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS",
    "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY",
    "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
    "WI", "WY", "DC", "PR",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn row(extra: &str) -> StateRow {
        toml::from_str(&format!(
            "code = \"KS\"\nname = \"Kansas\"\nchecked = \"2026-09-26\"\n\
             em_agency = \"Kansas Division of Emergency Management\"\n\
             em_url = \"https://www.kansastag.gov/kdem\"\n\
             refill = \"A pharmacist may give an emergency supply.\"\n\
             refill_sources = [\"healthcare_ready_refill_laws\"]\n{extra}"
        ))
        .expect("row parses")
    }

    #[test]
    fn a_row_with_nothing_confirmed_points_to_the_local_office() {
        let lines = row("").lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].topic, "Evacuation zones and registry");
        assert!(
            lines[0]
                .text
                .contains("Your county emergency management office handles evacuation zones")
        );
        assert!(lines[0].text.ends_with(
            "Find yours through the state agency: Kansas Division of Emergency Management \
             (https://www.kansastag.gov/kdem)."
        ));
        assert_eq!(lines[0].sources, cite(LOCAL_OFFICE_SOURCES));
        assert!(lines[1].text.starts_with("Emergency alerts reach phones"));
        assert_eq!(lines[2].topic, "Prescription refills");
        assert_eq!(lines[2].sources, cite(&["healthcare_ready_refill_laws"]));
    }

    #[test]
    fn a_confirmed_tool_is_named_and_the_other_line_falls_back() {
        let r = row(
            "zone = { name = \"Know Your Zone\", url = \"https://example.gov/zone\" }\n\
             registry = { name = \"Special Needs Registry\", url = \"https://example.gov/snr\", \
             note = \"Registering does not guarantee a shelter place.\" }\n",
        );
        let lines = r.lines();
        assert_eq!(lines[0].text, "Know Your Zone (https://example.gov/zone).");
        assert!(lines[0].sources.is_empty());
        assert!(
            lines[1]
                .text
                .ends_with("does not guarantee a shelter place.")
        );
        let only_zone =
            row("zone = { name = \"Know Your Zone\", url = \"https://example.gov/zone\" }\n");
        let l = only_zone.lines();
        assert_eq!(l[1].topic, "Registry");
        assert!(
            l[1].text
                .starts_with("Your county emergency management office handles any registry")
        );
        assert_eq!(only_zone.urls().len(), 2);
    }

    #[test]
    fn none_found_says_so_plainly() {
        let r =
            row("none_found = true\nlocal_office = \"municipio's emergency management office\"\n");
        let lines = r.lines();
        assert_eq!(
            lines[0].text,
            "No evacuation-zone lookup or registry for people who may need help in a disaster \
             was found for all of Kansas. Ask your municipio's emergency management office or the \
             agency for all of Kansas: Kansas Division of Emergency Management \
             (https://www.kansastag.gov/kdem)."
        );
        assert!(lines[0].sources.is_empty());
        assert!(
            lines[1]
                .text
                .starts_with("No alert sign-up was found for all of Kansas.")
        );
    }

    #[test]
    fn an_empty_local_office_means_the_agency_itself() {
        let r = row("local_office = \"\"\n");
        let lines = r.lines();
        assert_eq!(
            lines[0].text,
            "The Kansas Division of Emergency Management handles evacuation zones and any \
             registry for people who may need help in a disaster (https://www.kansastag.gov/kdem)."
        );
        assert!(lines[1].text.ends_with(
            "Ask the Kansas Division of Emergency Management about its own text or email alerts."
        ));
    }

    #[test]
    fn the_jurisdiction_list_has_fifty_two_codes() {
        let mut v = JURISDICTIONS.to_vec();
        v.sort_unstable();
        v.dedup();
        assert_eq!(v.len(), 52);
    }
}
