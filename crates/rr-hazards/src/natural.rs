//! Natural hazards: r_h = λ_h · a_h · m_h from the county record (DESIGN §4.2, research §3.1).
//!
//! - λ_h, the county frequency: the per-county episode rate from the `events` table when the
//!   pack has one (NOAA Storm Events: `heat`, `extreme_cold`, `winter_storm`, `ice_storm`,
//!   `high_wind` + SPC `severe_wind_day`), otherwise NRI `AFREQ` read with the record's
//!   `afreq_kind`. NRI counts heat waves, cold waves and winter weather in event-days, so those
//!   are divided by a typical episode length. Earthquakes use the USGS shaking chance in
//!   `county.seismic` in preference to NRI.
//! - a_h, the footprint (the chance one county event reaches this household): derived where the
//!   record allows it (NRI exposed population ÷ county population for sub-county hazards; NRI
//!   historic loss ratio ÷ a typical damage ratio for hail, tornado and landslide; flood-zone
//!   share for floods; a floor from recorded outages for windstorms), otherwise the PRIOR table
//!   in `params`.
//! - m_h, the household modifier: floor and basement (floods), setting (power-line exposure,
//!   wildfire), well or public water (drought). Heat and cold waves reach every household in the
//!   county; the home's cooling and heating change what they do, which `rr-consequence` applies.
//!
//! What one "household-significant event" means for each hazard is in the `verb` of each rate
//! and in `docs/RISK_MODEL.md` § "Hazard rates".

use rr_types::{HazardId, math};

use crate::cite;
use crate::climate::{self, Climate};
use crate::ctx::{Ctx, Notes};
use crate::estimate::Estimate;
use crate::params::*;
use crate::rate::HazardRate;

/// The hurricane rate split into Category 1–2 and major (Category 3+) parts, today and around
/// 2050. The `major_hurricane_direct_hit` scenario takes the major part.
#[derive(Debug, Clone)]
pub(crate) struct HurricaneSplit {
    /// NRI (or Storm Events) hurricane events a year in the county.
    pub county_rate: f64,
    /// Category 1–2 and tropical-storm household events, today.
    pub cat12_today: Estimate,
    /// The same around 2050.
    pub cat12_future: Estimate,
    /// Major-hurricane household events, today.
    pub major_today: Estimate,
    /// The same around 2050.
    pub major_future: Estimate,
}

/// All natural-hazard rates for one household.
#[derive(Debug, Clone, Default)]
pub(crate) struct Natural {
    /// One rate per natural hazard the county has, in `HazardId` order.
    pub rates: Vec<HazardRate>,
    /// The hurricane split, when the county has hurricanes.
    pub hurricane: Option<HurricaneSplit>,
}

fn spread(value: f64, factor: f64, sources: &[&str]) -> Estimate {
    Estimate::data(value, value / factor, value * factor, sources)
}

/// 1 / (days per episode), as an expert estimate.
fn per_episode(days: Triple) -> Estimate {
    Estimate::prior(
        1.0 / days.0,
        1.0 / days.2,
        1.0 / days.1,
        &[cite::RR_HAZARD_PRIORS],
    )
}

/// 1 − s, keeping the evidence and sources of s.
fn complement(s: &Estimate) -> Estimate {
    let mut out = s.clone();
    out.value = 1.0 - s.value;
    out.low = 1.0 - s.high;
    out.high = 1.0 - s.low;
    out
}

/// Caps an estimate's value and range at `max`.
fn cap(mut e: Estimate, max: f64) -> Estimate {
    e.value = e.value.min(max);
    e.low = e.low.min(max);
    e.high = e.high.min(max);
    e
}

/// Keys in `CountyRecord::events` that count a hazard's episodes (the data pack's event types,
/// then the hazard id). Windstorms add Storm Events high-wind episodes and SPC severe-wind days.
/// Hail, tornadoes and landslides use NRI's frequency, because their footprint comes from NRI's
/// loss ratio per NRI event.
fn event_keys(hazard: HazardId) -> &'static [&'static str] {
    use HazardId::*;
    match hazard {
        HeatWave => &["heat", "heat_wave"],
        ColdWave => &["extreme_cold", "cold_wave"],
        WinterWeather => &["winter_storm", "winter_weather"],
        IceStorm => &["ice_storm"],
        StrongWind => &["high_wind", "severe_wind_day", "strong_wind"],
        _ => &[],
    }
}

