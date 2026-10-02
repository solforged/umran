import { useMemo, useState } from "react";
import type { Catalog, NotebookNote, ReadEngine, Overview } from "../model";
import { YEARS } from "../model";
import { bookMarkdown, craftLines, download, fileName, givenNameLines, glossaryCsv, peopleLines, religionLines, renderingLines, stateLines } from "../takeout";
import { Renderings } from "./Renderings";

/// The export page, for taking things out of a world: who is called what,
/// a glossary of any language, and the world in forms other tools read.
export function Appendix({
  engine,
  catalog,
  version,
  generation,
  overview,
  title,
  notebook,
  variety,
  onBack,
  onShelf,
}: {
  engine: ReadEngine;
  catalog: Catalog;
  version: number;
  generation: number;
  overview: Overview;
  title: string;
  notebook: NotebookNote[];
  /// The tongue to open the glossary on.
  variety: number;
  onBack: () => void;
  onShelf: () => void;
}) {
  const [tongue, setTongue] = useState(variety);
  const [order, setOrder] = useState<"word" | "meaning">("word");
  const [copied, setCopied] = useState<string | null>(null);
  const name = (id: number) => overview.communities[id]?.name ?? "?";
  const chosen = overview.varieties[tongue] ?? overview.varieties[variety];
  const religions = religionLines(overview);
  const crafts = craftLines(overview, catalog);
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
      .filter((v) => v.spoken || v.sacredOf !== null)
      .map((v): [string, typeof rows] => [v.name, engine.lexicon(generation, v.id)]);
    download(fileName(title, "md"), bookMarkdown(title, overview, glossaries, catalog, notebook), "text/markdown");
  };

  return (
    <main className="appendix">
      <header className="appendix-head">
        <nav className="row">
          <button type="button" className="link brand" onClick={onShelf} title="Back to the shelf" aria-label="Back to the shelf">
            <span className="brand-name"><span>ʿUmrān</span></span>
          </button>
          <button type="button" className="link" onClick={onBack}>
            ← Back to the map
          </button>
        </nav>
        <h2>Export</h2>
        <p className="muted">
          As the world stands in year {generation * YEARS}. Copy or download anything here to use it elsewhere.
        </p>
      </header>

      <section>
        <div className="row spread">
          <h3>Names</h3>
          <button type="button" onClick={() => void copy("names", [
            ...peopleLines(overview),
            ...(overview.states.length > 0 ? ["", "States", ...stateLines(overview)] : []),
            ...(overview.religions.length > 0 ? ["", "Religions", ...religionLines(overview)] : []),
            ...overview.varieties.flatMap((v) => ["", `Given names in ${v.name}`, ...givenNameLines(v, overview)]),
          ].join("\n"))}>
            {copied === "names" ? "Copied" : "Copy names"}
          </button>
        </div>
        <table className="names-table">
          <thead>
            <tr>
              <th>People</th>
              <th>Meaning</th>
              <th>Language</th>
              <th>Once</th>
              <th>Called by others</th>
            </tr>
          </thead>
          <tbody>
            {overview.communities.filter((c) => c.ended === null).map((c) => {
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
        {overview.states.length > 0 ? (
          <>
            <h4>States</h4>
            <table className="names-table">
              <thead><tr><th>State</th><th>Meaning</th><th>Founder</th><th>Rulers</th><th>Once</th><th>Stands</th></tr></thead>
              <tbody>
                {overview.states.map((state) => (
                  <tr key={state.id}>
                    <td className={`word hand-${overview.varieties[overview.communities[state.rulers].variety].family % 5}`}>{state.name}</td>
                    <td>“{state.meaning}” <span className="ipa">/{state.ipa}/</span></td>
                    <td><span className="word">{state.founder.name}</span>, “{state.founder.meaning}” <span className="ipa">/{state.founder.ipa}/</span></td>
                    <td className="word">{name(state.rulers)}</td>
                    <td>{state.once ?? ""}</td>
                    <td>{state.fell === null ? "now" : `fell in year ${state.fell * YEARS}`}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </>
        ) : null}
      </section>

      <section>
        <div className="row spread">
          <h3>Religions</h3>
          <button type="button" disabled={overview.religions.length === 0}
            onClick={() => void copy("religions", overview.religions.flatMap((r) => [
              religions[r.id], ...renderingLines(r.words, overview), "",
            ]).join("\n"))}>
            {copied === "religions" ? "Copied" : "Copy religions"}
          </button>
        </div>
        {overview.religions.length === 0 ? <p className="muted">None founded yet.</p> :
          overview.religions.map((r) => (
            <details key={r.id} className="export-words">
              <summary><span className="word">{r.name}</span> · “{r.meaning}”</summary>
              <p>{religions[r.id]}</p>
              <Renderings rows={r.words} overview={overview} sacred={r.sacred} />
            </details>
          ))}
      </section>

      <section>
        <div className="row spread">
          <h3>Crafts</h3>
          <button type="button" onClick={() => void copy("crafts", overview.crafts.flatMap((c, i) => [
            crafts[i], ...renderingLines(c.words, overview), "",
          ]).join("\n"))}>
            {copied === "crafts" ? "Copied" : "Copy crafts"}
          </button>
        </div>
        {overview.crafts.map((craft, i) => (
          <details key={craft.id} className="export-words">
            <summary>{craft.name} · {craft.first === null ? "not yet held" : `first held in year ${craft.first * YEARS}`}</summary>
            <p>{crafts[i]}</p>
            <Renderings rows={craft.words} overview={overview} />
          </details>
        ))}
      </section>

      <section>
        <div className="row spread">
          <h3>Glossary</h3>
          <div className="row">
            <select aria-label="Language" value={chosen.id} onChange={(e) => setTongue(Number(e.target.value))}>
              {overview.varieties.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.name}
                  {v.sacredOf !== null ? " (sacred)" : v.spoken ? "" : " (no longer spoken)"}
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
                void copy("glossary", sorted.map((r) => `${r.spelled}${r.said === null ? "" : ` · said ${r.said}`} /${r.ipa}/ ${r.gloss}`).join("\n"))
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
                {r.said !== null ? <> <span className="muted">· said</span> <span className="word">{r.said}</span></> : null}
              </dt>
              <dd>{r.gloss}</dd>
            </div>
          ))}
        </dl>
        <div className="row spread">
          <h4>Given names in {chosen.name}</h4>
          <button type="button" onClick={() => void copy("given", givenNameLines(chosen, overview).join("\n"))}>
            {copied === "given" ? "Copied" : "Copy given names"}
          </button>
        </div>
        <p className="muted">
          {chosen.nameStyle === "double" ? "Two-part names." : "One-word names."}{" "}
          {chosen.written === null ? "Unwritten." : `Written or last respelled in year ${chosen.written * YEARS}.`}
          {chosen.sacredOf === null ? "" : ` Sacred to ${overview.religions[chosen.sacredOf].name}.`}
        </p>
        <table className="names-table">
          <thead><tr><th>Name</th><th>IPA</th><th>Meaning</th><th>From</th></tr></thead>
          <tbody>
            {chosen.names.map((name, i) => (
              <tr key={i}>
                <td className="word">{name.name}</td>
                <td className="ipa">/{name.ipa}/</td>
                <td>{name.meaning}</td>
                <td>{name.from === null ? "" : `${overview.varieties[name.from].name} (sacred)`}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section>
        <h3>The whole world</h3>
        <ul className="takeout">
          <li>
            <button type="button" onClick={whole}>
              As text
            </button>{" "}
            <span className="muted">
              Markdown: peoples, states, religions, crafts, history, glossaries, and every notebook entry, including archived notes.
            </span>
          </li>
          <li>
            <button
              type="button"
              onClick={() => download(fileName(title, "json"), engine.save(), "application/json")}
            >
              As a save file
            </button>{" "}
            <span className="muted">
              To bring back in from the shelf later, in this or another browser. It holds every telling and the complete notebook.
            </span>
          </li>
        </ul>
      </section>
    </main>
  );
}
