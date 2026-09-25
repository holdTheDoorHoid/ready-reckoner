//! Input validation: plain-language problems with a stable code and the JSON path of the field.
//!
//! A [`Problem`] means the input cannot be read one way only, or would make the arithmetic
//! meaningless (no people, a negative budget, a malformed ZIP code). Unusual but meaningful
//! choices, such as a zero budget or no insurance, are not problems; the plan's guardrail
//! warnings handle those, and they never block.

use serde::{Deserialize, Serialize};

use crate::{EngineError, PlanInput, PoweredDevice, is_well_formed_id};

/// The longest planning horizon `validate` accepts, in years.
pub const MAX_HORIZON_YEARS: u8 = 50;

string_enum! {
    /// What kind of problem an input has. Stable strings, so the app can react to a class of
    /// problem without matching on the message.
    pub enum ProblemCode: "problem code" {
        /// The JSON does not match the schema (unknown field, wrong type, missing field, bad date).
        Schema = "schema",
        /// A country other than the United States.
        UnsupportedCountry = "unsupported_country",
        /// Neither a ZIP code nor a county was given.
        LocationMissing = "location_missing",
        /// The ZIP code is not exactly five digits.
        ZipFormat = "zip_format",
        /// The county FIPS code is not exactly five digits.
        CountyFipsFormat = "county_fips_format",
        /// The household has no people.
        NoPeople = "no_people",
        /// A budget, amount, count, distance or quantity is below zero.
        NegativeValue = "negative_value",
        /// A number is not finite (NaN or infinity).
        NotFinite = "not_finite",
        /// A number is outside its allowed range (planning horizon, device watts, the 1 to 5 scale).
        OutOfRange = "out_of_range",
        /// `finances.income.earners` differs from the number of people marked `earner`.
        EarnersMismatch = "earners_mismatch",
        /// An id in `existing` or `dials.scenario_overrides` is not a well-formed id.
        IdFormat = "id_format",
        /// The same scenario appears twice in `dials.scenario_overrides`.
        DuplicateId = "duplicate_id",
    }
}

/// One thing wrong with an input, in plain language. `assess` returns the list in the details of a
/// `bad_input` error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Problem {
    /// The class of problem.
    pub code: ProblemCode,
    /// JSON path of the field, for example `people[1].commute.distance_km`; empty when the problem
    /// is with the input as a whole.
    pub field: String,
    /// What is wrong and how to fix it, in plain language.
    pub message: String,
}

impl Problem {
    /// Builds a problem.
    pub fn new(code: ProblemCode, field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            field: field.into(),
            message: message.into(),
        }
    }

    /// The problem for JSON that does not match the schema.
    pub fn schema(error: &serde_json::Error) -> Self {
        Self::new(
            ProblemCode::Schema,
            "",
            format!("This is not a Ready Reckoner plan the engine can read: {error}."),
        )
    }
}

fn is_five_digits(s: &str) -> bool {
    s.len() == 5 && s.bytes().all(|b| b.is_ascii_digit())
}

/// Collects problems in a fixed order, so the same input always gives the same list.
struct Checker {
    problems: Vec<Problem>,
}

impl Checker {
    fn push(&mut self, code: ProblemCode, field: impl Into<String>, message: impl Into<String>) {
        self.problems.push(Problem::new(code, field, message));
    }

    /// `value` must be a finite number that is zero or more. `what` starts with a capital.
    fn non_negative(&mut self, value: f32, field: impl Into<String>, what: &str) {
        if !value.is_finite() {
            self.push(
                ProblemCode::NotFinite,
                field,
                format!("{what} must be a number."),
            );
        } else if value < 0.0 {
            self.push(
                ProblemCode::NegativeValue,
                field,
                format!("{what} can't be less than zero."),
            );
        }
    }
}

