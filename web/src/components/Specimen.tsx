import type { SpecimenWord } from "../model";

/// A language's specimen: a few basic words, each under its meaning, so a
/// language can be known at a glance. With `changes`, words a sound change
/// reached show how they were before, and the others step back. A change
/// heard but not written, such as stress moving, shows as sound.
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
      {words.map((w) => {
        const reached = changes && (w.was !== null || w.wasIpa !== null);
        return (
          <div key={w.concept} className={reached ? "reached" : undefined}>
            <dt title={w.gloss}>{w.concept}</dt>
            <dd>
              {reached && w.was !== null ? <span className="was">{w.was} → </span> : null}
              {reached && w.was === null ? <span className="was ipa">/{w.wasIpa}/ → /{w.ipa}/ </span> : null}
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
        );
      })}
    </dl>
  );
}