/// Episodes a year from the events table: the first key present, except windstorms, where the
/// pack's two wind records (high-wind episodes, severe-wind days) are added.
fn events_rate(ctx: &Ctx<'_>, hazard: HazardId) -> Option<f64> {
    let keys = event_keys(hazard);
    if hazard == HazardId::StrongWind {
        let pack: f64 = ["high_wind", "severe_wind_day"]
            .iter()
            .filter_map(|k| ctx.event(k))
            .map(|e| f64::from(e.rate_per_year))
            .sum();
        if pack > 0.0 {
            return Some(pack);
        }
    }
    keys.iter()
        .find_map(|k| ctx.event(k))
        .map(|e| f64::from(e.rate_per_year))
}

/// County events a year: the Storm Events episode rate when the pack has one, otherwise NRI.
fn frequency(ctx: &Ctx<'_>, hazard: HazardId) -> Option<Estimate> {
    if let Some(r) = events_rate(ctx, hazard) {
        return Some(spread(r, STORM_EVENTS_SPREAD, &[cite::STORM_EVENTS]));
    }
    let a = ctx.afreq_rate(hazard)?;
    Some(spread(a, NRI_FREQUENCY_SPREAD, &[cite::NRI]))
}

/// NRI frequency only (for hazards whose footprint is NRI's loss ratio per NRI event).
fn nri_frequency(ctx: &Ctx<'_>, hazard: HazardId) -> Option<Estimate> {
    let a = ctx.afreq_rate(hazard)?;
    Some(spread(a, NRI_FREQUENCY_SPREAD, &[cite::NRI]))
}

/// Episodes a year for an event-day hazard: Storm Events episodes, or NRI event-days ÷ a typical
/// episode length. Also returns the event-days a year.
fn episodes(ctx: &Ctx<'_>, hazard: HazardId, days: Triple) -> Option<(Estimate, f64)> {
    if let Some(r) = events_rate(ctx, hazard) {
        return Some((
            spread(r, STORM_EVENTS_SPREAD, &[cite::STORM_EVENTS]),
            r * days.0,
        ));
    }
    let event_days = ctx.afreq_rate(hazard)?;
    let lam = spread(event_days, NRI_FREQUENCY_SPREAD, &[cite::NRI]);
    Some((lam.times(&per_episode(days)), event_days))
}

/// Share of residents exposed: NRI exposed population ÷ population, else a PRIOR.
fn exposure(ctx: &Ctx<'_>, hazard: HazardId) -> Estimate {
    match ctx.exposure_share(hazard) {
        Some(s) => Estimate::data(s, s, s, &[cite::NRI]),
        None => prior(EXPOSURE_FALLBACK_SHARE, &[cite::RR_HAZARD_PRIORS]),
    }
}

/// NRI historic loss ratio ÷ a PRIOR damage ratio: the chance one event damages a given
/// exposed home.
fn damage_footprint(ctx: &Ctx<'_>, hazard: HazardId, damage_ratio: Triple) -> Option<Estimate> {
    let h = ctx.hlrb(hazard)?;
    let ratio = prior(damage_ratio, &[cite::RR_HAZARD_PRIORS]);
    let inv = Estimate::new(
        1.0 / ratio.value,
        1.0 / ratio.high,
        1.0 / ratio.low,
        ratio.evidence,
        &[cite::RR_HAZARD_PRIORS],
    );
    Some(cap(Estimate::data(h, h, h, &[cite::NRI]).times(&inv), 1.0))
}

/// The household's flood exposure by floor: upper floors stay dry, basements take water.
fn flood_floor(ctx: &Ctx<'_>) -> (Estimate, bool) {
    let housing = &ctx.input.housing;
    if !ctx.ground_level() {
        (prior(FLOOD_UPPER_FLOOR, &[cite::RR_HAZARD_PRIORS]), false)
    } else if housing.basement {
        (prior(FLOOD_BASEMENT, &[cite::RR_HAZARD_PRIORS]), true)
    } else {
        (Estimate::exact(1.0), true)
    }
}

