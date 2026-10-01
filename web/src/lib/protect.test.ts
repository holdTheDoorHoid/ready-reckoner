/**
 * The protected plan file (DESIGN-DELTA-v3 §7): which plans count as sensitive, the file's exact
 * shape and WebCrypto parameters, a round trip, a wrong passphrase, damaged and newer files, and
 * the plain path (a plain file, and a version-1 file) left exactly as it was.
 */
import { describe, expect, it } from 'vitest';

import { FIXTURES } from '../engine/fixtures';
import { clone, savedFor, withoutOptional } from '../test/helpers';
import { exportText, parseImport, type SavedPlan } from './persistence';
import {
  decryptText,
  ENCRYPTED_FORMAT,
  encryptText,
  fromBase64,
  hasSensitiveAnswers,
  isEncryptedFile,
  IV_BYTES,
  KDF_ITERATIONS,
  openProtected,
  protectedExportText,
  readPlanFile,
  SALT_BYTES,
  toBase64,
} from './protect';

/** Philadelphia before any optional step is answered (the fixture itself may carry sample answers). */
const philly = withoutOptional(FIXTURES['philadelphia-renters-4']);
const PASS = 'correct horse battery staple';

function withAnswers(edit: (plan: SavedPlan) => void): SavedPlan {
  const plan = savedFor(philly);
  edit(plan);
  return plan;
}

describe('which saved plans are sensitive (§7)', () => {
  it('a plan with no optional answers, or only names and phone numbers, is not', () => {
    expect(hasSensitiveAnswers(savedFor(philly))).toBe(false);
    expect(hasSensitiveAnswers(withAnswers((p) => (p.input.people[0]!.profile = { name: 'Ana', phone: '555-0101' })))).toBe(false);
    // v2's family plan (meeting places, the trusted circle's names and numbers) is not on §7's list.
    expect(
      hasSensitiveAnswers(
        withAnswers((p) => (p.input.family_plan = { meeting_place_near: 'The corner', trusted_circle: [{ name: 'Rosa', phone: '555-0102' }], lawyer: { name: 'Legal aid' } })),
      ),
    ).toBe(false);
    // Blank answers and a place's kind alone say nothing.
    expect(hasSensitiveAnswers(withAnswers((p) => (p.input.people[0]!.profile = { email: '   ', place: { kind: 'work' }, medications: [{}] })))).toBe(false);
    expect(hasSensitiveAnswers(withAnswers((p) => (p.input.family_plan = { home: { address: '  ' }, vehicles: [{ plate: '' }], pets: [{ microchip: ' ' }], documents: { accounts: [{}] } })))).toBe(false);
  });

  it('anything in a profile beyond name and phone is', () => {
    const cases: [string, NonNullable<SavedPlan['input']['people'][number]['profile']>][] = [
      ['date of birth', { date_of_birth: '3 March 1984' }],
      ['email', { email: 'ana@example.org' }],
      ['a place', { place: { kind: 'school', name: 'Sample Elementary' } }],
      ['a doctor', { doctor: { name: 'Dr Sample' } }],
      ['a medicine', { medications: [{ name: 'Inhaler' }] }],
      ['allergies', { allergies: 'Penicillin' }],
      ['blood type', { blood_type: 'O+' }],
      ['insurance', { insurance: { member_id: 'XJ-000' } }],
      ['ID notes', { id_notes: 'Passport in the safe' }],
      ['notes', { notes: 'Calms down with music' }],
    ];
    for (const [what, profile] of cases) {
      expect(hasSensitiveAnswers(withAnswers((p) => (p.input.people[1]!.profile = { name: 'Sam', ...profile }))), what).toBe(true);
    }
  });

  it('the maps’ home pin or a drawn route is, as plainly as the address; a meeting-place pin alone is not', () => {
    const layers = { places: true, flood: false, surge: false, wildfire: false };
    expect(hasSensitiveAnswers(withAnswers((p) => (p.maps = { home: { lat: 39.93, lon: -75.15 }, routes: [], layers })))).toBe(true);
    expect(hasSensitiveAnswers(withAnswers((p) => (p.maps = { routes: [[{ lat: 39.9, lon: -75.1 }, { lat: 40, lon: -75.2 }]], layers })))).toBe(true);
    expect(hasSensitiveAnswers(withAnswers((p) => (p.maps = { meeting_far: { lat: 40.27, lon: -76.88 }, routes: [], layers })))).toBe(false);
  });

  it("the home's address, documents and money, a vehicle's plate and a pet's microchip are", () => {
    expect(hasSensitiveAnswers(withAnswers((p) => (p.input.family_plan = { home: { address: '12 Sample St' } })))).toBe(true);
    expect(hasSensitiveAnswers(withAnswers((p) => (p.input.family_plan = { documents: { where_copies: 'With Rosa' } })))).toBe(true);
    expect(hasSensitiveAnswers(withAnswers((p) => (p.input.family_plan = { documents: { accounts: [{ last4: '1234' }] } })))).toBe(true);
    expect(hasSensitiveAnswers(withAnswers((p) => (p.input.family_plan = { vehicles: [{ description: 'Blue hatchback', plate: 'ABC-123' }] })))).toBe(true);
    expect(hasSensitiveAnswers(withAnswers((p) => (p.input.family_plan = { pets: [{ name: 'Biscuit', microchip: '985 000 000' }] })))).toBe(true);
    // Other home and pet answers are not on the list.
    expect(hasSensitiveAnswers(withAnswers((p) => (p.input.family_plan = { home: { where_kit: 'Hall closet' }, pets: [{ name: 'Biscuit' }], vehicles: [{ description: 'Blue hatchback' }] })))).toBe(false);
  });
});

