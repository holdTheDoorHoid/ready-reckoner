//! "Why we think this": one short, plain-language reason for each hazard's rate, for the expert
//! drawer behind a hazard card (research §6.2: show priors "with a 'why we think this' note and a
//! user override"). The numbers match `params` and `docs/RISK_MODEL.md` § "Hazard rates".

use rr_types::HazardId;

/// Why the engine uses the rate it does for `hazard`, in plain language.
#[allow(deprecated)] // the retired `terrorism` still needs an arm while the id exists
pub fn why_we_think_this(hazard: HazardId) -> &'static str {
    use HazardId::*;
    match hazard {
        Avalanche => {
            "FEMA's National Risk Index counts avalanches in your county and how many people live \
             where they can reach. Few homes are in avalanche paths."
        }
        CoastalFlooding => {
            "FEMA's National Risk Index says how many people in your county live where the sea can \
             flood. Homes in a flood zone have about a 1 in 100 chance of flooding each year."
        }
        ColdWave => {
            "Your county's weather records count the spells of dangerous cold each year. Every home \
             in the county goes through them; what they do depends on your heating."
        }
        Drought => {
            "Droughts rarely change daily life on a public water system. A private well can run low. \
             This is an expert estimate, scaled by how often your county has drought."
        }
        Earthquake => {
            "The US Geological Survey's hazard model gives the yearly chance of shaking strong \
             enough to knock things off shelves where you live."
        }
        Hail => {
            "FEMA's National Risk Index records hail in your county and the damage it has done. We \
             turn the damage into the chance that hail damages one home."
        }
        HeatWave => {
            "Your county's weather records count heat waves each year. Every home in the county goes \
             through them; what they do depends on your cooling and your power."
        }
        Hurricane => {
            "FEMA's National Risk Index counts hurricanes and tropical storms that reach your \
             county. We estimate that about half of them cut a home's power or close roads, and \
             about 1 in 3 are major storms."
        }
        IceStorm => {
            "Your county's records count ice storms. We estimate that about 1 in 10 cut a given \
             home's power or keep the household in (fewer in a city, more in the country)."
        }
        Landslide => {
            "FEMA's National Risk Index counts landslides in your county and the homes near steep \
             ground. Few homes are damaged, but more have their road cut off."
        }
        Lightning => {
            "Your county has many days with lightning, but a strike rarely damages a home or cuts \
             its power: about 1 home in 10,000 on a lightning day, by our estimate."
        }
        RiverineFlooding => {
            "Homes in a mapped flood zone have at least a 1 in 100 chance of flooding each year; \
             homes outside it flood less often, but it happens, and basements flood more."
        }
        StrongWind => {
            "Your county's records count windstorms. We estimate about 6 in 100 cut a given home's \
             power (half that in a city, twice in the country), and raise the number to match your \
             county's recorded power cuts where we have them."
        }
        Tornado => {
            "FEMA's National Risk Index records tornadoes in your county and their damage. A \
             tornado's path is narrow, so we turn the damage into the chance one neighborhood is hit."
        }
        Tsunami => {
            "FEMA's National Risk Index says how many people in your county live in the tsunami \
             zone. Most recorded tsunamis are small; about 1 in 3 bring a warning to leave."
        }
        VolcanicActivity => {
            "FEMA's National Risk Index says how many people in your county live where ash or \
             mudflows from a volcano can reach."
        }
        Wildfire => {
            "FEMA's National Risk Index gives the chance that fire burns land near homes in your \
             county. Many more homes are told to leave than burn, and in the West utilities cut \
             power to prevent fires."
        }
        WinterWeather => {
            "Your county's records count winter storms. We estimate about 1 in 20 keep a given \
             household home or cut its power."
        }
        Pandemic => {
            "Five pandemics began in the last 108 years (1918, 1957, 1968, 2009 and 2020). About 1 \
             in 4 changed daily life as 1918 and 2020 did, so we use about 1 in 100 a year."
        }
        GridFailure => {
            "Blackouts across a whole region that last days and are not caused by weather are rare \
             (the 2003 Northeast blackout lasted up to four days). With no reliable count, this is \
             an expert estimate: about 1 in 200 a year."
        }
        CyberOutage => {
            "Computer outages have stopped pharmacies, insurers and card payments for days in \
             recent years. There is no count per household, so this is an expert estimate: about \
             1 in 50 a year."
        }
        CivilUnrest => {
            "Curfews during unrest touch many cities in some years and none in others. This is an \
             expert estimate: about 3 in 100 a year in a city, less in suburbs and the country."
        }
        SupplyChainDisruption => {
            "Runs on stores before storms and shortages of particular goods happen most years \
             somewhere. This expert estimate is about 1 in 5 a year."
        }
        HazmatRelease => {
            "Orders not to drink tap water after a chemical spill, or to stay inside near a plant or \
             rail line, are rare but real. This expert estimate is about 2 in 100 a year, higher \
             where many facilities report toxic chemicals."
        }
        NuclearPlantIncident => {
            "One US plant accident has needed action outside the plant (Three Mile Island, 1979) in \
             several thousand years of reactor operation. Near a plant the chance is very small."
        }
        NuclearAttack => {
            "Published forecasts of a nuclear attack on the US, times the chance your county would \
             be in a blast or heavy-fallout zone given where likely targets and missile fields \
             are. Expert estimates stacked together, so only the range is shown."
        }
        Terrorism => {
            "Attacks that close down the area where people live for half a day to two days are \
             rare. This is an expert estimate for a city, lower outside cities, and it counts the \
             disruption, not the chance of being hurt."
        }
        JobLoss => {
            "About 8 in 100 working people were out of work at some point in 2024 (Bureau of Labor \
             Statistics). Gig, seasonal and commission work loses income more often, so the rate \
             is higher for those jobs."
        }
        HouseFire => {
            "About 1 in 380 homes has a fire the fire department is called to each year (US Fire \
             Administration, 2023). Homes that share walls also burn when a neighbor's does."
        }
        MedicalEmergency => {
            "Americans make about 47 emergency room visits for every 100 people each year (CDC)."
        }
        VehicleStranding => {
            "Crashes (about 6 million reported a year) and breakdowns leave drivers stuck away from \
             home. This expert estimate is about 1 in 7 per vehicle a year."
        }
        LocalUtilityOutage => {
            "No one keeps a national record of boil-water notices or water main breaks per home \
             (EPA, 2024). This expert estimate is about 1 in 7 a year on public water."
        }
        Burglary => {
            "About 1 home in 100 is broken into each year, by our estimate; the national crime \
             survey figure will replace it."
        }
        EarnerDeathOrDisability => {
            "About 1 in 4 of today's 20-year-olds will become disabled before retirement age \
             (Social Security), and some working-age adults die each year: together about 1 in 110 \
             per earner a year."
        }
        ExtendedHouseholdIllness => {
            "An illness that keeps someone home for weeks: an expert estimate of about 1 in 100 per \
             person a year."
        }
        WildfireSmoke => {
            "Satellite smoke maps and air monitors count the days a year your county breathed \
             smoke at levels unhealthy for sensitive groups (2016 to 2023). A smoke episode lasts \
             about three days."
        }
        DustStorm => {
            "The National Weather Service records dust storms by forecast area. We estimate about \
             1 in 3 of your area's dust storms reaches a given household."
        }
        Sinkhole => {
            "USGS maps show how much of your county sits on karst, rock that dissolves into caves \
             and sinkholes. How often a home there is damaged is an expert estimate from Florida \
             records."
        }
        GeomagneticStorm => {
            "Five studies put a storm like the 1859 Carrington event at about once in 100 to 2,000 \
             years. Power grids nearer the magnetic pole take stronger currents, so your latitude \
             scales the chance of a long outage."
        }
        Vei7Eruption => {
            "Ice cores and geology suggest about a 1 in 6 chance of a very large eruption somewhere \
             in the world this century (Cassidy and Mani, 2022). It would raise food prices for a \
             year or two."
        }
        DamFailure => {
            "The national dam inventory lists high-hazard dams and the towns below them; about 2 in \
             10,000 dams fail each year. Levees add a small chance for homes behind them. Expert \
             estimates stacked together, so only the range is shown."
        }
        NetworkOutage => {
            "A carrier-wide phone outage of several hours happens about once a year (AT&T, \
             February 2024, blocked more than 92 million calls). About a third of households are \
             on the carrier that fails."
        }
        DrugShortage => {
            "Hundreds of medicines run short each year (323 at the peak in early 2024). For one \
             person's daily medicine this is an expert estimate of about 1 in 20 a year, more for \
             medicines that must stay cold."
        }
        BenefitInterruption => {
            "Federal funding gaps of two weeks or more came in 4 of the last 45 years, and one, in \
             November 2025, stopped SNAP payments. Social Security and VA payments have kept \
             coming through every shutdown."
        }
        AttackDisruption => {
            "About three or four attacks in 31 years closed a whole US metro area for half a day or \
             more. We weigh your area by its share of FEMA's terrorism-preparedness money. Expert \
             estimates stacked together, so only the range is shown."
        }
        MultiMonthBlackout => {
            "Your county's own outage record, plus the long tails of a severe solar storm, an EMP \
             and wartime attacks on the grid. Puerto Rico's outage after Hurricane Maria lasted \
             328 days for the last customers."
        }
        WarInfrastructure => {
            "Forecasters put a war between the US and Russia by 2030 at about 2 to 5 in 100. Given \
             a war, attacks on US power, water or phones are an expert estimate, lower far from \
             military sites and cities."
        }
        CbrnAttack => {
            "About 500 chemical, biological or radiological attacks, attempts and plots were \
             recorded worldwide from 1990 to 2017, most of them chemical. We weigh your area by \
             its share of FEMA's terrorism-preparedness money."
        }
        SeverePandemic => {
            "Records of epidemics over four centuries put a pandemic as deadly as COVID-19 at \
             about once in 200 years (Marani et al., 2021). One far deadlier is rarer: an expert \
             range."
        }
        FinancialCrisis => {
            "Banks closed nationwide once in the last century (1933), before deposit insurance. \
             Single banks fail about 23 times a year, and insured deposits move within days."
        }
        MassViolence => {
            "The FBI counted 23 people killed and 83 wounded in active-shooter attacks in 2024, \
             among about 336 million people: about 3 in 10 million a year."
        }
        WaterDamage => {
            "Insurers record water damage and freezing at about 1 in 67 insured homes a year. \
             Freezing weather and a basement raise the chance; renters report fewer claims."
        }
        Eviction => {
            "Courts order about 2 in 100 renting households to leave each year (Eviction Lab). \
             Savings and steadier income lower the chance."
        }
        ArrestOrDetention => {
            "The FBI counts about 7 million arrests a year, by age and sex. We add up the rates for \
             the ages in your household. It counts arrests, not guilt, and one person arrested \
             twice counts twice."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hazard_has_a_plain_reason() {
        for h in HazardId::ALL {
            let s = why_we_think_this(*h);
            assert!(!s.is_empty() && s.ends_with('.'), "{h}");
            assert!(!s.contains('_'), "{h}: looks like an id");
            assert!(s.len() < 320, "{h}: {} characters", s.len());
            assert!(!s.contains("  "), "{h}: double space");
        }
    }
}