fn heat_wave(ctx: &Ctx<'_>, notes: &mut Notes) -> Option<HazardRate> {
    let h = HazardId::HeatWave;
    let (episodes, event_days) = if let Some(r) = events_rate(ctx, h) {
        (
            spread(r, STORM_EVENTS_SPREAD, &[cite::STORM_EVENTS]),
            r * HEAT_EPISODE_DAYS.0,
        )
    } else {
        let mut days = ctx.afreq_rate(h)?;
        let mut lam_sources = vec![cite::NRI];
        if let Some(d90) = ctx
            .county
            .climate
            .get("days_over_90f_hist")
            .map(|v| f64::from(*v))
            .filter(|v| v.is_finite() && *v >= 0.0)
        {
            let ceiling = d90.max(HEAT_DAYS_CAP_FLOOR);
            if days > ceiling {
                notes.add(format!(
                    "Heat waves: {} records about {days:.1} heat-wave days a year, but its climate \
                     record has only about {ceiling:.1} days a year over 90 °F, so the smaller \
                     number was used.",
                    ctx.county_label()
                ));
                days = ceiling;
                lam_sources.push(cite::CMRA);
            }
        }
        let lam = spread(days, NRI_FREQUENCY_SPREAD, &lam_sources);
        (lam.times(&per_episode(HEAT_EPISODE_DAYS)), days)
    };
    // A heat wave reaches every household in the county. What it does depends on the home's
    // cooling (no air conditioning: dangerous heat indoors; air conditioning: danger only if the
    // power fails), which rr-consequence applies with its coupling rules.
    Some(
        HazardRate::new(h, episodes, "go through a heat wave", 500.0)
            .with_eal(ctx.eal_per_household(h))
            .with_climate(climate::treatment(
                ctx.county,
                h,
                event_days,
                ctx.ground_level(),
            )),
    )
}

fn cold_wave(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::ColdWave;
    let (episodes, _) = episodes(ctx, h, COLD_EPISODE_DAYS)?;
    // Like heat waves, a cold wave reaches every household; rr-consequence decides what it does
    // given the home's heating.
    Some(
        HazardRate::new(h, episodes, "go through a spell of dangerous cold", 500.0)
            .with_eal(ctx.eal_per_household(h))
            .with_climate(climate::treatment(ctx.county, h, 0.0, ctx.ground_level())),
    )
}

fn winter_weather(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::WinterWeather;
    let (episodes, _) = episodes(ctx, h, WINTER_EPISODE_DAYS)?;
    let today = episodes.times(&prior(WINTER_FOOTPRINT, &[cite::RR_HAZARD_PRIORS]));
    Some(
        HazardRate::new(
            h,
            today,
            "be snowed in or lose power in a winter storm",
            200.0,
        )
        .with_eal(ctx.eal_per_household(h))
        .with_climate(climate::treatment(ctx.county, h, 0.0, ctx.ground_level())),
    )
}

/// Windstorms, ice storms and lightning: county frequency × footprint × power-line exposure.
fn utility_storm(
    ctx: &Ctx<'_>,
    hazard: HazardId,
    footprint: Triple,
    verb: &str,
    loss: f64,
) -> Option<HazardRate> {
    let lam = frequency(ctx, hazard)?;
    let base = lam.times(&prior(footprint, &[cite::RR_HAZARD_PRIORS]));
    let today = base.times(&utility_exposure(ctx.setting()));
    Some(
        HazardRate::new(hazard, today, verb, loss)
            .with_county_average(base.value)
            .with_eal(ctx.eal_per_household(hazard))
            .with_climate(Climate::Unclear),
    )
}

fn hail(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::Hail;
    let lam = nri_frequency(ctx, h)?;
    let fp = damage_footprint(ctx, h, HAIL_DAMAGE_RATIO)
        .unwrap_or_else(|| prior(HAIL_FALLBACK_FOOTPRINT, &[cite::RR_HAZARD_PRIORS]));
    Some(
        HazardRate::new(
            h,
            lam.times(&fp),
            "have hail damage their home or car",
            5_000.0,
        )
        .with_eal(ctx.eal_per_household(h))
        .with_climate(Climate::Unclear),
    )
}

