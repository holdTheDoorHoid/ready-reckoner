//! The engine's function surface (docs/ENGINE-API.md): the result envelope, errors, and the
//! request and result types of the functions that are not plain inputs or outputs.

use serde::de::{DeserializeOwned, Error as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    BucketId, BucketKind, Citation, Date, GuidanceMeta, HazardId, HazardTier, Item,
    LocationResolved, PlanInput, Problem, TargetKind, TierId,
};

string_enum! {
    /// Why an engine call failed.
    pub enum ErrorCode: "error code" {
        /// The input does not match the schema or fails validation; `details` is
        /// [`BadInputDetails`].
        BadInput = "bad_input",
        /// No such ZIP code; `details` is [`LocationSuggestions`].
        UnknownZip = "unknown_zip",
        /// No such county; `details` is [`LocationSuggestions`].
        UnknownCounty = "unknown_county",
        /// The ZIP code spans several counties and none holds at least 80% of it; `details` is
        /// [`LocationSuggestions`] with one entry per county. The app asks the user to pick one
        /// and stores it in `location.county_fips`.
        AmbiguousZip = "ambiguous_zip",
        /// A data pack the call needs has not been loaded.
        PackMissing = "pack_missing",
        /// A data pack failed its checksum or could not be decoded.
        PackCorrupt = "pack_corrupt",
        /// A bug in the engine.
        Internal = "internal",
    }
}

/// An engine error: a stable code, a plain-language message, and optional structured details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
#[error("{code}: {message}")]
pub struct EngineError {
    /// What went wrong.
    pub code: ErrorCode,
    /// What went wrong, in plain language, fit to show the user.
    pub message: String,
    /// Structured details; the shape depends on `code` (see [`ErrorCode`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// `details` of a `bad_input` error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BadInputDetails {
    /// Every problem found, in a fixed order.
    pub problems: Vec<Problem>,
}

/// `details` of an `unknown_zip`, `unknown_county` or `ambiguous_zip` error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationSuggestions {
    /// Locations the user may have meant (for `ambiguous_zip`, the counties the ZIP code spans,
    /// largest share first).
    pub suggestions: Vec<LocationResolved>,
}

impl EngineError {
    /// An error with no details.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// A `bad_input` error listing `problems`. The message is the first problem's message, with a
    /// count when there are several.
    pub fn bad_input(problems: Vec<Problem>) -> Self {
        let message = match problems.as_slice() {
            [] => "The input is not valid.".to_owned(),
            [only] => only.message.clone(),
            [first, ..] => format!(
                "There are {} problems to fix. The first: {}",
                problems.len(),
                first.message
            ),
        };
        Self {
            code: ErrorCode::BadInput,
            message,
            details: serde_json::to_value(BadInputDetails { problems }).ok(),
        }
    }

    /// An `unknown_zip` error with suggestions.
    pub fn unknown_zip(zip: &str, suggestions: Vec<LocationResolved>) -> Self {
        Self::with_suggestions(
            ErrorCode::UnknownZip,
            format!("We don't have ZIP code {zip}. Check it, or choose your county instead."),
            suggestions,
        )
    }

    /// An `unknown_county` error with suggestions.
    pub fn unknown_county(county_fips: &str, suggestions: Vec<LocationResolved>) -> Self {
        Self::with_suggestions(
            ErrorCode::UnknownCounty,
            format!(
                "We don't have a county with code {county_fips}. Choose your county from the list."
            ),
            suggestions,
        )
    }

    /// An `ambiguous_zip` error listing the counties the ZIP code spans.
    pub fn ambiguous_zip(zip: &str, counties: Vec<LocationResolved>) -> Self {
        Self::with_suggestions(
            ErrorCode::AmbiguousZip,
            format!("ZIP code {zip} covers more than one county. Choose the one you live in."),
            counties,
        )
    }

    fn with_suggestions(
        code: ErrorCode,
        message: String,
        suggestions: Vec<LocationResolved>,
    ) -> Self {
        Self {
            code,
            message,
            details: serde_json::to_value(LocationSuggestions { suggestions }).ok(),
        }
    }

