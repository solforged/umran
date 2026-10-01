import { useMemo, useState } from "react";
import type { Engine, Variety } from "../model";
import { YEARS } from "../model";

/// One word in depth, or the language's sound system.
export function Inspector({
  engine,
  version,
  generation,
  variety,
  concept,
  onScrub,
  onOpenVariety,
}: {
  engine: Engine;
  version: number;
  generation: number;
  variety: Variety;
  concept: string | null;
  onScrub: (generation: number) => void;
  onOpenVariety: (variety: number) => void;
}) {
  const [tab, setTab] = useState<"word" | "sound">("word");
  const detail = useMemo(
    () => (concept ? engine.word(generation, variety.id, concept) : null),
    [engine, generation, variety.id, concept, version],
  );

  return (
    <aside className="pane inspector">
      <div className="tabs" role="tablist">
        <button type="button" role="tab" aria-selected={tab === "word"} onClick={() => setTab("word")}>
          Word
        </button>
        <button type="button" role="tab" aria-selected={tab === "sound"} onClick={() => setTab("sound")}>
          Sound
        </button>
      </div>

      {tab === "sound" ? (
        <div role="tabpanel">
          <h2>{variety.name}</h2>
          <p className="muted">Sound preferences: {variety.profile}</p>
          <h3>Consonants</h3>
          <p className="segments">{variety.consonants.join(" ")}</p>
          <h3>Vowels</h3>
          <p className="segments">{variety.vowels.join(" ")}</p>
          <h3>Sound laws</h3>
          {variety.laws.length === 0 ? (
            <p className="muted">None yet. Run some generations.</p>
          ) : (
            <ol className="history">
              {variety.laws.map((law, i) => (
                <li key={i}>
                  <button type="button" className="gen" onClick={() => onScrub(law.generation)}>
                    {law.generation}
                  </button>
                  {law.label}
                </li>
              ))}
            </ol>
          )}
        </div>
      ) : detail ? (
        <div role="tabpanel">
          <p className="eyebrow">
            {detail.field}
            {detail.rank !== null ? ` · core meaning #${detail.rank}` : ""}
          </p>
          <h2>{detail.gloss}</h2>
          {detail.variants.map((v, i) => (
            <article key={i} className="variant">
              <div className="variant-head">
                <span className="word big">{v.spelled}</span>
                <span className="ipa">/{v.ipa}/</span>
                {detail.variants.length > 1 ? (
                  <meter min={0} max={1} value={v.share} title={`${Math.round(v.share * 100)}% of uses`} />
                ) : null}
              </div>
              {v.senses.length > 1 ? <p className="muted">Also means: {v.senses.filter((s) => s !== detail.gloss).join(", ")}</p> : null}
              <ol className="history">
                {v.history.map((line, j) => (
                  <li key={j}>
                    <button
                      type="button"
                      className="gen"
                      title={`View generation ${line.generation} (about ${line.generation * YEARS} years)`}
                      onClick={() => onScrub(line.generation)}
                    >
                      {line.generation}
                    </button>
                    {line.text}
                  </li>
                ))}
              </ol>
            </article>
          ))}
          {detail.cognates.length > 0 ? (
            <>
              <h3>Cognates in related languages</h3>
              <ul className="cognates">
                {detail.cognates.map((c) => (
                  <li key={c.variety}>
                    <button type="button" className="link" onClick={() => onOpenVariety(c.variety)}>
                      {c.name}
                    </button>{" "}
                    <span className="word">{c.spelled}</span> <span className="ipa">/{c.ipa}/</span>
                  </li>
                ))}
              </ul>
            </>
          ) : null}
          {detail.related.length > 0 ? (
            <p className="muted">Related meanings a word can spread between: {detail.related.join(", ")}</p>
          ) : null}
        </div>
      ) : (
        <p className="muted">Choose a word.</p>
      )}
    </aside>
  );
}
