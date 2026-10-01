import { useMemo, useState } from "react";
import type { Engine, Overview } from "../model";
import { YEARS } from "../model";
import { bookMarkdown, download, fileName, glossaryCsv, peopleLines } from "../takeout";

/// The back of the book, for taking things out: who is called what, a
/// glossary of any tongue, and the book in forms other tools can read.
export function Appendix({
  engine,
  version,
  generation,
  overview,
  title,
  variety,
  onBack,
}: {
  engine: Engine;
  version: number;
  generation: number;
  overview: Overview;
  title: string;
  /// The tongue to open the glossary on.
  variety: number;
  onBack: () => void;
}) {
  const [tongue, setTongue] = useState(variety);
  const [order, setOrder] = useState<"word" | "meaning">("word");
  const [copied, setCopied] = useState<string | null>(null);
  const name = (id: number) => overview.communities[id]?.name ?? "?";
  const chosen = overview.varieties[tongue] ?? overview.varieties[variety];
  const rows = useMemo(
    () => engine.lexicon(generation, chosen.id),
    // `version` changes whenever the history does.
    [engine, generation, chosen.id, version],
  );
  const sorted = useMemo(
    () =>
      [...rows].sort((a, b) =>
        order === "word" ? a.spelled.localeCompare(b.spelled) : a.gloss.localeCompare(b.gloss),
      ),
    [rows, order],
  );

  const copy = async (what: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(what);
    } catch {
      setCopied(null);
    }
  };

  const whole = () => {
    const glossaries = overview.varieties
      .filter((v) => v.spoken)
      .map((v): [string, typeof rows] => [v.name, engine.lexicon(generation, v.id)]);
    download(fileName(title, "md"), bookMarkdown(title, overview, glossaries), "text/markdown");
  };

  return (
    <main className="appendix">
      <header className="appendix-head">
        <button type="button" className="link" onClick={onBack}>
          ← Back to the book
        </button>
        <h2>Appendix</h2>
        <p className="muted">
          As the book stands in year {generation * YEARS}. Copy or download anything here to use it elsewhere.
        </p>
      </header>

      <section>
        <div className="row spread">
          <h3>Names</h3>
          <button type="button" onClick={() => void copy("names", peopleLines(overview).join("\n"))}>
            {copied === "names" ? "Copied" : "Copy names"}
          </button>
        </div>
        <table className="names-table">
          <thead>
            <tr>
              <th>People</th>
              <th>Meaning</th>
              <th>Tongue</th>
              <th>Once</th>
              <th>Called by others</th>
            </tr>
          </thead>
          <tbody>
            {overview.communities.map((c) => {
              const v = overview.varieties[c.variety];
              return (
                <tr key={c.id}>
                  <td className={`word hand-${v.family % 5}`}>{c.name}</td>
                  <td>
                    “{c.meaning}” <span className="ipa">/{c.ipa}/</span>
                  </td>
                  <td>
                    <span className="word">{v.name}</span>
                    {v.meaning ? <span className="muted"> “{v.meaning}”</span> : null}
                  </td>
                  <td>{c.once ?? ""}</td>
                  <td>{c.exonyms.map((e) => `${e.name} (the ${name(e.by)})`).join(", ")}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </section>

      <section>
        <div className="row spread">
          <h3>Glossary</h3>
          <div className="row">
            <select aria-label="Tongue" value={chosen.id} onChange={(e) => setTongue(Number(e.target.value))}>
              {overview.varieties.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.name}
                  {v.spoken ? "" : " (no longer spoken)"}
                </option>
              ))}
            </select>
            <select aria-label="Order" value={order} onChange={(e) => setOrder(e.target.value as "word" | "meaning")}>
              <option value="word">By word</option>
              <option value="meaning">By meaning</option>
            </select>
            <button
              type="button"
              onClick={() =>
                void copy("glossary", sorted.map((r) => `${r.spelled} /${r.ipa}/ ${r.gloss}`).join("\n"))
              }
            >
              {copied === "glossary" ? "Copied" : "Copy"}
            </button>
            <button
              type="button"
              onClick={() => download(fileName(`${chosen.name} glossary`, "csv"), glossaryCsv(sorted), "text/csv")}
            >
              Download CSV
            </button>
          </div>
        </div>
        <dl className="glossary">
          {sorted.map((r) => (
            <div key={r.concept}>
              <dt>
                <span className="word">{r.spelled}</span> <span className="ipa">/{r.ipa}/</span>
              </dt>
              <dd>{r.gloss}</dd>
            </div>
          ))}
        </dl>
      </section>

      <section>
        <h3>Take the book with you</h3>
        <ul className="takeout">
          <li>
            <button type="button" onClick={whole}>
              The whole book as text
            </button>{" "}
            <span className="muted">Markdown: the peoples, the chronicle, and a glossary for every living tongue.</span>
          </li>
          <li>
            <button
              type="button"
              onClick={() => download(fileName(title, "json"), engine.save(), "application/json")}
            >
              The book itself
            </button>{" "}
            <span className="muted">
              A file to bring back in from the shelf later, on this or another browser. It holds every telling.
            </span>
          </li>
        </ul>
      </section>
    </main>
  );
}
