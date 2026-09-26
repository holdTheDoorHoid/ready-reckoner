//! One hazard's household rate, today and around 2050, with what the register needs to show it.

use rr_types::{DataConfidence, HazardDisplay, HazardId};

use crate::climate::Climate;
use crate::estimate::Estimate;

/// One hazard's household rate r_h = λ_h · a_h · m_h and what the register needs to show it.
#[derive(Debug, Clone)]
pub(crate) struct HazardRate {
    /// Which hazard.
    pub hazard: HazardId,
    /// Household-significant events a year in today's climate.
    pub today: Estimate,
    /// The same around 2050 (equal to `today` unless a projection applies).
    pub future: Estimate,
    /// How the 2050 dial treats this hazard.
    pub climate: Climate,
    /// County-average household rate today, without this household's modifiers: the
    /// denominator for the per-event loss behind severity.
    pub county_average: f64,
    /// NRI expected annual loss per household, in US dollars (natural hazards).
    pub eal_per_household: Option<f64>,
    /// Loss per event in US dollars used for severity when there is no expected annual loss
    /// (PRIOR).
    pub default_loss_usd: f64,
    /// Completes "Of 100 households like yours, about N will …".
    pub verb: String,
    /// Ranked list or the rare-catastrophe box.
    pub display: HazardDisplay,
    /// For rare catastrophes: the range-only sentence that replaces the natural frequency.
    pub range_sentence: Option<String>,
    /// For rare catastrophes: severity fixed rather than derived.
    pub fixed_severity: Option<f64>,
    /// For rare catastrophes: confidence fixed rather than derived from the range.
    pub fixed_confidence: Option<DataConfidence>,
}

impl HazardRate {
    /// A ranked hazard with no climate adjustment.
    pub fn new(
        hazard: HazardId,
        today: Estimate,
        verb: impl Into<String>,
        default_loss_usd: f64,
    ) -> Self {
        HazardRate {
            hazard,
            county_average: today.value,
            future: today.clone(),
            today,
            climate: Climate::NotApplicable,
            eal_per_household: None,
            default_loss_usd,
            verb: verb.into(),
            display: HazardDisplay::Ranked,
            range_sentence: None,
            fixed_severity: None,
            fixed_confidence: None,
        }
    }

    /// Applies a climate treatment: `future = today × multiplier`.
    pub fn with_climate(mut self, climate: Climate) -> Self {
        self.future = self.today.times(&climate.multiplier());
        self.climate = climate;
        self
    }

    /// Sets the county-average rate used for severity.
    pub fn with_county_average(mut self, rate: f64) -> Self {
        self.county_average = rate;
        self
    }

    /// Sets the NRI expected annual loss per household.
    pub fn with_eal(mut self, eal: Option<f64>) -> Self {
        self.eal_per_household = eal;
        self
    }

    /// The rate the plan uses: around 2050 when that dial is on, otherwise today.
    pub fn effective(&self, y2050: bool) -> &Estimate {
        if y2050 { &self.future } else { &self.today }
    }
}
