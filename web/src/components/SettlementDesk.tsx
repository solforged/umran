import { useLayoutEffect, useRef, useState } from "react";
import { ArrowRight, Footprints, GitFork, MapPinned, X } from "lucide-react";
import type { Catalog, HistoryPoint, Naming, Overview, SettlementChoice, SettlementIntent, SettlementPlan, SettlementPreview, WorldMap } from "../model";
import { YEARS } from "../model";
import { TERRAIN_NAME } from "../lore";
import { NamingSelect } from "./NamingSelect";
import { closeClosingDialogs } from "../motion";

export interface SettlementDraft {
  community: number;
  intent: SettlementIntent;
  share: number;
  destination: number | null;
  point: HistoryPoint;
}

const INTENTS = [
  { id: "partition", icon: GitFork, name: "Divide their lands", detail: "Two hearts, each with connected lands. People stay where they live." },
  { id: "settlers", icon: MapPinned, name: "Send settlers", detail: "A share departs to found a new people whose speech can diverge." },
  { id: "migration", icon: Footprints, name: "Move the people", detail: "Everyone moves together, keeping their identity and language." },
] as const;

const count = (n: number) => Math.round(n).toLocaleString();
export function landTitle(overview: Overview, map: WorldMap, region: number): string {
  return overview.places.find((p) => p.region === region)?.names.at(-1)?.spelled ??
    `${TERRAIN_NAME[map.regions[region].terrain]} · land ${region + 1}`;
}

export function SettlementAccount({ plan, overview, map }: { plan: SettlementPlan; overview: Overview; map: WorldMap }) {
  const migration = plan.choice.intent === "migration";
  const territorial = plan.choice.intent === "partition";
  return <div className="settlement-account">
    <div className="settlement-census" aria-label="Population allocation">
      <div><span>{migration ? "Old holdings" : "Remaining"}</span><strong>{migration ? plan.before.lands.length : count(plan.remaining.population)}</strong><small>{migration ? "lands relinquished" : `souls · ${plan.remaining.lands.length} lands`}</small></div>
      <ArrowRight size={18} aria-hidden="true" />
      <div><span>{territorial ? "New people" : "Arriving"}</span><strong>{count(plan.arriving.population)}</strong><small>souls · {plan.arriving.lands.length} {plan.arriving.lands.length === 1 ? "land" : "lands"}</small></div>
    </div>
    <p className="settlement-total">{count(plan.before.population)} souls accounted for · {territorial ? "no journey" : "all inhabited lands included"}</p>
    {plan.routes.length > 0 ? <details className="settlement-routes" open={plan.routes.length <= 2}>
      <summary>{plan.routes.length} {plan.routes.length === 1 ? "journey" : "journeys"} · up to {count(Math.max(...plan.routes.map((r) => r.effort)))} effort-km</summary>
      <ul>{plan.routes.map((r) => <li key={r.from}>
        <strong>{landTitle(overview, map, r.from)}</strong><span>{count(r.population)} souls · {r.by_sea ? "by sea" : "over land"} · {count(r.effort)} effort-km</span>
      </li>)}</ul>
      <p className="muted small">Travel effort includes terrain and embarkation. Journeys follow the roads and waters they can travel.</p>
    </details> : null}
    {plan.inhabitants.length > 0 ? <p className="small">Already living here: {plan.inhabitants.map(([c, n]) => `${overview.communities[c]?.name ?? "another people"} (${count(n)})`).join(", ")}.</p> : null}
    {plan.falling_states.length > 0 ? <p className="telling-note">Capital lost: {plan.falling_states.map((s) => overview.states[s].name).join(", ")}. The state falls with this decision.</p> : null}
  </div>;
}

