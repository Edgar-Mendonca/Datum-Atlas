// Title bar, quick access toolbar, Tell Me and the ribbon (Home, Select, Chart, Data, Report, View,
// Help, and Chart Tools › Format while a chart is open).
import { For, Show, JSX, createSignal, createMemo, onMount } from "solid-js";
import { call, out, tab, setTab, setBackstage, openMenu, Item, closeDialog, openDialog, theme, setTheme, view, show, chart, setChart, P, setChartId } from "./store";
import { C, ALL_CMDS } from "./cmds";
import * as D from "./dialogs";
import { Icon } from "./ui";
import { feature } from "./features";
import { soon } from "./roadmap";
import { tool, setTool } from "./chart";

const GH = "https://github.com/Edgar-Mendonca/Datum-Atlas";
const B = (p: { i: string; l: string; run?: (e: MouseEvent) => void; big?: boolean; on?: boolean; off?: boolean; dd?: (e: MouseEvent) => void; title?: string; soon?: boolean }) => p.big && p.run && p.dd ? (
  // MS Project split button: the picture runs the command, the label opens the menu.
  <div class="rbw big splitbig" classList={{ on: p.on }}>
    <button class="rb top" title={p.title ?? p.l} onClick={e => p.run!(e)}><Icon n={p.i} /></button>
    <button class="rb bot" title={`${p.l} options`} onClick={e => p.dd!(e)}><span>{p.l}</span><b class="car">▾</b></button>
  </div>
) : (
  <div class="rbw" classList={{ big: p.big }}>
    <button class="rb" classList={{ big: p.big, on: p.on, soonb: p.soon }} disabled={p.off} title={p.title ?? p.l} onClick={e => (p.run ?? p.dd)?.(e)}><Icon n={p.i} />{p.big ? <span>{p.l}{p.dd && !p.run ? " ▾" : ""}</span> : <span>{p.l}</span>}{!p.big && p.dd && !p.run && <b class="car">▾</b>}</button>
    {p.run && p.dd && <button class="rb split" onClick={e => p.dd!(e)}>▾</button>}
  </div>
);
const G = (p: { l: string; children: JSX.Element }) => <div class="grp"><div class="items">{p.children}</div><div class="cap">{p.l}</div></div>;
const S = (p: { children: JSX.Element }) => <div class="stack">{p.children}</div>;
/** A planned command: it has its place now and says when it arrives (feature catalogue id). */
const N = (p: { i: string; l: string; id: string; big?: boolean }) => { const f = feature(p.id); return <B i={p.i} l={p.l} big={p.big} soon title={`${p.l}: arrives in ${f?.[2]}`} run={() => soon(p.id)} />; };

/** Tell Me / command search (Ctrl+K, Alt+Q). */
export const palette = () => openDialog(() => {
  const [q, setQ] = createSignal(""), [k, setK] = createSignal(0);
  const list = createMemo(() => { const s = q().toLowerCase(); return ALL_CMDS.filter(c => c[0].toLowerCase().includes(s)).slice(0, 14); });
  const go = (i: number) => { const c = list()[i]; if (c) { closeDialog(); setTimeout(() => c[1]()); } };
  let input!: HTMLInputElement;
  onMount(() => input.focus());
  return (
    <div class="ov top" onMouseDown={e => e.target === e.currentTarget && closeDialog()}>
      <div class="pal">
        <input ref={input} placeholder="Tell me what you want to do" value={q()} onInput={e => { setQ(e.currentTarget.value); setK(0); }}
          onKeyDown={e => { if (e.key === "ArrowDown") { e.preventDefault(); setK(Math.min(list().length - 1, k() + 1)); } else if (e.key === "ArrowUp") { e.preventDefault(); setK(Math.max(0, k() - 1)); } else if (e.key === "Enter") { e.preventDefault(); go(k()); } else if (e.key === "Escape") closeDialog(); }} />
        <div class="pl"><For each={list()}>{(c, i) => <button classList={{ on: i() === k() }} onMouseEnter={() => setK(i())} onClick={() => go(i())}><span>{c[0]}</span>{c[2] && <kbd>{c[2]}</kbd>}</button>}</For></div>
      </div>
    </div>
  );
});