impl PlanInput {
    /// Every problem with this input, in a fixed order. Empty means the input is valid.
    pub fn validate(&self) -> Vec<Problem> {
        let mut c = Checker {
            problems: Vec::new(),
        };

        // Location.
        let loc = &self.location;
        if loc.country != "US" {
            c.push(
                ProblemCode::UnsupportedCountry,
                "location.country",
                "Ready Reckoner covers the United States only for now. Use the country code \"US\".",
            );
        }
        if loc.zip.is_none() && loc.county_fips.is_none() {
            c.push(
                ProblemCode::LocationMissing,
                "location",
                "Enter a ZIP code or choose a county.",
            );
        }
        if let Some(zip) = &loc.zip {
            if !is_five_digits(zip) {
                c.push(
                    ProblemCode::ZipFormat,
                    "location.zip",
                    "A ZIP code is 5 digits, like 19147.",
                );
            }
        }
        if let Some(fips) = &loc.county_fips {
            if !is_five_digits(fips) {
                c.push(
                    ProblemCode::CountyFipsFormat,
                    "location.county_fips",
                    "A county code (FIPS) is 5 digits, like 42101.",
                );
            }
        }

        // People.
        if self.people.is_empty() {
            c.push(
                ProblemCode::NoPeople,
                "people",
                "Add at least one person to your household.",
            );
        }
        for (i, person) in self.people.iter().enumerate() {
            if let PoweredDevice::Other { watts } = person.medical.powered_device {
                if !watts.is_finite() {
                    c.push(
                        ProblemCode::NotFinite,
                        format!("people[{i}].medical.powered_device"),
                        "The device's power use must be a number of watts.",
                    );
                } else if watts <= 0.0 {
                    c.push(
                        ProblemCode::OutOfRange,
                        format!("people[{i}].medical.powered_device"),
                        "Enter how many watts the device uses. It must be more than 0.",
                    );
                }
            }
            if let Some(commute) = &person.commute {
                c.non_negative(
                    commute.distance_km,
                    format!("people[{i}].commute.distance_km"),
                    "The commute distance",
                );
            }
        }
        let marked = self.people.iter().filter(|p| p.earner).count();
        let stated = usize::from(self.finances.income.earners);
        if marked != stated {
            let people = if marked == 1 {
                "person is"
            } else {
                "people are"
            };
            let earners = if stated == 1 { "earner" } else { "earners" };
            c.push(
                ProblemCode::EarnersMismatch,
                "finances.income.earners",
                format!(
                    "The household has {stated} {earners}, but {marked} {people} marked as \
                     earning. Make these match."
                ),
            );
        }

        // Money.
        let fin = &self.finances;
        c.non_negative(
            fin.monthly_budget_usd,
            "finances.monthly_budget_usd",
            "The monthly budget",
        );
        c.non_negative(
            fin.one_off_budget_usd,
            "finances.one_off_budget_usd",
            "The one-off budget",
        );
        c.non_negative(
            fin.emergency_fund_months,
            "finances.emergency_fund_months",
            "Months of savings",
        );
        if let Some(expenses) = fin.monthly_expenses_usd {
            c.non_negative(
                expenses,
                "finances.monthly_expenses_usd",
                "Monthly expenses",
            );
        }

        // What the household already has.
        for (i, owned) in self.existing.iter().enumerate() {
            if !owned.item_id.is_well_formed() {
                c.push(
                    ProblemCode::IdFormat,
                    format!("existing[{i}].item_id"),
                    format!(
                        "\"{}\" is not an item code. Item codes use lowercase letters, digits and \
                         underscores, like water_stored.",
                        owned.item_id
                    ),
                );
            }
            c.non_negative(
                owned.qty,
                format!("existing[{i}].qty"),
                "The quantity you have",
            );
            if let Some(paid) = owned.paid_usd {
                c.non_negative(
                    paid,
                    format!("existing[{i}].paid_usd"),
                    "The amount you paid",
                );
            }
        }

        // Dials and the self-rating.
        for (i, toggle) in self.dials.scenario_overrides.iter().enumerate() {
            if !is_well_formed_id(&toggle.id) {
                c.push(
                    ProblemCode::IdFormat,
                    format!("dials.scenario_overrides[{i}].id"),
                    format!(
                        "\"{}\" is not a scenario code. Scenario codes use lowercase letters, \
                         digits and underscores, like cascadia_m9.",
                        toggle.id
                    ),
                );
            } else if self.dials.scenario_overrides[..i]
                .iter()
                .any(|earlier| earlier.id == toggle.id)
            {
                c.push(
                    ProblemCode::DuplicateId,
                    format!("dials.scenario_overrides[{i}].id"),
                    format!(
                        "The scenario {} is listed twice. Keep one on or off choice for it.",
                        toggle.id
                    ),
                );
            }
        }
        if !(1..=MAX_HORIZON_YEARS).contains(&self.dials.horizon_years) {
            c.push(
                ProblemCode::OutOfRange,
                "dials.horizon_years",
                format!("The planning horizon must be between 1 and {MAX_HORIZON_YEARS} years."),
            );
        }
        if let Some(rating) = self.confidence_1to5 {
            if !(1..=5).contains(&rating) {
                c.push(
                    ProblemCode::OutOfRange,
                    "confidence_1to5",
                    "Pick a number from 1 to 5 for how confident you feel.",
                );
            }
        }

        c.problems
    }

