<!--
  Screen 1, Where you live: ZIP code (resolved to a county from the list built into the app, with
  a county picker when a ZIP code spans several and a county search as the fallback), the kind of
  area, and the home: type, floor, tenure, water, sewer, heating, cooling and backup power, and
  (contract v2, all optional) a bedroom below street level, the water system's record, raw water
  nearby to filter, and what the household cooks on.
  A small map confirms the county (or numbers the counties a ZIP code spans). The ZIP code list
  starts loading when someone starts typing one; a county found by name never needs it.
-->
<script lang="ts">
  import ChoiceGroup from '../components/ChoiceGroup.svelte';
  import CheckRow from '../components/CheckRow.svelte';
  import CountyMap from '../components/CountyMap.svelte';
  import Field from '../components/Field.svelte';
  import Icon from '../components/Icon.svelte';
  import InterviewNav from '../components/InterviewNav.svelte';
  import NumberField from '../components/NumberField.svelte';
  import ProgressSteps from '../components/ProgressSteps.svelte';
  import type { EngineError, Housing, HousingKind, LocationResolved, LocationSuggestions, WaterSource } from '../engine/types';
  import {
    BACKUP_POWER_KINDS,
    COOKING_FUELS,
    COOLING_KINDS,
    HEATING_KINDS,
    HOUSING_KINDS,
    RAW_WATER_SOURCES,
    SETTINGS,
    TENURES,
    WASTEWATER_KINDS,
    WATER_SOURCES,
    WATER_SYSTEM_RECORDS,
  } from '../engine/types';
  import { PLACEHOLDER_ZIP } from '../engine/data-files';
  import { useApp } from '../lib/app.svelte';
  import { BACKUP, COOKING, COOLING, HEATING, HOUSING_KIND, RAW_WATER, SETTING, SEWER, TENURE, WATER, WATER_RECORD } from '../lib/labels';
  import type { FieldProblem } from '../lib/ui-types';

  const app = useApp();
  const input = $derived(app.plan?.input);

  let zipText = $state(app.plan?.input.location.zip && app.plan.input.location.zip !== PLACEHOLDER_ZIP ? app.plan.input.location.zip : '');
  let zipTouched = $state(false);
  let resolved = $state<LocationResolved | null>(null);
  let lookupError = $state<EngineError | null>(null);
  let searchText = $state('');
  let searchResults = $state<LocationResolved[]>([]);
  let searchDone = $state(false);
  let searchOpen = $state(false);
  let searching = $state(false);
  /** A lookup is waiting for its answer (on a first visit, the county data may still be loading). */
  let looking = $state(false);
  /** Bumped by "Try again", so the lookup runs again. */
  let attempt = $state(0);

  let ambiguity = $state<{ zip: string; suggestions: LocationResolved[] } | null>(null);
  const suggestions = $derived(ambiguity?.suggestions ?? []);
  const isApartment = $derived(input?.housing.kind === 'apartment_high_rise' || input?.housing.kind === 'apartment_low_rise');

  function validZip(zip: string | undefined): string | undefined {
    return zip && zip !== PLACEHOLDER_ZIP && /^\d{5}$/.test(zip) ? zip : undefined;
  }

  // Look the location up whenever the ZIP code or the chosen county changes (on this device).
  // The ZIP code alone tells us whether it spans several counties; the chosen county decides.
  let lookupSeq = 0;
  $effect(() => {
    const loc = input?.location;
    void attempt;
    if (!loc || !app.engine) return;
    const zip = validZip(loc.zip);
    const county = loc.county_fips;
    const setting = loc.setting;
    const seq = ++lookupSeq;
    if (!zip && !county) {
      resolved = null;
      lookupError = null;
      ambiguity = null;
      looking = false;
      return;
    }
    const engine = app.engine;
    const timer = setTimeout(async () => {
      looking = true;
      let res: LocationResolved | null = null;
      let err: EngineError | null = null;
      let amb: { zip: string; suggestions: LocationResolved[] } | null = null;
      if (zip) {
        const r = await engine.resolve_location({ country: 'US', setting, zip });
        if (r.ok) res = r.value;
        else if (r.error.code === 'ambiguous_zip') amb = { zip, suggestions: (r.error.details as LocationSuggestions | undefined)?.suggestions ?? [] };
        else err = r.error;
      }
      if (county) {
        const r = await engine.resolve_location({ country: 'US', setting, county_fips: county, ...(zip ? { zip } : {}) });
        if (r.ok) {
          res = r.value;
          err = null;
        } else {
          err = r.error;
        }
      }
      if (seq !== lookupSeq) return;
      resolved = res;
      lookupError = err;
      ambiguity = amb;
      looking = false;
    }, 150);
    return () => clearTimeout(timer);
  });

  function tryAgain() {
    app.retryData();
    attempt += 1;
  }

  function setZip(value: string) {
    const digits = value.replace(/\D/g, '').slice(0, 5);
    // Someone is typing a ZIP code: start fetching the ZIP code list now, not at the fifth digit.
    if (digits.length > 0) app.prefetchZip();
    zipText = digits;
    if (!app.plan) return;
    const loc = app.plan.input.location;
    delete loc.county_fips;
    if (digits) loc.zip = digits;
    else delete loc.zip;
  }

  function pickCounty(c: LocationResolved, keepZip: boolean) {
    if (!app.plan) return;
    const loc = app.plan.input.location;
    loc.county_fips = c.county_fips;
    if (!keepZip) {
      delete loc.zip;
      zipText = '';
    }
    searchOpen = false;
    searchResults = [];
    searchText = '';
  }

  let searchSeq = 0;
  let searchError = $state('');
  async function runSearch() {
    if (!app.engine) return;
    const q = searchText;
    const seq = ++searchSeq;
    searching = q.trim().length > 0;
    const r = await app.engine.county_search(q);
    if (seq !== searchSeq) return;
    searching = false;
    searchResults = r.ok ? r.value : [];
    searchError = r.ok ? '' : r.error.message;
    searchDone = q.trim().length > 0;
  }

  /** "About 55% of this ZIP code's land area": shares are of land, not of homes. */
  function sharePhrase(share: number): string {
    const pct = Math.round(share * 100);
    return pct < 1 ? "Less than 1% of this ZIP code's land area" : `About ${pct}% of this ZIP code's land area`;
  }

  const zipError = $derived.by(() => {
    if (input?.location.county_fips && !input.location.zip) return undefined;
    if (zipText.length > 0 && zipText.length < 5 && zipTouched) return 'A ZIP code is five digits, like 19147.';
    if (lookupError?.code === 'unknown_zip') return "We couldn't find that ZIP code. Check the digits, or search for your county below.";
    return undefined;
  });

  const problems = $derived.by((): FieldProblem[] => {
    const out: FieldProblem[] = [];
    const loc = input?.location;
    if (!loc) return out;
    const hasZip = !!loc.zip && loc.zip !== PLACEHOLDER_ZIP;
    if (!hasZip && !loc.county_fips) out.push({ id: 'zip', message: 'Enter your ZIP code, or search for your county.' });
    else if (hasZip && !/^\d{5}$/.test(loc.zip!) && !loc.county_fips) out.push({ id: 'zip', message: 'A ZIP code is five digits, like 19147.' });
    else if (lookupError?.code === 'unknown_zip') out.push({ id: 'zip', message: "We couldn't find that ZIP code. Check it, or search for your county." });
    else if (ambiguity && !loc.county_fips) out.push({ id: 'county-pick-0', message: 'Choose which county you live in.' });
    return out;
  });

  function setKind(kind: HousingKind) {
    if (!app.plan) return;
    const h = app.plan.input.housing;
    h.kind = kind;
    if (kind !== 'apartment_high_rise' && kind !== 'apartment_low_rise') h.floor = 1;
    else h.basement = false;
    keepBelowGradeAnswer(h);
  }

  /** Asked only where someone could sleep below street level: a basement, or a flat below ground. */
  function belowGradeAsked(h: Housing): boolean {
    const apartment = h.kind === 'apartment_high_rise' || h.kind === 'apartment_low_rise';
    return apartment ? h.floor <= 0 : h.basement;
  }

  /** A hidden question keeps no answer: the plan never acts on something the screen no longer shows. */
  function keepBelowGradeAnswer(h: Housing) {
    if (!belowGradeAsked(h) && h.below_grade_bedroom) h.below_grade_bedroom = false;
  }

  function setWater(v: WaterSource) {
    // The water system's record is asked only of households on public water.
    if (v === 'well' && app.plan) delete app.plan.input.housing.water_system_record;
  }
