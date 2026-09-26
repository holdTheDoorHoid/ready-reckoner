/**
 * `explain()` for the mock engine: the plain-language "why" behind one hazard, bucket, item,
 * requirement or warning, with the arithmetic for the expert view and the sources used.
 */
import type { Citation, Explanation, ExplainRequest } from '../types';
import { chanceWithin, dayPhrase, naturalFrequency, percent, targetDays, targetMonths, usd } from '../../lib/format';
import { returnPeriodHelp } from '../../lib/labels';
import { citation } from './citations';
import { catalogueItem } from './items';
import type { ModelResult } from './model';

const DIAL: Record<string, { label: string; n: number }> = {
  one_in_10: { label: 'Common disruptions (1-in-10)', n: 10 },
  one_in_50: { label: 'Serious (1-in-50)', n: 50 },
  one_in_100: { label: 'Very serious (1-in-100)', n: 100 },
  one_in_500: { label: 'Rare catastrophes (1-in-500)', n: 500 },
};

function cites(ids: readonly string[]): Citation[] {
  return [...new Set(ids)].map((id) => citation(id)).filter((c): c is Citation => c !== undefined);
}

/** The explanation, or undefined when the id is not in this plan. */
export function explainFrom(req: ExplainRequest, r: ModelResult): Explanation | undefined {
  const o = r.output;
  const years = req.input.dials.horizon_years;
  const dial = DIAL[req.input.dials.return_period]!;
  switch (req.kind) {
    case 'hazard': {
      const h = r.register.find((x) => x.seed.id === req.id);
      if (!h) return undefined;
      const p = h.profile;
      const plain = [p.frequency_sentence];
      const buckets = p.buckets.map((b) => o.buckets.find((x) => x.id === b)?.name.toLowerCase()).filter(Boolean);
      if (buckets.length) plain.push(`When it happens, it can mean: ${buckets.join('; ')}.`);
      if (p.confidence === 'prior') plain.push('Good data does not exist for this, so the chance is an expert estimate. It is shown as a range for that reason.');
      if (p.climate_multiplier !== 1) {
        plain.push(p.climate_multiplier > 1
          ? `Around 2050 this is projected to happen more often (about ${Math.round((p.climate_multiplier - 1) * 100)}% more). Projections, not certainties.`
          : `Around 2050 this is projected to happen less often (about ${Math.round((1 - p.climate_multiplier) * 100)}% less). Projections, not certainties.`);
      }
      if (p.display === 'rare_catastrophic') {
        plain.push('It is very unlikely, so it sits in its own box and never takes over the budget. The first days of sheltering are covered by the supplies you already plan for.');
        if (p.id === 'nuclear_attack' || p.id === 'nuclear_plant_incident') {
          plain.push(
            'If it happens: get inside, stay inside, stay tuned. Go to the middle of a sturdy building or a basement, stay there for at least 24 hours unless officials say otherwise, and listen for instructions on a radio or phone.',
          );
          if (p.id === 'nuclear_plant_incident') {
            plain.push('Potassium iodide protects only the thyroid, only matters close to a plant, and is taken only when officials say so. They hand it out in the areas that need it.');
          }
        }
        if (p.id === 'terrorism') {
          plain.push('If you are caught up in an attack: getting away is the top priority. If you cannot, hide and silence your phone. Call 911 when it is safe.');
        }
      }
      const math = [
        `Events per year for your household: ${h.seed.rate} (area) × ${(h.rate / (h.seed.rate * p.climate_multiplier)).toFixed(2)} (your home) × ${p.climate_multiplier} (climate) = ${p.rate_per_year}`,
        `Chance in ${years} years: 1 − e^(−${years} × ${p.rate_per_year}) = ${percent(chanceWithin(p.rate_per_year, years))}`,
        `Range: ${p.rate_range[0]} to ${p.rate_range[1]} events per year`,
      ];
      return { title: `Why ${p.name.toLowerCase()} is on your list`, plain, math, sources: cites(p.sources) };
    }
    case 'bucket': {
      const b = o.buckets.find((x) => x.id === req.id);
      if (!b) return undefined;
      const plain: string[] = [];
      const math: string[] = [];
      const t = b.target;
      if (t.kind === 'days') {
        plain.push(`You chose ${dial.label}. ${returnPeriodHelp(req.input.dials.return_period)}`);
        plain.push(`For "${b.name.toLowerCase()}" that comes to ${targetDays(t.value, t.low, t.high)}. The range shows how unsure the underlying numbers are.`);
        math.push(`Target: the smallest ladder value d where the yearly rate of disruptions longer than d is at most 1/${dial.n} = ${1 / dial.n}`);
        math.push('Ladder: ½, 1, 2, 3, 5, 7, 10, 14, 21, 30, 45, 60, 90, 180, 365 days');
      } else if (t.kind === 'months') {
        plain.push(`Savings for ${targetMonths(t.value, t.low, t.high)} of expenses would cover the income gap you should plan for at ${dial.label}.`);
        plain.push('This is a savings goal on its own track. It is never bought out of the supplies budget.');
      } else if (t.kind === 'evacuate') {
        plain.push(`${naturalFrequency(t.p_need_10yr)} households like yours have to leave home quickly at least once in ten years.`);
      } else {
        plain.push(`${naturalFrequency(t.p_need_10yr)} households like yours need this at least once in ten years. You have ${t.done} of ${t.of} ready.`);
      }
      if (b.contributions.length) {
        plain.push(`Most of it comes from: ${b.contributions.map((c) => `${o.register.find((h) => h.id === c.hazard)?.name.toLowerCase() ?? c.hazard} (${Math.round(c.share * 100)} in 100)`).join(', ')}.`);
      }
      if (b.relief) {
        plain.push(`For an event this size, outside help plausibly arrives in about ${dayPhrase(b.relief.help_arrives_days)}, and service is mostly back in about ${dayPhrase(b.relief.mostly_restored_days)}.`);
      }
      return { title: `How we got the target for "${b.name}"`, plain, math, sources: cites([...b.sources, ...(b.relief?.sources ?? [])]) };
    }
    case 'item': {
      const item = catalogueItem(req.id);
      const planned = o.plan.months.flatMap((m) => m.items).find((i) => i.item_id === req.id);
      if (!item || !planned) return undefined;
      const plain = [planned.why, item.spec];
      if (item.look_for.length) plain.push(`Look for: ${item.look_for.join('; ')}.`);
      if (item.avoid.length) plain.push(`Avoid: ${item.avoid.join('; ')}.`);
      const math = item.free
        ? ['Free: no cost to the budget.']
        : [
            `Quantity for your household: ${planned.quantity} ${planned.unit}`,
            `Cost: about ${usd(planned.est_cost_usd)}, from a price band of ${usd(item.price_band_usd.low)}–${usd(item.price_band_usd.high)} per ${item.price_band_usd.per}`,
          ];
      return { title: `Why "${item.name}" is in your plan`, plain, math, sources: cites(item.citations) };
    }
    case 'requirement': {
      const line = o.requirements.find((x) => x.id === req.id);
      if (!line) return undefined;
      return {
        title: `How much: ${line.plain.replace(/\.$/, '')}`,
        plain: [line.plain],
        math: [`Rule ${line.rule}: ${line.quantity} ${line.unit} for the whole household (per ${line.per})`],
        sources: cites(line.citations),
      };
    }
    case 'warning': {
      const w = o.warnings.find((x) => x.id === req.id);
      if (!w) return undefined;
      return { title: w.message, plain: [w.why, 'This is a warning, not a rule. Keep your plan as it is if you know something the model does not.'], sources: [] };
    }
  }
}
