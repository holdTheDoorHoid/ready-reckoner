# The `rr` command line

`rr` is the Ready Reckoner engine on the command line. It calls the same functions the web app
calls (`assess`, `explain`, `catalogue`, `county_search`, `engine_info`) and prints the answers for
people, agents and CI. It is the project's oracle: when a number in the app looks wrong, `rr`
shows where it came from.

```
cargo run -p rr-cli -- <command> [options]            # from the repository root
cargo build --release -p rr-cli && target/release/rr <command> [options]
```

`rr --help` lists the commands and `rr <command> --help` lists each command's options. A debug
build takes about 4 seconds to load the data pack and 0.1–0.3 seconds to plan a household; a
release build takes about 0.8 seconds and 15–30 milliseconds.

## Where the county data comes from

| You pass | `rr` uses |
| --- | --- |
| nothing | the data pack in `./data` when it has a `manifest.json` (run from the repository root) |
| nothing, and there is no `./data/manifest.json` | the seven hand-built fixture counties, with a note on standard error |
| `--data <dir>` | the data pack in `<dir>`; an error (exit 2) if it has no `manifest.json` |
| `--fixtures` | the seven fixture counties: Maricopa AZ, Miami-Dade FL, Cook IL, Ellis KS, Coos OR, Philadelphia PA, Fort Bend TX |

A data pack is loaded exactly as the web app loads it: `manifest.json` first, then every file the
manifest lists, by its manifest path, each checked against its sha256. A file that fails its
checksum stops the command (exit 1) and names the file.

The fixture households in `fixtures/households/` plan against real county records on the data
pack (Philadelphia is county 42101 either way, but its numbers come from the national data), and
against the hand-built records with `--fixtures`. Both are useful: the fixtures never change under
you, and the pack is what users get.

## The household and the dials

Every command that plans takes `--household`, which can be:

- a PlanInput JSON file (`docs/ENGINE-API.md`), such as `fixtures/households/coos-bay-well-owner-2.json`
  or a plan exported from the app;
- `-` to read the JSON from standard input;
- a fixture name, such as `philadelphia-renters-4`.

These options override the file (each one replaces what it names, then the household is
validated, so a flag can fix a bad value in the file):

| Option | Effect |
| --- | --- |
| `--zip 19147` | plan for this ZIP code instead of the file's location |
| `--county 42101` | plan for this county instead; with `--zip`, picks the county for a ZIP code that spans several (as the app records it) |
| `--return-period one_in_10\|one_in_50\|one_in_100\|one_in_500` | how rare an event to be ready for |
| `--climate today\|y2050` | today's climate or the 2050 projection |
| `--water-level survival\|basic\|comfortable` | water per person per day |
| `--scenario cascadia_m9=off` | turn a named scenario on or off (repeat for several); a toggle for a scenario that does not apply to the place is pointed out |

## Exit status

| Code | Meaning |
| --- | --- |
| 0 | it worked |
| 1 | a check failed (golden files differ, `doctor` found a problem, a citation id is missing), or the engine or a file could not be read (a corrupt pack) |
| 2 | the input needs fixing: validation problems, an unknown or ambiguous ZIP code or county, a usage error |

Validation problems are listed one per line, with the field each one is about:

```
$ rr plan --household philadelphia-renters-4 --zip 1914
rr: The household has 1 problem to fix:
  - location.zip: A ZIP code is 5 digits, like 19147. [zip_format]
```

## Commands

### `rr plan`: the packet and the whole plan

```
rr plan --household <file> [dial options] [--format md|json|both] [--out <dir>]
```

Prints the printable packet (Markdown, the default) or the whole `PlanOutput` (`--format json`).
The JSON is written with the same function that writes the goldens, so for a fixture household on
the fixture counties it is byte for byte `fixtures/golden/<name>.json`. `--out <dir>` writes
`<name>.md` and/or `<name>.json` there instead; `--format both` needs `--out`.

```
$ rr plan --household fixtures/households/philadelphia-renters-4.json | head -4
# Your preparedness packet

**For:** 2 adults, 1 older adult and 1 child, with 1 dog
**Where:** Philadelphia County, Pennsylvania (ZIP code 19147)

$ rr plan --household coos-bay-well-owner-2 --scenario cascadia_m9=off --format both --out packets/
wrote packets/coos-bay-well-owner-2.md
wrote packets/coos-bay-well-owner-2.json
```

### `rr risks`: the risk register

```
rr risks --household <file> [dial options]
```

The ranked hazards, most important first, each with its ten-year natural frequency, its yearly
rate and range, how bad and how sure, the sentence the app shows and its sources. Then the rare
but severe hazards in their own box (never ranked by expected loss), the named scenarios, the
notes behind the numbers, and every source with its URL.

```
$ rr risks --household philadelphia-renters-4
...
Ranked, most important first (29 hazards)

  #  Hazard                              Next 10 years        Per year (range)             How bad      How sure
  1  Heat wave                           almost all           4.5 (3.4–5.8)                Serious      Based on data
     Nearly every household like yours will go through a heat wave in the next ten years (about 4.5
     times a year).
     Sources: noaa_storm_events
  2  Medical emergency                   almost all           1.9 (1.4–2.6)                Serious      Based on data
...
```