fn tornado(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::Tornado;
    let lam = nri_frequency(ctx, h)?;
    let fp = match damage_footprint(ctx, h, TORNADO_DAMAGE_RATIO) {
        Some(d) => cap(
            d.times(&prior(TORNADO_DISRUPTION, &[cite::RR_HAZARD_PRIORS])),
            1.0,
        ),
        None => prior(TORNADO_FALLBACK_FOOTPRINT, &[cite::RR_HAZARD_PRIORS]),
    };
    Some(
        HazardRate::new(
            h,
            lam.times(&fp),
            "have a tornado damage or cut off their neighborhood",
            20_000.0,
        )
        .with_eal(ctx.eal_per_household(h))
        .with_climate(Climate::Unclear),
    )
}

fn hurricane(ctx: &Ctx<'_>) -> Option<(HazardRate, HurricaneSplit)> {
    let h = HazardId::Hurricane;
    let lam = frequency(ctx, h)?;
    // The major share: HURDAT2 passages within 50 nautical miles at major-hurricane strength ÷
    // those at hurricane strength, when the pack has both (DERIVED for this county); otherwise
    // the national one-third (PRIOR informed by HURDAT2).
    let passages = |k: &str| ctx.event(k).map(|e| f64::from(e.rate_per_year));
    let share = match (
        passages("major_hurricane_passage"),
        passages("hurricane_passage"),
    ) {
        (Some(major), Some(all)) if all > 0.0 => {
            let s = (major / all).clamp(0.0, 1.0);
            Estimate::data(
                s,
                s / STORM_EVENTS_SPREAD,
                (s * STORM_EVENTS_SPREAD).min(1.0),
                &[cite::HURDAT2],
            )
        }
        _ => prior(
            MAJOR_HURRICANE_SHARE,
            &[cite::HURDAT2, cite::RR_HAZARD_PRIORS],
        ),
    };
    let a12 = prior(HURRICANE_FOOTPRINT, &[cite::RR_HAZARD_PRIORS]);
    let amaj = prior(MAJOR_HURRICANE_FOOTPRINT, &[cite::RR_HAZARD_PRIORS]);
    let split = |s: &Estimate| {
        (
            lam.times(&complement(s)).times(&a12),
            lam.times(s).times(&amaj),
        )
    };
    let (cat12_today, major_today) = split(&share);
    // Around 2050 the share of major storms rises ×1.1–1.3; how often hurricanes come does not
    // change (research §5.3–5.4).
    let intensity = prior(HURRICANE_INTENSITY, &[cite::NCA5, cite::RR_PRIORS]);
    let share_future = cap(share.times(&intensity), 0.95);
    let (cat12_future, major_future) = split(&share_future);
    let total_today = cat12_today.plus(&major_today);
    let total_future = cat12_future.plus(&major_future);
    // The card's multiplier and its range: total(k·s) / total(s) for k = 1.1 … 1.3.
    let total_at = |k: f64| {
        let s = (share.value * k).min(0.95);
        (1.0 - s) * a12.value + s * amaj.value
    };
    let base = total_at(1.0);
    let m = Estimate::prior(
        total_at(HURRICANE_INTENSITY.0) / base,
        total_at(HURRICANE_INTENSITY.1) / base,
        total_at(HURRICANE_INTENSITY.2) / base,
        &[cite::NCA5, cite::RR_PRIORS],
    );
    let mut rate = HazardRate::new(
        h,
        total_today,
        "lose power or have damage from a hurricane or tropical storm",
        3_000.0,
    )
    .with_eal(ctx.eal_per_household(h));
    rate.future = total_future;
    rate.climate = Climate::Projected {
        multiplier: m,
        what: "a larger share of hurricanes reaching major strength, 10 to 30 % more (how often \
               hurricanes come does not change)"
            .to_owned(),
    };
    let split = HurricaneSplit {
        county_rate: lam.value,
        cat12_today,
        cat12_future,
        major_today,
        major_future,
    };
    Some((rate, split))
}