    /// The problems of a `bad_input` error, if the details carry them.
    pub fn problems(&self) -> Option<Vec<Problem>> {
        let details = self.details.clone()?;
        serde_json::from_value::<BadInputDetails>(details)
            .ok()
            .map(|d| d.problems)
    }

    /// The suggestions of an `unknown_zip`, `unknown_county` or `ambiguous_zip` error, if the
    /// details carry them.
    pub fn suggestions(&self) -> Option<Vec<LocationResolved>> {
        let details = self.details.clone()?;
        serde_json::from_value::<LocationSuggestions>(details)
            .ok()
            .map(|d| d.suggestions)
    }
}

/// Parses any contract type from JSON, turning a schema error into a `bad_input` error whose
/// details carry one `schema` problem. `rr-wasm` uses it for every JSON argument.
pub fn parse_json<T: DeserializeOwned>(json: &str) -> Result<T, EngineError> {
    serde_json::from_str(json).map_err(|e| EngineError::bad_input(vec![Problem::schema(&e)]))
}

/// What every engine function returns: `{"ok": true, "value": …}` or
/// `{"ok": false, "error": …}`.
#[derive(Debug, Clone, PartialEq)]
pub enum Envelope<T> {
    /// The call succeeded.
    Value(T),
    /// The call failed.
    Error(EngineError),
}

impl<T> Envelope<T> {
    /// True for a successful call.
    pub fn is_ok(&self) -> bool {
        matches!(self, Envelope::Value(_))
    }

    /// Converts into a `Result`.
    pub fn into_result(self) -> Result<T, EngineError> {
        match self {
            Envelope::Value(v) => Ok(v),
            Envelope::Error(e) => Err(e),
        }
    }
}

impl<T> From<Result<T, EngineError>> for Envelope<T> {
    fn from(result: Result<T, EngineError>) -> Self {
        match result {
            Ok(v) => Envelope::Value(v),
            Err(e) => Envelope::Error(e),
        }
    }
}

impl<T: Serialize> Envelope<T> {
    /// Serialises to the JSON string the engine hands to JavaScript. Never fails: if the value
    /// cannot be serialised, the result is an `internal` error envelope instead.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            let fallback: Envelope<()> = Envelope::Error(EngineError::new(
                ErrorCode::Internal,
                format!("The engine could not encode its answer: {e}"),
            ));
            serde_json::to_string(&fallback).unwrap_or_else(|_| {
                r#"{"ok":false,"error":{"code":"internal","message":"The engine could not encode its answer."}}"#
                    .to_owned()
            })
        })
    }
}

impl<T: Serialize> Serialize for Envelope<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Envelope", 2)?;
        match self {
            Envelope::Value(v) => {
                s.serialize_field("ok", &true)?;
                s.serialize_field("value", v)?;
            }
            Envelope::Error(e) => {
                s.serialize_field("ok", &false)?;
                s.serialize_field("error", e)?;
            }
        }
        s.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Envelope<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Raw<T> {
            ok: bool,
            value: Option<T>,
            error: Option<EngineError>,
        }
        let raw = Raw::<T>::deserialize(deserializer)?;
        match (raw.ok, raw.value, raw.error) {
            (true, Some(v), None) => Ok(Envelope::Value(v)),
            (false, None, Some(e)) => Ok(Envelope::Error(e)),
            (true, _, _) => Err(D::Error::custom(
                "an envelope with ok: true needs `value` and no `error`",
            )),
            (false, _, _) => Err(D::Error::custom(
                "an envelope with ok: false needs `error` and no `value`",
            )),
        }
    }
}

/// A credit line or disclaimer the app must show (the About screen renders the list). Some
/// sources require one: the FEMA National Risk Index terms ask for the dataset version, the
/// access date and a statement that FEMA does not endorse the app; CC BY sources need credit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attribution {
    /// The dataset or publisher, for example "FEMA National Risk Index".
    pub source: String,
    /// The line to show, exactly as the source's terms require.
    pub text: String,
    /// Where the source lives.
    pub url: String,
    /// Dataset version, where the source has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// When the data was accessed.
    pub accessed: Date,
}