describe('the protected file', () => {
  it('has the agreed shape: PBKDF2-SHA-256 with 600,000 iterations, a 16-byte salt, a 12-byte IV, base64', async () => {
    const file = await encryptText('hello', PASS);
    expect(Object.keys(file)).toEqual(['format', 'version', 'kdf', 'iv', 'ciphertext']);
    expect(file.format).toBe('ready-reckoner-plan-encrypted');
    expect(file.version).toBe(1);
    expect(file.kdf).toEqual({ name: 'PBKDF2', hash: 'SHA-256', iterations: 600_000, salt: file.kdf.salt });
    expect(KDF_ITERATIONS).toBe(600_000);
    expect(fromBase64(file.kdf.salt)).toHaveLength(SALT_BYTES);
    expect(fromBase64(file.iv)).toHaveLength(IV_BYTES);
    expect([SALT_BYTES, IV_BYTES]).toEqual([16, 12]);
    // AES-GCM adds a 16-byte tag to the 5 bytes of "hello".
    expect(fromBase64(file.ciphertext)).toHaveLength(5 + 16);
    for (const b64 of [file.kdf.salt, file.iv, file.ciphertext]) expect(b64).toMatch(/^[A-Za-z0-9+/]+={0,2}$/);
    expect(isEncryptedFile(file)).toBe(true);
  });

  it('uses a fresh salt and IV every time, so the same plan never gives the same file', async () => {
    const a = await encryptText('same', PASS);
    const b = await encryptText('same', PASS);
    expect(a.kdf.salt).not.toBe(b.kdf.salt);
    expect(a.iv).not.toBe(b.iv);
    expect(a.ciphertext).not.toBe(b.ciphertext);
  });

  it('round-trips a plan with every kind of answer, exactly', async () => {
    const plan = withAnswers((p) => {
      p.input.people[0]!.profile = { name: 'Ana Sample', medications: [{ name: 'Blood pressure tablet', dose: '10 mg', schedule: 'Mornings', purpose: 'Blood pressure' }], blood_type: 'A-' };
      p.input.family_plan = { home: { address: '12 Sample St, Philadelphia' }, documents: { accounts: [{ institution: 'First Sample Bank', last4: '0042' }] } };
      p.maps = { home: { lat: 39.93, lon: -75.15 }, routes: [], layers: { places: true, flood: false, surge: false, wildfire: false } };
      p.purchases = [{ item_id: 'water_stored', tier: 'h72', qty: 3, date: '2026-10-05' }];
    });
    const text = await protectedExportText(plan, PASS, new Date('2026-10-06T12:00:00Z'));
    // Nothing of the plan is readable in the file.
    for (const secret of ['Ana Sample', 'Blood pressure', '12 Sample St', 'First Sample Bank', '0042', 'water_stored']) expect(text).not.toContain(secret);
    const read = readPlanFile(text);
    expect(read.kind).toBe('protected');
    if (read.kind !== 'protected') return;
    const opened = await openProtected(read.file, PASS);
    expect(opened.kind).toBe('plan');
    if (opened.kind === 'plan') expect(opened.plan).toEqual(plan);
    // It opens to exactly the text a plain file would hold.
    const inside = await decryptText(read.file, PASS);
    expect(inside.ok && inside.text).toBe(exportText(plan, new Date('2026-10-06T12:00:00Z')));
  });

  it('refuses a wrong passphrase plainly, and still opens with the right one', async () => {
    const file = await encryptText(exportText(savedFor(philly)), PASS);
    expect(await decryptText(file, 'correct horse battery stapler')).toEqual({ ok: false, reason: 'wrong_passphrase' });
    expect(await openProtected(file, 'Correct horse battery staple')).toEqual({ kind: 'wrong_passphrase' });
    expect((await openProtected(file, PASS)).kind).toBe('plan');
  });

  it('refuses a changed byte as it refuses a wrong passphrase: never opens to rubbish', async () => {
    const file = await encryptText('{"format":"ready-reckoner-plan"}', PASS);
    const bytes = fromBase64(file.ciphertext);
    bytes[0] = bytes[0]! ^ 1;
    expect((await decryptText({ ...file, ciphertext: toBase64(bytes) }, PASS)).ok).toBe(false);
    expect(await decryptText({ ...file, iv: 'not base64!' }, PASS)).toEqual({ ok: false, reason: 'damaged' });
    expect(await decryptText({ ...file, iv: toBase64(new Uint8Array(8)) }, PASS)).toEqual({ ok: false, reason: 'damaged' });
  });

  it('opens to a plan only if what is inside is one', async () => {
    const file = await encryptText('{"not":"a plan"}', PASS);
    const opened = await openProtected(file, PASS);
    expect(opened.kind).toBe('error');
  });
});