fn earthquake(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::Earthquake;
    let from_seismic = ctx.county.seismic.as_ref().and_then(|s| {
        let p = f64::from(s.p_pga_ge_0_1g_per_year);
        if p > 0.0 && p < 1.0 {
            return Some(-math::ln_1p(-p));
        }
        let p100 = f64::from(s.mmi6_100yr?);
        (p100 > 0.0 && p100 < 1.0).then(|| -math::ln_1p(-p100) / 100.0)
    });
    let today = match from_seismic {
        Some(r) => spread(r, SEISMIC_SPREAD, &[cite::USGS_NSHM]),
        None => spread(ctx.afreq_rate(h)?, SEISMIC_SPREAD, &[cite::NRI]),
    };
    Some(
        HazardRate::new(
            h,
            today,
            "feel an earthquake strong enough to knock things off shelves",
            40_000.0,
        )
        .with_eal(ctx.eal_per_household(h)),
    )
}

fn tsunami(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::Tsunami;
    let lam = spread(ctx.afreq_rate(h)?, NRI_FREQUENCY_SPREAD, &[cite::NRI]);
    let today = lam
        .times(&exposure(ctx, h))
        .times(&prior(TSUNAMI_WARNING_SHARE, &[cite::RR_HAZARD_PRIORS]));
    Some(
        HazardRate::new(
            h,
            today,
            "have to leave home or work for a tsunami warning",
            20_000.0,
        )
        .with_eal(ctx.eal_per_household(h)),
    )
}

fn riverine_flooding(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::RiverineFlooding;
    let afreq = ctx.afreq_rate(h);
    let flood = ctx
        .county
        .flood
        .as_ref()
        .filter(|f| f.sfha_home_share.is_finite() && (0.0..=1.0).contains(&f.sfha_home_share));
    if afreq.is_none() && flood.is_none() {
        return None;
    }
    let s = match (flood, ctx.exposure_share(h)) {
        (Some(f), _) => {
            let s = f64::from(f.sfha_home_share);
            Estimate::data(s, s, s, &[cite::NFIP])
        }
        (None, Some(x)) => Estimate::data(x, x, x, &[cite::NRI]),
        (None, None) => prior(SFHA_SHARE_FALLBACK, &[cite::RR_HAZARD_PRIORS]),
    };
    let p_in = match flood
        .and_then(|f| f.claims_per_1000_policies_year)
        .map(f64::from)
        .filter(|c| c.is_finite() && *c > 0.0)
    {
        Some(c) => {
            let (lo, hi) = NFIP_CLAIMS_BOUNDS;
            let p = (c / 1000.0).clamp(lo, hi);
            spread(p, 1.5, &[cite::NFIP])
        }
        None => data(SFHA_ANNUAL, &[cite::FEMA_FLOOD_ZONES]),
    };
    let scale = afreq
        .map(|a| (a / INLAND_FLOOD_AFREQ_MEDIAN).clamp(0.5, 2.0))
        .unwrap_or(1.0);
    let p_out = prior(OUTSIDE_SFHA_ANNUAL, &[cite::RR_HAZARD_PRIORS]).scaled(scale);
    let base = s.times(&p_in).plus(&complement(&s).times(&p_out));
    let (m, ground) = flood_floor(ctx);
    let verb = if ground {
        "have flood water reach their home"
    } else {
        "be cut off or lose building services in a flood"
    };
    Some(
        HazardRate::new(h, base.times(&m), verb, 40_000.0)
            .with_county_average(base.value)
            .with_eal(ctx.eal_per_household(h))
            .with_climate(climate::treatment(ctx.county, h, 0.0, ground)),
    )
}

fn coastal_flooding(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::CoastalFlooding;
    ctx.afreq_rate(h)?;
    let base = exposure(ctx, h).times(&data(COASTAL_ZONE_ANNUAL, &[cite::FEMA_FLOOD_ZONES]));
    let (m, ground) = flood_floor(ctx);
    let verb = if ground {
        "have coastal flooding reach their home"
    } else {
        "be cut off or lose building services in coastal flooding"
    };
    Some(
        HazardRate::new(h, base.times(&m), verb, 40_000.0)
            .with_county_average(base.value)
            .with_eal(ctx.eal_per_household(h))
            .with_climate(climate::treatment(ctx.county, h, 0.0, ground)),
    )
}