export function SettlementDesk({ draft, closing, preview, error, overview, map, catalog, latest, onChange, onCancel, onCommit, returnFocus }: {
  draft: SettlementDraft;
  closing: boolean;
  preview: SettlementPreview | null;
  error: string | null;
  overview: Overview;
  map: WorldMap;
  catalog: Catalog;
  latest: Overview;
  onChange: (next: SettlementDraft) => void;
  onCancel: () => void;
  onCommit: (choice: SettlementChoice, preview: SettlementPreview) => void;
  returnFocus: HTMLElement | null;
}) {
  const close = useRef<HTMLButtonElement>(null);
  const opener = useRef<HTMLElement | null>(null);
  const [naming, setNaming] = useState<Naming | null>(null);
  const [intensity, setIntensity] = useState(0.5);
  const people = overview.communities[draft.community];
  const plan = preview?.plan;
  const reason = error ?? preview?.reason;
  const changedPast = JSON.stringify(draft.point) !== JSON.stringify(latest.tellings.find((t) => t.id === latest.telling)?.tip);
  const options = (preview?.options ?? []).filter((o) => draft.intent !== "partition" || people.lands.includes(o.region)).toSorted((a, b) => Number(a.reason !== null) - Number(b.reason !== null) || a.region - b.region);
  const eligible = options.filter((o) => o.reason === null).length;
  useLayoutEffect(() => {
    closeClosingDialogs();
    opener.current = returnFocus ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    close.current?.focus({ preventScroll: true });
    const dismiss = (e: KeyboardEvent) => { if (e.key === "Escape") { e.preventDefault(); onCancel(); } };
    window.addEventListener("keydown", dismiss);
    return () => {
      window.removeEventListener("keydown", dismiss);
      queueMicrotask(() => {
        if (opener.current?.isConnected && document.activeElement === document.body) opener.current.focus({ preventScroll: true });
      });
    };
  }, []);
  useLayoutEffect(() => {
    if (closing && opener.current?.isConnected) opener.current.focus({ preventScroll: true });
    else if (!closing) close.current?.focus({ preventScroll: true });
  }, [closing]);
  return <aside className="settlement-desk" data-closing={closing || undefined} inert={closing} aria-hidden={closing || undefined} aria-label="Settlement account">
    <header><span>A choice of homeland · year {overview.generation * YEARS}</span><button ref={close} type="button" className="icon" aria-label="Close settlement account" onClick={onCancel}><X size={18} /></button></header>
    <div className="settlement-pages">
      <h2>{people.name}</h2>
      <p className="settlement-byline">{count(people.size)} souls · {people.lands.length} {people.lands.length === 1 ? "holding" : "holdings"}</p>
      <fieldset className="settlement-intents"><legend>What do they do?</legend>
        {INTENTS.map(({ id, icon: Icon, name, detail }) => <label key={id} className={draft.intent === id ? "selected" : ""}>
          <input type="radio" name="settlement-intent" value={id} checked={draft.intent === id} onChange={() => onChange({ ...draft, intent: id, destination: null })} />
          <Icon size={18} aria-hidden="true" /><span><strong>{name}</strong><small>{detail}</small></span>
        </label>)}
      </fieldset>
      {draft.intent === "settlers" ? <label className="settlement-share">The departing share <strong>{Math.round(draft.share * 100)}%</strong>
        <input type="range" min={5} max={95} step={5} value={draft.share * 100} onChange={(e) => onChange({ ...draft, share: Number(e.target.value) / 100 })} />
      </label> : null}
      <label className="settlement-destination">{draft.intent === "partition" ? "The new heart" : "Where they settle"}
        <select value={draft.destination ?? ""} onChange={(e) => onChange({ ...draft, destination: e.target.value === "" ? null : Number(e.target.value) })}>
          <option value="">Choose on the chart or here…</option>
          {options.map((o) => <option key={o.region} value={o.region}>{landTitle(overview, map, o.region)}{o.reason ? " · unavailable" : ""}</option>)}
        </select>
      </label>
      <p className="settlement-key"><i className="eligible" />{eligible} possible {draft.intent === "partition" ? "hearts" : "destinations"}<i className="remaining" />Remain<i className="arriving" />{draft.intent === "partition" ? "Part" : "Arrive"}</p>
      {draft.destination === null ? <p className="settlement-invitation">{eligible ? "Choose a marked land to read the account before deciding." : draft.intent === "partition" ? "A territorial division needs at least two connected holdings. Sending settlers can begin a new people elsewhere." : "No land can receive this group from all its inhabited places. Try a smaller share, or explore seafaring and nearer lands."}</p> : null}
      {reason ? <p role="status" className="notice settlement-refusal">{reason}</p> : null}
      {plan ? <>
        <h3>{landTitle(overview, map, plan.choice.destination)}</h3>
        <SettlementAccount plan={plan} overview={overview} map={map} />
        <ul className="settlement-notes">{plan.notes.map((note) => <li key={note}>{note}</li>)}</ul>
        <details className="settlement-details"><summary>Name and ties</summary>
          {draft.intent !== "migration" ? <label>The new people calls itself<NamingSelect catalog={catalog} value={naming} onChange={setNaming} daughter parent={people.name} /></label> : null}
          <label>Contact with kin and new neighbours · {Math.round(intensity * 100)}%<input type="range" min={0} max={1} step={0.1} value={intensity} onChange={(e) => setIntensity(Number(e.target.value))} /></label>
          <p className="muted small">Ties can form only where travel permits. Names are coined from their own language.</p>
        </details>
      </> : null}
      {changedPast ? <p className="telling-note">This rewrites from year {overview.generation * YEARS}, before the choice in view. The present history through year {latest.latest * YEARS} remains another telling.</p> : null}
    </div>
    <footer><button type="button" onClick={onCancel}>Leave undecided</button><button type="button" className="primary" disabled={!plan || !preview || !!error} onClick={() => { if (plan && preview) onCommit({ ...plan.choice, naming, intensity }, preview); }}>
      {draft.intent === "partition" ? "Divide these lands" : draft.intent === "settlers" ? "Found the settlement" : "Move the people"}
    </button></footer>
  </aside>;
}
