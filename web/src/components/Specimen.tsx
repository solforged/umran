import type { SpecimenWord } from "../model";

/// A language's specimen: a few basic words, each under its meaning, so a
/// language can be known at a glance. With `changes`, words a sound change
/// reached show how they sounded before, and the others step back.
export function Specimen({
  words,
  changes = false,
  onWord,
}: {
  words: SpecimenWord[];
  changes?: boolean;
  onWord?: (concept: string) => void;
}) {
  if (words.length === 0) return null;
  return (
    <dl className={changes ? "specimen changes" : "specimen"}>
      {words.map((w) => (
        <div key={w.concept} className={changes && w.was !== null ? "reached" : undefined}>
          <dt title={w.gloss}>{w.concept}</dt>
          <dd>
            {changes && w.was !== null ? <span className="was">{w.was} → </span> : null}
            {onWord ? (
              <button type="button" className="link word" title={`/${w.ipa}/`} onClick={() => onWord(w.concept)}>
                {w.spelled}
              </button>
            ) : (
              <span className="word" title={`/${w.ipa}/`}>
                {w.spelled}
              </span>
            )}
          </dd>
        </div>
      ))}
    </dl>
  );
}
