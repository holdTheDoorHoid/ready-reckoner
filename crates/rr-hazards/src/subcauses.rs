//! Named sub-causes on the ranked hazards (REVIEW H9; hazard-expansion Deliverable A, the 52
//! rows marked "c" in `hazard-candidates.csv`).
//!
//! A sub-cause has the same consequences and event class as its parent, so it is a note on the
//! parent's card, not a hazard of its own: "Also counted here: flash floods, basement flooding,
//! ice jams". Where the pack holds a county number for one (flash-flood episodes, the tropical
//! cyclones that reached the county, people behind levees), the note carries its own rate range;
//! the parent's rate already counts it and is not raised again, so nothing is counted twice. The
//! notes say where a sub-cause is counted when it is not in the parent (levee failure is on the
//! dam and levee card; carbon monoxide is shown beside house fires, not added to them).

use rr_types::{CitationId, HazardId, SubCause};

use crate::cite;
use crate::ctx::Ctx;
use crate::params::Triple;

/// Where a sub-cause applies.
#[derive(Debug, Clone, Copy)]
enum Where {
    /// Everywhere its parent applies.
    Always,
    /// Only in these states.
    States(&'static [&'static str]),
    /// Only in these counties.
    Counties(&'static [&'static str]),
}

/// One row of the table.
struct Def {
    parent: HazardId,
    id: &'static str,
    name: &'static str,
    note: &'static str,
    /// Its own yearly rate range for a household, where the CSV gives one (PRIOR unless the pack
    /// supplies a county figure, see [`county_rate`]).
    rate: Option<Triple>,
    sources: &'static [&'static str],
    when: Where,
}

use HazardId as H;

