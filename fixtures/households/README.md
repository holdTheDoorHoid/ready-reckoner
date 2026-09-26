# Fixture households

Synthetic households used by tests, goldens, the mock engine and screenshots. None is a real person.
Schema: `docs/DESIGN.md` §4.1 and `crates/rr-types` (`PlanInput`). Each is chosen to exercise a different part of the model:

| File | Why it exists |
| --- | --- |
| `philadelphia-renters-4.json` | The reference case: urban renters, one car, a senior on daily medication, a modest budget, no renters insurance |
| `coos-bay-well-owner-2.json` | Cascadia earthquake + tsunami, well and septic (no water without power), wood heat, generator owned, evacuation matters, the 1-in-500 return-period dial |
| `miami-condo-retiree-1.json` | Hurricane + heat, 14th floor (elevators, water pressure), refrigerated medication, cat, climate 2050 dial |
| `hays-kansas-farm-5.json` | Tornado/hail/winter/drought, county given without ZIP, livestock, propane, pregnancy, epinephrine, seasonal income |
| `phoenix-apartment-cpap-1.json` | Extreme heat, a powered medical device (CPAP), gig income, small budget with a one-off amount |
| `sugar-land-ev-household-3.json` | Hurricane/flood belt, an EV, insulin (refrigerated), an infant on formula, larger budget, the 1-in-50 return-period dial |
| `chicago-student-zero-budget-1.json` | Zero budget: the plan must still be useful (free actions), no vehicle, no insurance |
| `pending/cameron-insulin-well-farm-2.json` | Round 2 (v0.1.1), staged: insulin on a rural well with livestock in Cameron Parish, Louisiana (a 1½-month power target). Exercises the cold chain (a power station as a need, the guardrail), the well pump (14 days of stored animal water, a pump-rated generator with its interlock and fuel cans), an owner with no smoke alarms or extinguisher, and a rural bleeding-control kit. It sits in `pending/` because every file directly in this folder must be embedded in `rr_types::fixtures::RAW` and `web/src/engine/fixtures.ts`; moving it up one level and adding it there is the planner's step before the goldens are regenerated |

| `pending/minot-missile-field-3.json` | Contract v2 (v0.2.0), staged: Minot, North Dakota (Ward County, next to Minot Air Force Base): the missile-field county of the nuclear family (strategic class A) and a high magnetic latitude for solar storms. A federal employee (`benefits: federal_pay`), the rare allowance opened for two families (`rare_opt_in`), and lights with a `tested_on` date |
| `pending/sacramento-leveed-2.json` | Contract v2, staged: Natomas, Sacramento (ZIP 95834), a basin behind levees, with no flood policy and no sewer-backup cover: the dam and levee hazard and the insurance decisions |
| `pending/missoula-smoke-2.json` | Contract v2, staged: Missoula, Montana, a smoke county: no air conditioning, a senior at home, a river within reach (`raw_water_source`): the clean-air bucket and wildfire smoke |
| `pending/detroit-snap-3.json` | Contract v2, staged: Detroit renters on SNAP (`benefits: snap_wic`) with $10 a month and bare-minimum mode, a basement bedroom (`below_grade_bedroom`), a grandparent with a hearing need (`access_needs`), no car and no alarms, and the one filled-in family plan (meeting places, trusted circle, lawyer, numbers by heart) for the wallet cards |
| `pending/galveston-highrise-1.json` | Contract v2, staged: a retiree alone on the 9th floor on Galveston Island (a storm-surge zone), limited mobility, home health care, SSI/SSDI, no car: the evacuate-first rule and the registries |
| `pending/san-juan-2.json` | Contract v2, staged: San Juan, Puerto Rico: Spanish speakers (`limited_english`), insulin, a cistern (`rain_barrel`), a water system with frequent problems, and the long-horizon section switched on: territory restoration factors and water-system fragility |

The six contract v2 households sit in `pending/` for the same reason as the cameron household: the
goldens, the sample-county engine and the web app's fixture list cover only the files directly in
this folder. They are embedded in `rr_types::fixtures::PENDING` (`rr_types::fixtures::get` finds
them), so every crate can test against them now; moving them up a level, with their golden files and
sample counties, is the planner's step.

Unless its row says otherwise, a household uses the 1-in-100 return-period dial (`one_in_100`).
