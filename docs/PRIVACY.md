# Privacy

Ready Reckoner is a static website. There is no server behind it, no account to create, and no
analytics. The planning engine runs on your device, in your browser. This page says exactly what
that means: what stays on your computer, what the site downloads, what the host can see, and how
to check all of it yourself.

## What stays on your device

Everything you type — your location, your household, your budget, what you already own, what
you've checked off — is kept only in your browser's local storage. It is never sent anywhere.

Two separate entries are used:

- **`rr.plan.v1`** holds your household, your dial settings, what you already have, your check-offs
  and paid amounts, and your progress through the interview. This is the entry that "Save a copy of
  your plan" writes out as a file (`ready-reckoner-plan.json`) and "Open a saved plan" reads back
  in. It never leaves your browser unless you choose to save or share that file yourself.
- **`rr.prefs.v1`** holds two display preferences: your light/dark theme choice and whether the
  expert view is turned on. It holds no household information, and it is not part of the saved
  file — importing or exporting a plan never touches it.

"Forget everything" (on the Keep it up screen) deletes both entries, and anything else the app may
have stored, from your browser. Nothing is recoverable after that unless you had saved a copy.

The app uses no cookies, no `sessionStorage`, and no other browser database.

## What the site downloads

Ready Reckoner needs data to work: the list of counties, hazard rates, outage history, and so on.
All of it is fetched from the site's own address — never from anywhere else — and only as needed:

- The county data (hazards, outages, floods, earthquakes, climate projections, nearby facilities,
  community measures) is about 8.8 MB as stored, in 15 files, compressed to roughly 3 MB to
  download. It loads as soon as the app starts.
- The ZIP code list (about 1.9 MB, 3 files) only loads once you start typing a ZIP code, since not
  everyone uses one.
- The county outline map (about 1.4 MB) only loads when a map is actually shown.

These files are cached after the first visit, so the app keeps working offline. Fetching them tells
the site's host that a copy was downloaded — the same as loading any web page — but reveals nothing
about your location or household, because the lookup itself happens afterward, on your device.

## What the site's host sees

The site is hosted on GitHub Pages. Like any web host, its servers keep ordinary request logs: IP
address, browser identity, and which files were requested. GitHub keeps this kind of log for every
page it hosts, not only this one — it is the host's log, not ours, and Ready Reckoner has no
analytics of its own and adds nothing to it. The page also sets `no-referrer`, so clicking a source
link out to, say, FEMA's website does not tell FEMA which page on this site you came from.

**A caution about the address.** This site currently shares its address,
`holdthedoorhoid.github.io`, with the project owner's other pages. Browser storage belongs to the
whole address, not to one page on it, so in principle any page published at that same address could
read a saved plan sitting in `rr.plan.v1`. Before relying on this for a real emergency, the site
should move to an address of its own — a dedicated organisation or a custom domain. Until then,
treat this like any shared computer: use "Forget everything" when you're done if that matters to
you.

## The content security policy, and its limits

The built site carries a content security policy that blocks inline scripts and stops the page from
loading scripts, styles, fonts, images, or data from anywhere but itself:

```
default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline';
img-src 'self' data: blob:; font-src 'self'; connect-src 'self'; worker-src 'self';
manifest-src 'self'; object-src 'none'; base-uri 'self'; form-action 'none'
```

Two honest limits:

- It is delivered as a `<meta>` tag in the page itself, because GitHub Pages cannot be configured
  to send custom response headers. A tag-based policy cannot include `frame-ancestors`, so nothing
  stops another site from loading Ready Reckoner inside a frame. This does not expose your data —
  the page still runs with the same restrictions — but it is a gap compared to a policy sent as a
  real header.
- Styles allow `'unsafe-inline'` because the interface library sets some styles directly on
  elements (for example, a progress bar's width). Scripts have no such exception.

## Optional lookups that do not exist yet

Two features are planned but not built: checking your exact address against FEMA's flood maps, and
pulling live weather alerts for your area. Both would send something about your location to an
outside service, so neither will ever run without you turning it on first, on a screen that names
exactly who would see what. Until then, the app makes no request like this at all.

## How to check this yourself

You don't have to take this document's word for it:

- Open your browser's developer tools, go to the Network tab, and reload the site. Every request
  should go to the site's own address. Nothing should fire after the page and its data have loaded,
  other than more of the site's own files when you reach a screen that needs them (the map, the ZIP
  list).
- In developer tools, under Application (or Storage), open Local Storage for this site. You should
  see only `rr.plan.v1` and, if you've changed the theme or turned on the expert view,
  `rr.prefs.v1` — nothing else.
- The project's own automated end-to-end check (`web/scripts/e2e.mjs`) does this on every build: it
  records every request a first-time visit makes and fails the build if any of them leaves the
  site's own address, then reloads the page with the network turned off to confirm the plan still
  works.
