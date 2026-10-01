import type { Overview, RenderingRow } from "../model";
import { renderingOrigin } from "../lore";

/// The same meanings across sacred and spoken languages, with their origins.
export function Renderings({ rows, overview, sacred, onLanguage, onWord }: {
  rows: RenderingRow[];
  overview: Overview;
  sacred?: number;
  onLanguage?: (variety: number) => void;
  onWord?: (variety: number, concept: string) => void;
}) {
  const languages = [...new Set(rows.flatMap((row) => row.renderings.map((r) => r.variety)))];
  if (sacred !== undefined) {
    const at = languages.indexOf(sacred);
    if (at >= 0) languages.splice(at, 1);
    languages.unshift(sacred);
  }
  if (rows.length === 0) return <p className="muted">No words yet.</p>;
  return (
    <div className="compared renderings">
      <table>
        <thead>
          <tr>
            <th scope="col">Meaning</th>
            {languages.map((id) => (
              <th key={id} scope="col">
                {onLanguage ? (
                  <button type="button" className="link word" onClick={() => onLanguage(id)}>{overview.varieties[id].name}</button>
                ) : <span className="word">{overview.varieties[id].name}</span>}
                {id === sacred ? <small className="muted">sacred</small> : null}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.concept}>
              <th scope="row">{row.gloss}</th>
              {languages.map((id) => {
                const r = row.renderings.find((r) => r.variety === id);
                return (
                  <td key={id}>
                    {r ? <>
                      {onWord ? (
                        <button type="button" className="link word" onClick={() => onWord(id, row.concept)}>{r.spelled}</button>
                      ) : <span className="word">{r.spelled}</span>}
                      <span className="ipa"> /{r.ipa}/</span>
                      <small className="muted">
                        {r.how === "built" && r.from !== null ? <>built on <span className="word">{r.from}</span></> : renderingOrigin(r)}
                      </small>
                    </> : <span className="muted">—</span>}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
