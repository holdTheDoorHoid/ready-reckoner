/**
 * The engine behind the web app (docs/ENGINE-API.md). The mock engine (`mock.ts`, web-shell
 * workstream) and the WebAssembly adapter (web-engine workstream) both implement `Engine`, so
 * screens never know which one they talk to.
 */
import type {
  Catalogue,
  EngineInfo,
  Envelope,
  Explanation,
  ExplainRequest,
  LocationInput,
  LocationResolved,
  PackInfo,
  PlanInput,
  PlanOutput,
} from './types';

export * from './types';

/** The eight engine functions. Every call resolves to an envelope; none rejects for engine errors. */
export interface Engine {
  /** Versions, loaded packs and the attributions the About screen must show. */
  engine_info(): Promise<Envelope<EngineInfo>>;
  /** Hands a same-origin data pack to the engine; the engine itself never fetches. */
  load_pack(name: string, bytes: Uint8Array): Promise<Envelope<PackInfo>>;
  /** Free-text county search ("phila", "42101", "Cook, IL"): up to 10 matches. */
  county_search(query: string): Promise<Envelope<LocationResolved[]>>;
  /** ZIP or county to a location; errors unknown_zip, unknown_county or ambiguous_zip carry suggestions. */
  resolve_location(input: LocationInput): Promise<Envelope<LocationResolved>>;
  /** The whole plan. Pure and fast: called on every dial change. */
  assess(input: PlanInput): Promise<Envelope<PlanOutput>>;
  /** Why a hazard, bucket, item, requirement or warning is what it is. */
  explain(request: ExplainRequest): Promise<Envelope<Explanation>>;
  /** Items, citations, guidance metadata and the plain names of every id. */
  catalogue(): Promise<Envelope<Catalogue>>;
  /** A valid skeleton the interview starts from; replace its placeholder date and ZIP. */
  defaults(): Promise<Envelope<PlanInput>>;
}

/**
 * The string-in, string-out surface `rr-wasm` exports. Each function takes JSON (or bytes) and
 * returns the JSON of an `Envelope`; an adapter parses it into `Engine`.
 */
export interface RawEngine {
  engine_info(): string;
  load_pack(name: string, bytes: Uint8Array): string;
  county_search(query: string): string;
  resolve_location(input_json: string): string;
  assess(input_json: string): string;
  explain(request_json: string): string;
  catalogue(): string;
  defaults(): string;
}
