/**
 * The saved file, protected (DESIGN-DELTA-v3 §7). Once the plan holds sensitive answers (medical
 * details, insurance IDs, an address, accounts…), "Save a copy of your plan" offers to protect the
 * file with a passphrase, on by default; the plain file stays one choice away, behind a warning.
 *
 * Protection is the browser's own WebCrypto, nothing else: the passphrase is stretched with
 * PBKDF2-SHA-256 (600,000 iterations, a fresh 16-byte random salt) into an AES-GCM-256 key, and the
 * plan's export text is sealed with a fresh 12-byte random IV. The file keeps the name
 * `ready-reckoner-plan.json` and reads
 *
 *     { "format": "ready-reckoner-plan-encrypted", "version": 1,
 *       "kdf": { "name": "PBKDF2", "hash": "SHA-256", "iterations": 600000, "salt": "<base64>" },
 *       "iv": "<base64>", "ciphertext": "<base64>" }
 *
 * The ciphertext opens to exactly what an unprotected file holds, so a protected file imports
 * through the same checks. AES-GCM authenticates what it seals: a wrong passphrase or a changed
 * byte fails to open, never opens to rubbish. Nobody can recover a forgotten passphrase; the
 * printed binder is the household's backup. The passphrase is never stored or kept.
 */
import type { SavedPlan } from './persistence';
import { exportText, mapsHoldLocation, parseImport } from './persistence';
import { tidyProfile } from './profile';
import { tidyText } from './tidy';

export const ENCRYPTED_FORMAT = 'ready-reckoner-plan-encrypted';
export const ENCRYPTED_VERSION = 1;
export const KDF_ITERATIONS = 600_000;
export const SALT_BYTES = 16;
export const IV_BYTES = 12;
/** The shortest passphrase the save dialog accepts. */
export const PASSPHRASE_MIN = 8;

export interface EncryptedPlanFile {
  format: typeof ENCRYPTED_FORMAT;
  version: typeof ENCRYPTED_VERSION;
  kdf: { name: 'PBKDF2'; hash: 'SHA-256'; iterations: number; salt: string };
  iv: string;
  ciphertext: string;
}

// ---------------------------------------------------------------------------------------------
// What makes a saved plan sensitive (§7)
// ---------------------------------------------------------------------------------------------

/**
 * True when the plan holds any answer §7 counts as sensitive: anything in a person's profile
 * beyond their name and phone, the home's address, anything under documents and money, a vehicle's
 * plate, or a pet's microchip or tag number; and the maps' home pin or a drawn route, which say
 * where the household lives as plainly as the address (web-maps' `mapsHoldLocation`). Blank or
 * whitespace answers do not count (the engine drops them), and neither does a place's kind alone.
 */
export function hasSensitiveAnswers(plan: SavedPlan): boolean {
  if (mapsHoldLocation(plan.maps)) return true;
  const input = plan.input;
  for (const person of input.people ?? []) {
    const profile = tidyProfile(person.profile);
    if (profile && Object.keys(profile).some((key) => key !== 'name' && key !== 'phone')) return true;
  }
  const family = input.family_plan;
  if (!family) return false;
  if (tidyText(family.home?.address, Number.MAX_SAFE_INTEGER)) return true;
  if (hasText(family.documents)) return true;
  if ((family.vehicles ?? []).some((v) => tidyText(v?.plate, Number.MAX_SAFE_INTEGER))) return true;
  return (family.pets ?? []).some((p) => tidyText(p?.microchip, Number.MAX_SAFE_INTEGER));
}

/** True when any text anywhere inside `x` is more than whitespace. */
function hasText(x: unknown): boolean {
  if (typeof x === 'string') return x.trim() !== '';
  if (Array.isArray(x)) return x.some(hasText);
  if (typeof x === 'object' && x !== null) return Object.values(x).some(hasText);
  return false;
}

// ---------------------------------------------------------------------------------------------
// Base64 for bytes (the file's salt, IV and ciphertext)
// ---------------------------------------------------------------------------------------------

export function toBase64(bytes: Uint8Array): string {
  let binary = '';
  // Chunks keep String.fromCharCode's argument list short for a large ciphertext.
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(binary);
}

