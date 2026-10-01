/**
 * What the binder's page components need from the screen around them, passed down as one prop:
 * the binder and its page index (for links, "Turn to" and the Sources page), the household's
 * stored maps for the `map_slot` blocks, and the way to open the maps panel.
 */
import type { Binder, MapSlotKind } from '../../engine/types';
import { pageIndex, type PageEntry } from '../../lib/binder/model';
import type { MapRecord, MapsStatus } from '../../lib/maps/store';

export interface BinderView {
  binder: Binder;
  pages: Map<string, PageEntry>;
  /** The stored maps by slot kind, and why a slot has none. */
  maps: { status: MapsStatus; records: Partial<Record<MapSlotKind, MapRecord>> };
  /** Open the maps panel at its consent screen ("Add maps"); absent where maps cannot be added. */
  addMaps?: () => void;
}

/** A view with no maps (for a binder shown on its own). */
export function plainView(binder: Binder): BinderView {
  return { binder, pages: pageIndex(binder), maps: { status: 'none', records: {} } };
}