fn landslide(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::Landslide;
    let lam = spread(ctx.afreq_rate(h)?, NRI_FREQUENCY_SPREAD, &[cite::NRI]);
    let fp = match damage_footprint(ctx, h, LANDSLIDE_DAMAGE_RATIO) {
        Some(d) => cap(
            d.times(&prior(LANDSLIDE_ACCESS, &[cite::RR_HAZARD_PRIORS])),
            1.0,
        ),
        None => prior(LANDSLIDE_FALLBACK_FOOTPRINT, &[cite::RR_HAZARD_PRIORS]),
    };
    let today = lam.times(&exposure(ctx, h)).times(&fp);
    Some(
        HazardRate::new(
            h,
            today,
            "have a landslide damage their home or cut off their road",
            30_000.0,
        )
        .with_eal(ctx.eal_per_household(h))
        .with_climate(Climate::Unclear),
    )
}

fn avalanche(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::Avalanche;
    let lam = spread(ctx.afreq_rate(h)?, NRI_FREQUENCY_SPREAD, &[cite::NRI]);
    Some(
        HazardRate::new(
            h,
            lam.times(&exposure(ctx, h)),
            "have an avalanche reach their home or road",
            30_000.0,
        )
        .with_eal(ctx.eal_per_household(h))
        .with_climate(Climate::Unclear),
    )
}

fn volcanic_activity(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::VolcanicActivity;
    let lam = spread(ctx.afreq_rate(h)?, NRI_FREQUENCY_SPREAD, &[cite::NRI]);
    let today = lam
        .times(&exposure(ctx, h))
        .times(&prior(VOLCANO_SHARE, &[cite::RR_HAZARD_PRIORS]));
    Some(
        HazardRate::new(
            h,
            today,
            "have ash or mudflows from a volcano reach them",
            20_000.0,
        )
        .with_eal(ctx.eal_per_household(h)),
    )
}

fn wildfire(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::Wildfire;
    let raw = ctx.afreq_raw(h)?;
    // NRI gives a yearly burn probability for wildfire, used directly (a count would be turned
    // into the chance of at least one).
    let p = match ctx.nri(h)?.afreq_kind {
        rr_types::AfreqKind::AnnualProbability if raw < 1.0 => raw,
        _ => -math::exp_m1(-raw),
    };
    let burn_base = spread(p, NRI_FREQUENCY_SPREAD, &[cite::NRI])
        .times(&exposure(ctx, h))
        .times(&prior(
            WILDFIRE_EVACUATIONS_PER_BURN,
            &[cite::RR_HAZARD_PRIORS],
        ));
    let burn = burn_base.times(&wildfire_setting(ctx.setting()));
    let western = PSPS_STATES.contains(&ctx.county.state_abbr.as_str());
    let (psps, psps_avg) = if western {
        let base = prior(PSPS_RATE, &[cite::RR_PRIORS]);
        let avg = base.value * psps_setting(rr_types::Setting::Suburban).value;
        (base.times(&psps_setting(ctx.setting())), avg)
    } else {
        (Estimate::exact(0.0), 0.0)
    };
    Some(
        HazardRate::new(
            h,
            burn.plus(&psps),
            "have to leave home or lose power because of a wildfire",
            30_000.0,
        )
        .with_county_average(burn_base.value + psps_avg)
        .with_eal(ctx.eal_per_household(h))
        .with_climate(climate::treatment(ctx.county, h, 0.0, ctx.ground_level())),
    )
}

fn drought(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let h = HazardId::Drought;
    let a = ctx.afreq_rate(h)?;
    let scale = (a / DROUGHT_AFREQ_MEDIAN).clamp(0.25, 4.0);
    let (base, verb, loss) = if ctx.well() {
        (
            prior(DROUGHT_WELL, &[cite::RR_PRIORS]),
            "have their well run low in a drought",
            2_000.0,
        )
    } else {
        (
            prior(DROUGHT_MUNICIPAL, &[cite::RR_HAZARD_PRIORS]),
            "face water limits in a drought",
            300.0,
        )
    };
    Some(
        HazardRate::new(h, base.scaled(scale).cite(&[cite::NRI]), verb, loss)
            .with_eal(ctx.eal_per_household(h))
            .with_climate(climate::treatment(ctx.county, h, 0.0, ctx.ground_level())),
    )
}