    /// Parses and validates a plan from JSON: the single entry point `rr-wasm` and `rr-cli` use,
    /// so both reject bad input the same way. Schema errors and validation problems both come
    /// back as a `bad_input` error whose details list the problems.
    pub fn from_json(json: &str) -> Result<PlanInput, EngineError> {
        let input: PlanInput = crate::api::parse_json(json)?;
        let problems = input.validate();
        if problems.is_empty() {
            Ok(input)
        } else {
            Err(EngineError::bad_input(problems))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Commute, CommuteMode, ErrorCode, Owned, ScenarioToggle};

    fn codes(input: &PlanInput) -> Vec<(ProblemCode, String)> {
        input
            .validate()
            .into_iter()
            .map(|p| (p.code, p.field))
            .collect()
    }

    fn one(input: &PlanInput, code: ProblemCode, field: &str) {
        let problems = input.validate();
        assert_eq!(
            codes(input),
            vec![(code, field.to_owned())],
            "unexpected problems: {problems:#?}"
        );
        assert!(!problems[0].message.is_empty());
        assert!(
            problems[0].message.ends_with('.'),
            "{}",
            problems[0].message
        );
    }

    #[test]
    fn defaults_validate_clean() {
        assert_eq!(PlanInput::defaults().validate(), vec![]);
    }

    #[test]
    fn catches_each_problem_class() {
        let base = PlanInput::defaults;

        let mut x = base();
        x.location.country = "CA".into();
        one(&x, ProblemCode::UnsupportedCountry, "location.country");

        let mut x = base();
        x.location.zip = None;
        x.location.county_fips = None;
        one(&x, ProblemCode::LocationMissing, "location");

        for bad in ["1914", "191470", "19l47", "19147-1234", "", " 1914"] {
            let mut x = base();
            x.location.zip = Some(bad.into());
            one(&x, ProblemCode::ZipFormat, "location.zip");
        }

        for bad in ["4210", "421010", "42-01", "PA101"] {
            let mut x = base();
            x.location.zip = None;
            x.location.county_fips = Some(bad.into());
            one(&x, ProblemCode::CountyFipsFormat, "location.county_fips");
        }

        let mut x = base();
        x.people.clear();
        x.finances.income.earners = 0;
        one(&x, ProblemCode::NoPeople, "people");

        let mut x = base();
        x.finances.monthly_budget_usd = -5.0;
        one(
            &x,
            ProblemCode::NegativeValue,
            "finances.monthly_budget_usd",
        );

        let mut x = base();
        x.finances.one_off_budget_usd = -0.01;
        one(
            &x,
            ProblemCode::NegativeValue,
            "finances.one_off_budget_usd",
        );

        let mut x = base();
        x.finances.monthly_budget_usd = f32::NAN;
        one(&x, ProblemCode::NotFinite, "finances.monthly_budget_usd");

        let mut x = base();
        x.finances.emergency_fund_months = f32::INFINITY;
        one(&x, ProblemCode::NotFinite, "finances.emergency_fund_months");

        let mut x = base();
        x.finances.monthly_expenses_usd = Some(-1.0);
        one(
            &x,
            ProblemCode::NegativeValue,
            "finances.monthly_expenses_usd",
        );

        let mut x = base();
        x.people[0].commute = Some(Commute {
            distance_km: -3.0,
            mode: CommuteMode::Car,
            remote_possible: false,
        });
        one(
            &x,
            ProblemCode::NegativeValue,
            "people[0].commute.distance_km",
        );

        for watts in [0.0, -60.0] {
            let mut x = base();
            x.people[0].medical.powered_device = PoweredDevice::Other { watts };
            one(
                &x,
                ProblemCode::OutOfRange,
                "people[0].medical.powered_device",
            );
        }
        let mut x = base();
        x.people[0].medical.powered_device = PoweredDevice::Other { watts: f32::NAN };
        one(
            &x,
            ProblemCode::NotFinite,
            "people[0].medical.powered_device",
        );

        for years in [0, MAX_HORIZON_YEARS + 1, u8::MAX] {
            let mut x = base();
            x.dials.horizon_years = years;
            one(&x, ProblemCode::OutOfRange, "dials.horizon_years");
        }

        for rating in [0, 6, 255] {
            let mut x = base();
            x.confidence_1to5 = Some(rating);
            one(&x, ProblemCode::OutOfRange, "confidence_1to5");
        }

        let mut x = base();
        x.finances.income.earners = 2;
        one(&x, ProblemCode::EarnersMismatch, "finances.income.earners");
        assert!(
            x.validate()[0]
                .message
                .contains("2 earners, but 1 person is")
        );

        let mut x = base();
        x.existing.push(Owned {
            item_id: "Water Stored".into(),
            qty: 1.0,
            paid_usd: None,
        });
        one(&x, ProblemCode::IdFormat, "existing[0].item_id");

        let mut x = base();
        x.dials.scenario_overrides.push(ScenarioToggle {
            id: "Cascadia M9".into(),
            on: false,
        });
        one(&x, ProblemCode::IdFormat, "dials.scenario_overrides[0].id");

        let mut x = base();
        for on in [true, false] {
            x.dials.scenario_overrides.push(ScenarioToggle {
                id: "cascadia_m9".into(),
                on,
            });
        }
        one(
            &x,
            ProblemCode::DuplicateId,
            "dials.scenario_overrides[1].id",
        );

        let mut x = base();
        x.existing.push(Owned {
            item_id: "water_stored".into(),
            qty: -1.0,
            paid_usd: None,
        });
        one(&x, ProblemCode::NegativeValue, "existing[0].qty");

        let mut x = base();
        x.existing.push(Owned {
            item_id: "water_stored".into(),
            qty: 4.0,
            paid_usd: Some(-2.0),
        });
        one(&x, ProblemCode::NegativeValue, "existing[0].paid_usd");
    }

    #[test]
    fn every_problem_code_is_exercised_somewhere() {
        // ProblemCode::Schema comes from from_json; the rest from validate (above).
        let err = PlanInput::from_json("{").unwrap_err();
        assert_eq!(err.code, ErrorCode::BadInput);
        let problems = err.problems().unwrap();
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].code, ProblemCode::Schema);
        assert_eq!(problems[0].field, "");
    }

    #[test]
    fn valid_edge_values_are_accepted() {
        let mut x = PlanInput::defaults();
        x.location.zip = Some("00501".into());
        x.location.county_fips = Some("42101".into()); // both given is fine
        x.dials.horizon_years = 1;
        x.confidence_1to5 = Some(5);
        x.finances.monthly_expenses_usd = Some(0.0);
        x.people[0].medical.powered_device = PoweredDevice::Other { watts: 0.5 };
        x.existing.push(Owned {
            item_id: "generator_portable".into(),
            qty: 0.0,
            paid_usd: Some(0.0),
        });
        x.dials.scenario_overrides = vec![
            ScenarioToggle {
                id: "cascadia_m9".into(),
                on: false,
            },
            ScenarioToggle {
                id: "new_madrid_m7".into(),
                on: true,
            },
        ];
        assert_eq!(x.validate(), vec![]);
        x.dials.horizon_years = MAX_HORIZON_YEARS;
        x.confidence_1to5 = Some(1);
        assert_eq!(x.validate(), vec![]);
    }

    #[test]
    fn problems_come_in_a_fixed_order() {
        let mut x = PlanInput::defaults();
        x.location.zip = Some("abc".into());
        x.people.clear();
        x.finances.monthly_budget_usd = -1.0;
        x.dials.horizon_years = 0;
        let got: Vec<ProblemCode> = x.validate().into_iter().map(|p| p.code).collect();
        assert_eq!(
            got,
            [
                ProblemCode::ZipFormat,
                ProblemCode::NoPeople,
                ProblemCode::EarnersMismatch,
                ProblemCode::NegativeValue,
                ProblemCode::OutOfRange
            ]
        );
        assert_eq!(x.validate(), x.clone().validate());
    }

    #[test]
    fn from_json_parses_validates_and_reports() {
        let json = serde_json::to_string(&PlanInput::defaults()).unwrap();
        assert_eq!(PlanInput::from_json(&json).unwrap(), PlanInput::defaults());

        let mut bad = PlanInput::defaults();
        bad.finances.monthly_budget_usd = -10.0;
        let err = PlanInput::from_json(&serde_json::to_string(&bad).unwrap()).unwrap_err();
        assert_eq!(err.code, ErrorCode::BadInput);
        assert_eq!(err.message, "The monthly budget can't be less than zero.");
        let problems = err.problems().unwrap();
        assert_eq!(problems[0].field, "finances.monthly_budget_usd");

        let typo = json.replace("\"setting\"", "\"setings\"");
        let err = PlanInput::from_json(&typo).unwrap_err();
        let problems = err.problems().unwrap();
        assert_eq!(problems[0].code, ProblemCode::Schema);
        assert!(
            problems[0].message.contains("setings"),
            "{}",
            problems[0].message
        );
    }
}