const DEFS: &[Def] = &[
    // Flooding from rivers and heavy rain (5).
    Def {
        parent: H::RiverineFlooding,
        id: "flash_flood",
        name: "Flash flood",
        note: "Water rises in minutes on roads and near creeks; most flood deaths are people driving \
               into it. Counted in the flood rate; turn around, don't drown.",
        rate: Some((0.006, 0.001, 0.05)),
        sources: &[cite::STORM_EVENTS, cite::READY_FLOODS, cite::RR_PRIORS],
        when: Where::Always,
    },
    Def {
        parent: H::RiverineFlooding,
        id: "urban_basement_flood",
        name: "Basement or street flooding in heavy rain",
        note: "Drains overflow and water or sewage comes up into basements outside the mapped flood \
               zone. It is the outside-the-zone part of the flood rate, higher with a basement.",
        rate: Some((0.002, 0.0005, 0.005)),
        sources: &[cite::NFIP, cite::RR_PRIORS],
        when: Where::Always,
    },
    Def {
        parent: H::RiverineFlooding,
        id: "levee_failure",
        name: "Levee failure or overtopping",
        note: "Counted on the dam and levee card, not here: leveed land is mapped outside the \
               high-risk flood zone.",
        rate: None,
        sources: &[cite::USACE_NLD],
        when: Where::Always,
    },
    Def {
        parent: H::RiverineFlooding,
        id: "ice_jam_flood",
        name: "Ice-jam flood",
        note: "River ice jams in late winter and backs water into riverside homes. Counted in the \
               flood rate.",
        rate: None,
        sources: &[cite::READY_FLOODS],
        when: Where::States(&[
            "AK", "ME", "NH", "VT", "NY", "PA", "OH", "MI", "WI", "MN", "ND", "SD", "MT", "IA",
            "NE", "ID", "WY", "MA", "CT", "IL", "IN",
        ]),
    },
    Def {
        parent: H::RiverineFlooding,
        id: "glacial_outburst_flood",
        name: "Glacial outburst flood",
        note: "A lake dammed by a glacier drains suddenly and floods the valley below (Juneau's \
               Mendenhall River). Counted in the flood rate.",
        rate: None,
        sources: &[cite::READY_FLOODS],
        when: Where::Counties(&["02110"]),
    },
    // Wind, winter and roads (5).
    Def {
        parent: H::StrongWind,
        id: "derecho",
        name: "Derecho",
        note: "A fast line of storms with hurricane-force gusts over hundreds of miles; outages can \
               last days. Counted in the windstorm rate.",
        rate: None,
        sources: &[cite::STORM_EVENTS],
        when: Where::Always,
    },
    Def {
        parent: H::WinterWeather,
        id: "blizzard_lake_effect",
        name: "Blizzard or lake-effect snow",
        note: "Whiteouts or very deep snow close roads for days. Counted in the winter-storm rate.",
        rate: None,
        sources: &[cite::STORM_EVENTS, cite::READY_WINTER],
        when: Where::Always,
    },
    Def {
        parent: H::WinterWeather,
        id: "snow_load_collapse",
        name: "Roof damage under snow",
        note: "Heavy snow or ice loads a roof until it sags or fails. Counted in the winter-storm \
               rate; clear flat roofs early.",
        rate: None,
        sources: &[cite::READY_WINTER],
        when: Where::Always,
    },
    Def {
        parent: H::VehicleStranding,
        id: "black_ice_fog",
        name: "Black ice or dense fog",
        note: "Chain-reaction crashes and closed highways. Counted in the stranding rate.",
        rate: None,
        sources: &[cite::STORM_EVENTS],
        when: Where::Always,
    },
    Def {
        parent: H::VehicleStranding,
        id: "transit_shutdown",
        name: "Transit shutdown or strike",
        note: "Buses and trains stop and commuters without a car are stuck. Counted for commuters \
               who travel by transit.",
        rate: None,
        sources: &[cite::RR_PRIORS],
        when: Where::Always,
    },
    // Hurricanes, heat and cold (3).
    Def {
        parent: H::Hurricane,
        id: "inland_tropical_flood",
        name: "Inland flooding from a tropical storm",
        note: "A weakening hurricane can drop a foot or more of rain far inland (Helene in western \
               North Carolina, 2024), cutting water service for weeks.",
        rate: None,
        sources: &[cite::HURDAT2, cite::EPA_ASHEVILLE],
        when: Where::Always,
    },
    Def {
        parent: H::HeatWave,
        id: "heat_blackout_compound",
        name: "A blackout during a heat wave",
        note: "When the power fails in extreme heat, air conditioning fails for everyone at once. \
               It is the most dangerous combination; where it matters most it is a named scenario.",
        rate: None,
        sources: &[cite::STONE_2023],
        when: Where::Always,
    },
    Def {
        parent: H::ColdWave,
        id: "cold_gas_curtailment",
        name: "Rolling blackouts in extreme cold",
        note: "Gas supply and power plants fail in deep cold and the grid cuts power in rotation or \
               for days (Texas, February 2021). Already in the county's outage record.",
        rate: None,
        sources: &[cite::EAGLE_I],
        when: Where::Always,
    },
    // Networks (1).
    Def {
        parent: H::NetworkOutage,
        id: "gps_disruption",
        name: "GPS or radio disruption",
        note: "Space weather or jamming can degrade navigation and timing. Keep a paper map for the \
               way home.",
        rate: None,
        sources: &[cite::RR_PRIORS],
        when: Where::Always,
    },
    // Earthquakes and tsunamis (5).
    Def {
        parent: H::Earthquake,
        id: "liquefaction",
        name: "Liquefaction",
        note: "Wet, sandy ground turns to slurry in strong shaking; pipes and foundations fail. \
               Water outages last longer where it happens.",
        rate: None,
        sources: &[cite::USGS_NSHM, cite::READY_EARTHQUAKES],
        when: Where::Always,
    },
    Def {
        parent: H::Earthquake,
        id: "fire_following_earthquake",
        name: "Fire after an earthquake",
        note: "Broken gas lines and low water pressure let fires spread after a big earthquake. \
               Know how to shut off the gas, and leave it to the gas company to turn it back on.",
        rate: None,
        sources: &[cite::READY_EARTHQUAKES],
        when: Where::Always,
    },
    Def {
        parent: H::Earthquake,
        id: "induced_seismicity",
        name: "Earthquakes set off by wastewater injection",
        note: "Deep injection of oil-field wastewater raised earthquake rates in parts of Oklahoma \
               and Kansas; the hazard model counts them.",
        rate: None,
        sources: &[cite::USGS_NSHM],
        when: Where::States(&["OK", "KS", "TX", "AR", "CO", "NM"]),
    },
    Def {
        parent: H::Earthquake,
        id: "more_quake_scenarios",
        name: "Named earthquake scenarios",
        note: "A large earthquake on a named fault near you is listed with the scenarios, with its \
               published chance.",
        rate: None,
        sources: &[cite::USGS_NSHM],
        when: Where::States(&["UT", "CA", "WA"]),
    },
    Def {
        parent: H::Tsunami,
        id: "fjord_landslide_tsunami",
        name: "Landslide tsunami in a fjord",
        note: "A slope falling into a fjord can send a wave through nearby harbours within minutes \
               (the Barry Arm slide in Prince William Sound). Walk to high ground if the ground \
               shakes or the water pulls back.",
        rate: None,
        sources: &[cite::USGS_BARRY_ARM],
        when: Where::States(&["AK"]),
    },
    // Local utilities (4).
    Def {
        parent: H::LocalUtilityOutage,
        id: "harmful_algal_bloom",
        name: "Toxic algae in the water supply",
        note: "Algae toxins reach the treatment plant and a do-not-drink order follows (Toledo, \
               2014). Counted in the water-notice rate.",
        rate: None,
        sources: &[cite::EPA_BWA],
        when: Where::Always,
    },
    Def {
        parent: H::LocalUtilityOutage,
        id: "gas_distribution_explosion",
        name: "Gas main over-pressure or leak",
        note: "Gas mains over-pressure or leak and homes must be evacuated (Merrimack Valley, \
               2018). If you smell gas, leave first, then call.",
        rate: None,
        sources: &[cite::READY_HOME_FIRES],
        when: Where::Always,
    },
    Def {
        parent: H::LocalUtilityOutage,
        id: "major_water_system_failure",
        name: "Water system failure for weeks",
        note: "A treatment plant or system fails for weeks (Jackson, Mississippi, 2022; Asheville, \
               2024). The water targets use the county's drinking-water violations to size it.",
        rate: None,
        sources: &[cite::EPA_ASHEVILLE, cite::EPA_BWA],
        when: Where::Always,
    },
    Def {
        parent: H::LocalUtilityOutage,
        id: "wastewater_failure",
        name: "Sewer failure",
        note: "Pumps stop or mains break and toilets cannot be flushed. The sanitation supplies \
               follow the water target.",
        rate: None,
        sources: &[cite::EPA_BWA],
        when: Where::Always,
    },
    // Disease (3).
    Def {
        parent: H::Pandemic,
        id: "vector_borne_outbreak",
        name: "Mosquito- or tick-borne outbreak",
        note: "Local dengue or West Nile spread brings spraying and advisories.",
        rate: None,
        sources: &[cite::READY_PANDEMIC],
        when: Where::Always,
    },
    Def {
        parent: H::Pandemic,
        id: "local_epidemic",
        name: "Local disease outbreak",
        note: "Measles, hepatitis A or Legionnaires' in the community keeps people out of school \
               or work. Check your vaccinations.",
        rate: None,
        sources: &[cite::READY_PANDEMIC],
        when: Where::Always,
    },
    Def {
        parent: H::Pandemic,
        id: "h5n1_influenza",
        name: "Bird flu (H5N1)",
        note: "A candidate pandemic strain that so far passes from animals to people, not between \
               people.",
        rate: None,
        sources: &[cite::CDC_H5N1],
        when: Where::Always,
    },
    // Chemical releases (6).
    Def {
        parent: H::HazmatRelease,
        id: "rmp_facility_release",
        name: "Release from a chemical plant",
        note: "A plant that uses large amounts of toxic or flammable chemicals has a release or \
               explosion (West, Texas, 2013). Counted in the chemical-release rate.",
        rate: None,
        sources: &[cite::EPA_TRI, cite::READY_CHEMICAL],
        when: Where::Always,
    },
    Def {
        parent: H::HazmatRelease,
        id: "rail_hazmat",
        name: "Train derailment with chemicals",
        note: "A train carrying chemicals derails and burns or leaks; homes within a mile or two \
               are evacuated (East Palestine, 2023).",
        rate: None,
        sources: &[cite::READY_CHEMICAL],
        when: Where::Always,
    },
    Def {
        parent: H::HazmatRelease,
        id: "highway_hazmat",
        name: "Tanker truck spill",
        note: "A tanker crash closes roads and may bring an order to stay inside nearby.",
        rate: None,
        sources: &[cite::READY_CHEMICAL],
        when: Where::Always,
    },
    Def {
        parent: H::HazmatRelease,
        id: "pipeline_rupture",
        name: "Pipeline rupture",
        note: "A gas or oil transmission pipeline ruptures and burns or spills; nearby homes are \
               evacuated.",
        rate: None,
        sources: &[cite::READY_CHEMICAL],
        when: Where::Always,
    },
    Def {
        parent: H::HazmatRelease,
        id: "refinery_port_fire",
        name: "Refinery, tank farm or port fire",
        note: "A large industrial fire brings orders to stay inside and smoke for days.",
        rate: None,
        sources: &[cite::EPA_TRI, cite::READY_CHEMICAL],
        when: Where::Always,
    },
    Def {
        parent: H::HazmatRelease,
        id: "chemical_water_contamination",
        name: "Chemical spill into the water supply",
        note: "A spill upstream of a water intake brings a do-not-use order for days (Elk River, \
               West Virginia, 2014). Counted in the chemical-release rate.",
        rate: None,
        sources: &[cite::EPA_BWA],
        when: Where::Always,
    },
    // Dams (1).
    Def {
        parent: H::DamFailure,
        id: "mine_tailings_failure",
        name: "Mine tailings or coal slurry dam failure",
        note: "A mining-waste dam fails and sends slurry downstream. Counted with the other dams in \
               the inventory.",
        rate: None,
        sources: &[cite::USACE_NID],
        when: Where::Always,
    },
    // Computers and payments (2).
    Def {
        parent: H::CyberOutage,
        id: "software_failure",
        name: "Critical software failure",
        note: "A faulty update or cloud outage takes down hospital, airline, bank or 911 systems \
               for a day or more (July 2024). Counted in this rate.",
        rate: None,
        sources: &[cite::READY_CYBER],
        when: Where::Always,
    },
    Def {
        parent: H::CyberOutage,
        id: "payment_outage",
        name: "Card or bank payment outage",
        note: "Cards, bank apps or cash machines stop for hours to days. Keep some cash in small \
               bills.",
        rate: None,
        sources: &[cite::READY_CYBER],
        when: Where::Always,
    },
    // Supplies (2).
    Def {
        parent: H::SupplyChainDisruption,
        id: "fuel_shortage",
        name: "Fuel shortage",
        note: "Gas stations run dry for days (the Colonial Pipeline shutdown, May 2021, and before \
               hurricanes). It matters most with a generator or a long drive.",
        rate: None,
        sources: &[cite::RR_PRIORS],
        when: Where::Always,
    },
    Def {
        parent: H::SupplyChainDisruption,
        id: "community_isolation",
        name: "Cut off: an island or a single road",
        note: "A bridge, ferry or the only road closes and the community is cut off for days. \
               Plan for longer if that is you.",
        rate: None,
        sources: &[cite::RR_PRIORS],
        when: Where::Always,
    },
    // Medical emergencies (3).
    Def {
        parent: H::MedicalEmergency,
        id: "ed_access",
        name: "Far from emergency care",
        note: "Ambulances take about twice as long to reach rural homes (a median of about 14 \
               minutes against 7).",
        rate: None,
        sources: &[cite::MELL_2017_EMS],
        when: Where::Always,
    },
    Def {
        parent: H::MedicalEmergency,
        id: "falls_older_adults",
        name: "Falls",
        note: "Falls are a leading reason older adults need emergency care; clear floors and light \
               the way at night.",
        rate: None,
        sources: &[cite::NHAMCS_ED],
        when: Where::Always,
    },
    Def {
        parent: H::MedicalEmergency,
        id: "birth_during_disaster",
        name: "Going into labour during a disaster",
        note: "Labour can start while roads are closed or the power is out. Know two ways to the \
               hospital and pack the go-bag early.",
        rate: None,
        sources: &[cite::CDC_PREGNANCY],
        when: Where::Always,
    },
    // The grid (2).
    Def {
        parent: H::GridFailure,
        id: "grid_physical_attack",
        name: "Attack on power substations",
        note: "Gunfire or sabotage at substations can cut power to a county for days (Moore County, \
               North Carolina, 2022). Utilities report about 78 cases of attack, vandalism or theft \
               a year, most with no outage (2019–2023).",
        rate: None,
        sources: &[cite::PNNL_OE417],
        when: Where::Always,
    },
    Def {
        parent: H::GridFailure,
        id: "grid_cyberattack",
        name: "Cyberattack on the grid",
        note: "Hackers switched off parts of Ukraine's grid in 2015; no US case has cut power. \
               Utilities report about 7 cyber events a year (2019–2023).",
        rate: None,
        sources: &[cite::PNNL_OE417],
        when: Where::Always,
    },
    // Medicine (1).
    Def {
        parent: H::DrugShortage,
        id: "medical_supply_shock",
        name: "Hospital supply shock",
        note: "A key hospital supply runs short (IV fluids after Hurricane Helene closed a plant \
               making about 60 % of the US supply in 2024) and planned care is delayed.",
        rate: None,
        sources: &[cite::IV_FLUIDS_2024],
        when: Where::Always,
    },
    // Income (2).
    Def {
        parent: H::JobLoss,
        id: "livestock_disease",
        name: "Livestock or poultry disease",
        note: "An animal-disease outbreak (bird flu in dairy or poultry) can hit a farm's income.",
        rate: None,
        sources: &[cite::CDC_H5N1],
        when: Where::Always,
    },
    Def {
        parent: H::JobLoss,
        id: "recession_layoffs",
        name: "Recession layoffs",
        note: "Layoffs cluster in downturns; the high end of the job-loss range is a recession \
               year.",
        rate: None,
        sources: &[cite::BLS_JOLTS],
        when: Where::Always,
    },
    // Unrest and crime (2).
    Def {
        parent: H::CivilUnrest,
        id: "political_violence_elections",
        name: "Unrest around elections",
        note: "Protests, clashes or curfews around elections or political events.",
        rate: None,
        sources: &[cite::CSIS_TERRORISM],
        when: Where::Always,
    },
    Def {
        parent: H::Burglary,
        id: "identity_theft_fraud",
        name: "Identity theft and disaster scams",
        note: "Accounts opened in your name, or fake contractors and FEMA impostors after a \
               disaster. A credit freeze is free.",
        rate: None,
        sources: &[cite::FTC_SENTINEL, cite::FTC_DISASTER_SCAMS],
        when: Where::Always,
    },
    // Water damage (1).
    Def {
        parent: H::WaterDamage,
        id: "sewer_backup",
        name: "Sewer backup",
        note: "Sewage backs up through basement drains; standard home policies leave it out \
               unless you add sewer-backup cover.",
        rate: None,
        sources: &[cite::III_WATER],
        when: Where::Always,
    },
    // House fire (2).
    Def {
        parent: H::HouseFire,
        id: "co_poisoning",
        name: "Carbon monoxide poisoning",
        note: "Carbon monoxide from a furnace, generator or car builds up indoors, most often after \
               storms when generators run too close to the house. Shown beside fires, not added to \
               them; a CO alarm is the defence.",
        rate: Some((1.5e-4, 7.0e-5, 3.0e-4)),
        sources: &[cite::CDC_CO, cite::CDC_CO_BASICS],
        when: Where::Always,
    },
    Def {
        parent: H::HouseFire,
        id: "battery_fire",
        name: "Lithium battery fire",
        note: "E-bike, scooter and power-bank batteries can catch fire while charging; charge them \
               where you can see them.",
        rate: None,
        sources: &[cite::READY_HOME_FIRES],
        when: Where::Always,
    },
    // Earner loss (1).
    Def {
        parent: H::EarnerDeathOrDisability,
        id: "household_death_nonearner",
        name: "Death in the family",
        note: "A family member who does not earn dies: funeral costs and time off work.",
        rate: None,
        sources: &[cite::NCHS_ACCIDENTS],
        when: Where::Always,
    },
];