export const TitleBar = (p: { update: string | null }) => (
  <header class="top">
    <div class="bar">
      <div class="qat">
        <img src="logo.svg" alt="" class="logo" />
        <button title="Save (Ctrl+S)" onClick={() => C.save()}><Icon n="save" /></button>
        <button title="Undo (Ctrl+Z)" disabled={!out()?.undo} onClick={C.undo}><Icon n="undo" /></button>
        <button title="Redo (Ctrl+Y)" disabled={!out()?.redo} onClick={C.redo}><Icon n="redo" /></button>
        <button title="Print (Ctrl+P)" onClick={C.print}><Icon n="print" /></button>
      </div>
      <div class="title">{out() ? `${out()!.project.name}${out()!.dirty ? " •" : ""} — Datum Atlas` : "Datum Atlas"}</div>
      <button class="search" onClick={palette} title="Tell me what you want to do (Alt+Q)"><Icon n="search" /><span>Tell me what you want to do</span><kbd>Ctrl+K</kbd></button>
      <Show when={p.update}><button class="upd" onClick={() => setBackstage("about")}>Update {p.update} available</button></Show>
      <button class="theme" title="Theme: Auto follows Windows" onClick={() => setTheme(theme() === "auto" ? "light" : theme() === "light" ? "dark" : "auto")}>Theme · {theme()}</button>
    </div>
    <nav class="tabs">
      <button class="file" onClick={() => setBackstage("home")}>File</button>
      <For each={[["home", "Home"], ["select", "Select"], ["chart", "Chart"], ["data", "Data"], ["report", "Report"], ["view", "View"], ["help", "Help"]]}>{([k, l]) => <button classList={{ on: tab() === k }} onClick={() => setTab(k)}>{l}</button>}</For>
      <Show when={view() === "charts"}><div class="ctxgrp"><small>Chart Tools</small><button classList={{ on: tab() === "format" }} onClick={() => setTab("format")}>Format</button></div></Show>
    </nav>
  </header>
);

/** Shrinks the ribbon in steps (labels of small buttons, then of big ones) instead of scrolling sideways. */
const fitRibbon = (el: HTMLElement) => {
  // Tighten everything first, then fold groups from the right: small-button labels, then big-button labels.
  const fit = () => {
    const gs = [...el.querySelectorAll<HTMLElement>(":scope > .grp")];
    el.dataset.fit = "0"; gs.forEach(g => delete g.dataset.fit);
    const over = () => el.scrollWidth > el.clientWidth + 1;
    if (!over()) return;
    el.dataset.fit = "1";
    for (const lvl of ["2", "3"]) for (let i = gs.length - 1; i >= 0 && over(); i--) gs[i].dataset.fit = lvl;
  };
  new ResizeObserver(fit).observe(el);
  new MutationObserver(() => requestAnimationFrame(fit)).observe(el, { childList: true, subtree: false });
};
const PAIRS: [string, string, string][] = [["Modulus – density", "rho", "E"], ["Strength – density", "rho", "sy"], ["Modulus – strength", "sy", "E"], ["Toughness – strength", "sy", "kic"], ["Toughness – modulus", "E", "kic"],
  ["Modulus – price", "price", "E"], ["Strength – price", "price", "sy"], ["Conductivity – expansion", "alpha", "lam"], ["Max temperature – density", "rho", "tmax"], ["CO₂ – price", "price", "co2"], ["Strength – max temperature", "tmax", "sy"]];
const chartMenu = (): Item[] => PAIRS.map(([l, x, y]) => ({ label: l, icon: "atlas", run: () => C.newChart(x, y) }));
const lineMenu = (): Item[] => [[1, "Slope 1 · E/ρ, light stiff tie"], [2, "Slope 2 · E^½/ρ, light stiff beam"], [3, "Slope 3 · E^⅓/ρ, light stiff panel"], [0.5, "Slope ½ · σy²/E, springs"], [-1, "Slope −1"]]
  .map(([n, l]) => ({ label: l as string, icon: "index", run: () => C.line(n as number) }));
