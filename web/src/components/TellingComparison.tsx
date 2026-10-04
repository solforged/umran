import { useMemo, useState } from "react";
import type { Community, Engine, Overview, WorldMap } from "../model";
import { YEARS } from "../model";
import { message } from "../engine";
import { LIVELIHOOD_NAME } from "../lore";
import { Modal } from "./Modal";
import { MapView, type MapCamera } from "./MapView";
import { Specimen } from "./Specimen";
import { FamilyTree } from "./FamilyTree";
import type { Focus } from "./Pedia";

const souls = (n: number) => Math.round(n).toLocaleString();

export function TellingComparison({ open, engine, overview, map, other, onClose, onRead, onContinue }: {
  open: boolean;
  engine: Engine;
  overview: Overview;
  map: WorldMap;
  other: number;
  onClose: () => void;
  onRead: (telling: number, generation?: number, focus?: Focus) => void;
  onContinue: (reading: Overview) => void;
}) {
  const [rightId, setRightId] = useState(other);
  const leftId = overview.telling;
  const leftTitle = overview.tellings.find((t) => t.id === leftId)!;
  const rightTitle = overview.tellings.find((t) => t.id === rightId)!;
  const through = Math.min(leftTitle.latest, rightTitle.latest);
  const [year, setYear] = useState(Math.min(overview.generation, through));
  const generation = Math.min(year, through);
  const [person, setPerson] = useState(0);
  const [concept, setConcept] = useState("water");
  const [camera, setCamera] = useState<MapCamera>([0, 0, map.width, map.height]);
  const result = useMemo(() => {
    try { return { value: engine.compare(leftId, rightId, generation), error: null }; }
    catch (e) { return { value: null, error: message(e) }; }
  }, [engine, overview.mutation, leftId, rightId, generation]);
  const readings = useMemo(() => [engine.read(leftId), engine.read(rightId)] as const, [engine, leftId, rightId]);
  const comparison = result.value;
  const shared = comparison?.sharedPeoples ?? [];
  const selected = shared.includes(person) ? person : shared[0];
  const sides = comparison ? [comparison.left, comparison.right] : [];
  const lexicons = useMemo(() => sides.map((side, index) => {
    const c = side.communities[selected];
    return c && readings ? readings[index].lexicon(generation, c.variety) : [];
  }), [comparison, selected, readings, generation]);
  const geography = useMemo(() => sides.map((_, index) => ({
    climate: readings[index].climate(generation),
    names: map.rivers.map((river) => readings[index].river(generation, river.id)),
  })), [comparison, readings, generation, map]);
  const concepts = [...new Map(lexicons.flat().map((w) => [w.concept, w.gloss])).entries()].sort((a, b) => a[1].localeCompare(b[1]));
  const read = (side: Overview, focus?: Focus) => onRead(side.telling, generation, focus);
  const title = (side: Overview) => side.tellings.find((t) => t.id === side.telling)!.name;
  const state = (side: Overview, c: Community) => side.states.find((s) => s.fell === null && (s.rulers === c.id || s.members.some((m) => m.community === c.id && m.left === null)))?.name ?? "Independent";
  return <Modal open={open} wide title="Two tellings, one year" onClose={onClose}>
    <div className="telling-comparison">
      <div className="comparison-tools">
        <p><strong>{leftTitle.name}</strong><span className="muted"> compared with</span></p>
        <label>Another telling<select value={rightId} onChange={(e) => setRightId(Number(e.target.value))}>
          {overview.tellings.filter((t) => t.id !== leftId).map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
        </select></label>
        <label className="comparison-year">Year {generation * YEARS}<input aria-label="Comparison year" type="range" min={0} max={through} value={generation} disabled={through === 0} onChange={(e) => setYear(Number(e.target.value))} /></label>
      </div>
      <p className="muted small">Both tellings are read at the same year. The shorter reaches year {through * YEARS}; continuing either telling is a separate decision.</p>
      {result.error ? <p className="notice error" role="alert">{result.error} Both saved tellings are still kept.</p> : null}
      {comparison ? <>
        <div className="comparison-maps">
          {sides.map((side, index) => <section key={side.telling} aria-label={`${title(side)} chart`}>
            <h3>{title(side)}</h3>
            <div className="comparison-map"><MapView map={map} overview={side} generation={generation} tint={{ kind: "peoples" }}
              climate={geography[index].climate} riverNames={geography[index].names}
              selectedVariety={side.communities[selected]?.variety}
              names routes={false} contacts={false} states zoomable camera={camera} onCamera={setCamera}
              chosen={new Set(selected === undefined ? [] : [selected])} lands={new Set(side.communities[selected]?.lands ?? [])}
              onPeople={(id) => { if (shared.includes(id)) setPerson(id); else read(side, { kind: "people", id }); }}
              onLand={(region) => read(side, { kind: "land", region })}
              onRiver={(id) => read(side, { kind: "river", id })} /></div>
            <p className="small">{side.communities.filter((c) => c.ended === null).length} peoples · {side.varieties.filter((v) => v.spoken).length} spoken languages</p>
            <div className="row"><button type="button" onClick={() => read(side)}>Read this telling</button>
              <button type="button" onClick={() => onContinue(side)}>Continue 25 years from here</button></div>
          </section>)}
        </div>
        <section className="comparison-investigation">
          <h3>A people through both tellings</h3>
          <p className="muted small">These peoples were already present in year {comparison.diverged * YEARS}. Later descendants belong to their own telling.</p>
          {shared.length ? <label>Follow a shared people<select value={selected} onChange={(e) => setPerson(Number(e.target.value))}>
            {shared.map((id) => <option key={id} value={id}>{comparison.left.communities[id].name}{comparison.left.communities[id].name !== comparison.right.communities[id].name ? ` / ${comparison.right.communities[id].name}` : ""}</option>)}
          </select></label> : <p>No people had yet been founded when these tellings separated.</p>}
          <div className="comparison-accounts">
            {sides.map((side) => {
              const c = side.communities[selected];
              if (!c) return <div key={side.telling} />;
              const v = side.varieties[c.variety];
              const descendants = side.communities.filter((child) => !shared.includes(child.id) && child.parents.includes(c.id));
              return <article key={side.telling}>
                <p className="comparison-caption">{title(side)}</p>
                <h3><button type="button" className="link word" onClick={() => read(side, { kind: "people", id: c.id })}>{c.name}</button></h3>
                <dl className="comparison-facts"><dt>People</dt><dd>{c.ended === null ? `${souls(c.size)} souls` : `Ended in year ${c.ended * YEARS}`}</dd>
                  <dt>Holdings</dt><dd>{c.ended === null ? c.lands.length : "None now"}</dd>
                  <dt>Livelihood</dt><dd>{LIVELIHOOD_NAME[c.livelihood]}</dd>
                  <dt>Rule</dt><dd>{state(side, c)}</dd>
                  <dt>Faith</dt><dd>{c.faith === null ? "Their own gods" : side.religions[c.faith].name}</dd>
                  <dt>Speech</dt><dd><button type="button" className="link word" onClick={() => read(side, { kind: "language", variety: c.variety })}>{v.name}</button></dd></dl>
                <Specimen words={v.specimen} onWord={(word) => setConcept(word)} />
                {descendants.length > 0 ? <p className="small">Descendants in this telling: {descendants.map((d, i) => <span key={d.id}>{i > 0 ? ", " : ""}<button type="button" className="link word" onClick={() => read(side, { kind: "people", id: d.id })}>{d.name}</button></span>)}.</p> : null}
                <details><summary>Family of their language</summary><FamilyTree overview={side} family={v.family} chosen={v.id} onOpen={(variety) => read(side, { kind: "language", variety })} /></details>
              </article>;
            })}
          </div>
          {concepts.length ? <section className="comparison-words"><label>Read a meaning in both languages<select value={concept} onChange={(e) => setConcept(e.target.value)}>
            {concepts.map(([id, gloss]) => <option value={id} key={id}>{gloss}</option>)}
          </select></label>
            <div className="comparison-accounts">{sides.map((side, index) => {
              const word = lexicons[index].find((w) => w.concept === concept);
              return <article key={side.telling}><p className="comparison-caption">{title(side)}</p>
                {word ? <><button type="button" className="link word comparison-form" onClick={() => read(side, { kind: "word", variety: side.communities[selected].variety, concept })}>{word.spelled}</button>
                  <p className="ipa">/{word.ipa}/</p><p className="muted small">{word.origin.kind}{word.origin.from ? ` from ${word.origin.from}` : ""} · recorded in year {word.origin.generation * YEARS}</p></> : <p>No word recorded for this meaning.</p>}
              </article>;
            })}</div>
          </section> : null}
        </section>
      </> : null}
    </div>
  </Modal>;
}