/// How many sub-cause rows the table holds (52 in the CSV, less the one that lives on a rare
/// family: bank failure, under the financial crisis).
#[cfg(test)]
const COUNT: usize = 51;

/// A county figure for a sub-cause, where the pack has one: flash floods from the county's
/// Storm Events episodes (× the CSV's 0.3–5 % footprint); the tropical cyclones that reached the
/// county.
fn county_rate(ctx: &Ctx<'_>, id: &str) -> Option<[f64; 2]> {
    match id {
        "flash_flood" => ctx.event("flash_flood").map(|e| {
            let r = f64::from(e.rate_per_year);
            [r * 0.003, r * 0.05]
        }),
        "inland_tropical_flood" => ctx.event("tropical_cyclone_impact").map(|e| {
            let r = f64::from(e.rate_per_year);
            [r * 0.3, r * 0.8]
        }),
        _ => None,
    }
}

fn applies(ctx: &Ctx<'_>, w: Where) -> bool {
    match w {
        Where::Always => true,
        Where::States(list) => list.contains(&ctx.county.state_abbr.as_str()),
        Where::Counties(list) => list.contains(&ctx.county.fips.as_str()),
    }
}

/// The sub-causes to name on `hazard`'s card for this household.
pub(crate) fn for_hazard(ctx: &Ctx<'_>, hazard: HazardId) -> Vec<SubCause> {
    DEFS.iter()
        .filter(|d| d.parent == hazard && applies(ctx, d.when))
        .map(|d| {
            let mut sources: Vec<CitationId> =
                d.sources.iter().map(|s| CitationId::from(*s)).collect();
            let rate_range = match county_rate(ctx, d.id) {
                Some(r) => {
                    let s = CitationId::from(cite::STORM_EVENTS);
                    if !sources.contains(&s) {
                        sources.push(s);
                    }
                    Some(r)
                }
                None => d.rate.map(|t| [t.1, t.2]),
            };
            SubCause {
                id: d.id.to_owned(),
                name: d.name.to_owned(),
                note: d.note.to_owned(),
                rate_range,
                sources,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_the_csv_sub_causes_once_each() {
        assert_eq!(DEFS.len(), COUNT);
        let mut ids: Vec<&str> = DEFS.iter().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), DEFS.len(), "a sub-cause is listed twice");
        for d in DEFS {
            assert!(rr_types::is_well_formed_id(d.id), "{}", d.id);
            assert!(!d.sources.is_empty(), "{}", d.id);
            assert!(d.note.ends_with('.') && !d.note.contains("  "), "{}", d.id);
            assert!(
                !d.parent.is_rare(),
                "{}: rare parents carry their own",
                d.id
            );
            if let Some((v, lo, hi)) = d.rate {
                assert!(0.0 < lo && lo <= v && v <= hi, "{}", d.id);
            }
        }
    }
}
