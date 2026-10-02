import type { GrammarRendering } from "../model";
import "./sample.css";

/** Lay out the facade's words, sounds, and glosses without deriving forms. */
export function Sample({ rendering, label }: { rendering: GrammarRendering; label: string }) {
  const words = rendering.text.split(/\s+/);
  const sounds = rendering.ipa.split(/\s+/);
  return <div className="grammar-sample" role="group" aria-label={label}>
    <div className="sample-columns">
      {words.map((word, i) => <div className="sample-column" key={i}>
        <span className="word">{word}</span>
        <span className="ipa">{sounds[i]}</span>
        <span className="sample-gloss">{rendering.gloss[i]?.split(/(OBJ|PAST)/).map((part, j) =>
          part === "OBJ" || part === "PAST" ? <span className="sample-category" key={j}>{part.toLowerCase()}</span> : part
        )}</span>
      </div>)}
    </div>
  </div>;
}
