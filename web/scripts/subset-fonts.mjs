// Makes the four font files the PDF binder embeds (web/src/lib/binder/pdf/fonts/NotoSans-*.woff)
// from the Noto Sans release named in fonts/coverage.json: the unhinted TrueType faces, cut down
// to the characters listed there (Latin, Latin-1, Latin Extended-A, the Latin Extended-B and
// Extended Additional letters that names in the United States use, punctuation, currency) with
// kerning kept, and stored as WOFF (zlib-compressed tables, which pdfmake's font reader opens;
// its WOFF2 path fails to embed). The files are committed, so a build never needs the network.
//
// Run it again only to change the font or its characters:
//
//   pip install fonttools zopfli        # pyftsubset; zopfli makes the files about 5% smaller
//   node scripts/subset-fonts.mjs       # downloads the release (about 117 MB) into a temp folder
//   node scripts/subset-fonts.mjs --zip ~/Downloads/NotoSans-v2.015.zip
//
// It checks the release's SHA-256, writes the four .woff files and OFL.txt, and records in
// coverage.json the characters all four faces really have (`covered`), which the app checks the
// binder's text against before it builds a PDF. Noto Sans is under the SIL Open Font License 1.1
// with no Reserved Font Name, so a subset may keep the name.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const out = fileURLToPath(new URL('../src/lib/binder/pdf/fonts/', import.meta.url));
const coverageFile = `${out}coverage.json`;
const coverage = JSON.parse(readFileSync(coverageFile, 'utf8'));
const python = process.env.PYTHON ?? 'python3';
const pyftsubset = process.env.PYFTSUBSET ?? 'pyftsubset';

const zipArg = process.argv.indexOf('--zip');
const work = mkdtempSync(join(tmpdir(), 'rr-fonts-'));
try {
  let zip = zipArg >= 0 ? process.argv[zipArg + 1] : undefined;
  if (!zip) {
    zip = join(work, 'noto.zip');
    console.log(`Downloading ${coverage.source}`);
    const res = await fetch(coverage.source);
    if (!res.ok) throw new Error(`download failed: ${res.status}`);
    writeFileSync(zip, Buffer.from(await res.arrayBuffer()));
  }
  const sha = createHash('sha256').update(readFileSync(zip)).digest('hex');
  if (sha !== coverage.sha256) throw new Error(`${zip}: SHA-256 ${sha}, expected ${coverage.sha256}`);

  // Only the four faces and the licence come out of the archive.
  const members = [...Object.values(coverage.faces), 'OFL.txt'];
  execFileSync(python, ['-c', 'import sys, zipfile; z = zipfile.ZipFile(sys.argv[1]); [z.extract(m, sys.argv[2]) for m in sys.argv[3:]]', zip, work, ...members]);
  writeFileSync(`${out}OFL.txt`, readFileSync(join(work, 'OFL.txt')));

  const unicodes = coverage.ranges.map(([a, b]) => (a === b ? `U+${a}` : `U+${a}-${b}`)).join(',');
  const files = {};
  for (const [style, member] of Object.entries(coverage.faces)) {
    const name = member.split('/').pop().replace(/\.ttf$/, '.woff');
    const args = [join(work, member), `--unicodes=${unicodes}`, `--layout-features=${coverage.features.join(',')}`, '--name-IDs=0,1,2,3,4,5,6,13,14', '--flavor=woff', `--output-file=${out}${name}`];
    try {
      execFileSync(pyftsubset, [...args, '--with-zopfli'], { stdio: 'pipe' });
    } catch {
      execFileSync(pyftsubset, args, { stdio: 'pipe' });
    }
    files[style] = name;
    console.log(`${name}: ${statSync(`${out}${name}`).size} bytes`);
  }

  // What every face really covers (a range asked for may hold characters the font lacks).
  const covered = execFileSync(
    python,
    [
      '-c',
      `
import sys
from fontTools.ttLib import TTFont
sets = [set(TTFont(p).getBestCmap().keys()) for p in sys.argv[1:]]
cps = sorted(set.intersection(*sets))
runs = []
for cp in cps:
    if runs and runs[-1][1] == cp - 1:
        runs[-1][1] = cp
    else:
        runs.append([cp, cp])
print(";".join("%04X-%04X" % (a, b) for a, b in runs))
`,
      ...Object.values(files).map((f) => `${out}${f}`),
    ],
    { encoding: 'utf8' },
  ).trim();
  coverage.files = files;
  coverage.covered = covered.split(';').map((r) => r.split('-'));
  // One range a line: ["0020", "007E", "Basic Latin"].
  const json = JSON.stringify(coverage, null, 2).replace(/\[\s+("[^"]*"(?:,\s+"[^"]*")*)\s+\]/g, (_m, inner) => `[${inner.replace(/,\s+/g, ', ')}]`);
  writeFileSync(coverageFile, `${json}\n`);
  console.log(`coverage.json: ${coverage.covered.length} runs of characters`);
} finally {
  rmSync(work, { recursive: true, force: true });
}