/// What `engine_info()` returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineInfo {
    /// Engine version (semver).
    pub engine_version: String,
    /// [`crate::ENGINE_API_VERSION`]; the app checks it matches its own copy.
    pub api_version: u32,
    /// Version of the loaded data packs; absent until a pack is loaded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_pack_version: Option<String>,
    /// Version of the embedded content.
    pub content_version: String,
    /// Names of the packs loaded so far.
    pub packs_loaded: Vec<String>,
    /// Credit lines and disclaimers the app must show.
    pub attributions: Vec<Attribution>,
    /// How the model did against past disasters, from the bundled validation table, for the
    /// `#/validation` page (contract v2; REVIEW R10). Omitted when no table is bundled.
    #[serde(default, skip_serializing_if = "ValidationSummary::is_empty")]
    pub validation: ValidationSummary,
}

/// How the model did against the frozen set of past disasters in `docs/VALIDATION.md` (contract
/// v2; REVIEW R10): how many event-and-household pairs the target covered, partly covered, fell
/// short on, or could not model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationSummary {
    /// Event-and-household pairs tested.
    pub events_tested: u16,
    /// Pairs whose target covered what happened.
    pub covered: u16,
    /// Pairs covered in part.
    pub partial: u16,
    /// Pairs whose target fell short.
    pub short: u16,
    /// Pairs the model could not represent.
    pub not_modelled: u16,
    /// Version of the data pack the backtest ran on.
    pub data_pack: String,
    /// Where the app shows the full table, for example `"#/validation"`.
    pub url_anchor: String,
}

impl ValidationSummary {
    /// True when no table is bundled (every count zero and no pack named).
    pub fn is_empty(&self) -> bool {
        *self == ValidationSummary::default()
    }
}

/// What `load_pack(name, bytes)` returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackInfo {
    /// Pack name, for example `"core"`.
    pub name: String,
    /// Pack version.
    pub version: String,
    /// Rows loaded.
    pub rows: u32,
}

/// A hazard id with its plain name and tier, for the app's labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazardInfo {
    /// The id.
    pub id: HazardId,
    /// Plain name.
    pub name: String,
    /// Natural, societal or personal.
    pub tier: HazardTier,
}

/// A bucket id with its plain name and kinds, for the app's labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BucketInfo {
    /// The id.
    pub id: BucketId,
    /// Plain name.
    pub name: String,
    /// Duration, readiness or money.
    pub kind: BucketKind,
    /// How the bucket's target is expressed.
    pub target_kind: TargetKind,
}

/// A tier id with its plain name and days, for the app's labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TierInfo {
    /// The id.
    pub id: TierId,
    /// Plain name.
    pub name: String,
    /// Days of self-sufficiency the tier stands for.
    pub days: u16,
}

impl From<HazardId> for HazardInfo {
    fn from(id: HazardId) -> Self {
        Self {
            id,
            name: id.name().to_owned(),
            tier: id.tier(),
        }
    }
}

impl From<BucketId> for BucketInfo {
    fn from(id: BucketId) -> Self {
        Self {
            id,
            name: id.name().to_owned(),
            kind: id.kind(),
            target_kind: id.target_kind(),
        }
    }
}

impl From<TierId> for TierInfo {
    fn from(id: TierId) -> Self {
        Self {
            id,
            name: id.name().to_owned(),
            days: id.days(),
        }
    }
}

impl HazardInfo {
    /// Every hazard the engine may emit, in [`HazardId::ACTIVE`] order: the retired `terrorism`
    /// is left out.
    pub fn all() -> Vec<HazardInfo> {
        HazardId::ACTIVE.iter().map(|&h| h.into()).collect()
    }
}

impl BucketInfo {
    /// Every bucket, in [`BucketId::ALL`] order.
    pub fn all() -> Vec<BucketInfo> {
        BucketId::ALL.iter().map(|&b| b.into()).collect()
    }
}

impl TierInfo {
    /// Every tier, in [`TierId::ALL`] order.
    pub fn all() -> Vec<TierInfo> {
        TierId::ALL.iter().map(|&t| t.into()).collect()
    }
}