### `rr targets`: how long to be ready for each need

```
rr targets --household <file> [dial options] [--sweep]
```

First what the dial means (the same sentence as the packet, model review M-04): each target holds
for its own need, and the chance that at least one need runs past its target is higher. At the
default setting:

```
For any one need, something worse than its target comes in about 1 of every 10 ten-year stretches.
Across all your needs together, the chance that at least one runs out is higher, roughly 1 in 3.
That is why the plan also gives you ways to cope when a target runs out.
```

Then, for each need that lasts days (power, water, supplies, heat and cold, medicine, phone), the
target with its range (10th to 90th percentile), what the plan covers, when outside help plausibly
arrives and service is mostly back (for the event behind the target: its outages that last a day
or more; "not known" when there are no records for it, or when the time would be under a third of
the target), the step that is enough, the frequency sentences and the hazards that drive it. Then
the readiness needs (leaving home, getting home, a medical emergency, fire, security, the home
itself) with the ten-year chance of needing each, the income savings goal, and any warning that
one event drives the answer.

```
$ rr targets --household philadelphia-renters-4
...
 Need                                      Be ready for                        Plan covers  Help arrives  Mostly back   Enough at
 No grid power at home                     about 3 days (up to 7 days)         3 days       about 3 days  about 6 days  three days
       Of 100 households like yours, about 35 (30–55) will lose grid power for a day or more in the
       next 10 years.
...
       Driven by: Hurricane 65%, Strong wind 14%, Heat wave 9%, Cold wave 5%
```

`--sweep` runs the engine at every dial setting and shows the targets side by side, with what each
setting means for the plan. A cell marked `!` is at least three times the setting to its left: the
answer jumps between settings, usually because one rare event sits close to the dial (the cliff
rule, `docs/DESIGN.md` §4.4). The engine's own cliff warnings follow, with the settings that raise
them.

```
$ rr targets --household fixtures/households/coos-bay-well-owner-2.json --sweep
...
Days to be ready for (income in months), with the range in brackets. * marks the household's setting.

 Need                                      1 in 10      1 in 50       1 in 100       1 in 500 *
 No grid power at home                     1 (1–3)      5 (3–10) !    21 (5–90) !    180 (60–365) !
 Tap water must be treated                 -            -             -              -
 No tap water at all                       2 (2–7)      30 (10–90) !  90 (30–180) !  365 (180–365) !
 Can't get to a store                      3 (2–7)      10 (7–21) !   21 (10–45)     60 (45–180)
 Dangerous heat or cold indoors            5 (3–5)      7 (5–10)      10 (7–10)      14 (10–14)
 Medication and medical-supply continuity  2 (0–5)      10 (7–21) !   30 (14–45) !   90 (45–180) !
 No phone, internet or card payments       1 (0.5–3)    5 (2–10) !    7 (3–14)       45 (21–90) !
 Loss of income (months)                   1 (0.5–1.5)  4 (2–7) !     6 (3–9)        12 (8–24)

 Worse for one need, of 100 ten-year stretches  65      20            10             2
 Enough for this household                 two weeks    one month     three months   one year
 Plan done by month                        4            6             9              15
 Purchases in the plan                     $1,076       $1,386        $1,755         $2,634
...
```

Add `--scenario cascadia_m9=off` to see the same table without the Cascadia earthquake.

### `rr explain`: why a number is what it is

```
rr explain <hazard|bucket|item|requirement|warning> <id> --household <file> [dial options] [--json]
```

The engine's explanation: plain sentences first, then the arithmetic (the household event rate,
Λ at the target, the value of holding the days, the quantity formula), then the sources. `--json`
prints the `Explanation` object. An id that names nothing in the plan exits 2 and lists the ids
that would work, closest first.

```
$ rr explain bucket power --household philadelphia-renters-4
No grid power at home

Be ready for about 3 days (up to 7 days) (three days is enough for this need).
...
The arithmetic
  - Λ(d) = Σ r_h · q_h,b · S_h,b(d): disruptions a year lasting longer than d days. Dial rate Λ* =
    0.01054 a year (about 1 in 100).
  - Design duration: the smallest d with Λ(d) ≤ Λ* is 3.01 days; on the day ladder 3 days (10th to
    90th percentile 3 to 7).
...

$ rr explain hazard hurrican --household philadelphia-renters-4
rr: There is no hazard with the id "hurrican" in this plan.
  Did you mean: hurricane
...
```

Ids: hazards and buckets are the ids in `docs/ENGINE-API.md`; items are catalogue ids
(`rr catalogue`); requirement lines and warnings are the `id` fields in `rr plan --format json`.

### `rr catalogue`: the item catalogue

```
rr catalogue [--category <name>] [--json]
```

Every item by category with its step, typical price, flags and the needs it helps with.
`--json` prints exactly what the engine's `catalogue()` returns (items filtered by `--category`),
so it can be compared with the web app's mock. Needs no county data.

### `rr citations`: the sources

```
rr citations [--missing]
```

