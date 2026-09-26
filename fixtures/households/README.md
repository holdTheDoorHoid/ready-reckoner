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

Unless its row says otherwise, a household uses the 1-in-100 return-period dial (`one_in_100`).