/// What `catalogue()` returns: the content, plus the plain names of every id so the app never
/// hard-codes them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    /// Every catalogue item.
    pub items: Vec<Item>,
    /// Every citation.
    pub citations: Vec<Citation>,
    /// Every guidance block's metadata.
    pub guidance: Vec<GuidanceMeta>,
    /// Every hazard id the engine may emit, with its name and tier ([`HazardInfo::all`]).
    pub hazards: Vec<HazardInfo>,
    /// Every bucket id with its name and kinds ([`BucketInfo::all`]).
    pub buckets: Vec<BucketInfo>,
    /// Every tier id with its name and days ([`TierInfo::all`]).
    pub tiers: Vec<TierInfo>,
}

string_enum! {
    /// What `explain` is asked about.
    pub enum ExplainKind: "explain kind" {
        /// A hazard id.
        Hazard = "hazard",
        /// A bucket id.
        Bucket = "bucket",
        /// A catalogue item id.
        Item = "item",
        /// A requirement line id.
        Requirement = "requirement",
        /// A warning id.
        Warning = "warning",
    }
}

/// The argument of `explain`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplainRequest {
    /// What kind of thing `id` names.
    pub kind: ExplainKind,
    /// The id to explain.
    pub id: String,
    /// The household, so the explanation uses its numbers.
    pub input: PlanInput,
}

