// Slot math of the 0x09 macro packet (crates/shark-core/src/protocol/macros.rs): a pause after an event longer than
// 1270 ms is sent as up to 10 filler events of 200 ms plus the remainder, and the packet holds maxMacroEvents pairs.
import type { MacroEvent } from "../../api/client";

const FILLER_MS = 200;
const MAX_FILLERS = 10;
const MAX_SINGLE_MS = 1270;
/** Longest pause the packet can carry after one event. */
export const MAX_DELAY_MS = MAX_FILLERS * FILLER_MS + MAX_SINGLE_MS;

export function slotsUsed(events: MacroEvent[]): number {
  return events.reduce((n, e) => n + 1 + (e.delayMs > MAX_SINGLE_MS ? Math.min(MAX_FILLERS, Math.floor(e.delayMs / FILLER_MS)) : 0), 0);
}
