import { useMemo, useState } from "react";
import type { Community, ContactKind, Engine, Overview, Terrain, WorldMap } from "../model";
import { YEARS } from "../model";

const TERRAIN_NAME: Record<Terrain, string> = {
  plains: "Plains",
  forest: "Forest",
  steppe: "Steppe",
  hills: "Hills",
  mountains: "Mountains",
  desert: "Desert",
  sea: "Sea",
};

const CONTACT_NAME: Record<ContactKind, string> = {
  neighbours: "Neighbours",
  trade: "Trade",
  rule: "Rule",
  religion: "Religion",
  intermarriage: "Intermarriage",
};

/// Gap between stacked labels of peoples sharing a region, in map units.
const LINE = 0.3;

/// A colour for the `n`th family or root, far from its neighbours in hue.
function hue(n: number): string {
  return `hsl(${Math.round((n * 137.508) % 360)} 55% 48%)`;
}

/// The book's map: every people where it lives, coloured by language
/// family, with the dealings between them, or every people's word for one
/// meaning, coloured by the root it descends from. It follows the
/// timeline like the rest of the book.
export function Atlas({
  engine,
  map,
  version,
  generation,
  overview,
  selected,
  onSelect,
  onRead,
}: {
  engine: Engine;
  map: WorldMap;
  version: number;
  generation: number;
  overview: Overview;
  selected: number;
  onSelect: (community: number) => void;
  /// Turn to the chronicle with this people chosen.
  onRead: (community: number) => void;
}) {
  const [show, setShow] = useState<"peoples" | "words">("peoples");
  const [concept, setConcept] = useState("fire");
  const [contacts, setContacts] = useState(true);
  const chosen = overview.communities[selected];

  const concepts = useMemo(
    () =>
      engine
        .lexicon(generation, chosen.variety)
        .map((r) => ({ id: r.concept, gloss: r.gloss }))
        .sort((a, b) => a.gloss.localeCompare(b.gloss)),
    // `version` changes whenever the history does.
    [engine, generation, chosen.variety, version],
  );
  const words = useMemo(
    () => (show === "words" ? engine.wordMap(generation, concept) : null),
    [engine, generation, concept, show, version],
  );

  // Peoples sharing a region stack their labels around its centre.
  const placed = useMemo(() => {
    const byRegion = new Map<number, Community[]>();
    for (const c of overview.communities) byRegion.set(c.region, [...(byRegion.get(c.region) ?? []), c]);
    const out = new Map<number, [number, number]>();
    for (const [region, here] of byRegion) {
      const [x, y] = map.regions[region].site;
      here.forEach((c, i) => out.set(c.id, [x, y + (i - (here.length - 1) / 2) * LINE]));
    }
    return { byRegion, at: out };
  }, [overview.communities, map]);

  const family = (c: Community) => overview.varieties[c.variety].family;
  const wordBy = useMemo(() => new Map(words?.words.map((w) => [w.community, w])), [words]);

  // A region takes the colour of its largest people: its family, or the
  // root of that people's word.
  const tint = (region: number): string | null => {
    const here = placed.byRegion.get(region);
    if (!here) return null;
    const largest = here.reduce((a, b) => (b.size > a.size ? b : a));
    if (show === "peoples") return hue(family(largest));
    const word = wordBy.get(largest.id);
    return word ? hue(word.group) : null;
  };

  const groups = useMemo(() => {
    if (!words) return [];
    const seen = new Map<number, string[]>();
    for (const w of words.words) seen.set(w.group, [...(seen.get(w.group) ?? []), w.spelled]);
    return [...seen.entries()].map(([group, forms]) => ({ group, forms: [...new Set(forms)] }));
  }, [words]);

  // Peoples on the same land need no line between them.
  const dealings = contacts
    ? overview.contacts.filter((k) => overview.communities[k.a].region !== overview.communities[k.b].region)
    : [];
  const regionOf = map.regions[chosen.region];
  const partners = overview.contacts
    .filter((k) => k.a === chosen.id || k.b === chosen.id)
    .map((k) => ({ kind: k.kind, other: overview.communities[k.a === chosen.id ? k.b : k.a] }));

  return (
    <main className="atlas">
      <figure className="atlas-map">
        <svg
          viewBox={`0 0 ${map.width} ${map.height}`}
          role="img"
          aria-label={`Map of ${map.regions.length} lands in year ${generation * YEARS}`}
        >
          <g className="lands">
            {map.regions.map((r) => {
              const colour = tint(r.id);
              const points = r.outline.map(([x, y]) => `${x},${y}`).join(" ");
              return (
                <g key={r.id}>
                  <polygon className={`land terrain-${r.terrain}`} points={points} />
                  {colour ? <polygon className="claim" points={points} style={{ fill: colour }} /> : null}
                </g>
              );
            })}
          </g>
          <g className="dealings">
            {dealings.map((k, i) => {
              const [ax, ay] = placed.at.get(k.a)!;
              const [bx, by] = placed.at.get(k.b)!;
              return (
                <line
                  key={i}
                  className={`dealing dealing-${k.kind}`}
                  x1={ax}
                  y1={ay}
                  x2={bx}
                  y2={by}
                  style={{ strokeOpacity: 0.35 + 0.65 * k.intensity }}
                />
              );
            })}
          </g>
          <g className="peoples">
            {overview.communities.map((c) => {
              const [x, y] = placed.at.get(c.id)!;
              const word = wordBy.get(c.id);
              const label = show === "words" ? (word?.spelled ?? "—") : c.name;
              return (
                <g
                  key={c.id}
                  className={c.id === selected ? "people chosen" : "people"}
                  role="button"
                  tabIndex={0}
                  aria-label={show === "words" ? `${c.name}: ${label}` : c.name}
                  onClick={() => onSelect(c.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onSelect(c.id);
                    }
                  }}
                >
                  <title>
                    {show === "words" && word
                      ? `${c.name}: ${word.spelled} /${word.ipa}/`
                      : `${c.name}, ${Math.round(c.size).toLocaleString()} souls`}
                  </title>
                  <text
                    x={x}
                    y={y}
                    className={`hand-${family(c) % 5}`}
                    style={{ fill: show === "words" && word ? hue(word.group) : hue(family(c)) }}
                  >
                    {label}
                  </text>
                </g>
              );
            })}
          </g>
        </svg>
      </figure>

      <aside className="atlas-key">
        <fieldset>
          <legend>Show</legend>
          <label>
            <input type="radio" name="show" checked={show === "peoples"} onChange={() => setShow("peoples")} />
            Peoples, by language family
          </label>
          <label>
            <input type="radio" name="show" checked={show === "words"} onChange={() => setShow("words")} />
            The word for{" "}
            <select
              value={concept}
              onChange={(e) => {
                setConcept(e.target.value);
                setShow("words");
              }}
            >
              {concepts.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.gloss}
                </option>
              ))}
            </select>
          </label>
          <label>
            <input type="checkbox" checked={contacts} onChange={(e) => setContacts(e.target.checked)} />
            Dealings between peoples
          </label>
        </fieldset>

        {words ? (
          <section>
            <h3>“{words.gloss}”</h3>
            <p className="muted">Words of one colour come down from one root.</p>
            <ul className="atlas-legend">
              {groups.map((g) => (
                <li key={g.group}>
                  <span className="swatch" style={{ background: hue(g.group) }} />
                  <span className="word">{g.forms.join(", ")}</span>
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        <section className="atlas-chosen">
          <h3 className={`word hand-${family(chosen) % 5}`}>{chosen.name}</h3>
          <p>
            “{chosen.meaning}”, speaking <span className="word">{overview.varieties[chosen.variety].name}</span>.
          </p>
          <p className="muted">
            {Math.round(chosen.size).toLocaleString()} souls on {regionOf.coastal ? "coastal " : ""}
            {TERRAIN_NAME[regionOf.terrain].toLowerCase()}.
          </p>
          {partners.length > 0 ? (
            <ul className="atlas-partners">
              {partners.map((p, i) => (
                <li key={i}>
                  {CONTACT_NAME[p.kind]}:{" "}
                  <button type="button" className="link" onClick={() => onSelect(p.other.id)}>
                    {p.other.name}
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted">They deal with no one.</p>
          )}
          <button type="button" onClick={() => onRead(chosen.id)}>
            Read their language
          </button>
        </section>

        <section>
          <h3>Key</h3>
          <ul className="atlas-legend">
            {Object.entries(TERRAIN_NAME).map(([id, name]) => (
              <li key={id}>
                <svg className="swatch" viewBox="0 0 1 1" aria-hidden="true">
                  <rect className={`land terrain-${id}`} width="1" height="1" />
                </svg>
                {name}
              </li>
            ))}
          </ul>
          {contacts ? (
            <ul className="atlas-legend">
              {Object.entries(CONTACT_NAME).map(([id, name]) => (
                <li key={id}>
                  <svg className="swatch line" viewBox="0 0 2 1" aria-hidden="true">
                    <line className={`dealing dealing-${id}`} x1="0" y1="0.5" x2="2" y2="0.5" />
                  </svg>
                  {name}
                </li>
              ))}
            </ul>
          ) : null}
        </section>
      </aside>
    </main>
  );
}