</script>

<div class="page page--narrow">
  <ProgressSteps current="where" />
  <h1 id="page-title" tabindex="-1">Where you live</h1>
  <p class="lead">Your county decides which hazards matter. Your home decides what they would do to you.</p>

  {#if input}
    <p class="privacy"><Icon name="lock" /> Your county is looked up in a list built into this app. Nothing is sent anywhere.</p>

    <section aria-labelledby="area-title">
      <h2 id="area-title">Your area</h2>
      <Field id="zip" label="ZIP code" help="Five digits. Used only to find your county." error={zipError}>
        {#snippet children({ describedBy, invalid })}
          <input
            id="zip"
            class="input input--short num"
            type="text"
            inputmode="numeric"
            autocomplete="postal-code"
            maxlength="5"
            aria-describedby={describedBy}
            aria-invalid={invalid}
            value={zipText}
            oninput={(e) => setZip((e.currentTarget as HTMLInputElement).value)}
            onblur={() => (zipTouched = true)}
          />
        {/snippet}
      </Field>

      <div class="lookup" aria-live="polite">
        {#if looking && !resolved && suggestions.length === 0}
          <p class="small muted" aria-busy="true">
            Looking up your county{app.data?.core.phase === 'loading'
              ? ' (the county data is still loading)'
              : app.data?.zip.phase === 'loading'
                ? ' (getting the ZIP code list, once)'
                : ''}…
          </p>
        {/if}
        {#if lookupError?.code === 'pack_missing'}
          <div class="lookup-problem">
            <p class="small">{lookupError.message}</p>
            <button type="button" class="button button--small" onclick={tryAgain}>Try again</button>
          </div>
        {/if}
        {#if resolved && suggestions.length === 0}
          <div class="found-row">
            <div>
              <p class="found"><Icon name="check" /> <span>That's <strong>{resolved.county_name}, {resolved.state_name}</strong>.</span></p>
              {#if resolved.data_note}<p class="small muted">{resolved.data_note}</p>{/if}
            </div>
            <CountyMap location={resolved} size="small" caption={false} />
          </div>
        {/if}
        {#if suggestions.length > 0}
          <fieldset class="pick">
            <legend>That ZIP code covers more than one county. Which one do you live in?</legend>
            <p class="help">
              The shares are of land area, not of homes. Not sure which county you are in? It is often printed on a voter registration card or
              a car registration.
            </p>
            <div class="pick__body">
              <div class="choices">
                {#each suggestions as s, i (s.county_fips)}
                  <label class="choice">
                    <input
                      id="county-pick-{i}"
                      type="radio"
                      name="county-pick"
                      checked={input.location.county_fips === s.county_fips}
                      onchange={() => pickCounty(s, true)}
                    />
                    <span class="choice__num" aria-hidden="true">{i + 1}</span>
                    <span class="choice__text">
                      <span>{s.county_name}, {s.state_abbr}</span>
                      {#if s.zip_county_share !== undefined}<span class="choice__help">{sharePhrase(s.zip_county_share)}</span>{/if}
                    </span>
                  </label>
                {/each}
              </div>
              <CountyMap candidates={suggestions} selected={input.location.county_fips} size="small" />
            </div>
            {#if resolved && input.location.county_fips}
              <p class="found"><Icon name="check" /> <span>That's <strong>{resolved.county_name}, {resolved.state_name}</strong>.</span></p>
            {/if}
          </fieldset>
        {/if}
      </div>

      <details class="search" bind:open={searchOpen}>
        <summary>Search by county instead</summary>
        <div class="search__body">
          <label for="county-search">County name, or "name, state"</label>
          <span class="help" id="county-search-help">For example: Philadelphia, or Cook, IL.</span>
          <div class="search__row">
            <input
              id="county-search"
              class="input input--medium"
              type="search"
              aria-describedby="county-search-help"
              bind:value={searchText}
              oninput={runSearch}
            />
          </div>
          {#if searching && searchResults.length === 0}
            <p class="small muted" aria-busy="true">Searching{app.data && app.data.core.phase === 'loading' ? ' (the county data is still loading)' : ''}…</p>
          {:else if searchResults.length > 0}
            <ul class="results" aria-label="Matching counties">
              {#each searchResults as c (c.county_fips + (c.zip ?? ''))}
                <li><button type="button" class="button button--small" onclick={() => pickCounty(c, false)}>{c.county_name}, {c.state_abbr}</button></li>
              {/each}
            </ul>
          {:else if searchError}
            <p class="small">{searchError}</p>
          {:else if searchDone}
            <p class="small">No county matches that. Try fewer letters.</p>
          {/if}
        </div>
      </details>

      <ChoiceGroup
        legend="What kind of area is it?"
        name="setting"
        help="Changes how far stores and help are, and how long outages last."
        options={SETTINGS.map((v) => ({ value: v, label: SETTING[v].label, help: SETTING[v].help }))}
        bind:value={input.location.setting}
        columns={3}
      />
    </section>

    <section aria-labelledby="home-title">
      <h2 id="home-title">Your home</h2>
      <ChoiceGroup
        legend="What kind of home?"
        name="kind"
        options={HOUSING_KINDS.map((v) => ({ value: v, label: HOUSING_KIND[v].label, help: HOUSING_KIND[v].help }))}
        value={input.housing.kind}
        onchange={setKind}
        columns={2}
      />
      {#if isApartment}
        <NumberField
          id="floor"
          label="Which floor do you live on?"
          help="1 is street level. Use a negative number for below ground. High floors depend on elevators and water pumps."
          value={input.housing.floor}
          integer
          min={-5}
          max={127}
          example="3"
          onchange={(v) => {
            if (app.plan && v !== undefined) {
              app.plan.input.housing.floor = v;
              keepBelowGradeAnswer(app.plan.input.housing);
            }
          }}
        />
      {:else}
        <CheckRow
          label="The home has a basement"
          help="Basements flood first, and can be a safe spot in a tornado."
          bind:checked={input.housing.basement}
          onchange={() => app.plan && keepBelowGradeAnswer(app.plan.input.housing)}
        />
      {/if}
      {#if belowGradeAsked(input.housing)}
        <CheckRow
          label="Someone sleeps below street level"
          help="A bedroom in the basement, or a flat below ground. Water can fill these rooms in minutes in a flash flood."
          checked={input.housing.below_grade_bedroom ?? false}
          onchange={(on) => (input.housing.below_grade_bedroom = on)}
        />
      {/if}
      <ChoiceGroup legend="Do you own or rent?" name="tenure" help="Renters can't always change the building, so the plan leans on things you can take with you." options={TENURES.map((v) => ({ value: v, label: TENURE[v].label }))} bind:value={input.housing.tenure} columns={2} />
      <ChoiceGroup
        legend="Where does your water come from?"
        name="water"
        help="Decides whether a power cut also cuts your water."
        options={WATER_SOURCES.map((v) => ({ value: v, label: WATER[v].label, help: WATER[v].help }))}
        bind:value={input.housing.water}
        onchange={setWater}
        columns={2}
      />
      {#if input.housing.water === 'municipal'}
        <ChoiceGroup
          legend="Has your water system had problems?"
          name="water-record"
          help="Optional. Some systems have more boil-water notices and main breaks. Your answer is combined with EPA records for your county."
          options={WATER_SYSTEM_RECORDS.map((v) => ({ value: v, label: WATER_RECORD[v].label, help: WATER_RECORD[v].help }))}
          value={input.housing.water_system_record}
          onchange={(v) => (input.housing.water_system_record = v)}
          columns={2}
        />
      {/if}
      <ChoiceGroup
        legend="Is there water nearby you could filter if the taps stopped?"
        name="raw-water"
        help="Optional. A water filter only helps if there is water to filter."
        options={RAW_WATER_SOURCES.map((v) => ({ value: v, label: RAW_WATER[v].label, help: RAW_WATER[v].help }))}
        value={input.housing.raw_water_source}
        onchange={(v) => (input.housing.raw_water_source = v)}
        columns={2}
      />
      <ChoiceGroup legend="Where does wastewater go?" name="sewer" help="Decides what to do when toilets can't flush." options={WASTEWATER_KINDS.map((v) => ({ value: v, label: SEWER[v].label }))} bind:value={input.housing.sewer} columns={2} />
      <Field id="heating" label="Main heating" help="Most furnaces and heat pumps need electricity to run, even gas ones.">
        {#snippet children({ describedBy })}
          <select id="heating" class="input input--medium" aria-describedby={describedBy} bind:value={input.housing.heating}>
            {#each HEATING_KINDS as v (v)}<option value={v}>{HEATING[v].label}</option>{/each}
          </select>
        {/snippet}
      </Field>
      <ChoiceGroup
        legend="What do you cook on?"
        name="cooking"
        help="Optional. Decides whether you can cook and boil water when the power is out."
        options={COOKING_FUELS.map((v) => ({ value: v, label: COOKING[v].label, help: COOKING[v].help }))}
        value={input.housing.cooking}
        onchange={(v) => (input.housing.cooking = v)}
        columns={2}
      />
      <ChoiceGroup legend="Cooling" name="cooling" help="Air conditioning stops in a power cut; the plan adds a cooling plan either way." options={COOLING_KINDS.map((v) => ({ value: v, label: COOLING[v].label }))} bind:value={input.housing.cooling} columns={3} />
      <ChoiceGroup
        legend="Backup power at home"
        name="backup"
        help="What you could run during an outage, if anything."
        options={BACKUP_POWER_KINDS.map((v) => ({ value: v, label: BACKUP[v].label, help: BACKUP[v].help }))}
        bind:value={input.housing.backup_power}
        columns={2}
      />
    </section>

    <details class="lookups">
      <summary>Could this be more precise? (online lookups)</summary>
      <p>
        Everything here is worked out for your whole county from data built into the app, so nothing about you leaves this device.
        A later version may offer an optional flood-zone check for your exact address. It would send your address's approximate
        location to FEMA's flood map service, only when you ask for it, and it would say so each time. That check is not available yet.
      </p>
    </details>

    <InterviewNav step="where" {problems} />
  {:else}
    <p>Start a plan first. <a href="#/">Go to the start</a></p>
  {/if}
</div>

<style>
  .privacy {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    padding: var(--s3) var(--s4);
    background: var(--surface-2);
    border-radius: var(--r2);
    font-weight: 560;
  }
  .lookup {
    margin: calc(-1 * var(--s3)) 0 var(--s4);
  }
  .found {
    display: flex;
    gap: var(--s2);
    align-items: center;
    color: var(--good);
    margin-bottom: var(--s1);
  }
  .found strong {
    color: var(--text);
  }
  .found-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    align-items: flex-start;
    justify-content: space-between;
  }
  .found-row > :global(.county-map) {
    flex: 0 1 13.5rem;
  }
  .lookup-problem {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2) var(--s3);
    align-items: center;
    padding: var(--s2) var(--s3);
    border-left: 4px solid var(--warn-edge);
    background: var(--warn-soft);
    border-radius: var(--r1);
  }
  .lookup-problem p {
    margin: 0;
  }
  .pick {
    margin-top: var(--s3);
  }
  .pick__body {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    align-items: flex-start;
  }
  .pick__body .choices {
    flex: 1 1 16rem;
  }
  .pick__body > :global(.county-map) {
    flex: 0 1 13.5rem;
  }
  .choice__num {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 1.6rem;
    height: 1.6rem;
    border-radius: 50%;
    border: 1.5px solid var(--map-focus-line);
    font-weight: 700;
    font-size: var(--text-sm);
  }
  .search {
    margin-bottom: var(--s5);
  }
  .search__body {
    padding: var(--s2) 0 0;
  }
  .search__body label {
    display: block;
  }
  .results {
    list-style: none;
    padding: 0;
    margin: var(--s3) 0 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
  }
  .results li + li {
    margin-top: 0;
  }
  .lookups {
    margin-top: var(--s5);
    padding: var(--s3) var(--s4);
    border: 1px solid var(--border);
    border-radius: var(--r2);
    background: var(--surface);
  }
  .lookups p {
    margin: var(--s2) 0 var(--s2);
  }
</style>
