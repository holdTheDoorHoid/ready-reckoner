---
id: topic_how_numbers_are_made
title: How the numbers are made
applies_to: [topic:how_numbers_are_made]
citations: [fema_nri_v120, ornl_eagle_i_outages, shaffer_2026_texas_boil_notices, rr_research_risk_model, rr_risk_model_priors, akl_2011_cochrane_frequencies, gigerenzer_2007_statistics, fema_nri_disclaimer]
---
Every number in your plan has a source you can open. Here is how they fit together.

**From place to days.** Where you live gives each hazard a yearly chance, mostly from FEMA's National Risk Index.[^fema_nri_v120] Who lives in your home changes what each hazard would do to you. Each hazard then turns into a few plain consequences, such as no power or no water, that last some number of days. Outage lengths come from utility outage records,[^ornl_eagle_i_outages] and boil-water notice lengths from state records.[^shaffer_2026_texas_boil_notices] Adding up all the hazards gives, for each consequence, how many days to be ready for. Those days, times the people in your home, become gallons, calories and supplies, each with its own source.[^rr_research_risk_model]

**When data is thin.** Where no good data exists, we use an expert estimate, label it as one, and explain the reasoning.[^rr_risk_model_priors] Ranges show how sure we are.

**Why counts, not percents.** Chances appear as "about 12 of 100 households like yours," because people understand counts better than percentages.[^akl_2011_cochrane_frequencies][^gigerenzer_2007_statistics]

Ready Reckoner uses FEMA's National Risk Index data, but it is not endorsed by FEMA.[^fema_nri_disclaimer]

## Sources

[^fema_nri_v120]: FEMA, National Risk Index, version 1.20.0 (December 2025), county data (2025).
[^ornl_eagle_i_outages]: Brelsford C. et al., Oak Ridge National Laboratory, A dataset of recorded electricity outages by United States county 2014–2022 (EAGLE-I), Scientific Data 11:271, data updated through 2025 (2024).
[^shaffer_2026_texas_boil_notices]: Shaffer M., Awad N., Davenport F. and Fakhreddine S., Evaluating the Resilience of Drinking Water Systems to Severe Weather Events Using Boil Water Notices and Bottled Water Sales (Environmental Science and Technology 60(31):21648–21660) (2026).
[^rr_research_risk_model]: Ready Reckoner contributors, Quantitative core model specification (research report): hazards, consequence buckets, durations (2026).
[^rr_risk_model_priors]: Ready Reckoner contributors, Ready Reckoner expert estimates for hazard rates and disruption durations (each one listed with its reasoning) (2026).
[^akl_2011_cochrane_frequencies]: Akl E.A. et al., Using alternative statistical formats for presenting risks and risk reductions (Cochrane Database of Systematic Reviews CD006776) (2011).
[^gigerenzer_2007_statistics]: Gigerenzer G. et al., Helping Doctors and Patients Make Sense of Health Statistics (Psychological Science in the Public Interest 8(2):53–96) (2007).
[^fema_nri_disclaimer]: FEMA, National Risk Index terms of use: the statement every product that uses its data must show (2025).
