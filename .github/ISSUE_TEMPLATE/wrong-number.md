---
name: Wrong number
about: A risk, target, quantity, price or date in the app or the packet looks wrong
title: "Wrong number: "
labels: ["wrong-number"]
---

<!--
Thank you. A few details make it possible to check the number against its source.

GitHub issues are public. Please do not include your address or anything else that identifies you
or your household. A saved plan file (ready-reckoner-plan.json) holds your household's details:
attach one only if you are happy for anyone to read it. A made-up household that shows the same
number works just as well, and so does one of the example households in fixtures/households/.
-->

## What the app showed

- **The number, as shown:** <!-- e.g. "Be ready for about 5 days without power (3–7)" -->
- **Where:** <!-- screen (Risks, Plan, Packet...) and the card or section -->
- **Settings:** <!-- e.g. "Very serious (1-in-100)", today's climate, basic water -->
- **County:** <!-- e.g. Philadelphia, Pennsylvania (a county is enough; no street address) -->

## What you believe is right

<!-- The number you expected, and why. -->

## Your source

<!-- A link, a document with page or table, or a dataset with the field name. -->

## Versions

<!-- On the About screen, under "Found a wrong number?", press "Copy this line" and paste it here. -->

## To reproduce (optional)

<!--
Attach a household that shows the number: a made-up one, one of fixtures/households/*.json, or
your own saved plan if you are comfortable sharing it (the "Open a saved plan" button reads all
of these). The command-line planner gives the same numbers from a household file; for a saved
plan, use the part under "input":  cargo run -p rr-cli -- plan --household <file>.json
-->
