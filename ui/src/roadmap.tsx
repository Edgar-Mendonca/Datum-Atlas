// The whole app laid out: a card for every feature that is not built yet, and the Roadmap page.
import { For, Show, createSignal, createMemo } from "solid-js";
import { FEATURES, AREAS, RELEASES, feature, ready, VERSION_NOW } from "./features";
import { openDialog, closeDialog, setBackstage } from "./store";
import { Dialog, Pick } from "./ui";

const rel = (r: string) => `${r.split(".").length === 2 ? `${r}.0` : r}`;
export const Chip = (p: { id: string }) => { const f = feature(p.id)!; return ready(p.id) ? <span class="chip ok">Ready</span> : <span class="chip soon" title={RELEASES[f[2]]}>In {rel(f[2])}</span>; };

/** What a feature that is not built yet will do, and when it comes. */
export const soon = (id: string) => openDialog(() => {
  const f = feature(id);
  if (!f) return <Dialog title="Coming later"><p>This feature is planned.</p></Dialog>;
  return (
    <Dialog title={f[1]} extra={<button class="btn" onClick={() => { closeDialog(); setBackstage("roadmap"); }}>Whole roadmap</button>}>
      <div class="soonbox"><span class="fid">{f[0]}</span><Chip id={f[0]} /></div>
      <p>{f[3]}</p>
      <p class="muted">{ready(id) ? "Available now." : `Arrives in Datum Atlas ${rel(f[2])}: ${RELEASES[f[2]]}.`}</p>
    </Dialog>
  );
});

/** Help › Roadmap: every feature, what it does and when it comes. */
export const RoadmapPage = () => {
  const [q, setQ] = createSignal(""), [r, setR] = createSignal("all");
  const list = createMemo(() => FEATURES.filter(f => (r() === "all" || (r() === "ready" ? ready(f[0]) : f[2] === r())) && (f[0] + f[1] + f[3]).toLowerCase().includes(q().toLowerCase())));
  const done = FEATURES.filter(f => ready(f[0])).length;
  return (
    <div class="roadmap"><h1>Roadmap</h1>
      <p class="muted">Everything Datum Atlas will do, in the order it is being built. {done} of {FEATURES.length} features are ready in {rel(VERSION_NOW)}. Planned commands already have their place in the ribbon and the sidebar; clicking one tells you when it arrives.</p>
      <div class="rmbar"><input class="find" placeholder="Search features (e.g. index, eco, import)" value={q()} onInput={e => setQ(e.currentTarget.value)} />
        <Pick value={r()} set={setR} options={[["all", "All releases"], ["ready", "Ready now"], ...Object.entries(RELEASES).map(([k, v]) => [k, `${rel(k)} · ${v}`] as [string, string])]} /></div>
      <div class="rmrel"><For each={Object.entries(RELEASES)}>{([k, v]) => <div classList={{ now: k === VERSION_NOW }}><b>{rel(k)}</b><span>{v}</span><small>{FEATURES.filter(f => f[2] === k).length} features</small></div>}</For></div>
      <For each={Object.entries(AREAS)}>{([a, name]) => { const fs = () => list().filter(f => f[0].replace(/\d+/, "") === a); return <Show when={fs().length}>
        <h2>{a} · {name}</h2>
        <table class="tbl rmtbl"><tbody><For each={fs()}>{f => <tr><td class="fid">{f[0]}</td><td><b>{f[1]}</b><div class="muted">{f[3]}</div></td><td><Chip id={f[0]} /></td></tr>}</For></tbody></table>
      </Show>; }}</For>
    </div>
  );
};
