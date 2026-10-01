import type { Overview } from "../model";
import type { DialogKind } from "./ActionDialog";

/// Communities, the actions that change them, and how their languages
/// relate.
export function Communities({
  overview,
  selected,
  onSelect,
  onDialog,
}: {
  overview: Overview;
  selected: number;
  onSelect: (id: number) => void;
  onDialog: (kind: DialogKind) => void;
}) {
  const current = overview.communities[selected];
  const name = (id: number) => overview.communities[id]?.name ?? "?";
  const contacts = overview.contacts.filter((c) => c.a === selected || c.b === selected);
  const closest = overview.intelligibility
    .filter((p) => p.a === current.variety || p.b === current.variety)
    .map((p) => ({ variety: p.a === current.variety ? p.b : p.a, score: p.score }))
    .sort((x, y) => y.score - x.score);
  const families = new Map<number, typeof overview.varieties>();
  for (const v of overview.varieties) {
    families.set(v.family, [...(families.get(v.family) ?? []), v]);
  }

  return (
    <aside className="pane communities">
      <h2>Peoples</h2>
      <ul className="list" role="listbox" aria-label="Communities">
        {overview.communities.map((c) => (
          <li key={c.id}>
            <button
              type="button"
              role="option"
              aria-selected={c.id === selected}
              className={c.id === selected ? "item selected" : "item"}
              onClick={() => onSelect(c.id)}
            >
              <span className={`item-title hand-${overview.varieties[c.variety].family % 5}`}>{c.name}</span>
              <span className="item-sub">{overview.varieties[c.variety].name}</span>
              <span className="item-meta">
                {Math.round(c.size).toLocaleString()} people · prestige {c.prestige.toFixed(2)}
              </span>
            </button>
          </li>
        ))}
      </ul>
      <div className="actions">
        <button type="button" onClick={() => onDialog("found")} title="Found a new people with a language of its own">A new people</button>
        <button type="button" onClick={() => onDialog("split")} title="Split this people in two">Part ways</button>
        <button type="button" onClick={() => onDialog("connect")} disabled={overview.communities.length < 2}>
          Bring together
        </button>
        <button type="button" onClick={() => onDialog("shift")} disabled={overview.communities.length < 2}>
          Change tongue
        </button>
      </div>

      <h3 className={`hand-${overview.varieties[current.variety].family % 5}`}>{current.name}</h3>
      <p className="muted small">
        “{current.meaning}”{current.once ? `, once ${current.once}` : ""} · speaks {overview.varieties[current.variety].name}
        {overview.varieties[current.variety].meaning ? ` (“${overview.varieties[current.variety].meaning}”)` : ""}
      </p>
      <dl className="facts">
        <dt>Power</dt>
        <dd>{current.power.toFixed(2)}</dd>
        <dt>Openness</dt>
        <dd>{current.openness.toFixed(2)}</dd>
        <dt>Contacts</dt>
        <dd>
          {contacts.length === 0
            ? "none"
            : contacts
                .map((c) => `${name(c.a === selected ? c.b : c.a)} (${c.kind}, ${Math.round(c.intensity * 100)}%)`)
                .join("; ")}
        </dd>
        {current.exonyms.length > 0 ? (
          <>
            <dt>Called</dt>
            <dd>{current.exonyms.map((e) => `${e.name} by ${name(e.by)}`).join("; ")}</dd>
          </>
        ) : null}
      </dl>
      {closest.length > 0 ? (
        <>
          <h3>Closest languages</h3>
          <ul className="bars">
            {closest.slice(0, 6).map((p) => (
              <li key={p.variety}>
                <span>{overview.varieties[p.variety].name}</span>
                <meter min={0} max={1} value={p.score} title={`intelligibility ${p.score.toFixed(2)}`} />
              </li>
            ))}
          </ul>
        </>
      ) : null}

      <h3>Language families</h3>
      {[...families.values()].map((members) => (
        <ul key={members[0].id} className="tree">
          {members.map((v) => {
            let depth = 0;
            for (let p = v.parent; p !== null; p = overview.varieties[p].parent) depth += 1;
            return (
              <li key={v.id} style={{ paddingLeft: `${depth * 0.9}rem` }} className={v.spoken ? "" : "extinct"}>
                {v.name}
                {v.forkedAt !== null ? <span className="muted"> · from gen {v.forkedAt}</span> : null}
                {v.spoken ? null : <span className="muted"> · no longer spoken</span>}
              </li>
            );
          })}
        </ul>
      ))}
    </aside>
  );
}
