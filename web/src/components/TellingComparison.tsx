import { use, useDeferredValue, useMemo, useState } from "react";
import type { Community, Engine, Overview, Read, SpeechDelta, WorldMap } from "../model";
import { YEARS } from "../model";
import { attempt } from "../engine";
import { LIVELIHOOD_NAME } from "../lore";
import { Modal } from "./Modal";
import { MapView, type MapCamera } from "./MapView";
import { Specimen } from "./Specimen";
import { FamilyTree } from "./FamilyTree";
import type { Focus } from "./Pedia";
import { Pending, useAnswer } from "./Pending";

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
  // Both tellings are read on the engine's thread; while another year or
  // people is read, the comparison keeps showing the last one.
  const reading = useDeferredValue(useMemo(() => ({ rightId, generation, person }), [rightId, generation, person]));
  const result = use(attempt(engine.compare(leftId, reading.rightId, reading.generation)));
  const speechRead = engine.speechDelta(leftId, reading.rightId, reading.generation);
  const speech = useAnswer(attempt(speechRead))?.value;
  const readings = [engine.read(leftId), engine.read(reading.rightId)] as const;
  const comparison = result.value;
  const shared = comparison?.sharedPeoples ?? [];
  const selected = shared.includes(reading.person) ? reading.person : shared[0];
  const sides = comparison ? [comparison.left, comparison.right] : [];
  // Ask for both sides' views before reading any, so they arrive together.
  const asked = sides.map((side, index) => ({
    lexicon: side.communities[selected] ? readings[index].lexicon(reading.generation, side.communities[selected].variety) : null,
    climate: readings[index].climate(reading.generation),
    names: readings[index].rivers(reading.generation),
  }));
  const lexicons = asked.map((side) => side.lexicon ? use(side.lexicon) : []);
  const geography = asked.map((side) => ({ climate: use(side.climate), names: use(side.names) }));
  const concepts = [...new Map(lexicons.flat().map((w) => [w.concept, w.gloss])).entries()].sort((a, b) => a[1].localeCompare(b[1]));
  const read = (side: Overview, focus?: Focus) => onRead(side.telling, reading.generation, focus);
  const title = (side: Overview) => side.tellings.find((t) => t.id === side.telling)!.name;
  const state = (side: Overview, c: Community) => side.states.find((s) => s.fell === null && (s.rulers === c.id || s.members.some((m) => m.community === c.id && m.left === null)))?.name ?? "Independent";
  const heading = speech
    ? `Diverged in year ${speech.diverged * YEARS}: ${speech.decision?.text.replace(/\*/g, "") ?? "no decision recorded"}`
    : comparison ? `Diverged in year ${comparison.diverged * YEARS}` : "Two tellings, one year";
  return <Modal open={open} wide title={heading} onClose={onClose}>
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
        <section className="comparison-investigation comparison-speech">
          <h3>What changed in speech</h3>
          <Pending reading={speechRead} label="Reading what changed in speech…">
            <SpeechChanges reading={speechRead} left={comparison.left} right={comparison.right} onRead={read} />
          </Pending>
        </section>
        <div className="comparison-maps">
          {sides.map((side, index) => <section key={side.telling} aria-label={`${title(side)} chart`}>
            <h3>{title(side)}</h3>
            <div className="comparison-map"><MapView map={map} overview={side} generation={reading.generation} tint={{ kind: "peoples" }}
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

function SpeechChanges({ reading, left, right, onRead }: {
  reading: Read<SpeechDelta>;
  left: Overview;
  right: Overview;
  onRead: (side: Overview, focus?: Focus) => void;
}) {
  const delta = use(reading);
  const title = (side: Overview) => side.tellings.find((telling) => telling.id === side.telling)!.name;
  const languages = delta.languages.filter((language) => {
    const rightLoans = new Map(language.loansRight.map((loan) => [loan.donor, loan.count]));
    return language.onlyLeft.length > 0 || language.onlyRight.length > 0 || language.changedWords.length > 0
      || language.left.descendants !== language.right.descendants || language.left.alive !== language.right.alive
      || language.loansLeft.length !== language.loansRight.length
      || language.loansLeft.some((loan) => rightLoans.get(loan.donor) !== loan.count);
  });
  if (languages.length === 0) return <p>The tellings speak alike so far.</p>;
  return <>
    <p className="muted small">Languages shared before year {delta.diverged * YEARS}, read in year {delta.generation * YEARS}. Later descendants are counted in their own telling, not paired by their numbers.</p>
    {languages.map((language) => <article className="comparison-speech-language" key={language.variety}>
      <h4><button type="button" className="link word" onClick={() => onRead(left, { kind: "language", variety: language.variety })}>{left.varieties[language.variety].name}</button>
        {left.varieties[language.variety].name !== right.varieties[language.variety].name ? <> / <button type="button" className="link word" onClick={() => onRead(right, { kind: "language", variety: language.variety })}>{right.varieties[language.variety].name}</button></> : null}
      </h4>
      <div className="comparison-accounts">
        {([left, right] as const).map((side, index) => {
          const speech = index === 0 ? language.left : language.right;
          const only = index === 0 ? language.onlyLeft : language.onlyRight;
          const loans = index === 0 ? language.loansLeft : language.loansRight;
          return <article key={side.telling}>
            <p className="comparison-caption">{title(side)}</p>
            <p className="small"><button type="button" className="link word" onClick={() => onRead(side, { kind: "language", variety: language.variety })}>{side.varieties[language.variety].name}</button>
              {" "}is {speech.alive ? "spoken" : "silent"} · {speech.descendants} descendant {speech.descendants === 1 ? "language" : "languages"}.</p>
            <h5>Sound laws only here</h5>
            {only.length ? <ul className="comparison-speech-laws">{only.map((id) => {
              const law = speech.laws.find((entry) => entry.id === id)!;
              return <li key={id}><span className="gen">{law.generation * YEARS}</span>{" "}
                <button type="button" className="link" onClick={() => onRead(side, { kind: "law", id })}>{law.label}</button>
              </li>;
            })}</ul> : <p className="small muted">No sound law only in this telling.</p>}
            <h5>Loans since they parted</h5>
            {loans.length ? <ul className="comparison-speech-loans">{loans.map((loan) => <li key={loan.donor}>
              {loan.count} {loan.count === 1 ? "word" : "words"} from{" "}
              <button type="button" className="link word" onClick={() => onRead(side, { kind: "language", variety: loan.donor })}>{side.varieties[loan.donor].name}</button>
            </li>)}</ul> : <p className="small muted">No loans recorded.</p>}
          </article>;
        })}
      </div>
      <h5>Changed specimen words</h5>
      {language.changedWords.length ? <ul className="comparison-speech-words">
        {language.changedWords.map((word) => <li key={word.concept}>
          <span className="comparison-speech-gloss">{word.gloss}</span>
          {word.left ? <span className="comparison-speech-form">
            <button type="button" className="link word" title={`Read in ${title(left)}`} onClick={() => onRead(left, { kind: "word", variety: language.variety, concept: word.concept })}>{word.left.spelled}</button>
            <span className="ipa">/{word.left.ipa}/</span>
          </span> : <span className="muted">No form</span>}
          <span aria-hidden="true">→</span><span className="sr-only">to</span>
          {word.right ? <span className="comparison-speech-form">
            <button type="button" className="link word" title={`Read in ${title(right)}`} onClick={() => onRead(right, { kind: "word", variety: language.variety, concept: word.concept })}>{word.right.spelled}</button>
            <span className="ipa">/{word.right.ipa}/</span>
          </span> : <span className="muted">No form</span>}
        </li>)}
      </ul> : <p className="small muted">The specimen words have not changed between these tellings.</p>}
    </article>)}
  </>;
}
