import { useMemo, useState } from "react";
import { presetDesign, preview } from "../engine";
import type { Catalog, Naming } from "../model";
import { Designer, randomSeed, type Founding } from "./Designer";
import { NamingSelect } from "./NamingSelect";

/// Words the title page shows: ordinary nouns and verbs, which show how
/// the language sounds better than its nursery words do.
const SAMPLE = ["sun", "moon", "water", "fire", "stone", "river", "mountain", "bird", "heart"];

function pick<T>(list: T[]): T {
  return list[Math.floor(Math.random() * list.length)];
}

function anyNaming(catalog: Catalog): Naming {
  return pick<Naming>([
    { kind: "people" },
    { kind: "speakers" },
    ...catalog.namePlaces.map((place) => ({ kind: "place" as const, place })),
    ...catalog.nameEpithets.map((epithet) => ({ kind: "epithet" as const, epithet })),
  ]);
}

/// Begins a book in a few choices: a sound to start from and what the
/// people call themselves, with their names and words shown at once. The
/// full sound chart waits behind "Adjust the sounds".
export function TitlePage({
  catalog,
  onBegin,
  onCancel,
}: {
  catalog: Catalog;
  onBegin: (founding: Founding) => void;
  onCancel?: () => void;
}) {
  const [preset, setPreset] = useState("typical");
  const [seed, setSeed] = useState(() => randomSeed());
  const [worldSeed] = useState(() => randomSeed());
  const [naming, setNaming] = useState<Naming>({ kind: "people" });
  const [adjusting, setAdjusting] = useState(false);
  const design = useMemo(() => presetDesign(preset, seed), [preset, seed]);
  const result = useMemo(() => preview(design, seed, naming), [design, seed, naming]);
  const founding: Founding = { naming, design, seed, power: 0.5, openness: 0.5, worldSeed };

  if (adjusting) {
    return (
      <div className="title-page adjusting">
        <h1>Adjust the sounds</h1>
        <Designer catalog={catalog} newWorld initial={founding} onFound={onBegin} onCancel={() => setAdjusting(false)} />
      </div>
    );
  }

  return (
    <form
      className="title-page"
      onSubmit={(e) => {
        e.preventDefault();
        if (typeof result !== "string") onBegin(founding);
      }}
    >
      <h1>A new book</h1>
      <p className="muted">Choose how its first people sound and what they call themselves. Everything else can come later.</p>

      <div className="title-body">
        <div className="title-choices">
          <fieldset className="sounds">
            <legend>Their speech sounds</legend>
            <div className="presets">
              {catalog.presets.map((p) => (
                <label key={p.id} className={p.id === preset ? "preset chosen" : "preset"}>
                  <input type="radio" name="preset" checked={p.id === preset} onChange={() => setPreset(p.id)} />
                  <strong>{p.name}</strong>
                  <span>{p.description}</span>
                </label>
              ))}
              <button
                type="button"
                className="preset surprise"
                onClick={() => {
                  setPreset(pick(catalog.presets).id);
                  setNaming(anyNaming(catalog));
                  setSeed(randomSeed());
                }}
              >
                <strong>Surprise me</strong>
                <span>Any sound and any name.</span>
              </button>
            </div>
          </fieldset>

          <label className="naming">
            They call themselves
            <NamingSelect catalog={catalog} value={naming} onChange={(n) => n && setNaming(n)} />
          </label>
        </div>

        <section className="frontispiece" aria-live="polite">
          {typeof result === "string" ? (
            <p className="error">{result}</p>
          ) : (
            <>
              <p className="names">
                <span className="endonym">{result.people.name}</span>
                <span className="muted"> “{result.people.meaning}”</span>
              </p>
              <p className="names">
                who speak <span className="glottonym">{result.language.name}</span>
                <span className="muted"> “{result.language.meaning}”</span>
              </p>
              <dl className="sample">
                {result.words.filter((w) => SAMPLE.includes(w.gloss)).map((w) => (
                  <div key={w.gloss}>
                    <dt>{w.gloss}</dt>
                    <dd className="word">{w.spelled}</dd>
                  </div>
                ))}
              </dl>
            </>
          )}
          <button type="button" className="link" onClick={() => setSeed(randomSeed())}>
            Other words with the same sounds
          </button>
        </section>
      </div>

      <div className="row title-actions">
        <button type="button" className="link" onClick={() => setAdjusting(true)}>
          Adjust the sounds…
        </button>
        <span className="row">
          {onCancel ? (
            <button type="button" onClick={onCancel}>
              Back to the shelf
            </button>
          ) : null}
          <button type="submit" className="primary" disabled={typeof result === "string"}>
            Begin the book
          </button>
        </span>
      </div>
    </form>
  );
}
