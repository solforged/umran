import { useMemo, type ReactNode } from "react";
import type { LoanCause, Overview, ReadEngine, Variant } from "../model";
import { YEARS } from "../model";
import type { ParadigmView } from "./LanguageChapter";

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
  renderCause,
  renderOrigin,
  family,
  overview,
}: {
  engine: ReadEngine;
  version: number;
  generation: number;
  variety: number;
  concept: string | null;
  onScrub?: (generation: number) => void;
  onOpenVariety: (variety: number) => void;
  /// How the loan on this line came, drawn by the card that knows the world.
  renderCause?: (cause: LoanCause, generation: number) => ReactNode;
  renderOrigin?: (variant: Variant) => ReactNode;
  family?: number;
  overview?: Overview;
}) {
  const detail = useMemo(
    () => (concept ? engine.word(generation, variety, concept) : null),
    // `version` changes whenever the history does.
    [engine, generation, variety, concept, version],
  );
  if (!detail) {
    return <p className="muted gloss-hint">Choose a word, and its chronicle will be noted here.</p>;
  }
  const cognates = detail.cognates.filter((c) => family === undefined || overview?.varieties[c.variety]?.family === family);

  return (
    <article className="gloss">
      <p className="eyebrow">
        {detail.field}
        {detail.rank !== null ? ` · core meaning #${detail.rank}` : ""}
      </p>
      <h3>{detail.gloss}</h3>
      {detail.variants.map((v, i) => {
        const extension = v.history.findLast((line) => line.text === `Also came to mean '${detail.gloss}'`);
        return (
        <section key={i} className="variant">
          <div className="variant-head">
            <span className="word big">{v.spelled}</span>
            {v.said !== null ? <span> <span className="muted">· said</span> <span className="word">{v.said}</span></span> : null}
            <span className="ipa">/{v.ipa}/</span>
            {detail.variants.length > 1 ? (
              <meter min={0} max={1} value={v.share} title={`${Math.round(v.share * 100)}% of uses`} />
            ) : null}
          </div>
          {renderOrigin ? <p>{renderOrigin(v)}</p> : null}
          {extension ? <p className="word-extension">
            Stretched to “{detail.gloss}” in year {extension.generation * YEARS}. The word’s root origin is recorded above.
          </p> : null}
          {(v as Variant & { paradigms?: ParadigmView[] }).paradigms?.map((p) => (
            <section className="word-paradigm" key={p.category}>
              <h4>{p.label}</h4>
              <ul className="roster">
                {p.realizations.map((form, k) => <li key={k}>
                  <span className="word">{form.spelled || "∅"}</span>
                  {form.said !== null ? <> · said <span className="word">{form.said}</span></> : null}
                  {" "}<span className="ipa">/{form.ipa}/</span> · {Math.round(form.share * 100)}% of uses
                  {form.retired !== null ? `; retired in year ${form.retired * YEARS}` : ""}.
                  <details><summary>Recorded changes</summary><ol className="history">
                    {form.history.map((line, j) => <li key={j}><span className="gen">{line.generation * YEARS}</span><span>{line.text}</span></li>)}
                  </ol></details>
                </li>)}
              </ul>
            </section>
          ))}
          {v.senses.length > 1 ? (
            <p className="muted">Also means: {v.senses.filter((s) => s !== detail.gloss).join(", ")}</p>
          ) : null}
          <ol className="history">
            {v.history.map((line, j) => (
              <li key={j}>
                {onScrub ? <button
                  type="button"
                  className="gen"
                  title={`See the world in year ${line.generation * YEARS}`}
                  onClick={() => onScrub(line.generation)}
                >
                  {line.generation * YEARS}
                </button> : <span className="gen">{line.generation * YEARS}</span>}
                <span>
                  {line.text}
                  {/* A loan's later adaptation lines repeat its cause; tell it once. */}
                  {line.cause && renderCause && !(renderOrigin && v.origin.cause) && !v.history.slice(0, j).some((l) => l.cause)
                    ? renderCause(line.cause, line.generation)
                    : null}
                </span>
              </li>
            ))}
          </ol>
        </section>
        );
      })}
      {cognates.length > 0 ? (
        <>
          <h4>In related languages</h4>
          <ul className="cognates">
            {cognates.map((c) => (
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
