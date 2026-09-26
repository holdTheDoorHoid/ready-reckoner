# Changelog

Ready Reckoner has no accounts and no server, so nothing changes on you without notice. This page
lists what shipped in each version, grouped by area, in plain language. It also lists what we
already know still needs work.

## v0.1.0 — September 2026

The first version that works end to end, for the whole country, on real data. Everything below runs
in your browser. None of it needs a server.

### The risk register

- Every county in the United States now gets a risk score. That is 3,232 counties in all, including
  Connecticut's planning regions and Puerto Rico's municipios.
- Each county is scored for three kinds of hazard: natural (using FEMA, NOAA and USGS data),
  societal (a grid failure, a pandemic, a cyberattack, and others like them), and personal (a job
  loss, a house fire, a medical emergency, and others like them).
- Each hazard is turned into plain consequences: no power, no safe water, can't leave home, must
  leave home, no income, and more. Each consequence gets a target: a number of days to be ready for,
  based on how careful you want to be and how far ahead you want to plan.
- Two counting mistakes were found during testing and fixed before release. A small number of
  counties no longer show an impossible full year without power. The "major hurricane" scenario no
  longer counts some counties' storms two to four times over.

### Your plan and budget

- A month-by-month purchase plan. Free steps come first: copying documents, writing a family plan,
  storing water in containers you already own, testing your smoke alarms. Paid steps follow,
  cheapest risk reduction first, until every consequence is covered. Then the plan tells you to
  stop.
- The plan adjusts for your home and household: which floor you live on, a well or septic system, a
  mobile home, medical needs, pets, and vehicles.
- Every item says what to look for, what to avoid, and a typical price range. No brands. No store
  links.

### The interview and the website

- A full guided interview: where you live, who is in your household, how you get around, your
  budget, and what you already have. It runs the real planning engine in your browser
  (WebAssembly), not a placeholder.
- Works from a ZIP code or a county name. A ZIP code that covers more than one county shows each
  county's share, with a map, so you can pick the right one.
- Light and dark themes that follow your system setting. A keyboard-only path through the whole app.
  A printable packet that reads fine in black and white.
- Fixed: the plan screen used to fail to display when a saved-up purchase and a monthly deposit fell
  in the same month. It now shows both correctly.

### The data behind it

- A national data pack, about 3 MB compressed, built entirely from public federal sources. Every
  figure traces back to where it came from and when it was fetched. A scheduled job re-downloads and
  rebuilds the pack every quarter and opens a pull request. Nothing updates itself without a person
  reviewing it first.

### Content and sources

- A full catalogue of items and guidance text. Every number in it has a source, listed in
  `content/citations.toml`.
- Wording and citation fixes found during testing. A home-loss statistic now matches its source. An
  emergency-prescription-refill claim now says the real count — 10 of 51 states — instead of "many
  states." A food-cost constant was corrected. The nuclear-risk card no longer calls a calculated
  range an "expert estimate."

### Checked before release

An adversarial review ran the plan for 500 households, picked at random, in every part of the
country. It also checked specific numbers against their original sources. Nothing crashed. No plan
ever spent more than its budget. Every number and step had a source. And the targets always moved
the right way when a setting changed — more people never needed less water, for example. Full
results are in [docs/VERIFICATION.md](docs/VERIFICATION.md).

### Documentation

- Added this changelog, a dedicated [privacy page](docs/PRIVACY.md), and the screenshots now in the
  README.
- Documented `rr.prefs.v1`. This is a small, separate storage entry that remembers your theme and
  your expert-view choice. Earlier documentation only described the plan's own storage entry,
  `rr.plan.v1`.

## Known limitations

The README's "What it is not" section covers what Ready Reckoner does not try to do at all: give
medical, legal or financial advice; work outside the United States; or plan at better than county
detail. This list is different. It covers specific rough edges found while building v0.1.0, which we
plan to smooth out.

- **79 counties have no outage history of their own** (37 in Nebraska, 21 in Alaska, among others).
  For now, they fall back to a simpler estimate, and can read lower than a similar county next door.
  A fix is planned that borrows the state's outage pattern instead.
- **A few coastal counties may read too high.** We are re-checking how earthquake risk and outage
  history combine in some Pacific coast counties. Coos Bay, Oregon is one example, where water and
  power targets come out higher than expected.
- **A couple of hazard cards show advice for the wrong hazard.** A cold-wave card has briefly shown
  avalanche advice, for instance. This comes from guidance shared across a family of related
  hazards. A fix is in progress to keep each card's advice specific to itself.
- **Heat wave and cold wave can read as less serious than they are** for a household with a baby, an
  older adult, or no cooling or heating. A fix is planned to weigh who is in the household, not just
  typical dollar cost.
- **Livestock water may be over-sized on a well with a generator.** If a generator is already
  planned to keep the well pump running, the plan should only need a few days of separate stored
  water for animals, not the full target. This is under review.
- **The printed packet's monthly spending can look higher than it is.** In a month where a saved-up
  purchase completes and a regular deposit also lands, both currently show in that month's total. A
  fix is proposed.
- **The packet needs print polish.** Its source list is not laid out in columns yet, and a section
  can still split across a page break.
- **A handful of source links point to a secondary copy** — a mirror, or a search results page —
  instead of the original document. We are replacing these with direct links where one exists.
- **Some price ranges run higher than typical** in a few item categories. These are being checked
  against current prices.
- **Long checklists show every step at once.** A few lists, such as everything involved in leaving
  home quickly, show every step instead of the most important few first.
- **Loading can briefly pause the page**, well under a second, when the full ZIP code list or the
  first batch of county data comes in. This is more noticeable on a slower device or connection.
- **Offline use was not confirmed in every browser** during this round of testing. If the app does
  not load without a connection for you, please open an issue.
- **The content security policy can't cover everything.** GitHub Pages can't send the custom
  response headers a full policy needs, so ours is delivered as a tag in the page instead. It still
  blocks inline scripts and outside script or data sources. But it cannot stop another site from
  loading this one inside a frame. See [docs/PRIVACY.md](docs/PRIVACY.md).
- **This site currently shares its address with the project owner's other work**
  (`holdthedoorhoid.github.io`), so browser storage is technically shared with them too. A dedicated
  address is recommended before relying on this for a real emergency. See
  [docs/PRIVACY.md](docs/PRIVACY.md).
- **No independent accessibility audit yet.** The interface is built to the WCAG 2.2 AA standard:
  keyboard navigation, colour-blind-safe severity colours, reduced-motion support. But it has not
  yet had a dedicated audit, the way the numbers and the reading level have.