describe('reading a chosen file', () => {
  it('reads a plain file exactly as before', () => {
    const plan = savedFor(philly, { confidence: { before: 2 } });
    const text = exportText(plan);
    const read = readPlanFile(text);
    expect(read).toEqual({ kind: 'plan', plan: (parseImport(text) as { ok: true; plan: SavedPlan }).plan });
    expect(read.kind === 'plan' && read.plan).toEqual(plan);
    // A bare household (a fixture) still opens as a new plan.
    const bare = readPlanFile(JSON.stringify(philly));
    expect(bare.kind === 'plan' && bare.plan.input).toEqual(philly);
  });

  it('reads a version-1 file from v0.2.0 as a plan', () => {
    const old = { ...clone(savedFor(philly)), version: 1 };
    const read = readPlanFile(JSON.stringify(old));
    expect(read.kind).toBe('plan');
    if (read.kind === 'plan') {
      expect(read.plan.version).toBe(2);
      expect(read.plan.input).toEqual(philly);
    }
  });

  it('says why a file cannot be read', async () => {
    const r = readPlanFile('not json');
    expect(r.kind === 'error' && r.reason).toMatch(/could not be read/);
    const newer = readPlanFile(JSON.stringify({ format: ENCRYPTED_FORMAT, version: 2 }));
    expect(newer.kind === 'error' && newer.reason).toMatch(/newer version/);
    const file = await encryptText('x', PASS);
    const damaged = readPlanFile(JSON.stringify({ ...file, kdf: { ...file.kdf, hash: 'MD5' } }));
    expect(damaged.kind === 'error' && damaged.reason).toMatch(/damaged/);
    expect(isEncryptedFile({ ...file, kdf: { ...file.kdf, iterations: 1e12 } })).toBe(false);
  });
});
