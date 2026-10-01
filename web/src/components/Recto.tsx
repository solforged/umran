import { useState } from "react";
import type { Engine, Overview } from "../model";
import { YEARS } from "../model";
import { bond } from "../words";
import { Dictionary } from "./Dictionary";
import { WordGloss } from "./WordGloss";

type Leaf = "words" | "sounds" | "kin";

/// The right page: the chosen people's tongue, as a dictionary with notes
/// in the margin, its sounds, and its kin.
export function Recto({
  engine,
  version,
  generation,
  overview,
  selected,
  concept,
  onConcept,
  onScrub,
  onOpenVariety,
}: {
  engine: Engine;
  version: number;
  generation: number;
  overview: Overview;
  selected: number;
  concept: string | null;
  onConcept: (concept: string) => void;
  onScrub: (generation: number) => void;
  onOpenVariety: (variety: number) => void;
}) {
  const [leaf, setLeaf] = useState<Leaf>("words");
  const community = overview.communities[selected];
  const variety = overview.varieties[community.variety];
  const name = (id: number) => overview.communities[id]?.name ?? "?";
  const contacts = overview.contacts.filter((c) => c.a === selected || c.b === selected);
  const hand = `hand-${variety.family % 5}`;

  return (
    <section className="page recto" aria-label={`The ${variety.name} tongue`}>
      <header className="recto-head">
        <h2 className={hand}>{variety.name}</h2>
        <p className="muted">
          {variety.meaning ? `“${variety.meaning}”, ` : ""}the tongue of the <span className={hand}>{community.name}</span>{" "}
          (“{community.meaning}”{community.once ? `; once ${community.once}` : ""})
          {community.exonyms.length > 0
            ? `, whom ${community.exonyms.map((e) => `the ${name(e.by)} call ${e.name}`).join(", ")}`
            : ""}
          .
        </p>
        {contacts.length > 0 ? (
          <p className="muted small">
            They{" "}
            {contacts.map((c, i) => (
              <span key={i}>
                {i === 0 ? "" : i === contacts.length - 1 ? " and " : ", "}
                <span title={`${c.kind}, ${Math.round(c.intensity * 100)}%`}>
                  {bond(c.kind, c.intensity)} the {name(c.a === selected ? c.b : c.a)}
                </span>
              </span>
            ))}
            .
          </p>
        ) : null}
        <div className="tabs" role="tablist">
          {(
            [
              ["words", "Words"],
              ["sounds", "Sounds"],
              ["kin", "Kin"],
            ] as const
          ).map(([id, label]) => (
            <button key={id} type="button" role="tab" aria-selected={leaf === id} onClick={() => setLeaf(id)}>
              {label}
            </button>
          ))}
        </div>
      </header>

      {leaf === "words" ? (
        <div className="with-margin" role="tabpanel">
          <Dictionary
            engine={engine}
            version={version}
            generation={generation}
            variety={variety.id}
            concept={concept}
            onConcept={onConcept}
          />
          <aside className="margin" aria-label="Note on the chosen word">
            <WordGloss
              engine={engine}
              version={version}
              generation={generation}
              variety={variety.id}
              concept={concept}
              onScrub={onScrub}
              onOpenVariety={onOpenVariety}
            />
          </aside>
        </div>
      ) : leaf === "sounds" ? (
        <div role="tabpanel" className="leaf">
          <p className="muted">Sound preferences: {variety.profile}</p>
          <h3>Consonants</h3>
          <p className="segments">{variety.consonants.join(" ")}</p>
          <h3>Vowels</h3>
          <p className="segments">{variety.vowels.join(" ")}</p>
          <p className="muted">
            Sound change never wears a word shorter than {variety.minimalWord}; shorter words are felt as worn and
            renewed.
          </p>
          <h3>Word building</h3>
          <p className="muted">Builds words with {variety.wordBuilding}.</p>
          <dl className="facts builders">
            {variety.builders.map((b) => (
              <div key={b.relation} className="builder">
                <dt>{b.relation}</dt>
                <dd className="ipa">{b.shape}</dd>
              </div>
            ))}
          </dl>
          <h3>Sound laws</h3>
          {variety.laws.length === 0 ? (
            <p className="muted">None yet. Let some years pass.</p>
          ) : (
            <ol className="history">
              {variety.laws.map((law, i) => (
                <li key={i}>
                  <button type="button" className="gen" onClick={() => onScrub(law.generation)}>
                    {law.generation * YEARS}
                  </button>
                  {law.label}
                </li>
              ))}
            </ol>
          )}
        </div>
      ) : (
        <Kin overview={overview} variety={variety.id} onOpenVariety={onOpenVariety} />
      )}
    </section>
  );
}

/// How the chosen tongue relates to the others: who understands it, and
/// the family trees.
function Kin({
  overview,
  variety,
  onOpenVariety,
}: {
  overview: Overview;
  variety: number;
  onOpenVariety: (variety: number) => void;
}) {
  const closest = overview.intelligibility
    .filter((p) => p.a === variety || p.b === variety)
    .map((p) => ({ variety: p.a === variety ? p.b : p.a, score: p.score }))
    .sort((x, y) => y.score - x.score);
  const families = new Map<number, typeof overview.varieties>();
  for (const v of overview.varieties) {
    families.set(v.family, [...(families.get(v.family) ?? []), v]);
  }
  const open = (id: number, label: string) => (
    <button type="button" className="link" onClick={() => onOpenVariety(id)} disabled={!overview.varieties[id].spoken}>
      {label}
    </button>
  );

  return (
    <div role="tabpanel" className="leaf">
      <h3>Closest tongues</h3>
      {closest.length === 0 ? (
        <p className="muted">No other tongue is spoken yet.</p>
      ) : (
        <ul className="bars">
          {closest.slice(0, 6).map((p) => (
            <li key={p.variety}>
              {open(p.variety, overview.varieties[p.variety].name)}
              <meter min={0} max={1} value={p.score} title={`intelligibility ${p.score.toFixed(2)}`} />
            </li>
          ))}
        </ul>
      )}
      <h3>Families</h3>
      {[...families.values()].map((members) => (
        <ul key={members[0].id} className="tree">
          {members.map((v) => {
            let depth = 0;
            for (let p = v.parent; p !== null; p = overview.varieties[p].parent) depth += 1;
            return (
              <li
                key={v.id}
                style={{ paddingLeft: `${depth * 0.9}rem` }}
                className={`${v.spoken ? "" : "extinct"}${v.id === variety ? " chosen" : ""}`}
              >
                <span className={`hand-${v.family % 5}`}>{open(v.id, v.name)}</span>
                {v.forkedAt !== null ? <span className="muted"> · from year {v.forkedAt * YEARS}</span> : null}
                {v.spoken ? null : <span className="muted"> · no longer spoken</span>}
              </li>
            );
          })}
        </ul>
      ))}
    </div>
  );
}
