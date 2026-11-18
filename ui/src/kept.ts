// Research work kept in the project (crates/core research.rs): the micrograph, the EBSD map and
// the DIC field, each with its page settings. The window gets what to show (`info`) and a
// fingerprint (`id`); the data itself stays in the engine.
import { createEffect, on } from "solid-js";
import { act, out } from "./store";

export type Which = "image" | "ebsd" | "dic" | "xrd";
export interface Kept { info: any; id: string; settings: Record<string, any> }
export const kept = (w: Which): Kept | null => (out()?.project as any)?.research?.[w] ?? null;
/** Changes only when other work is opened (a new file, another project, undo), not on every edit. */
export const keptId = (w: Which) => kept(w)?.id ?? "";
/** Removes the work from the project (undo brings it back). */
export const forget = (w: Which) => act({ Research: { which: w, item: null } });

const timers: Record<string, number> = {};
/** A page's settings saved with the kept work under `key`: put back when other work is opened,
 *  and saved (a moment after the last change) when they change. Call it once, in the page. */
export const keepSettings = <T extends object>(w: Which, key: string, get: () => T, set: (v: T) => void) => {
  const d = get();
  const now = (): T => ({ ...d, ...(kept(w)?.settings?.[key] ?? {}) });
  createEffect(on(() => keptId(w), () => set(now())));
  createEffect(on(get, v => {
    const t = `${w}.${key}`;
    clearTimeout(timers[t]);
    timers[t] = window.setTimeout(() => { if (kept(w) && JSON.stringify(v) !== JSON.stringify(now())) act({ ResearchSet: { which: w, key, value: v } }, true); }, 400);
  }, { defer: true }));
};
