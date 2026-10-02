# Privacy

Ready Reckoner is a static website. There is no server behind it, no account to create, and no
analytics. The planning engine runs on your device, in your browser. This page says exactly what
that means: what stays on your computer, what the site downloads, what the host can see, what the
one feature that talks to anyone else sends and to whom, and how to check all of it yourself.

## What stays on your device

Everything you type — your location, your household, your budget, what you already own, what
you've checked off, and, since v0.3.0, the optional details behind your binder's pages (names,
medical conditions and medications, insurance, where people spend the day, home and vehicle
details, account numbers trimmed to their last four digits) — is kept only in your browser's local
storage. It is never sent anywhere.

Three separate places are used:

- **`rr.plan.v1`** holds your household, your dial settings, what you already have, your check-offs
  and paid amounts, your progress through the interview, your family plan (the out-of-area contact,
  meeting places, and the trusted circle of people who'd help), and, since v0.3.0, the answers
  behind your binder's pages: each person's profile, your home, your neighbourhood, your pets and
  vehicles, and your documents-and-money page. **This is the entry most likely to hold someone
  else's personal details, not just your own**, and since v0.3.0 it can hold real medical and
  financial details too. This is the entry that "Save a copy of your plan" writes out as a file and
  "Open a saved plan" reads back in. It never leaves your browser unless you choose to save or share
  that file yourself, and nothing in it is used to work out your plan's numbers — the people,
  places, pets, vehicles and documents answers are echoed back onto your binder's pages exactly as
  you typed them, never checked, corrected or computed with.
- **`rr-maps`**, new in v0.3.0, is a separate browser database (IndexedDB, not `localStorage`) that
  holds the three map images you've fetched for your binder, each with the date it was fetched and
  its legend. It exists only if you have pressed "Fetch maps" at least once. The pins and routes you
  drew to make those maps live in `rr.plan.v1` instead, as part of your plan, because they are small
  enough to export with it; the map images themselves do not travel with an exported file, so an
  imported plan with pins but no images shows "Maps need refreshing" until you fetch them again.
- **`rr.prefs.v1`** holds two display preferences: your light/dark theme choice and whether the
  expert view is turned on. It holds no household information, and it is not part of the saved
  file — importing or exporting a plan never touches it.

"Forget everything" (on the Keep it up screen) deletes all three — including `rr-maps` — and
anything else the app may have stored, from your browser. Nothing is recoverable after that unless
you had saved a copy.

The app uses no cookies and no `sessionStorage`.

## Keeping a saved file safe

Once your saved plan holds something sensitive — anything in a person's profile beyond their name
and phone, your home address, anything under documents and money, a vehicle's plate, a pet's
microchip number, or a map pin or drawn route marking where you live — "Save a copy of your plan"
offers to **protect the file with a passphrase**, ticked on by default. This uses only your
browser's own, standard cryptography (WebCrypto), nothing of our own devising: your passphrase is
stretched with PBKDF2-SHA-256 (600,000 rounds, a fresh random 16-byte salt) into a key, which then
seals the file's contents with AES-GCM-256 and a fresh random 12-byte value for that file alone. The
protected file looks like this:

```json
{ "format": "ready-reckoner-plan-encrypted", "version": 1,
  "kdf": { "name": "PBKDF2", "hash": "SHA-256", "iterations": 600000, "salt": "…" },
  "iv": "…", "ciphertext": "…" }
```

Opening it asks for the passphrase back, and says plainly if it's wrong. **Limits worth knowing:**

- The passphrase is never stored anywhere, by us or by your browser, so **there is no way to
  recover a forgotten one.** The printed binder is your backup if that happens.
- A wrong passphrase, or a file that has been altered since it was saved, fails to open rather than
  opening to garbled or incorrect data (AES-GCM checks this for you).
- This protects the **file**, not the copy of your answers still sitting in your browser's own
  storage (above) on the device where you made the plan. If you share a computer and that matters to
  you, use "Forget everything" when you're done, the same as you would with any shared computer.
- Unticking the offer saves a plain, readable file instead, behind a one-sentence warning about what
  it would expose; this remains your choice to make.

## What the site downloads

Ready Reckoner needs data to work: the list of counties, hazard rates, outage history, and so on.
All of it is fetched from the site's own address — never from anywhere else — and only as needed:

- The county data (hazards, outages, floods, earthquakes, climate projections, nearby facilities,
  community measures, and, since v0.3.0, county eviction rates and the tables that used to be
  separate optional downloads) loads as soon as the app starts, and is cached after the first visit
  so the app keeps working offline.
- The ZIP code list only loads once you start typing a ZIP code, since not everyone uses one.
- The county outline map only loads when a map is actually shown.
- The county's list of hospitals with an emergency room (new in v0.3.0) only loads the first time
  you open your Binder tab, for its Neighborhood page. A household that never opens the Binder tab
  never downloads it.
- The tools used to build and show your binder as a PDF (new in v0.3.0) only load the first time you
  press "Download PDF".
- Fetching any of this tells the site's host that a copy was downloaded — the same as loading any
  web page — but reveals nothing about your location or household, because the lookup itself happens
  afterward, on your device. A first visit to the site, before you've answered anything, downloads
  everything needed to use the whole app offline afterward (county data, the planning engine itself,
  and the app's own code): a few megabytes, most of it the county data every household eventually
  needs. Maps, the PDF tool and the hospital list only add to that if and when you actually use them.

## What the site's host sees

The site is hosted on GitHub Pages. Like any web host, its servers keep ordinary request logs: IP
address, browser identity, and which files were requested. GitHub keeps this kind of log for every
page it hosts, not only this one — it is the host's log, not ours, and Ready Reckoner has no
analytics of its own and adds nothing to it. The page also sets `no-referrer` by default, so
clicking a source link out to, say, FEMA's website does not tell FEMA which page on this site you
came from. (The one exception is the maps feature below, which deliberately sends the site's address
— and nothing more — to the handful of map services it uses, because their own rules ask for it.)

**A caution about the address.** This site currently shares its address,
`holdthedoorhoid.github.io`, with the project owner's other pages. Browser storage belongs to the
whole address, not to one page on it, so in principle any page published at that same address could
read a saved plan sitting in `rr.plan.v1` or the map images in `rr-maps` — and since v0.2.0, that can
include the names and phone numbers in your family plan, and since v0.3.0, real medical and
financial details and exactly where you live, not only your own answers. The interview's optional
steps say this in the same words: *"Names and phone numbers are other people's details too, so keep
a saved file somewhere safe. Until Ready Reckoner has a web address of its own, other pages at the
same address could in principle read what this browser keeps. If that matters to you, write this
plan on the printed binder instead."* Before relying on this for a real emergency, the site should
move to an address of its own — a dedicated organisation or a custom domain. Until then, treat this
like any shared computer: use "Forget everything" when you're done if that matters to you, protect
a saved file with a passphrase (on by default once it would need it), and consider keeping sensitive
answers on the printed binder rather than in the browser if you'd rather not store them there at all.

## The content security policy, and its limits

The built site carries a content security policy that blocks inline scripts and stops the page from
loading scripts, styles, fonts, images, or data from anywhere but itself and, since v0.3.0, the
handful of map services named below:

```
default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline';
img-src 'self' data: blob: <the 7 map origins below>; font-src 'self';
connect-src 'self' <the 7 map origins below>; worker-src 'self'; manifest-src 'self';
object-src 'none'; base-uri 'self'; form-action 'none'
```

The seven map origins are exactly the ones the "Maps" section below names: OpenStreetMap's tile and
search servers, the Census Bureau's fallback map, the two Overpass "nearby places" servers, FEMA's
flood-map service, and the US Forest Service's wildfire-map service. They are built into the policy
automatically from the one settings file that also drives the consent screen, so the two can never
drift apart — a request to anywhere not on that list is blocked by the browser itself, not merely
discouraged.

Two honest limits:

- It is delivered as a `<meta>` tag in the page itself, because GitHub Pages cannot be configured
  to send custom response headers. A tag-based policy cannot include `frame-ancestors`, so nothing
  stops another site from loading Ready Reckoner inside a frame. This does not expose your data —
  the page still runs with the same restrictions — but it is a gap compared to a policy sent as a
  real header.
- Styles allow `'unsafe-inline'` because the interface library sets some styles directly on
  elements (for example, a progress bar's width). Scripts have no such exception.

## Maps: the one feature that asks before it contacts anyone else

Everything above happens with no outside request at all. Since v0.3.0, the Binder tab (and the
Getting-out card in the interview) can add three printable maps of your area. This is the first,
and so far the only, feature that sends anything to an outside service — and it never does so
without asking you first, **every single time**, on a screen that names the recipient and what it
would receive before you turn it on. A first visit, and a household that never presses "Add maps,"
makes no request to any of these; this is checked automatically on every build of the site.

**Who sees what**, if and when you turn a layer on. Every path here has its own consent screen,
shown every time, never skipped: the full screen's **"Fetch maps"** button covers the street map and
whichever of nearby places, flood zones and wildfire hazard you tick; placing your pins instead, from
"Set your home point on a map," shows a shorter screen naming only the street map, then opens the
same interactive map to place pins on (since placing a pin needs a map to click on, even before
anything else has been fetched).

| Who | Address | When | What it receives | Checked against |
| --- | --- | --- | --- | --- |
| OpenStreetMap Foundation (UK) | `tile.openstreetmap.org` | Opening the pin map; "Fetch maps" with "Street map" ticked | The map squares around your home, your meeting places, and anywhere else you look at on the pin map | The OSMF Tile Usage Policy |
| U.S. Census Bureau (fallback) | `tigerweb.geo.census.gov` | Only if OpenStreetMap's tile server doesn't answer | The same map squares | Public domain; no published limits |
| Overpass (FOSSGIS e.V., Germany) | `overpass-api.de` | "Fetch maps" with "Nearby places" ticked | A query naming the boxes around your home and the kinds of place you'd see on the map (never your exact address) | The OSM wiki's Overpass guidance |
| Overpass fallback (Private.coffee) | `overpass.private.coffee` | Only if the first Overpass server is busy or fails | The same query | The same guidance |
| FEMA | `hazards.fema.gov` | "Fetch maps" with "Flood zones" ticked | The box around your neighbourhood map (about 1.5 km/1 mile across) | Federal, public |
| U.S. Forest Service | `imagery.geoplatform.gov` | "Fetch maps" with "Wildfire hazard" ticked | The box around your city or county map | Federal, public |
| OpenStreetMap Foundation (search) | `nominatim.openstreetmap.org` | Only if you choose "Type an address instead" (after its own warning), on each "Search" press | The address you typed | The Nominatim Usage Policy |

Every one of these also sees your internet (IP) address, what browser you use, and this site's
address, the same as any website you visit sees — that's unavoidable for any network request. **None
of them is ever sent your name, your household, or your answers**, and every request is made without
cookies and with only the site's address as its Referer (never the page you were on or the `#/…`
route), so none of them can tell that two requests came from the same visit, let alone the same
household. Frame centres are also snapped to a grid about 230 m across before any request is made,
so no single map pinpoints your home more precisely than that.

**Storm surge is deliberately left out.** NOAA does not currently publish its storm-surge risk maps
as a map service this app can use (only as downloads meant for GIS software, or through a private
company's cache we did not want to add as an eighth recipient); where it would matter, the map and
its legend say so and point to your state's own evacuation-zone tool instead.

**What stays on your device:** the three finished map images, each built in your browser from the
tiles and data above, go into `rr-maps` (above) — never the raw tiles, and never anything sent back
out. Refreshing or removing the maps is always available from the binder's map panel.

## How to check this yourself

You don't have to take this document's word for it:

- Open your browser's developer tools, go to the Network tab, and reload the site. Every request
  should go to the site's own address. Nothing should fire after the page and its data have loaded,
  other than more of the site's own files when you reach a screen that needs them (the map, the ZIP
  list, the county hospital list, the PDF tool) — and, if you choose to use it, the maps feature,
  whose requests you can watch appear only after you press "Fetch maps" or "Search," each going only
  to the addresses listed above.
- In developer tools, under Application (or Storage), open Local Storage for this site. You should
  see `rr.plan.v1` and, if you've changed the theme or turned on the expert view, `rr.prefs.v1` —
  nothing else. If you've ever fetched a map, you'll also see an IndexedDB database named `rr-maps`
  alongside it.
- The project's own automated end-to-end check (`web/scripts/e2e.mjs`) does this on every build: it
  records every request a first-time visit makes and fails the build if any of them leaves the
  site's own address; it separately confirms the maps' consent screen appears before any map
  request and that requests go only to the services named above; and it reloads the page with the
  network turned off to confirm the plan, and the binder, still work.
