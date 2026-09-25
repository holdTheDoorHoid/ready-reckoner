# Ready Reckoner

**A disaster-preparedness planner that tells you what is actually likely to happen where you
live, what it would do to your household, and what to buy first with the money you have.**

Enter your location and a few facts about your household. Ready Reckoner works out, from public
hazard data and cited research, which emergencies you should plan for, how many days you should be
able to manage without power, water, income or help, and what to do about it in what order: a
72-hour kit and a get-home bag first, then two weeks, then a month, and only as far beyond that as
your risk justifies. It hands you a printable packet you can keep in a drawer.

It exists so that nobody prepares for a nuclear war and gets caught out by a pandemic, a week-long
ice storm, or a lost job.

## What makes it different

- **It plans by consequence, not by hazard.** You don't prepare for a hurricane; you prepare for ten
  days without power. Every hazard where you live maps onto a handful of consequences (no power, no
  safe water, can't leave home, must leave home, no income, no medical help, no communications), and
  the plan is sized against those. The same two weeks of water covers the hurricane, the pandemic and
  the water-main break. Hazard-specific extras (a gas shut-off wrench, N95 masks, a staged go-bag)
  ride on top.
- **Every number has a source.** Water per person per day, calories, medication supply, outage
  durations, event frequencies: each traces to FEMA, NOAA, USGS, the CDC, the Sphere Handbook or the
  peer-reviewed literature, and you can see which.
- **It respects your budget.** Tell it what you can spend per month. It orders purchases by risk
  reduced per dollar, starting with the things that cost nothing (documents, a plan, water in bottles
  you already own, knowing your neighbours), and it tells you when you have done enough.
- **It is honest about likelihood.** Risks are shown as natural frequencies ("of 100 households like
  yours, about 12 will face a week without power in the next ten years"), as ranges, with a dial for
  how rare an event you want to be ready for.
- **Nothing leaves your computer.** It is a static website that works offline. There is no server,
  no account, no analytics. Your address and your family's medical details stay in your browser.
- **Specs, not brands.** Each item says what to look for, what to avoid, and a typical price range.
  It never links to a store.

## Who it is for

Anyone starting from nothing who wants a professional's judgement without hiring one, and
experienced preparers who want the assumptions exposed and adjustable. The guided interview produces
the plan; every dial behind it is there if you want it.

## Status

Planning and early construction (September 2026). See [docs/DESIGN.md](docs/DESIGN.md) for the
authoritative design, [docs/ROADMAP.md](docs/ROADMAP.md) for what is being built in what order, and
the GitHub issues for the live work list.

## Repository layout

| Path | What |
| --- | --- |
| `crates/` | The Rust engine: hazards, consequences, supply sizing, budget allocation, plan and packet generation, plus the CLI, the WebAssembly bindings and the data pipeline |
| `web/` | The website (Svelte + TypeScript), which runs the engine in the browser |
| `data/` | Built data packs and their provenance manifest (raw downloads are never committed) |
| `content/` | The guidance text and item catalogue (CC BY-SA 4.0) |
| `fixtures/` | Fixture households and golden plans used by the tests |
| `docs/` | Design, risk model, data sources, content standards, UI, roadmap, research |

## Licence

Code is licensed under the GNU General Public License v3.0 ([LICENSE](LICENSE)). The written
guidance, item catalogue and derived tables under `content/` and `docs/` are licensed under Creative
Commons Attribution-ShareAlike 4.0 ([LICENSE-CONTENT](LICENSE-CONTENT)). Federal data and text
incorporated from United States government sources are in the public domain; their provenance is
recorded in `data/manifest.json` and inline citations.

## Not medical, legal or financial advice

Ready Reckoner summarises published guidance and public data. It is not a substitute for a
prescriber, an insurer, an attorney or your local emergency management agency. Where it touches
medication, it tells you to talk to a prescriber; where it touches insurance, it tells you what to
ask. It never tells you which drug to take for which condition.
