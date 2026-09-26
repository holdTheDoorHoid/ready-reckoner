/**
 * The mock packet: `packet_markdown` with the ten sections of docs/DESIGN.md §9. The app renders
 * it with a sanitising Markdown renderer and prints each `##` section on its own page. It uses
 * headings, paragraphs, lists, task lists, tables, block quotes and footnotes, so the renderer and
 * the print stylesheet are exercised the way the real packet will exercise them.
 */
import type { BucketId, PlanInput, PlanItem, TierId } from '../types';
import { band, dayPhrase, formatDate, addMonths, monthsPhrase, naturalFrequency, noticeRange, quantity, severityBand, targetDays, targetMonths, usd, CONFIDENCE_LABELS } from '../../lib/format';
import { accessNeedsLine, familyPlanSections } from './family';
import { catalogueItem } from './items';
import type { ModelResult } from './model';
import { TIERS } from './names';

const DIAL_LABEL: Record<PlanInput['dials']['return_period'], string> = {
  one_in_10: 'Common disruptions (1-in-10)',
  one_in_50: 'Serious (1-in-50)',
  one_in_100: 'Very serious (1-in-100)',
  one_in_500: 'Rare catastrophes (1-in-500)',
};

const WATER_LABEL = {
  survival: 'survival, about 3 litres per person per day (drinking only)',
  basic: 'basic, about 1 gallon per person per day',
  comfortable: 'comfortable, about 15 litres per person per day (drinking and washing)',
} as const;

const AGE_WORDS: Record<string, [string, string]> = {
  infant: ['baby', 'babies'],
  toddler: ['toddler', 'toddlers'],
  child: ['child', 'children'],
  teen: ['teenager', 'teenagers'],
  adult: ['adult', 'adults'],
  senior: ['older adult', 'older adults'],
};

function tierName(id: TierId): string {
  return TIERS.find((t) => t.id === id)!.name;
}