/// What `explain` returns: why a number is what it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Explanation {
    /// Heading.
    pub title: String,
    /// The explanation in plain language, one paragraph per entry.
    pub plain: Vec<String>,
    /// The arithmetic, one step per entry, for the expert view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub math: Option<Vec<String>>,
    /// Sources.
    pub sources: Vec<Citation>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FacilityFlags, LatLon, ProblemCode};

    fn place() -> LocationResolved {
        LocationResolved {
            country: "US".into(),
            county_fips: "42101".into(),
            county_name: "Philadelphia".into(),
            state_abbr: "PA".into(),
            state_name: "Pennsylvania".into(),
            zip: Some("19147".into()),
            zip_county_share: Some(1.0),
            centroid: LatLon {
                lat: 40.0,
                lon: -75.1,
            },
            nca_region: "northeast".into(),
            coastal: false,
            tsunami_zone: false,
            facility_flags: FacilityFlags {
                nuclear_plant_within_16km: false,
                nuclear_plant_within_80km: true,
                hazmat_facilities_within_5km: 3,
            },
            data_note: None,
            exposure: crate::Exposure::default(),
        }
    }

    #[test]
    fn envelope_json_shapes() {
        let ok: Envelope<u32> = Envelope::Value(7);
        assert_eq!(ok.to_json_string(), r#"{"ok":true,"value":7}"#);
        let err: Envelope<u32> = Envelope::Error(EngineError::new(
            ErrorCode::PackMissing,
            "Load the core pack first.",
        ));
        assert_eq!(
            err.to_json_string(),
            r#"{"ok":false,"error":{"code":"pack_missing","message":"Load the core pack first."}}"#
        );
        assert_eq!(
            serde_json::from_str::<Envelope<u32>>(r#"{"ok":true,"value":7}"#).unwrap(),
            ok
        );
        assert_eq!(
            serde_json::from_str::<Envelope<u32>>(&err.to_json_string()).unwrap(),
            err
        );
    }

    #[test]
    fn envelope_rejects_inconsistent_shapes() {
        for bad in [
            r#"{"ok":true}"#,
            r#"{"ok":false}"#,
            r#"{"ok":true,"value":1,"error":{"code":"internal","message":"x"}}"#,
            r#"{"ok":false,"value":1}"#,
            r#"{"ok":"true","value":1}"#,
            r#"{"value":1}"#,
            r#"{"ok":true,"value":1,"extra":0}"#,
        ] {
            assert!(serde_json::from_str::<Envelope<u32>>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn envelope_result_conversions() {
        let e: Envelope<u8> = Ok(3).into();
        assert!(e.is_ok());
        assert_eq!(e.into_result(), Ok(3));
        let e: Envelope<u8> = Err(EngineError::new(ErrorCode::Internal, "x")).into();
        assert!(!e.is_ok());
        assert_eq!(e.into_result().unwrap_err().code, ErrorCode::Internal);
    }

    #[test]
    fn unencodable_values_become_internal_errors() {
        // A map with non-string keys cannot be JSON; the envelope degrades to an internal error.
        let mut m = std::collections::BTreeMap::new();
        m.insert((1, 2), 3);
        let json = Envelope::Value(m).to_json_string();
        let back: Envelope<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.into_result().unwrap_err().code, ErrorCode::Internal);
    }

    #[test]
    fn error_details_have_typed_shapes() {
        let problems = vec![
            Problem::new(
                ProblemCode::NoPeople,
                "people",
                "Add at least one person to your household.",
            ),
            Problem::new(
                ProblemCode::ZipFormat,
                "location.zip",
                "A ZIP code is 5 digits, like 19147.",
            ),
        ];
        let e = EngineError::bad_input(problems.clone());
        assert_eq!(e.code, ErrorCode::BadInput);
        assert!(e.message.starts_with("There are 2 problems to fix."));
        assert_eq!(e.problems(), Some(problems.clone()));
        assert_eq!(e.suggestions(), None);
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["details"]["problems"][1]["code"], "zip_format");
        assert_eq!(
            EngineError::bad_input(problems[..1].to_vec()).message,
            problems[0].message
        );

        for (e, code) in [
            (
                EngineError::unknown_zip("99999", vec![place()]),
                "unknown_zip",
            ),
            (
                EngineError::unknown_county("99999", vec![place()]),
                "unknown_county",
            ),
            (
                EngineError::ambiguous_zip("19147", vec![place(), place()]),
                "ambiguous_zip",
            ),
        ] {
            let json = serde_json::to_value(&e).unwrap();
            assert_eq!(json["code"], code);
            assert_eq!(json["details"]["suggestions"][0]["county_fips"], "42101");
            assert!(!e.suggestions().unwrap().is_empty());
            assert_eq!(e.problems(), None);
            let back: EngineError = serde_json::from_value(json).unwrap();
            assert_eq!(back, e);
        }
        assert_eq!(
            EngineError::ambiguous_zip("19147", vec![place(), place()])
                .suggestions()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            EngineError::new(ErrorCode::BadInput, "x").to_string(),
            "bad_input: x"
        );
    }

    #[test]
    fn parse_json_reports_schema_problems() {
        let err = parse_json::<PackInfo>(r#"{"name":"core","version":"1","rows":-1}"#).unwrap_err();
        assert_eq!(err.code, ErrorCode::BadInput);
        assert_eq!(err.problems().unwrap()[0].code, ProblemCode::Schema);
        let ok = parse_json::<PackInfo>(r#"{"name":"core","version":"1","rows":3143}"#).unwrap();
        assert_eq!(ok.rows, 3143);
    }

    #[test]
    fn name_tables_cover_every_id() {
        let h = HazardInfo::all();
        assert_eq!(
            h.len(),
            53,
            "every active hazard; the retired terrorism is left out"
        );
        assert_eq!(h[0].id, HazardId::Avalanche);
        assert_eq!(h[0].name, "Avalanche");
        assert!(h.iter().all(|i| !i.id.is_retired()));
        let listed: Vec<HazardId> = h.iter().map(|i| i.id).collect();
        assert_eq!(listed, HazardId::ACTIVE);
        let b = BucketInfo::all();
        assert_eq!(b.len(), 15);
        assert_eq!(b[12].id, BucketId::CleanAir);
        assert_eq!(b[12].name, "Unhealthy air indoors");
        assert_eq!(b[12].kind, BucketKind::Readiness);
        assert_eq!(b[12].target_kind, TargetKind::Readiness);
        assert_eq!(b[13].id, BucketId::Income);
        assert_eq!(b[13].kind, BucketKind::Money);
        assert_eq!(b[13].target_kind, TargetKind::Months);
        assert_eq!(b[14].id, BucketId::HomeLoss);
        assert_eq!(b[14].kind, BucketKind::Money);
        assert_eq!(b[14].target_kind, TargetKind::Readiness);
        let t = TierInfo::all();
        assert_eq!(t.len(), 7);
        let json = serde_json::to_value(&t).unwrap();
        assert_eq!(json[2]["id"], "w2");
        assert_eq!(json[2]["days"], 14);
    }
}
