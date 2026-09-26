#!/usr/bin/env bash
# Builds the engine for the web site and puts the data packs next to it, so the site serves both
# from its own origin:
#
#   web/public/pkg/rr_wasm.js, rr_wasm_bg.wasm     the engine (wasm-pack, --target web)
#   web/public/data/manifest.json, core/, geo/      copies of data/ (the packs rr-etl builds)
#
# Usage (from anywhere in the repository):
#
#   bash crates/rr-wasm/build-web.sh               # engine + data
#   bash crates/rr-wasm/build-web.sh --data-only   # refresh the data copies only
#
# Size is steered with environment variables, never with --profile (the wasm-pack the Pages
# workflow installs rejects it): CARGO_PROFILE_RELEASE_OPT_LEVEL (default "s", as the workspace's
# release profile) and the wasm-opt flags in crates/rr-wasm/Cargo.toml. The script prints the raw
# and gzipped size of the .wasm; the budget is 1.5 MB gzipped.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
pkg="$repo/web/public/pkg"
data_in="$repo/data"
data_out="$repo/web/public/data"
budget_bytes=$((1500 * 1000))

data_only=false
for arg in "$@"; do
  case "$arg" in
    --data-only) data_only=true ;;
    *) echo "build-web.sh: unknown argument $arg (use --data-only or nothing)" >&2; exit 2 ;;
  esac
done

gz_size() { gzip -9 -c "$1" | wc -c | tr -d ' '; }
mb() { awk -v b="$1" 'BEGIN { printf "%.2f MB", b / 1000000 }'; }

# --- Engine -------------------------------------------------------------------------------------
if [ "$data_only" = false ]; then
  if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "build-web.sh: wasm-pack is not installed (https://rustwasm.github.io/wasm-pack/installer/)" >&2
    exit 1
  fi
  export CARGO_PROFILE_RELEASE_OPT_LEVEL="${CARGO_PROFILE_RELEASE_OPT_LEVEL:-s}"
  echo "Building the engine (release, opt-level ${CARGO_PROFILE_RELEASE_OPT_LEVEL}) into web/public/pkg/"
  rm -rf "$pkg"
  # --out-dir is relative to the crate directory.
  wasm-pack build "$here" --target web --release --no-typescript --out-dir ../../web/public/pkg --out-name rr_wasm
  # The site needs only the module and the .wasm; drop wasm-pack's npm packaging files.
  rm -f "$pkg/package.json" "$pkg/README.md" "$pkg/.gitignore" "$pkg"/LICENSE*
fi

# --- Data packs ---------------------------------------------------------------------------------
rm -rf "$data_out"
if [ -f "$data_in/manifest.json" ]; then
  mkdir -p "$data_out"
  cp "$data_in/manifest.json" "$data_out/"
  for pack in core geo; do
    if [ -d "$data_in/$pack" ]; then
      cp -R "$data_in/$pack" "$data_out/$pack"
    fi
  done
  echo "Copied data/manifest.json, data/core/ and data/geo/ into web/public/data/"
else
  echo "build-web.sh: data/manifest.json is missing, so the site will plan with the engine's seven built-in sample counties" >&2
fi

# --- Sizes --------------------------------------------------------------------------------------
if [ -f "$pkg/rr_wasm_bg.wasm" ]; then
  raw=$(wc -c < "$pkg/rr_wasm_bg.wasm" | tr -d ' ')
  gz=$(gz_size "$pkg/rr_wasm_bg.wasm")
  js_gz=$(gz_size "$pkg/rr_wasm.js")
  echo "SIZE rr_wasm_bg.wasm: ${raw} bytes raw ($(mb "$raw")), ${gz} bytes gzipped ($(mb "$gz")); budget $(mb "$budget_bytes") gzipped"
  echo "SIZE rr_wasm.js: ${js_gz} bytes gzipped"
  if [ "$gz" -gt "$budget_bytes" ]; then
    echo "build-web.sh: WARNING: the engine is over its size budget (see docs/ENGINE-API.md, Loading)" >&2
  fi
fi
if [ -d "$data_out/core" ]; then
  core_gz=0
  for f in "$data_out"/core/*; do core_gz=$((core_gz + $(gz_size "$f"))); done
  echo "SIZE data/core: ${core_gz} bytes gzipped file by file ($(mb "$core_gz"))"
fi
