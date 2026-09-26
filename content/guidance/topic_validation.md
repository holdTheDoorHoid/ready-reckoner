---
id: topic_validation
title: How well do these numbers hold up?
kind: topic
applies_to: [topic:validation]
citations: [rr_design_decision_log_2026, epa_asheville_boil_notice_2024, npr_maria_2018, ornl_eagle_i_outages]
---
No model of rare events is exact, so we test this one against real disasters and publish what we find.

**The test.** For each past disaster, we take a household that lived through it, run the planner for that household, and compare its targets with what happened: how long the power, the tap water or the home was gone. A result is *covered* when the target was long enough, *partly covered* when it was long enough for some needs but not all, *short* when it fell short, and *not modelled* when the planner has no way to show that event.

**What the first test found.** Against 22 past disasters, the first version's targets covered 6, partly covered 5, fell short on 10 and could not model 1. The review found the model's misses in failures of public water systems and in the longest power cuts.[^rr_design_decision_log_2026] After Hurricane Helene in 2024, Asheville's boil-water notice was not lifted until November 18.[^epa_asheville_boil_notice_2024] After Hurricane Maria, it took about 11 months to bring power back to all of Puerto Rico.[^npr_maria_2018] The county outage records the model learns from begin in 2014.[^ornl_eagle_i_outages]

**What it means for you.** Treat each target as a floor, not a promise. If your area has lived through something longer, plan for that. The validation table shows how the current version does on the same test.

## Sources

[^rr_design_decision_log_2026]: Ready Reckoner contributors, Design document, section 14 decision log: the round-2 review and the v0.2.0 decisions (entry of 2026-09-26) (2026).
[^epa_asheville_boil_notice_2024]: U.S. Environmental Protection Agency, City of Asheville lifts systemwide boil water notice issued after Hurricane Helene (2024).
[^npr_maria_2018]: NPR, 11 Months After Hurricane Maria Hit Puerto Rico, Officials Say All Power Is Restored (2018).
[^ornl_eagle_i_outages]: Brelsford C. et al., Oak Ridge National Laboratory, A dataset of recorded electricity outages by United States county 2014–2022 (EAGLE-I), Scientific Data 11:271, data updated through 2025 (2024).
