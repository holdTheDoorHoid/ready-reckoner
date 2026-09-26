import { describe, expect, it } from 'vitest';

import type { HouseholdFacts } from './conditions';
import { blockText, guidanceBlock, parseBlock, type GuidanceBlock } from './guidance';

/** A block in the shape of content/guidance/plan_shelter.md. */
const SHELTER = `---
id: plan_shelter
title: Your shelter plan
kind: plan
applies_to: [plan:shelter]
citations: [ready_gov_plan, ready_gov_tornadoes, ready_gov_hurricanes, fema_hurricane_tips]
---
Pick a spot for each danger below, at home and at work, and practice going there.[^ready_gov_plan]

{if:tornado}**Tornado.** A small inside room with no windows, on the lowest floor.[^ready_gov_tornadoes]{/if}

{if:hurricane}**Hurricane.** If you live in an evacuation zone and officials tell you to leave, go right away.[^ready_gov_hurricanes]{/if} {if:home:apartment_high_rise}In a tall building, shelter on or below the 10th floor.[^fema_hurricane_tips]{/if}

## Sources

[^ready_gov_plan]: FEMA / Ready.gov, Make A Plan (2026).
[^ready_gov_tornadoes]: FEMA / Ready.gov, Tornadoes (2026).
[^ready_gov_hurricanes]: FEMA / Ready.gov, Hurricanes (2026).
[^fema_hurricane_tips]: FEMA, Hurricane safety tips (2012).
`;

const household = (hazards: string[], home: string): HouseholdFacts => ({
  hazardRelevant: (h) => hazards.includes(h),
  home: () => home,
  hasAccessNeed: () => false,
  hasItem: () => false,
  hasBenefit: () => false,
});

describe('guidance blocks on screen', () => {
  it('read the front matter: id, title and citations', () => {
    const b = parseBlock(SHELTER)!;
    expect(b.id).toBe('plan_shelter');
    expect(b.title).toBe('Your shelter plan');
    expect(b.citations).toEqual(['ready_gov_plan', 'ready_gov_tornadoes', 'ready_gov_hurricanes', 'fema_hurricane_tips']);
    expect(b.body.startsWith('Pick a spot')).toBe(true);
    expect(parseBlock('no front matter')).toBeUndefined();
  });

  it('are trimmed for the household as the packet trims them, without the Sources heading', () => {
    const b = parseBlock(SHELTER)!;
    const kansas = blockText(b, household(['tornado'], 'detached'));
    expect(kansas).toContain('**Tornado.**');
    expect(kansas).not.toContain('**Hurricane.**');
    expect(kansas).not.toContain('10th floor');
    expect(kansas).not.toMatch(/^## Sources/m);
    expect(kansas).not.toMatch(/\n{3,}/);
    // The notes stay for the renderer, which lists only the ones the text still uses.
    expect(kansas).toContain('[^ready_gov_tornadoes]: FEMA / Ready.gov, Tornadoes (2026).');
    const miami = blockText(b, household(['hurricane'], 'apartment_high_rise'));
    expect(miami).toContain('go right away.[^ready_gov_hurricanes] In a tall building');
    expect(miami).not.toContain('**Tornado.**');
    // With no household yet, every span stays.
    expect(blockText(b, null)).toContain('10th floor');
  });

  it('are looked up by id; a build without the file has none', () => {
    const blocks = new Map<string, GuidanceBlock>([['plan_shelter', parseBlock(SHELTER)!]]);
    expect(guidanceBlock('plan_shelter', blocks)?.title).toBe('Your shelter plan');
    expect(guidanceBlock('plan_communication', blocks)).toBeUndefined();
  });
});
