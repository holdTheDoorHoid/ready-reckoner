# Fixture households

Synthetic households used by tests, goldens, the mock engine and screenshots. None is a real person.
Schema: `docs/DESIGN.md` §4.1. Each is chosen to exercise a different part of the model:

| File | Why it exists |
| --- | --- |
| `philadelphia-renters-4.json` | The reference case: urban renters, one car, a senior on daily medication, a modest budget, no renters insurance |
| `coos-bay-well-owner-2.json` | Cascadia earthquake + tsunami, well and septic (no water without power), wood heat, generator owned, evacuation matters |
| `miami-condo-retiree-1.json` | Hurricane + heat, 14th floor (elevators, water pressure), refrigerated medication, cat, climate 2050 dial |
| `hays-kansas-farm-5.json` | Tornado/hail/winter/drought, county given without ZIP, livestock, propane, pregnancy, epinephrine, seasonal income |
| `phoenix-apartment-cpap-1.json` | Extreme heat, a powered medical device (CPAP), gig income, small budget with a one-off amount |
| `sugar-land-ev-household-3.json` | Hurricane/flood belt, an EV, insulin (refrigerated), an infant on formula, larger budget, 19-in-20 confidence |
| `chicago-student-zero-budget-1.json` | Zero budget: the plan must still be useful (free actions), no vehicle, no insurance |