/** Escape text for a Markdown table cell or list item. */
function md(text: string): string {
  return text.replace(/\|/g, '\\|').replace(/([*_`[\]])/g, '\\$1');
}

export function householdPhrase(input: PlanInput): string {
  const counts = new Map<string, number>();
  for (const p of input.people) counts.set(p.age_band, (counts.get(p.age_band) ?? 0) + 1);
  const parts = ['adult', 'senior', 'teen', 'child', 'toddler', 'infant']
    .filter((b) => counts.has(b))
    .map((b) => {
      const n = counts.get(b)!;
      const [one, many] = AGE_WORDS[b]!;
      return `${n} ${n === 1 ? one : many}`;
    });
  const pets: string[] = [];
  const { dogs, cats, small, large_animals } = input.pets;
  if (dogs) pets.push(`${dogs} ${dogs === 1 ? 'dog' : 'dogs'}`);
  if (cats) pets.push(`${cats} ${cats === 1 ? 'cat' : 'cats'}`);
  if (small) pets.push(`${small} small ${small === 1 ? 'pet' : 'pets'}`);
  if (large_animals) pets.push(`${large_animals} large ${large_animals === 1 ? 'animal' : 'animals'}`);
  const people = parts.length > 1 ? `${parts.slice(0, -1).join(', ')} and ${parts[parts.length - 1]}` : (parts[0] ?? 'nobody yet');
  return pets.length ? `${people}, with ${pets.join(' and ')}` : people;
}

function taskLine(item: PlanItem): string {
  const amount = item.kind === 'free_action' ? 'free' : `${quantity(item.quantity, item.unit)}, about ${usd(item.est_cost_usd)} (${band(item.price_band.low, item.price_band.high)})`;
  return `- [${item.done ? 'x' : ' '}] **${md(item.name)}**: ${md(amount)}`;
}

export function buildPacket(input: PlanInput, r: ModelResult): string {
  const out: string[] = [];
  const o = r.output;
  const loc = o.location;
  const place = `${loc.county_name}, ${loc.state_name}`;
  const years = input.dials.horizon_years;
  const b = (id: BucketId) => o.buckets.find((x) => x.id === id)!;
  const days = (id: BucketId) => {
    const t = b(id).target;
    return t.kind === 'days' ? t.value : 0;
  };
  const allItems = o.plan.months.flatMap((m) => m.items);
  const f = r.facts;

  // 1. Summary ---------------------------------------------------------------------------------
  out.push('# Your preparedness packet', '');
  out.push(
    '> **Sample packet.** This was made by the stand-in (mock) engine used while the real one is built. ' +
      'The numbers and sources are placeholders. Do not use them for real decisions yet.',
    '',
  );
  out.push(`**For:** ${md(householdPhrase(input))}, in ${md(place)}  `);
  out.push(`**Plan started:** ${formatDate(input.planning_date)}  `);
  out.push(`**Ready for:** ${DIAL_LABEL[input.dials.return_period]}, ${input.dials.climate === 'y2050' ? 'climate around 2050' : "today's climate"}`, '');

  const statements: string[] = [];
  const power = days('power');
  const water = days('water_out');
  statements.push(
    power === water
      ? `Be ready to manage **${dayPhrase(power)}** at home with no power or tap water.`
      : `Be ready to manage **${dayPhrase(power)}** without power and **${dayPhrase(water)}** without tap water.`,
  );
  statements.push(
    f.dailyRx > 0
      ? `Keep **${dayPhrase(days('supplies'))} of food** you normally eat, and **${dayPhrase(days('medication'))} of daily medicine** on hand at all times.`
      : `Keep **${dayPhrase(days('supplies'))} of food** you normally eat.`,
  );
  const income = b('income').target;
  if (f.earners > 0 && income.kind === 'months') {
    statements.push(`Your biggest long disruption is **income**. Aim for about **${monthsPhrase(income.value)}** of expenses in savings over time, separate from this supplies budget.`);
  } else {
    const top = o.register.find((h) => h.display === 'ranked');
    if (top) statements.push(`The most likely disruption here is **${md(top.name.toLowerCase())}**. The plan covers it first.`);
  }
  out.push('> **The three things that matter most**', '>');
  statements.forEach((s, i) => out.push(`> ${i + 1}. ${s}`));
  out.push('');
  const reached = o.tier_reached === 'now' ? 'getting started' : tierName(o.tier_reached).toLowerCase();
  out.push(`**Where you are:** ${reached}. **Where your risks point:** ${tierName(o.tier_recommended).toLowerCase()} of supplies.`, '');

  // The family plan and wallet cards follow the summary (packet v2): it is what goes on the fridge.
  const whenWeLeave = ['**When we leave.** If an evacuation warning covers our zone, we leave within 30 minutes. The bags are by the door. We meet at the place above.', ''];
  if (r.register.some((h) => h.seed.id === 'tsunami')) {
    whenWeLeave.push('**Tsunami.** If we feel strong or long shaking near the coast, we walk uphill at once. We do not wait for an alert and we do not drive.', '');
  }
  if (r.register.some((h) => h.seed.id === 'tornado' && h.rate >= 0.005)) {
    whenWeLeave.push('**Tornado.** On a warning we go to our shelter spot (basement or inner room on the lowest floor) and stay until it passes.', '');
  }
  if (r.register.some((h) => h.seed.id === 'hurricane' && h.rate >= 0.05)) {
    whenWeLeave.push('**Hurricane.** We know our evacuation zone. If it is ordered to leave, we go early, to the place above.', '');
  }
  out.push(...familyPlanSections(input, whenWeLeave));

  // 2. Risks -----------------------------------------------------------------------------------
  out.push('## Your risks', '');
  out.push(`Chances are for households like yours in ${md(place)}, over the next ${years} ${years === 1 ? 'year' : 'years'}. They are for your county, not your street.[^mock_nri_county]`, '');
  out.push('| What could happen | Households like yours | How bad | How sure |', '| --- | --- | --- | --- |');
  const ranked = o.register.filter((h) => h.display === 'ranked');
  for (const h of ranked.slice(0, 12)) {
    const p = 1 - Math.exp(-h.rate_per_year * years);
    out.push(`| ${md(h.name)} | ${naturalFrequency(p)} | ${severityBand(h.severity).label} | ${CONFIDENCE_LABELS[h.confidence]} |`);
  }
  out.push('');
  const rare = o.register.filter((h) => h.display === 'rare_catastrophic');
  if (rare.length) {
    out.push('### Rare but severe', '');
    out.push('These are shown apart because a tiny chance times a huge loss would otherwise crowd out everything else.', '');
    out.push('| What | How likely | How bad |', '| --- | --- | --- |');
    for (const h of rare) {
      out.push(`| ${md(h.name)} | ${naturalFrequency(1 - Math.exp(-h.rate_per_year * years))} | ${severityBand(h.severity).label} |`);
    }
    out.push('');
    out.push('Your three-day supplies already cover the first days of sheltering: get inside, stay inside, stay tuned.[^mock_nuclear_guidance]', '');
  }
  out.push('*County map: a map of your county will appear here in a later version.*', '');

  // 3. Targets ---------------------------------------------------------------------------------
  out.push('## Your targets', '');
  out.push(`**Settings:** ${DIAL_LABEL[input.dials.return_period]}; ${input.dials.climate === 'y2050' ? 'climate around 2050' : "today's climate"}; water at the ${WATER_LABEL[input.dials.water_level ?? 'basic']}.`, '');
  out.push('| If this happens | Be ready for | Help likely arrives | Mostly back |', '| --- | --- | --- | --- |');
  for (const bucket of o.buckets) {
    if (bucket.target.kind !== 'days') continue;
    const t = bucket.target;
    const rel = bucket.relief;
    out.push(`| ${md(bucket.name)} | ${targetDays(t.value, t.low, t.high)} | ${rel ? `about ${dayPhrase(rel.help_arrives_days)}` : '—'} | ${rel ? `about ${dayPhrase(rel.mostly_restored_days)}` : '—'} |`);
  }
  out.push('');
  out.push('### Things to have ready', '');
  for (const bucket of o.buckets) {
    const t = bucket.target;
    if (t.kind === 'evacuate') {
      out.push(`- **${md(bucket.name)}**: ${naturalFrequency(t.p_need_10yr)} households like yours in ten years. Notice could be ${noticeRange(t.notice_hours_low, t.notice_hours_high)}. Plan to be away about ${dayPhrase(t.days_away)}.`);
    } else if (t.kind === 'readiness' && bucket.id !== 'home_loss') {
      out.push(`- **${md(bucket.name)}**: ${naturalFrequency(t.p_need_10yr)} households like yours in ten years. Ready: ${t.done} of ${t.of}.`);
    }
  }
  out.push('');
  if (income.kind === 'months' && o.plan.savings_track) {
    const s = o.plan.savings_track;
    out.push('### Savings track (separate from the supplies budget)', '');
    out.push(`Aim for ${targetMonths(income.value, income.low, income.high)} of expenses (about ${usd(s.target_usd)}). You have ${monthsPhrase(s.current_months)} saved. ${s.monthly_suggestion_usd > 0 ? `About ${usd(s.monthly_suggestion_usd)} a month would get there in about three years.` : ''}`.trim(), '');
  }

  // 4. Plan ------------------------------------------------------------------------------------
  out.push('## Your plan', '');
  const monthly = input.finances.monthly_budget_usd;
  out.push(`Monthly budget: ${usd(monthly)}${input.finances.one_off_budget_usd > 0 ? `, plus ${usd(input.finances.one_off_budget_usd)} once at the start` : ''}. Free steps come first, then the cheapest risk reduction.`, '');
  const [first, second] = o.plan.months;
  if (first) {
    out.push(`### This month (from ${formatDate(input.planning_date)}): ${usd(first.budget_usd)}`, '');
    first.items.forEach((item) => out.push(taskLine(item)));
    out.push('');
  }
  if (second) {
    out.push(`### Month ${second.index + 1} (from ${formatDate(addMonths(input.planning_date, second.index))}): ${usd(second.budget_usd)}`, '');
    second.items.forEach((item) => out.push(taskLine(item)));
    out.push('');
  }
  const later = o.plan.months.slice(2);
  if (later.length) {
    out.push('### The rest of the plan', '');
    out.push('| Month | What | How much | About |', '| --- | --- | --- | --- |');
    for (const m of later) {
      for (const item of m.items) {
        out.push(`| ${m.index + 1} | ${md(item.name)} | ${md(quantity(item.quantity, item.unit))} | ${usd(item.est_cost_usd)} |`);
      }
    }
    out.push('');
  }
  if (o.plan.done_month !== undefined) {
    out.push(`By month ${o.plan.done_month + 1} (${formatDate(addMonths(input.planning_date, o.plan.done_month))}) every target is covered. After that you are done for your risk: keep up the maintenance calendar.`, '');
  } else if (monthly <= 0 && input.finances.one_off_budget_usd <= 0) {
    out.push('With no budget, the plan is the free steps above. They still cover a lot. Add even a small monthly amount to start on supplies.', '');
  } else {
    out.push('At this budget the plan runs beyond three years. The early months do the most good.', '');
  }

  // 5. Checklists per tier ---------------------------------------------------------------------
  out.push('## Checklists', '');
  const byTier = new Map<TierId, PlanItem[]>();
  for (const item of allItems) byTier.set(item.tier, [...(byTier.get(item.tier) ?? []), item]);
  for (const tier of TIERS) {
    const items = byTier.get(tier.id);
    if (!items?.length) continue;
    out.push(`### ${tier.id === 'now' ? 'Free steps' : `${tier.name} of supplies`}`, '');
    for (const item of items) {
      out.push(`- [${item.done ? 'x' : ' '}] ${md(item.name)}${item.kind === 'free_action' ? '' : `: ${md(quantity(item.quantity, item.unit))}`}`);
    }
    out.push('');
  }
  if (f.commuters > 0) {
    out.push(`### Get-home bag (one for each person who travels 3 miles or more)`, '');
    for (const line of ['Walking shoes you have already worn in', 'Water and snacks', 'A small light', 'A phone charger or power bank', 'A paper map of the route home', 'A rain layer or warm layer for the season']) {
      out.push(`- [ ] ${line}`);
    }
    out.push('');
  }

  // 6. Family plan: printed after the summary (above), then the wallet cards.

  // 7. Documents and money ---------------------------------------------------------------------
  out.push('## Documents and money', '');
  out.push('Keep paper copies in a waterproof bag and photos you can reach from any phone.', '');
  const docs = ['IDs for everyone', 'Insurance policies and the agent’s phone number', input.housing.tenure === 'own' ? 'Deed or mortgage papers' : 'Lease', 'Medicine list and prescriptions', 'Bank and card contact numbers (not PINs)', 'Birth certificates and passports'];
  if (f.pets > 0 || f.large > 0) docs.push('Pet and animal records');
  for (const d of docs) out.push(`- [ ] ${d}`);
  out.push('');
  out.push('**Questions for your insurance:**', '');
  out.push('- Does my policy pay for somewhere to stay if the home cannot be lived in?');
  out.push('- Is flood damage covered? (Usually it is not.)');
  if (r.register.some((h) => h.seed.id === 'earthquake' && h.rate >= 0.005)) out.push('- Is earthquake damage covered? (Usually it is not.)');
  out.push('');
  out.push(`**Cash:** keep about ${usd(100 * Math.ceil(f.n / 2))} in small bills for when card readers are down.`, '');

  // 8. Special needs ---------------------------------------------------------------------------
  out.push('## Special needs', '');
  const needs: string[] = [];
  if (f.dailyRx > 0) needs.push(`**Daily medicine** (${f.dailyRx} ${f.dailyRx === 1 ? 'person' : 'people'}): refill when 7 days are left and ask the prescriber about a 90-day fill. The plan builds the extra supply to about ${dayPhrase(days('medication'))}.`);
  if (f.fridgeRx) needs.push('**Medicine that must stay cold:** a cooler and gel packs keep it safe for about a day without power. Ask the pharmacist how long it keeps out of the fridge.');
  if (f.deviceWatts.length > 0) needs.push(`**Powered medical device** (${f.deviceWatts.map((w) => `${w} watts`).join(', ')}): a battery sized for at least one night, and a plan for where to go if the outage runs long. Tell your power company if they keep a medical-needs list.`);
  if (f.seniors > 0) needs.push('**Older adults:** a check-in plan for heat, cold and outages, and a way to get to a cooling or warming center.');
  if (f.infants > 0) needs.push('**Babies:** ready-to-feed formula (it needs no water), diapers and wipes for the same number of days as food.');
  if (input.people.some((p) => p.medical.mobility !== 'none')) needs.push('**Getting around:** plan who helps with stairs and transport if you have to leave, and keep mobility aids by the door.');
  if (input.people.some((p) => p.pregnant_or_nursing)) needs.push('**Pregnancy or nursing:** keep prenatal records with your papers and extra water and food for the extra need.');
  if (input.people.some((p) => p.medical.epinephrine)) needs.push('**Epinephrine:** keep a spare auto-injector in the go-bag and check its date.');
  if (f.pets > 0 || f.large > 0) needs.push('**Animals:** food, water, a carrier or leash, and records; know which shelters and hotels on your route take pets.');
  const access = accessNeedsLine(input);
  if (access) needs.push(access);
  if (needs.length === 0) needs.push('Nobody in the household listed medical needs. Keep the medicine list current anyway.');
  needs.forEach((n) => out.push(`- ${n}`));
  out.push('');
  out.push('Stress after an emergency is normal, and talking helps. The 988 Suicide & Crisis Lifeline (call or text 988) and the Disaster Distress Helpline (call or text 1-800-985-5990) are free and open every day, all day.', '');

  // 9. Maintenance calendar --------------------------------------------------------------------
  out.push('## Maintenance calendar', '');
  const intervals = new Map<number, Set<string>>();
  const add = (months: number, what: string) => intervals.set(months, (intervals.get(months) ?? new Set()).add(what));
  for (const item of allItems) {
    const m = catalogueItem(item.item_id)?.maintenance;
    if (m?.rotate_months) add(m.rotate_months, `Replace or use up: ${item.name.toLowerCase()}`);
    if (m?.check_months) add(m.check_months, `Check: ${item.name.toLowerCase()}`);
  }
  add(12, 'Review this plan and your answers');
  out.push('| How often | What |', '| --- | --- |');
  for (const months of [...intervals.keys()].sort((a, c) => a - c)) {
    const label = months === 1 ? 'Every month' : months === 12 ? 'Every year' : months % 12 === 0 ? `Every ${months / 12} years` : `Every ${months} months`;
    out.push(`| ${label} | ${[...intervals.get(months)!].map(md).join('; ')} |`);
  }
  out.push('');

  // 10. Sources --------------------------------------------------------------------------------
  out.push('## Sources', '');
  out.push('Every number in this packet comes from one of these. **These are mock placeholders**, not real sources.', '');
  o.provenance.forEach((c, i) => {
    out.push(`${i + 1}. **${md(c.title)}**. ${md(c.publisher)}${c.year ? `, ${c.year}` : ''}. <${c.url}> (retrieved ${c.retrieved})${c.prior ? '. Expert estimate.' : '.'}`);
  });
  out.push('');
  out.push('[^mock_nri_county]: Mock: county hazard frequencies (stands in for the FEMA National Risk Index).');
  out.push('[^mock_nuclear_guidance]: Mock: shelter guidance (stands in for Ready.gov).');
  out.push('');
  return out.join('\n');
}
