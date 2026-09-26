/**
 * Learn articles (docs/UI.md screen 10). The content workstream owns the final text; until
 * `content/learn/*.md` exists, these drafts stand in. A file there with front matter (`title:`,
 * `summary:`) and the same slug as its file name replaces the draft automatically.
 *
 * awaiting: content — review, citation ids, reading-level check.
 */

export interface Article {
  slug: string;
  title: string;
  summary: string;
  body: string;
  draft: boolean;
}

const DRAFTS: Article[] = [
  {
    slug: 'consequences',
    title: 'Plan for what happens, not what causes it',
    summary: 'Why the plan is built around no power, no water and no store, instead of a list per disaster.',
    draft: true,
    body: `Most emergency advice starts with the cause: a hurricane list, an earthquake list, a pandemic list. Ready Reckoner starts with what an event would do to your home instead.

A winter storm, a heat wave and a broken transformer all end the same way: no power. A flood, a water main break and a boil-water notice all end the same way: no safe tap water. So the plan asks a simpler question. How long should you be able to manage without power, without safe water, without a trip to the store, and without income?

This has two big benefits. First, you never buy the same thing twice. The water that covers a storm also covers a water main break. Second, rare and dramatic events stop crowding out the likely ones. The supplies for a common three-day outage already cover the first days of most rarer emergencies.

Hazards still matter. They explain why a consequence matters where you live, and a few add something specific, like strapping a water heater in earthquake country or planning a walking route uphill near the coast. But the core of the plan is the same short list of consequences, sized for your household.`,
  },
  {
    slug: 'myths',
    title: 'Disaster myths',
    summary: 'Panic and looting are much rarer than films suggest. Neighbours are the first help.',
    draft: true,
    body: `Films show panic, looting and people turning on each other. Research on real disasters finds something different.

- Panic is rare. People in disasters usually stay calm, help each other and make sensible choices, even when they are frightened.[^clarke]
- Looting and crime are far less common than news coverage suggests. Reports after Hurricane Katrina greatly exaggerated them.[^tierney]
- The first help almost always comes from neighbours and bystanders, long before emergency services arrive.[^aldrich]

Why it matters for your plan: if you expect chaos, you might spend on dramatic gear and skip the things that help most, like water, medicine, a written plan and knowing the people next door. One of the most useful things many households can add is two neighbours' phone numbers.

It is also normal to underestimate how long a disruption can last, and normal to underestimate how well you would cope. The plan tries to correct both. It sizes supplies from data, and it starts with small steps you can finish.

[^clarke]: Lee Clarke, "Panic: Myth or Reality?", Contexts, 2002.
[^tierney]: Kathleen Tierney, Christine Bevc and Erica Kuligowski, "Metaphors Matter", The Annals of the American Academy of Political and Social Science, 2006.
[^aldrich]: Daniel Aldrich and Yasuyuki Sawada, "The physical and social determinants of mortality in the 3.11 tsunami", Social Science & Medicine, 2015.`,
  },
  {
    slug: 'numbers',
    title: 'How the numbers are made',
    summary: 'From your county and household to days of water and a monthly plan, in five steps.',
    draft: true,
    body: `Every number in your plan has a source you can open. Here is how they fit together.

1. **Where you live gives each hazard a yearly chance.** Natural hazards come from federal data for your county. Hazards without good data, like a regional grid failure, use expert estimates, and the app labels them that way.
2. **Who you are changes what those hazards would do to you.** A private well means a power cut is also a water cut. A medical device that needs electricity makes power matter more.
3. **Each hazard becomes a few plain consequences** that last some number of days: no power, no water, can't get to a store.
4. **Adding up the hazards gives the days to be ready for**, at the setting you choose. The usual setting, "Very serious (1-in-100)", plans for disruptions so long that only about 10 in 100 households like yours would see a longer one in ten years.
5. **Those days, times your household, become amounts** of water, food and medicine. Your budget then buys the cheapest protection first, month by month, and the plan tells you when you have done enough.

Chances are shown as "about N of 100 households like yours" because that is easier to picture than a percentage. Targets come with a range, like "about 3 days (2–5)", because the underlying numbers are uncertain. When one rare event decides most of the answer, such as a Cascadia earthquake on the Oregon coast, the app says so and lets you plan with or without it.`,
  },
  {
    slug: 'children',
    title: 'Talking with children about emergencies',
    summary: 'Keep it calm, give them a job, and practise together.',
    draft: true,
    body: `Children cope better when they know what the plan is and have a part in it.

- **Keep it simple and calm.** Explain that storms and power cuts happen sometimes, and that your family has a plan.
- **Give them a job.** Younger children can pack a comfort item and a flashlight in their go-bag. Older children can learn the meeting place, the out-of-area contact, and how to text instead of call.
- **Practise together.** A short fire drill or a "power-out evening" with flashlights turns the plan into something familiar, even fun.
- **Answer questions honestly, at their level.** It is fine to say you don't know, and that adults are working to keep everyone safe.
- **Afterwards, keep routines** as normal as you can, and watch for changes in sleep, eating or behaviour. Ask for help if worries last.

Ready.gov has activities for children about getting ready.[^readykids] Child trauma organizations have guidance for after a disaster.[^nctsn]

[^readykids]: Ready.gov, "Ready Kids", https://www.ready.gov/kids
[^nctsn]: National Child Traumatic Stress Network, "Disasters", https://www.nctsn.org/what-is-child-trauma/trauma-types/disasters`,
  },
  {
    slug: 'community',
    title: 'Neighbours are part of your plan',
    summary: 'Knowing the people nearby is one of the strongest protections there is, and it is free.',
    draft: true,
    body: `In most disasters the first help comes from people nearby. Studies of floods, heat waves and tsunamis find that places where people know each other come through better, especially older and low-income residents.[^aldrich2][^semenza]

That is why your plan treats neighbours as a supply, alongside water and food:

- Swap phone numbers with at least two neighbours.
- Agree who checks on whom in a heat wave, a storm or a long outage, especially anyone who lives alone or can't get out easily.
- If you can, take a Community Emergency Response Team (CERT) course or join a block group. Many counties offer CERT training.

These steps cost nothing, and they work both ways: you are someone's neighbour too.

[^aldrich2]: Daniel Aldrich and Michelle Meyer, "Social Capital and Community Resilience", American Behavioral Scientist, 2015.
[^semenza]: Jan Semenza and others, "Heat-related deaths during the July 1995 heat wave in Chicago", New England Journal of Medicine, 1996.`,
  },
];

