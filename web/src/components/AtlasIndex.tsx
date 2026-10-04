import { useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { ArrowUpRight, Earth, Hammer, Landmark, Languages, MapPin, Search, Sparkles, Users, Waves, type LucideIcon } from "lucide-react";
import type { LakeNamesView, Overview, RiverNamesView, WorldMap } from "../model";
import { YEARS } from "../model";
import { hue, TERRAIN_NAME, unnamedName } from "../lore";
import type { Focus } from "./Pedia";
import { searchText } from "../history";
import { Modal } from "./Modal";

const KINDS = ["All", "Peoples", "Languages", "Lands", "States", "Faiths", "Crafts"] as const;
type Kind = typeof KINDS[number];
interface Entry {
  key: string;
  kind: Kind;
  name: string;
  detail: string;
  aliases?: string;
  icon: LucideIcon;
  focus: Focus;
  tone?: string;
  past?: boolean;
}

export function AtlasIndex({ open, overview, map, riverNames, lakeNames, onClose, go }: {
  open: boolean;
  overview: Overview;
  map: WorldMap;
  riverNames: RiverNamesView[];
  lakeNames: LakeNamesView[];
  onClose: () => void;
  go: (focus: Focus) => void;
}) {
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<Kind>("All");
  const [past, setPast] = useState(false);
  const results = useRef<HTMLUListElement>(null);
  const entries = useMemo<Entry[]>(() => [
    ...overview.continents.map((c): Entry => ({
      key: `continent-${c.landmass}`, kind: "Lands", name: c.name?.name ?? unnamedName(map.landmasses[c.landmass].kind),
      detail: `Continent · ${map.landmasses[c.landmass].regions.length} lands`, aliases: c.name?.meaning,
      icon: Earth, focus: { kind: "continent", landmass: c.landmass },
    })),
    ...overview.communities.map((c): Entry => ({
      key: `people-${c.id}`, kind: "Peoples", name: c.name,
      detail: c.ended === null ? `${Math.round(c.size).toLocaleString()} souls · ${overview.varieties[c.variety].name}` : `Ended in year ${c.ended * YEARS}`,
      aliases: [c.meaning, c.once, ...c.exonyms.map((n) => n.name)].join(" "),
      icon: Users, tone: hue(overview.varieties[c.variety].family), past: c.ended !== null,
      focus: { kind: "people", id: c.id },
    })),
    ...overview.varieties.map((v): Entry => ({
      key: `language-${v.id}`, kind: "Languages", name: v.name,
      detail: `${v.spoken ? "Spoken" : v.sacredOf !== null ? "Sacred" : v.classicalOf !== null ? "Classical" : "No longer spoken"} · ${v.words.toLocaleString()} words`,
      aliases: v.meaning, icon: Languages, tone: hue(v.family), past: !v.spoken,
      focus: { kind: "language", variety: v.id },
    })),
    ...overview.states.map((s): Entry => ({
      key: `state-${s.id}`, kind: "States", name: s.name,
      detail: s.fell === null ? `Standing · ${s.lands.length} lands` : `Fell in year ${s.fell * YEARS}`,
      aliases: s.meaning, icon: Landmark, past: s.fell !== null, focus: { kind: "state", id: s.id },
    })),
    ...overview.religions.map((r): Entry => ({
      key: `faith-${r.id}`, kind: "Faiths", name: r.name, detail: `${r.followers.length} ${r.followers.length === 1 ? "people" : "peoples"} · ${r.meaning}`,
      icon: Sparkles, focus: { kind: "religion", id: r.id },
    })),
    ...overview.crafts.map((c): Entry => ({
      key: `craft-${c.id}`, kind: "Crafts", name: c.name, detail: `Held by ${c.holders.length} peoples`,
      icon: Hammer, focus: { kind: "craft", id: c.id },
    })),
    ...overview.places.flatMap((p): Entry[] => {
      const name = p.names.at(-1);
      return name ? [{
        key: `land-${p.region}`, kind: "Lands", name: name.spelled,
        detail: `${TERRAIN_NAME[map.regions[p.region].terrain]} · ${name.meaning}`,
        aliases: [...p.names.map((n) => n.spelled), ...p.exonyms.map((n) => n.spelled)].join(" "),
        icon: MapPin, focus: { kind: "land", region: p.region },
      }] : [];
    }),
    ...riverNames.map((view): Entry => ({
      key: `river-${view.river}`, kind: "Lands", name: view.names.at(-1)?.spelled ?? unnamedName("river"),
      detail: `River · ${map.rivers[view.river].course.length} lands`,
      aliases: [...view.names.map((n) => `${n.spelled} ${n.meaning}`), ...view.exonyms.map((n) => n.spelled)].join(" "),
      icon: Waves, focus: { kind: "river", id: view.river },
    })),
    ...lakeNames.map((view): Entry => ({
      key: `lake-${view.lake}`, kind: "Lands", name: view.names.at(-1)?.spelled ?? unnamedName("lake"),
      detail: `Lake · ${map.lakes[view.lake].regions.length} lands`,
      aliases: [...view.names.map((n) => `${n.spelled} ${n.meaning}`), ...view.exonyms.map((n) => n.spelled)].join(" "),
      icon: Waves, focus: { kind: "lake", id: view.lake },
    })),
  ], [overview, map, riverNames, lakeNames]);
  const terms = searchText(query.trim()).split(/\s+/).filter(Boolean);
  const matches = entries.filter((entry) =>
    (kind === "All" || entry.kind === kind) && (past || !entry.past) &&
    terms.every((term) => searchText(`${entry.name} ${entry.detail} ${entry.aliases ?? ""}`).includes(term)),
  );
  const shown = matches.slice(0, 100);
  const navigate = (focus: Focus) => {
    // Release modal inertness before keyboard navigation focuses the card.
    flushSync(onClose);
    go(focus);
  };
  return (
    <Modal open={open} title="Find in this world" onClose={onClose} initialFocus="input[type=search]" wide>
      <p className="index-intro">Find a people, follow a language, or set out for a distant land.</p>
      <div className="index-search">
        <Search size={20} aria-hidden="true" />
        <input autoFocus type="search" aria-label="Search this world" placeholder="A name, an older name, or a meaning…"
          value={query} onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") { e.preventDefault(); results.current?.querySelector("button")?.focus(); }
            if (e.key === "Enter" && matches.length === 1) navigate(matches[0].focus);
          }} />
      </div>
      <div className="index-kinds" aria-label="Kinds of entry">
        {KINDS.map((k) => <button key={k} type="button" aria-pressed={kind === k} onClick={() => setKind(k)}>{k}</button>)}
      </div>
      <div className="index-count">
        <span role="status">{matches.length} {matches.length === 1 ? "entry" : "entries"} in year {overview.generation * YEARS}</span>
        <label><input type="checkbox" checked={past} onChange={(e) => setPast(e.target.checked)} /> Include the past</label>
      </div>
      {shown.length === 0 ? <p className="index-empty">No entries found. Try part of a name or include the past.</p> : null}
      <ul className="index-results" ref={results} onKeyDown={(e) => {
        if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
        const buttons = [...e.currentTarget.querySelectorAll("button")];
        const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
        const next = e.key === "Home" ? 0 : e.key === "End" ? buttons.length - 1 : current + (e.key === "ArrowDown" ? 1 : -1);
        e.preventDefault(); buttons[Math.max(0, Math.min(buttons.length - 1, next))]?.focus();
      }}>
        {shown.map(({ key, name, kind, detail, icon: Icon, focus, tone }) => (
          <li key={key}><button type="button" onClick={() => navigate(focus)}>
            <Icon size={19} aria-hidden="true" style={{ color: tone }} />
            <span><strong>{name}</strong><small>{detail}</small></span>
            <span className="index-kind">{kind}</span><ArrowUpRight size={16} aria-hidden="true" />
          </button></li>
        ))}
      </ul>
      {matches.length > shown.length ? <p className="muted small">Showing the first {shown.length}. Narrow the search to find more.</p> : null}
    </Modal>
  );
}
