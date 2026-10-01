import { useMemo } from "react";
import type { Engine } from "../model";
import { YEARS } from "../model";

/// A marginal note on one word: its forms, how it came to be, and its kin
/// in related languages.
export function WordGloss({
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
  variety: number;
  concept: string | null;
  onScrub: (generation: number) => void;
  onOpenVariety: (variety: number) => void;
}) {
  const detail = useMemo(
    () => (concept ? engine.word(generation, variety, concept) : null),
    // `version` changes whenever the history does.
    [engine, generation, variety, concept, version],
  );
  if (!detail) {
    return <p className="muted gloss-hint">Choose a word, and its history will be noted here.</p>;
  }

  return (
    <article className="gloss">
      <p className="eyebrow">
        {detail.field}
        {detail.rank !== null ? ` · core meaning #${detail.rank}` : ""}
      </p>
      <h3>{detail.gloss}</h3>
      {detail.variants.map((v, i) => (
        <section key={i} className="variant">
          <div className="variant-head">
            <span className="word big">{v.spelled}</span>
            <span className="ipa">/{v.ipa}/</span>
            {detail.variants.length > 1 ? (
              <meter min={0} max={1} value={v.share} title={`${Math.round(v.share * 100)}% of uses`} />
            ) : null}
          </div>
          {v.senses.length > 1 ? (
            <p className="muted">Also means: {v.senses.filter((s) => s !== detail.gloss).join(", ")}</p>
          ) : null}
          <ol className="history">
            {v.history.map((line, j) => (
              <li key={j}>
                <button
                  type="button"
                  className="gen"
                  title={`View generation ${line.generation}`}
                  onClick={() => onScrub(line.generation)}
                >
                  {line.generation * YEARS}
                </button>
                {line.text}
              </li>
            ))}
          </ol>
        </section>
      ))}
      {detail.cognates.length > 0 ? (
        <>
          <h4>In related tongues</h4>
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
        <p className="muted small">A word can spread to: {detail.related.join(", ")}</p>
      ) : null}
    </article>
  );
}
