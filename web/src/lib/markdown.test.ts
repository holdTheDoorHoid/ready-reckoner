import { describe, expect, it } from 'vitest';

import { FIXTURE_NAMES, FIXTURES } from '../engine/fixtures';
import { createMockEngine } from '../engine/mock';
import { renderMarkdown } from './markdown';

/** Anything that could run script, load a resource, or restyle the page. */
function unsafe(html: string): string[] {
  const doc = new DOMParser().parseFromString(`<div id="root">${html}</div>`, 'text/html');
  const root = doc.getElementById('root')!;
  const found: string[] = [];
  for (const el of root.querySelectorAll('*')) {
    const tag = el.tagName.toLowerCase();
    if (['script', 'style', 'iframe', 'object', 'embed', 'img', 'svg', 'math', 'form', 'link', 'meta', 'base', 'video', 'audio', 'source'].includes(tag)) found.push(`<${tag}>`);
    for (const attr of el.attributes) {
      if (attr.name.startsWith('on')) found.push(`${tag}[${attr.name}]`);
      if (attr.name === 'style' || attr.name === 'src' || attr.name === 'srcset' || attr.name === 'formaction') found.push(`${tag}[${attr.name}]`);
      if (attr.name === 'href' && !/^(?:https?:\/\/|mailto:|#)/i.test(attr.value)) found.push(`${tag}[href=${attr.value}]`);
    }
    if (tag === 'input' && (el.getAttribute('type') !== 'checkbox' || !el.hasAttribute('disabled'))) found.push('live input');
  }
  return found;
}

const ATTACKS = [
  '<script>alert(1)</script>',
  '<img src=x onerror="alert(1)">',
  '<a href="javascript:alert(1)">click</a>',
  '[click](javascript:alert(1))',
  '[data](data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==)',
  '![tracker](https://tracker.example/pixel.png)',
  '<iframe src="https://evil.example"></iframe>',
  '<p style="position:fixed;inset:0">cover</p>',
  '<svg><script>alert(1)</script></svg>',
  '<div onclick="steal()">x</div>',
  '<form action="https://evil.example"><input name="q"></form>',
  '<details open ontoggle=alert(1)>',
  '<math><mi xlink:href="javascript:alert(1)">x</mi></math>',
  '[x](  JaVaScRiPt:alert(1))',
  '<a href="https://ok.example" onmouseover="x()">ok</a>',
  '- [ ] task <input type="text" value="type here">',
];

describe('packet rendering', () => {
  it('renders every fixture packet with nothing unsafe in it', async () => {
    const engine = createMockEngine();
    for (const name of FIXTURE_NAMES) {
      const r = await engine.assess(FIXTURES[name]);
      if (!r.ok) throw new Error(name);
      const html = renderMarkdown(r.value.packet_markdown, { idPrefix: 'pk', headingOffset: 1 });
      expect(unsafe(html), name).toEqual([]);
      expect(html).toContain('<h2 class="md-h1">Your preparedness packet</h2>');
      expect(html).toContain('<h3 class="md-h2">Sources</h3>');
      expect(html).toMatch(/<input[^>]*type="checkbox"/);
      expect(html).toContain('class="footnotes"');
    }
  });

  it('neutralises every attack in the list', () => {
    for (const attack of ATTACKS) {
      const html = renderMarkdown(`Before\n\n${attack}\n\nAfter`);
      expect(unsafe(html), attack).toEqual([]);
      expect(html).toContain('After');
    }
  });

  it('shows raw HTML as text instead of dropping it silently', () => {
    const html = renderMarkdown('<b>bold?</b>');
    expect(html).toContain('&lt;b&gt;bold?&lt;/b&gt;');
  });

  it('names each checklist tick box by its own line', () => {
    const html = renderMarkdown('- [ ] **Water**: 3 gallons\n- [x] Done thing');
    expect(html).toContain('<li class="task-item"><label><input type="checkbox" disabled=""> <strong>Water</strong>: 3 gallons</label></li>');
    expect(html).toContain('<input type="checkbox" disabled="" checked=""> Done thing</label>');
  });

  it('turns images into their description, so nothing is fetched', () => {
    expect(renderMarkdown('![A county map](https://example.org/map.png)')).toContain('[A county map]');
  });

  it('opens outside links in a new tab without passing the page along', () => {
    const html = renderMarkdown('[FEMA](https://www.fema.gov/)');
    expect(html).toContain('target="_blank"');
    expect(html).toContain('rel="noopener noreferrer"');
    expect(html).toContain('(opens in a new tab)');
  });

  it('numbers footnotes and links them both ways', () => {
    const html = renderMarkdown('One.[^a] Two.[^b] Again.[^a]\n\n[^a]: First source.\n[^b]: Second source.', { idPrefix: 't' });
    expect(html).toContain('href="#t-fn-a"');
    expect(html).toContain('id="t-fn-b"');
    expect(html).toContain('href="#t-fnref-a-1"');
    expect((html.match(/aria-label="Note 1"/g) ?? []).length).toBe(2);
    expect(html).toContain('First source.');
  });

  it('shifts heading levels under the page title, and wraps tables for small screens', () => {
    const html = renderMarkdown('# Title\n\n## Section\n\n| a | b |\n| - | - |\n| 1 | 2 |', { headingOffset: 1 });
    expect(html).toContain('<h2 class="md-h1">Title</h2>');
    expect(html).toContain('<h3 class="md-h2">Section</h3>');
    expect(html).toContain('<div class="table-wrap" tabindex="0" role="region" aria-label="Table 1: a">');
  });
});
