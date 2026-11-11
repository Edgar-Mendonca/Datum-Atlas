// Microstructure module › Structures: the typical structure of every class of material, drawn by the
// engine (micro.rs) as a labelled schematic. A datasheet's "Typical structure" link opens it on
// that material. Image analysis, the generator, EBSD and DIC come in later releases.
import { For, Show, createResource, createSignal } from "solid-js";
import { call, db, mat, openSheet } from "./store";
import { Help } from "./help";

/** A material's schematic microstructure (SVG and caption). */
export const MicroFig = (p: { id: string; small?: boolean }) => {
  const [m] = createResource(() => p.id, id => call<{ svg: string; caption: string }>("microstructure", { id }));
  return <Show when={m()}>{x => <figure class="micro" classList={{ small: p.small }}><div class="msvg" innerHTML={x().svg} /><figcaption>{x().caption}</figcaption></figure>}</Show>;
};

/** The material the page opens on (from a datasheet's "Typical structure"). */
export const [microFocus, setMicroFocus] = createSignal("");

export const StructuresPage = () => {
  // One card per class: the first material of each family › class › subclass path.
  const classes = () => {
    const seen = new Map<string, string[]>();
    for (const m of db().materials) { const k = m.family.slice(0, 3).join(" › "); seen.set(k, [...(seen.get(k) ?? []), m.id]); }
    return [...seen.entries()];
  };
  const fams = () => [...new Set(classes().map(([k]) => k.split(" › ")[0]))];
  return (
    <div class="page structpage">
      <h1>Typical structures <Help k="micro" /></h1>
      <p class="muted">What each class of material is made of, as it would look under a microscope: grains, phases, fibres, cells or crystals. These are schematics drawn by Datum Atlas from a recipe for each class, never photographs of real samples; a datasheet shows a real micrograph when an open-licence one exists.</p>
      <Show when={mat(microFocus())}>{m => <section class="panel focus"><h2>{m().name}</h2>
        <div class="fwrap"><MicroFig id={m().id} /><div><p>{m().family.join(" › ")}</p><p><button class="btn sm" onClick={() => openSheet(m().id)}>Open its datasheet</button></p></div></div></section>}</Show>
      <For each={fams()}>{f => <><h2 class="famh">{f}</h2><div class="sgrid">
        <For each={classes().filter(([k]) => k.split(" › ")[0] === f)}>{([k, ids]) => <div class="scard">
          <MicroFig id={ids[0]} small /><b>{k.split(" › ").slice(1).join(" › ") || f}</b>
          <span class="muted small">{ids.slice(0, 4).map(id => mat(id)?.name).join(" · ")}{ids.length > 4 ? ` and ${ids.length - 4} more` : ""}</span>
        </div>}</For></div></>}</For>
    </div>
  );
};