/** Articles from `content/learn/*.md`, if the content workstream has added any. */
const FROM_CONTENT = import.meta.glob('../../../content/learn/*.md', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

function parseFrontMatter(raw: string): { meta: Record<string, string>; body: string } {
  const m = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(raw);
  if (!m) return { meta: {}, body: raw };
  const meta: Record<string, string> = {};
  for (const line of m[1]!.split(/\r?\n/)) {
    const kv = /^([A-Za-z_]+):\s*(.*)$/.exec(line);
    if (kv) meta[kv[1]!] = kv[2]!.replace(/^["']|["']$/g, '');
  }
  return { meta, body: raw.slice(m[0].length) };
}

function fromContent(): Article[] {
  return Object.entries(FROM_CONTENT).map(([path, raw]) => {
    const slug = path.replace(/^.*\//, '').replace(/\.md$/, '');
    const { meta, body } = parseFrontMatter(raw);
    return { slug, title: meta.title ?? slug, summary: meta.summary ?? '', body, draft: false };
  });
}

export const ARTICLES: Article[] = (() => {
  const real = fromContent();
  const bySlug = new Map(real.map((a) => [a.slug, a]));
  const merged = DRAFTS.map((d) => bySlug.get(d.slug) ?? d);
  for (const a of real) if (!DRAFTS.some((d) => d.slug === a.slug)) merged.push(a);
  return merged;
})();

export function article(slug: string): Article | undefined {
  return ARTICLES.find((a) => a.slug === slug);
}
