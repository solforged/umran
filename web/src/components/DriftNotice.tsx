import type { Drift, DriftExample } from "../model";
import { YEARS } from "../model";

/// How many renamed peoples or languages the table lists before it counts.
const LISTED = 8;

function Rows({ kind, examples, more }: { kind: string | null; examples: DriftExample[]; more: number }) {
  return <>
    {examples.map((example, index) => <tr key={`${kind ?? ""}${example.of}${index}`}>
      <th scope="row">{kind ?? example.of}</th>
      <td><span className="word">{example.was}</span></td>
      <td>{example.now === null ? <span className="muted">nothing</span> : <span className="word">{example.now}</span>}</td>
    </tr>)}
    {more > 0 ? <tr><td colSpan={3} className="muted">and {more.toLocaleString()} more</td></tr> : null}
  </>;
}

/// What this engine reads differently from the world's frozen record: the
/// engine's own comparison, set out as counts and a few examples.
export function DriftReport({ drift }: { drift: Drift }) {
  const year = drift.generation * YEARS;
  if (drift.same) {
    return <>Revision {drift.now} reads it exactly as revision {drift.was} did in year {year}.</>;
  }
  const of = (changed: number, total: number) => `${changed.toLocaleString()} of ${total.toLocaleString()}`;
  const renamed = drift.peoples.changed.length + drift.languages.changed.length;
  const sentences = [
    renamed ? `${of(drift.peoples.changed.length, drift.peoples.total)} peoples and ${of(drift.languages.changed.length, drift.languages.total)} languages have other names.` : null,
    drift.names.changed ? `${of(drift.names.changed, drift.names.total)} names of lands, cities, states, and faiths differ.` : null,
    drift.words.changed ? `${of(drift.words.changed, drift.words.total)} words differ.` : null,
  ].filter((sentence): sentence is string => sentence !== null);
  const listed = (examples: DriftExample[]) => examples.slice(0, LISTED);
  return <>
    Revision {drift.now} reads it differently from revision {drift.was}’s record of year {year}.
    {drift.partsWays !== null ? ` The chronicles part ways in year ${drift.partsWays * YEARS}.` : ""}
    {sentences.length ? ` ${sentences.join(" ")}` : " Its names and words are the same. The difference is in other detail."}
    {sentences.length ? <details className="drift">
      <summary>What differs</summary>
      <table>
        <thead><tr><th scope="col">What</th><th scope="col">As recorded</th><th scope="col">Now</th></tr></thead>
        <tbody>
          <Rows kind="a people" examples={listed(drift.peoples.changed)} more={drift.peoples.changed.length - listed(drift.peoples.changed).length} />
          <Rows kind="a language" examples={listed(drift.languages.changed)} more={drift.languages.changed.length - listed(drift.languages.changed).length} />
          <Rows kind={null} examples={drift.names.examples} more={drift.names.changed - drift.names.examples.length} />
          <Rows kind={null} examples={drift.words.examples} more={drift.words.changed - drift.words.examples.length} />
        </tbody>
      </table>
    </details> : null}
  </>;
}