const hullMenu = (): Item[] => [["family", "Envelopes around families"], ["class", "Envelopes around classes"], ["none", "No envelopes"]].map(([k, l]) => ({ label: l, on: chart()?.hull === k, run: () => setChart(c => { c.hull = k; }) }));
const chartsMenu = (): Item[] => P().charts.map(c => ({ label: c.name, on: c.id === chart()?.id, run: () => { setChartId(c.id); show("charts"); } }));
const sel = () => chart()?.select;
export const Ribbon = () => (
  <section class="ribbon" ref={fitRibbon}>
    <Show when={tab() === "home"}>
      <G l="Start"><B big i="setup" l="Setup" title="The requirements: function, constraints, objective" run={() => show("setup")} /><B big i="atlas" l="Property Chart" run={() => show("charts")} dd={e => openMenu(e, chartsMenu())} /></G>
      <G l="Select"><B big i="filter" l="Add Limit" title="A minimum or maximum value" run={C.addLimit} /><B big i="index" l="Selection Line" run={() => C.line(chart() && (sel() as any)?.Line ? (sel() as any).Line.slope : 2)} dd={e => openMenu(e, lineMenu())} /><B big i="results" l="Results" run={() => show("results")} /></G>
      <G l="Materials"><S><B i="tree" l="Browse" run={() => show("browse")} /><B i="datasheet" l="Datasheet" run={C.sheet} /><B i="material" l="Add Material" run={C.addMaterial} /></S></G>
      <G l="Find"><S><B i="search" l="Tell Me" run={palette} /><N i="find" l="Search Everything" id="A5" /><N i="units" l="Units" id="A4" /></S></G>
      <G l="File"><S><B i="save" l="Save" run={() => C.save()} /><B i="print" l="Print" run={C.print} /><B i="export" l="Export Chart" run={() => C.exportChart("svg")} /></S></G>
    </Show>
    <Show when={tab() === "select"}>
      <G l="Stages"><B big i="filter" l="Limit" title="Minimum and maximum values, yes/no and ratings" run={C.addLimit} /><B big i="index" l="Line" title="A selection line of a slope: a performance index" run={() => C.line(2)} dd={e => openMenu(e, lineMenu())} />
        <B big i="box" l="Box" title="Drag a box on the chart" run={() => { show("charts"); setTool("box"); }} on={tool() === "box"} /><B big i="tree" l="Families" title="Leave families or classes out" run={() => show("stages")} /><S><N i="poly" l="Polygon" id="C8" /><B i="del" l="Clear Line or Box" run={C.clearSel} off={!sel()} /><B i="list" l="All Stages" run={() => show("stages")} /></S></G>
      <G l="Index"><N big i="builder" l="Index Builder" id="C6" /><S><N i="list" l="Index Library" id="C7" /><B i="setup" l="Design Problems" run={() => show("setup")} /></S></G>
      <G l="Results"><B big i="results" l="Results" run={() => show("results")} /><S><N i="compare" l="Compare" id="D2" /><N i="eco" l="Greener Substitutes" id="E3" /></S></G>
    </Show>
    <Show when={tab() === "chart"}>
      <G l="Chart"><B big i="atlas" l="New Chart" run={() => C.newChart()} dd={e => openMenu(e, chartMenu())} /><S><B i="list" l="Open Chart" dd={e => openMenu(e, chartsMenu())} /><B i="del" l="Delete Chart" run={C.delChart} /><N i="bar" l="Bar Chart" id="B6" /></S></G>
      <G l="Axes"><S><B i="log" l="Log X" on={!!chart()?.logX} run={() => setChart(c => { c.logX = !c.logX; c.select = null; })} /><B i="log" l="Log Y" on={!!chart()?.logY} run={() => setChart(c => { c.logY = !c.logY; c.select = null; })} /><B i="swap" l="Swap Axes" run={() => setChart(c => { [c.x, c.y, c.logX, c.logY] = [c.y, c.x, c.logY, c.logX]; c.select = null; })} /></S>
        <S><N i="fx" l="Expression Axis" id="B8" /><B i="fit" l="Fit" run={C.fit} /></S></G>
      <G l="Tools"><B big i="pan" l="Pan" on={tool() === "pan"} run={() => setTool("pan")} /><B big i="index" l="Line" on={tool() === "line"} run={() => setTool("line")} dd={e => openMenu(e, lineMenu())} /><B big i="box" l="Box" on={tool() === "box"} run={() => setTool("box")} /></G>
      <G l="Show"><B big i="hull" l="Envelopes" dd={e => openMenu(e, hullMenu())} /><S><B i="tag" l="Labels" on={!!chart()?.labels} run={() => setChart(c => { c.labels = !c.labels; })} /><B i="index" l="Guide Lines" on={!!chart()?.guides.length} run={() => setChart(c => { c.guides = c.guides.length ? [] : [1, 2, 3]; })} /><N i="tag" l="Smart Labels" id="B9" /></S></G>
      <G l="Share"><B big i="export" l="Export" run={() => C.exportChart("svg")} dd={e => openMenu(e, [{ label: "SVG vector", icon: "export", run: () => C.exportChart("svg") }, { label: "PNG picture", run: () => C.exportChart("png") }, { label: "PDF (Print › Save as PDF)", icon: "print", run: C.print }])} /><S><N i="copy" l="Copy Picture" id="B11" /><B i="print" l="Print" run={C.print} /></S></G>
    </Show>
    <Show when={tab() === "format"}>
      <G l="Envelopes"><S><For each={[["family", "Families"], ["class", "Classes"], ["none", "None"]]}>{([k, l]) => <B i="hull" l={l} on={chart()?.hull === k} run={() => setChart(c => { c.hull = k; })} />}</For></S></G>
      <G l="Families"><S><B i="eye" l="Show All Families" run={() => setChart(c => { c.hidden = []; })} /><B i="tag" l="Labels" on={!!chart()?.labels} run={() => setChart(c => { c.labels = !c.labels; })} /><B i="index" l="Guide Lines" on={!!chart()?.guides.length} run={() => setChart(c => { c.guides = c.guides.length ? [] : [1, 2, 3]; })} /></S></G>
      <G l="Style"><N big i="style" l="Chart Styles" id="B10" /><S><N i="style" l="Publication" id="B10" /><N i="style" l="Monochrome" id="B10" /></S></G>
    </Show>
    <Show when={tab() === "data"}>
      <G l="Materials"><B big i="tree" l="Browse" run={() => show("browse")} /><B big i="datasheet" l="Datasheet" run={C.sheet} /><B big i="material" l="Add Material" run={C.addMaterial} /></G>
      <G l="Databases"><B big i="db" l="Data and Sources" run={() => show("data")} /><S><N i="import" l="Import Excel / CSV" id="A8" /><N i="export" l="Export Database" id="A8" /><N i="db" l="Open Database" id="A9" /></S></G>
      <G l="Properties"><S><N i="fx" l="Derived Property" id="A6" /><N i="units" l="Units" id="A4" /><N i="image" l="Pictures" id="A10" /></S></G>
    </Show>
    <Show when={tab() === "report"}>
      <G l="Compare"><N big i="compare" l="Compare" id="D2" /><N big i="radar" l="Radar Chart" id="D3" /></G>
      <G l="Report"><N big i="report" l="Selection Report" id="D4" /><S><B i="print" l="Print" run={C.print} /><B i="results" l="Results" run={() => show("results")} /></S></G>
      <G l="Eco"><N big i="eco" l="Eco Audit" id="E1" /><S><N i="bar" l="Eco Charts" id="E2" /><N i="eco" l="Greener Substitutes" id="E3" /></S></G>
      <G l="Process"><N big i="process" l="Process Selection" id="F2" /><S><N i="list" l="Processes" id="F1" /><N i="bar" l="Cost Model" id="F3" /></S></G>
    </Show>
    <Show when={tab() === "view"}>
      <G l="Views"><B big i="atlas" l="Chart" run={() => show("charts")} /><B big i="tree" l="Browse" run={() => show("browse")} /><B big i="results" l="Results" run={() => show("results")} /><S><B i="filter" l="Stages" run={() => show("stages")} /><B i="db" l="Data" run={() => show("data")} /><B i="setup" l="Setup" run={() => show("setup")} /></S></G>
      <G l="Zoom"><S><B i="zin" l="Zoom In" run={() => C.zoom(1.4)} /><B i="zout" l="Zoom Out" run={() => C.zoom(1 / 1.4)} /><B i="fit" l="Fit" run={C.fit} /></S></G>
      <G l="Theme"><S><For each={[["auto", "Auto (Windows)"], ["light", "Light"], ["dark", "Dark"]]}>{([k, l]) => <B i="style" l={l} on={theme() === k} run={() => setTheme(k)} />}</For></S></G>
    </Show>
    <Show when={tab() === "help"}>
      <G l="Learn"><B big i="report" l="Getting Started" run={() => D.gettingStarted()} /><B big i="list" l="Roadmap" title="Everything Datum Atlas will do, and when" run={() => setBackstage("roadmap")} /><B big i="info" l="Glossary" title="Bubble, envelope, index and other terms" run={() => D.glossary()} /><B big i="keys" l="Shortcuts" run={() => D.shortcuts()} /><B big i="search" l="Tell Me" run={palette} /></G>
      <G l="Datum Atlas on GitHub"><B big i="github" l="GitHub" title="The project and its code" run={() => call("open_url", { url: GH })} /><B big i="tag" l="Releases" title="Every version and its change log" run={() => call("open_url", { url: `${GH}/releases` })} /><B big i="bug" l="Report an Issue" run={() => call("open_url", { url: `${GH}/issues/new` })} /></G>
      <G l="Updates"><B big i="upd" l="Check for Updates" run={() => setBackstage("about")} /></G>
    </Show>
  </section>
);