export function fromBase64(text: string): Uint8Array<ArrayBuffer> {
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

// ---------------------------------------------------------------------------------------------
// Sealing and opening
// ---------------------------------------------------------------------------------------------

function subtle(): SubtleCrypto {
  const s = globalThis.crypto?.subtle;
  if (!s) throw new Error('This browser cannot protect files (no WebCrypto).');
  return s;
}

/** True when the browser can protect a file (WebCrypto is only there on https and localhost). */
export function canProtect(): boolean {
  return !!globalThis.crypto?.subtle;
}

async function keyFrom(passphrase: string, salt: Uint8Array<ArrayBuffer>, iterations: number): Promise<CryptoKey> {
  const material = await subtle().importKey('raw', new TextEncoder().encode(passphrase), 'PBKDF2', false, ['deriveKey']);
  return subtle().deriveKey({ name: 'PBKDF2', hash: 'SHA-256', salt, iterations }, material, { name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt']);
}

/** Seal `plaintext` with `passphrase` into the protected file's object. */
export async function encryptText(plaintext: string, passphrase: string): Promise<EncryptedPlanFile> {
  const salt = globalThis.crypto.getRandomValues(new Uint8Array(SALT_BYTES));
  const iv = globalThis.crypto.getRandomValues(new Uint8Array(IV_BYTES));
  const key = await keyFrom(passphrase, salt, KDF_ITERATIONS);
  const sealed = await subtle().encrypt({ name: 'AES-GCM', iv }, key, new TextEncoder().encode(plaintext));
  return {
    format: ENCRYPTED_FORMAT,
    version: ENCRYPTED_VERSION,
    kdf: { name: 'PBKDF2', hash: 'SHA-256', iterations: KDF_ITERATIONS, salt: toBase64(salt) },
    iv: toBase64(iv),
    ciphertext: toBase64(new Uint8Array(sealed)),
  };
}

export type Opened = { ok: true; text: string } | { ok: false; reason: 'wrong_passphrase' | 'damaged' | 'unsupported' };

/** Open a protected file with `passphrase`. A wrong passphrase and a damaged file both fail cleanly. */
export async function decryptText(file: EncryptedPlanFile, passphrase: string): Promise<Opened> {
  if (!canProtect()) return { ok: false, reason: 'unsupported' };
  let salt: Uint8Array<ArrayBuffer>;
  let iv: Uint8Array<ArrayBuffer>;
  let sealed: Uint8Array<ArrayBuffer>;
  try {
    salt = fromBase64(file.kdf.salt);
    iv = fromBase64(file.iv);
    sealed = fromBase64(file.ciphertext);
  } catch {
    return { ok: false, reason: 'damaged' };
  }
  if (iv.length !== IV_BYTES || salt.length === 0) return { ok: false, reason: 'damaged' };
  let key: CryptoKey;
  try {
    key = await keyFrom(passphrase, salt, file.kdf.iterations);
  } catch {
    return { ok: false, reason: 'damaged' };
  }
  try {
    const opened = await subtle().decrypt({ name: 'AES-GCM', iv }, key, sealed);
    return { ok: true, text: new TextDecoder().decode(opened) };
  } catch {
    // AES-GCM refuses anything it did not seal with this key: almost always a mistyped passphrase.
    return { ok: false, reason: 'wrong_passphrase' };
  }
}

/** True when `x` is a protected plan file this version can open. */
export function isEncryptedFile(x: unknown): x is EncryptedPlanFile {
  if (typeof x !== 'object' || x === null) return false;
  const f = x as Record<string, unknown>;
  const kdf = f.kdf as Record<string, unknown> | undefined;
  return (
    f.format === ENCRYPTED_FORMAT &&
    f.version === ENCRYPTED_VERSION &&
    typeof kdf === 'object' &&
    kdf !== null &&
    kdf.name === 'PBKDF2' &&
    kdf.hash === 'SHA-256' &&
    typeof kdf.iterations === 'number' &&
    Number.isInteger(kdf.iterations) &&
    kdf.iterations > 0 &&
    kdf.iterations <= 10_000_000 &&
    typeof kdf.salt === 'string' &&
    typeof f.iv === 'string' &&
    typeof f.ciphertext === 'string'
  );
}

/** The protected file's text, ready to download: the plan's export, sealed with `passphrase`. */
export async function protectedExportText(plan: SavedPlan, passphrase: string, now: Date = new Date()): Promise<string> {
  const sealed = await encryptText(exportText(plan, now), passphrase);
  return `${JSON.stringify(sealed, null, 2)}\n`;
}

// ---------------------------------------------------------------------------------------------
// Reading a chosen file: plain, protected, or neither
// ---------------------------------------------------------------------------------------------

export type PlanFile =
  | { kind: 'plan'; plan: SavedPlan }
  | { kind: 'protected'; file: EncryptedPlanFile }
  | { kind: 'error'; reason: string };

/** Read a chosen file: a plain plan (or bare household), a protected plan that needs its passphrase, or a reason it cannot be read. */
export function readPlanFile(text: string): PlanFile {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { kind: 'error', reason: 'The file could not be read. Choose the .json file you saved from Ready Reckoner.' };
  }
  if (typeof data === 'object' && data !== null && (data as { format?: unknown }).format === ENCRYPTED_FORMAT) {
    if (isEncryptedFile(data)) return { kind: 'protected', file: data };
    const version = (data as { version?: unknown }).version;
    return {
      kind: 'error',
      reason:
        typeof version === 'number' && version > ENCRYPTED_VERSION
          ? 'The file was protected by a newer version of Ready Reckoner. Reload the page to update, then try again.'
          : 'The protected file is damaged and cannot be opened.',
    };
  }
  const check = parseImport(text);
  return check.ok ? { kind: 'plan', plan: check.plan } : { kind: 'error', reason: check.reason };
}

/** Open a protected file and check the plan inside it as any plain file is checked. */
export async function openProtected(file: EncryptedPlanFile, passphrase: string): Promise<PlanFile | { kind: 'wrong_passphrase' }> {
  const opened = await decryptText(file, passphrase);
  if (!opened.ok) {
    if (opened.reason === 'wrong_passphrase') return { kind: 'wrong_passphrase' };
    if (opened.reason === 'unsupported') return { kind: 'error', reason: 'This browser cannot open protected files. Try a current browser, over https.' };
    return { kind: 'error', reason: 'The protected file is damaged and cannot be opened.' };
  }
  const check = parseImport(opened.text);
  return check.ok ? { kind: 'plan', plan: check.plan } : { kind: 'error', reason: check.reason };
}
