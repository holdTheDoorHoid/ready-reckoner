# Contributing

Thank you for helping people prepare for the right things.

## Ground rules

- **Cite it or leave it out.** Any quantity, rate, price band or duration that a user might see needs
  a source in `content/citations.toml`. Government guidance, standards bodies (Sphere, WHO, CDC),
  peer-reviewed research and reputable non-profits qualify. Blogs, forums and retailers do not, except
  as the source of a *price band*, which is clearly marked as a market observation.
- **Specs, not brands.** Items describe what to look for and what to avoid, and give a price range.
  Pull requests that add product names, store links, affiliate links or sponsor mentions will be closed.
- **Consequences before causes.** New guidance should attach to a consequence bucket (no power, no
  water, must leave, no income...) first, and to a hazard only for genuinely hazard-specific items.
- **Follow `docs/PRINCIPLES.md`** on medical, firearms, nuclear and children's content. These are
  policy, not style.
- **Plain language.** Aim for an eighth-grade reading level in anything a user reads.

## Licences

By contributing code you agree it is licensed under GPLv3. By contributing text, tables or catalogue
entries under `content/` or `docs/` you agree they are licensed under CC BY-SA 4.0. Do not paste text
that is under a more restrictive licence. United States federal government text is public domain and
may be included with a citation.

## Developer setup

See `CLAUDE.md` for the build and test commands (they apply to humans too) and `docs/DESIGN.md` for
the architecture. Open an issue before starting anything larger than a fix; the roadmap is in
`docs/ROADMAP.md`.

## Adding a data source

Read `docs/DATA_SOURCES.md`. A source needs: a stable download URL or API, a licence that permits
redistribution (public domain preferred), a documented update cadence, and an entry in the ETL
(`crates/rr-etl`) that records URL, retrieval date, checksum and licence in `data/manifest.json`.

## Reporting a wrong number

Open an issue titled "Wrong number: <what>" with the number the app shows, the number you believe is
right, and the source. These get priority.
