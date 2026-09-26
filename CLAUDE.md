# Ready Reckoner — rules for agents working in this repo

Read `docs/DESIGN.md` first. It is authoritative. Then `docs/PRINCIPLES.md` (what we will and will
not say) and `docs/ENGINE-API.md` (the contract between the Rust engine and the web app). If you
change the contract, bump `ENGINE_API_VERSION`, update `docs/ENGINE-API.md` and `web/src/engine/types.ts`
in the same commit.

## What this is

A privacy-preserving, offline-capable disaster-preparedness planner. Location + household -> cited
quantitative risk register -> consequence-bucket targets ("N days without power") -> budgeted, phased
plan -> printable packet. Rust engine compiled to WebAssembly, Svelte/TypeScript web app, static site
on GitHub Pages. No server, no accounts, no analytics.

## Hard rules

1. **Deterministic engine.** Same inputs, same outputs, byte for byte. No wall clock inside the engine
   (the planning date is an input). No OS entropy and no `rand` crate (`getrandom` breaks wasm32); if
   you ever need randomness, use the seeded generator in `rr-types`. Everything under `crates/` except
   `rr-cli` and `rr-etl` must compile for `wasm32-unknown-unknown`. **Transcendental math goes through
   `rr_types::math` (`exp`, `ln`, `pow`, `norm_cdf`, pure-Rust libm)**, never `f64::exp`/`ln`/`powf`:
   the browser and glibc round differently and CLI goldens would drift from browser output.
   `clippy.toml` rejects the std methods.
2. **Every number carries a citation.** A quantity, rate, price band or duration that reaches the user
   has a `CitationId` resolving to `content/citations.toml`, or it does not ship. Expert priors are
   allowed but are tagged `Prior` and rendered as such.
3. **No network from the engine.** The web app fetches only same-origin data packs by default. Any
   lookup that would send the user's location elsewhere sits behind an explicit, per-use consent
   screen that names the recipient. Warn, do not block.
4. **Content policy is in `docs/PRINCIPLES.md`.** No brands, no store links, no drug-by-condition
   dosing, firearms never in the budget, no fear appeals without a paired action. Do not "improve"
   on these; open an issue.
5. **Licences.** Code GPLv3. Text and tables under `content/` and `docs/` CC BY-SA 4.0. Data packs
   are built from public-domain federal sources; each source is recorded in `data/manifest.json`
   with URL, retrieval date, checksum and licence. Never commit raw downloads (`data/raw/` is ignored).
   Do not add a data source whose licence forbids redistribution or requires attribution beyond a
   line in the manifest without asking.
6. **Plain language.** User-facing strings read at roughly an eighth-grade level. Name things by what
   they do. Natural frequencies ("about 12 of 100 households like yours") before percentages.

## Building and testing

- `cargo build`, `cargo test`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`
  must all pass. `.cargo/config.toml` sets `jobs = 3` and turns incremental compilation off; do not
  change either (sccache is the shared cache and is incompatible with incremental; the machine has
  8 cores and 15 GB and runs several agents at once).
- Web: `cd web && npm ci && npm test -- --maxWorkers=2 && npm run check && npm run build`.
- Never run the Rust and web test suites at the same time on this machine.
- The CLI is the oracle: `cargo run -p rr-cli -- plan --household fixtures/households/<name>.json`
  prints the full packet. Golden packets live in `fixtures/golden/`; a change to a golden must be
  explained in the commit message.
- Property tests guard monotonicity (more people -> more water, higher confidence -> more days, more
  budget -> never less coverage) and invariants (spend never exceeds budget, every item cites).
- CI parses every workflow file before running anything. In YAML, quote any `run:` line that contains
  `::` (an unquoted `cargo test foo::` once broke Pages silently for four pushes on another project).

## Agent process

- Work in your own worktree: `git -C ~/Desktop/ready-reckoner worktree add ~/Desktop/ready-reckoner-wt/<name> -b agent/<name>`.
  Each worktree has its own `target/`. **Remove your worktree when your branch is merged**
  (`git worktree remove`); the disk has ~30 GB free and a Rust target dir is 2–4 GB.
- Commit early and often with clear messages. End each commit message with a
  `Co-Authored-By: <your model name> <noreply@anthropic.com>` line. Never push; the planner merges.
- Briefs, research reports and screenshots for agents live in `~/Desktop/ready-reckoner-briefs/`
  (durable; the session scratchpad under `/tmp` has been wiped before).
- Do not edit another agent's crate or files unless your brief says so. If you need something from a
  crate that is not merged yet, code against `docs/ENGINE-API.md` / `rr-types` and leave a
  `// awaiting: <crate>` comment.
- Merge conflicts in docs are usually appends: keep both sides, then re-read the result.
- When done, report: what you built, how you verified it (commands + results), what you did not do,
  and any decision you made that the planner or the owner should know about.

## Talking to the owner

The owner is not a programmer. Report what a user will see or what changed in the numbers, not
function names. When a tradeoff needs their input, state the consequences (what it costs, what
breaks, who it affects) and recommend one option.
