//! Data-pack v2 records this crate reads from the outage workstream: the regional outage model,
//! the worst-event stress line, the pooled restoration curves and the temperature shares. The
//! types are `rr_types::calibration`'s; `crate::CountyData::from_record` reads a county's
//! `outage_model` and `temperature`, and the restoration curves, which are not per county, come
//! from `rr_data::DataStore::restoration_curves()` through [`crate::CountyData::with_curves`]
//! (`rr-plan` passes them). The drinking-water and smoke columns come from
//! `CountyRecord::exposure`.

pub use rr_types::{
    OUTAGE_MARKS_DAYS, OutageModel, PoolBasis, RestorationCurve, StressEvent, TemperatureProfile,
};

/// The outage-pooling region of a county, as `rr_data::outage_region` names it: its NCA5 region,
/// except that Puerto Rico and the US Virgin Islands (separate grids) are regions of their own
/// (taking the two fields, since this crate does not depend on `rr-data`).
pub fn outage_region(state_abbr: &str, nca_region: &str) -> String {
    match state_abbr {
        "PR" => "puerto_rico".to_owned(),
        "VI" => "virgin_islands".to_owned(),
        _ => nca_region.to_owned(),
    }
}

/// The pooled restoration curve for `region` and `class`, if the pack has it.
pub fn curve<'c>(
    curves: &'c [RestorationCurve],
    region: &str,
    class: &str,
) -> Option<&'c RestorationCurve> {
    curves
        .iter()
        .find(|c| c.region == region && c.class == class)
}

/// A hand-copied historic curve (`historic:<id>`) for `region`, if the pack has one: Hurricane
/// Maria for Puerto Rico, Irma and Maria for the US Virgin Islands (island grids, M-10).
pub fn historic_curve<'c>(
    curves: &'c [RestorationCurve],
    region: &str,
) -> Option<&'c RestorationCurve> {
    curves
        .iter()
        .find(|c| c.region == region && c.class.starts_with("historic:"))
}