Lists `content/citations.toml`. `--missing` lists every citation id that something refers to but
the registry does not define: the catalogue items, the guidance blocks (front matter and `[^id]`
footnotes), the glossary, every id the hazards, consequence, supply, budget and plan crates can
emit, and the base rates of the loaded data. It exits 1 unless each missing id is formally awaited
(requested in `docs/CITATION_IDS.md` and listed in `rr_plan::provenance::AWAITING_CONTENT`). It
never runs a plan, so it still works when an unknown id would make a debug build of the engine
stop.

```
$ rr citations --missing
Checked 219 citation ids referred to by the content, the engine crates and the data (data pack 25f3ed156688 (data)).
1 is not defined in content/citations.toml:

 Id                         Status                                       Referred to by
 county_boil_water_records  awaited (requested in docs/CITATION_IDS.md)  rr-consequence
```

### `rr county`: find a county, show its record

```
rr county search <query>        # "phila", "Cook, IL", "42" (a FIPS prefix); up to ten
rr county show <fips> [--json]  # the CountyRecord the hazard model reads
```

`show` prints the county's National Risk Index fields per hazard, power outage statistics (with
the exact outage definition), event rates, earthquake shaking, climate ratios, flood, facilities and
social vulnerability. A search with no match exits 1.

```
$ rr county search "Cook, IL"
1 match for "Cook, IL" in data pack 25f3ed156688 (data)

 FIPS   County  State          Population
 17031  Cook    Illinois (IL)   5,273,000
```

### `rr data`: check and describe the packs

```
rr data verify
rr data info
```

`verify` runs the data layer's own checks: the store's (every file loads by its manifest path,
matches its sha256 and parses; row counts match the manifest), then the checks `rr-etl verify`
runs (`rr_data::verify`, so `rr` needs no network stack: every county joins across every pack or
is listed as missing with a reason, the Connecticut crosswalk, ZIP shares), then that every
fixture household resolves to a county. On the fixtures it says
`no data pack; fixture counties in use` and checks those instead.

```
$ rr data verify
Verifying the data pack in data

  ok  store: 18 files load by their manifest path and match their sha256 (pack 25f3ed156688)
  ok  store: row counts match the manifest for 18 of 18 files
  ok  store: 3,232 county records, 33,791 ZIP codes, 3,222 counties on the map
  ok  engine: 7 of 7 fixture households resolve to a county with a record
  ok  rr-etl verify: 18 files, 69 checks, 0 problems
...
Everything checks out.
```

`info` prints the engine, content and pack versions, when the pack was refreshed, one line per ETL
job (when its sources were retrieved, how many, their licences, how many counties it has no data
for), and the attributions the About screen and every packet must show, the FEMA National Risk
Index statement first.

### `rr golden`: the golden packets

```
rr golden [--update]
```

Compares `fixtures/golden/*.md` and `*.json` with a fresh rendering by rr-plan's own helper
(`rr_plan::golden::render_all`, the same one `cargo test -p rr-plan --test goldens` checks),
showing the changed lines of every file that differs. `--update` rewrites them with
`rr_plan::golden::write_all`; explain the change in the commit message. `rr` itself knows nothing
about what a golden contains.

With `--data <dir>` or `--fixtures`, the fixture households are rendered on that source instead
and compared with the same files. This is read-only (`--update` refuses it): a preview of what
would change if the goldens moved to that source. The goldens are rendered on the data pack in
`data/`, so `rr --data data golden` matches them, and `rr --fixtures golden` shows how the sample
counties would differ.

### `rr doctor`: every fixture, checked

```
rr doctor [--runs N]
```

Runs every fixture household through the engine on the chosen source and reports the median and
slowest `assess` time over `N` runs (default 5; the target is under 50 ms in a native release
build), whether the output is byte-for-byte the same on every run, the guardrail warnings each
household sees, and anything that reaches a household without a source: catalogue items or plan
steps with no citation, requirement lines, hazards or targets with no sources, placeholder sources.
It also runs the content validator. It exits 1 on a fixture that cannot be planned, output that
changes between runs, content errors or anything uncited. Timing is reported but never fails:
debug builds and CI machines are slower than the target describes.

```
$ cargo run --release -p rr-cli -- doctor
rr doctor: 7 fixture households on data pack 25f3ed156688 (data)
Engine 0.1.0 (API 1), content 2026.09.25+1318eadb, release build, 5 timed runs each

  ok  content validation: 0 errors, 0 warnings
  ok  catalogue: 104 items, 0 without a resolvable citation

 Fixture                        County            Median ms  Max ms  < 50 ms  Same bytes  Warnings  Uncited
 chicago-student-zero-budget-1  Cook, IL                 11      12  yes      yes                3        0
 coos-bay-well-owner-2          Coos, OR                 17      19  yes      yes                2        0
...
Result: OK. Every fixture plans, repeats byte for byte, and cites every number.
```

## In CI

The `cli` job in `.github/workflows/ci.yml` runs, in order: `rr golden` (the goldens match
rr-plan's helper), `rr doctor` on the committed data pack, `rr --fixtures doctor`,
`rr citations --missing` and `rr data verify`. Any of them failing fails the job.