/// Chance that one household-significant event of a storm hazard cuts the power.
fn outage_share(hazard: HazardId) -> Option<f64> {
    use HazardId::*;
    Some(match hazard {
        StrongWind => OUTAGE_SHARE_STRONG_WIND,
        WinterWeather => OUTAGE_SHARE_WINTER,
        IceStorm => OUTAGE_SHARE_ICE,
        Hurricane => OUTAGE_SHARE_HURRICANE,
        Tornado => OUTAGE_SHARE_TORNADO,
        Lightning => OUTAGE_SHARE_LIGHTNING,
        Hail => OUTAGE_SHARE_HAIL,
        _ => return None,
    })
}

/// The outage floor: when recorded outages (EAGLE-I) show more storm power cuts than the storm
/// rates explain, the difference is counted as windstorms, the most common cause.
fn outage_floor(ctx: &Ctx<'_>, rates: &mut Vec<HazardRate>, notes: &mut Notes) {
    let county = ctx.county_label();
    let Some(o) = ctx.county.outages.as_ref() else {
        notes.add(format!(
            "No power-outage records for {county}: the storm power-cut chances rest on \
             estimates."
        ));
        return;
    };
    let recorded = f64::from(o.events_per_customer_year);
    if !(recorded.is_finite() && recorded > 0.0) {
        return;
    }
    let weather = spread(recorded, OUTAGE_RATE_SPREAD, &[cite::EAGLE_I]).times(&prior(
        OUTAGE_WEATHER_SHARE,
        &[cite::DO_2023, cite::RR_HAZARD_PRIORS],
    ));
    let modelled: f64 = rates
        .iter()
        .filter_map(|r| outage_share(r.hazard).map(|c| r.today.value * c))
        .sum();
    if weather.value <= modelled {
        return;
    }
    let c = OUTAGE_SHARE_STRONG_WIND;
    let mut extra = weather.clone();
    extra.value = (weather.value - modelled) / c;
    extra.low = ((weather.low - modelled) / c).max(0.0);
    extra.high = (weather.high - modelled) / c;
    notes.add(format!(
        "Power-outage records ({}) show homes in {county} caught in an outage about {} times a \
         year, more often than the county's storm records explain. The extra outages are \
         counted as windstorms, the most common cause.",
        o.years_covered,
        crate::sentence::sig2(recorded)
    ));
    match rates.iter_mut().find(|r| r.hazard == HazardId::StrongWind) {
        Some(wind) => {
            wind.today = wind.today.plus(&extra);
            wind.future = wind.today.clone();
            wind.county_average += extra.value;
        }
        None => rates.push(
            HazardRate::new(
                HazardId::StrongWind,
                extra,
                "lose power or have damage in a windstorm",
                300.0,
            )
            .with_climate(Climate::Unclear),
        ),
    }
}

/// Every natural hazard the county has, for this household, today and around 2050.
pub(crate) fn assess(ctx: &Ctx<'_>, notes: &mut Notes) -> Natural {
    let mut out = Natural::default();
    for &h in HazardId::ALL {
        use HazardId::*;
        let rate = match h {
            Avalanche => avalanche(ctx),
            CoastalFlooding => coastal_flooding(ctx),
            ColdWave => cold_wave(ctx),
            Drought => drought(ctx),
            Earthquake => earthquake(ctx),
            Hail => hail(ctx),
            HeatWave => heat_wave(ctx, notes),
            Hurricane => hurricane(ctx).map(|(r, split)| {
                out.hurricane = Some(split);
                r
            }),
            IceStorm => utility_storm(
                ctx,
                h,
                ICE_STORM_FOOTPRINT,
                "lose power or be stuck at home in an ice storm",
                500.0,
            ),
            Landslide => landslide(ctx),
            Lightning => utility_storm(
                ctx,
                h,
                LIGHTNING_FOOTPRINT,
                "have lightning damage their home or cut their power",
                1_000.0,
            ),
            RiverineFlooding => riverine_flooding(ctx),
            StrongWind => utility_storm(
                ctx,
                h,
                STRONG_WIND_FOOTPRINT,
                "lose power or have damage in a windstorm",
                300.0,
            ),
            Tornado => tornado(ctx),
            Tsunami => tsunami(ctx),
            VolcanicActivity => volcanic_activity(ctx),
            Wildfire => wildfire(ctx),
            WinterWeather => winter_weather(ctx),
            _ => None,
        };
        out.rates.extend(rate);
    }
    outage_floor(ctx, &mut out.rates, notes);
    out.rates.sort_by_key(|r| r.hazard);
    out
}
