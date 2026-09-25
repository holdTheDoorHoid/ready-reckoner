# Principles and content policy

These are the commitments that make Ready Reckoner trustworthy. They are policy, not style. A change
to any of them is an owner decision, recorded in `docs/DESIGN.md`.

## 1. Prepare for Tuesday before doomsday

The plan is ordered by likelihood times consequence, over a stated horizon. A lost job, a house fire,
a week-long outage and a pandemic outrank a nuclear exchange for almost every household, and the plan
says so. Rare catastrophic risks are named honestly, shown with their probability, and given the
mitigation that general supplies already provide; they never hijack the budget.

## 2. Consequences, not causes

Supplies and actions are sized against consequence buckets (no power, no safe water, cannot leave
home, must leave home, dangerous heat or cold, no medical access, no income, no communications,
shortages, loss of the home, fire, security). Hazards explain *why* a bucket matters here and add a
short list of hazard-specific extras. Two hazards that produce the same consequence share the same
supplies; the app never tells anyone to buy the same water twice.

## 3. Show the work

Every number has a source the user can open. Probabilities are shown as natural frequencies ("about
12 of 100 households like yours, in the next ten years") and as ranges, never as false-precision
decimals. The confidence dial (how rare an event to be ready for) and the climate horizon are
visible, adjustable, and explained. Expert judgement is labelled as such.

## 4. Efficacy over fear

Following the Extended Parallel Process Model: a threat statement is never shown without a paired,
achievable action and a sense of what it accomplishes. Copy avoids catastrophising, countdowns,
scarcity language and imagery of suffering. Progress is visible and celebrated at every tier. The
user is told when they have done enough for their risk.

## 5. Cheapest risk reduction first

The allocator starts with actions that cost nothing or nearly nothing: documents copied, a family
communication plan, water stored in containers already owned, smoke and CO alarms tested,
medications refilled early, neighbours' numbers saved. Purchases are ordered by risk reduced per
dollar with diminishing returns, so the first three days of water rank far above days 30 to 33.

## 6. Community is a supply

Social capital is the strongest predictor of who comes through a disaster well. Knowing neighbours,
joining a CERT or block group, and having a mutual-aid arrangement appear in the plan as first-class
items with the same standing as a water filter.

## 7. Privacy by construction

The site is static and works offline. Location and household details stay in the browser. There
are no accounts, no analytics, no third-party scripts, and no network calls from the engine. Any
optional lookup that would send the user's location to another party is behind a per-use consent
screen that names the recipient and what it learns.

## 8. Not a store

Items are specified by what to look for, what to avoid, and a typical price band. No brands, no
product links, no affiliate relationships, no sponsors, no "recommended kits". The user records what
they actually bought and paid, and the budget tracker uses their numbers.

## 9. Sensitive content

**Medical.** First aid, over-the-counter supplies and prescription continuity are fully in scope,
cited to the Red Cross, CDC, FDA and state emergency-refill law. Antibiotics: the app explains what
an emergency antibiotic kit is, that the legitimate route is a licensed prescriber (including
telehealth services built for this), what it typically costs, how to store it, and the situations
where it is appropriate versus dangerous. It never gives drug-by-condition or dose guidance, never
suggests veterinary or "fish" antibiotics, and always routes to a prescriber. Same for any
prescription device or drug.

**Firearms and security.** Home and personal security is a real category: locks, lighting, fire
safety, situational awareness, knowing neighbours, not presenting as a target, and de-escalation.
Firearms are named as a personal and legal decision; the app points to safe storage and training,
notes that unsecured firearms raise household risk, and never includes firearms, ammunition or
tactical equipment in a budget or checklist.

**Nuclear, radiological, war.** Presented with honest probabilities and the actual guidance
(shelter, distance, time; the first 24–72 hours of sheltering are covered by the general shelter-in-
place supplies; potassium iodide only matters within the emergency planning zone of a plant and is
distributed by authorities). Allocation to these hazards is capped; the app says why.

**Children and vulnerable readers.** Language stays calm and concrete. No graphic descriptions.
Guidance for talking with children about emergencies is included, cited to child-development
sources.

**Mental health.** Stress, sleep, routine, and post-disaster distress are part of preparedness. The
app includes the 988 line and disaster-distress resources and normalises using them.

## 10. Plain language and access

Eighth-grade reading level for user-facing text. WCAG 2.2 AA. Works on a phone, works printed in
black and white, works with a screen reader. Every screen has one job.

## 11. Warn, do not block

Guardrails flag a plan that looks off (budget zero, no water at all after month three, a CPAP with no
power plan) but never prevent the user from proceeding. They may know something the model does not.

## 12. Name things by what they do

"Two weeks of water" not "Tier 2". "Be ready for what happens in 9 of 10 ten-year periods" not
"90th percentile". Jargon appears in brackets after the plain phrase, once, for people who want it.
