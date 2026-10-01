import { useMemo, useState } from "react";
import type { Community, ContactKind, Engine, Overview, PlaceName, Terrain, WorldMap } from "../model";
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
/// Generations over which a route fades to its faintest.
const ROUTE_FADE = 40;

/// A colour for the `n`th family or root, far from its neighbours in hue.
function hue(n: number): string {
  return `hsl(${Math.round((n * 137.508) % 360)} 55% 48%)`;
}

/// A gently curved path from `a` to `b`, stopping short of `b` so its
/// arrowhead does not cover the label there.
function route([ax, ay]: [number, number], [bx, by]: [number, number]): string {
  const [dx, dy] = [bx - ax, by - ay];
  const length = Math.hypot(dx, dy) || 1;
  const end: [number, number] = [bx - (dx / length) * 0.18, by - (dy / length) * 0.18];
  const bend: [number, number] = [ax + dx / 2 - dy * 0.2, ay + dy / 2 + dx * 0.2];
  return `M${ax},${ay} Q${bend[0]},${bend[1]} ${end[0]},${end[1]}`;
}

/// The book's map: every people where it lives, coloured by language
/// family, with the dealings between them and the roads they took, or
/// every people's word for one meaning, coloured by the root it descends
/// from. Each land carries the name its holders give it. It follows the
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
  const [names, setNames] = useState(true);
  const [routes, setRoutes] = useState(true);
  // A land picked on the map; otherwise the chosen people's land.
  const [picked, setPicked] = useState<number | null>(null);
  const chosen = overview.communities[selected];
  const choose = (community: number) => {
    setPicked(null);
    onSelect(community);
  };

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

  const placeOf = useMemo(() => new Map(overview.places.map((p) => [p.region, p.names])), [overview.places]);

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
  const nameOf = (region: number): string | null => placeOf.get(region)?.at(-1)?.spelled ?? null;
  const wanderings = overview.moves.filter((m) => m.community === chosen.id);

  const land = picked ?? chosen.region;
  const landRegion = map.regions[land];
  const landNames = placeOf.get(land) ?? [];
  const dwellers = placed.byRegion.get(land) ?? [];

  return (
    <main className="atlas">
      <figure className="atlas-map">
        <svg
          viewBox={`0 0 ${map.width} ${map.height}`}
          role="img"
          aria-label={`Map of ${map.regions.length} lands in year ${generation * YEARS}`}
        >
          <defs>
            <marker
              id="route-head"
              viewBox="0 0 10 10"
              refX="6"
              refY="5"
              markerWidth="5"
              markerHeight="5"
              orient="auto-start-reverse"
            >
              <path className="route-head" d="M0,0 L10,5 L0,10 z" />
            </marker>
          </defs>
          <g className="lands">
            {map.regions.map((r) => {
              const colour = tint(r.id);
              const points = r.outline.map(([x, y]) => `${x},${y}`).join(" ");
              const sea = r.terrain === "sea";
              return (
                <g key={r.id} onClick={sea ? undefined : () => setPicked(r.id)}>
                  <title>{sea ? "Sea" : (nameOf(r.id) ?? `Unnamed ${TERRAIN_NAME[r.terrain].toLowerCase()}`)}</title>
                  <polygon
                    className={`land terrain-${r.terrain}${sea ? "" : " open"}${r.id === land ? " shown" : ""}`}
                    points={points}
                  />
                  {colour ? <polygon className="claim" points={points} style={{ fill: colour }} /> : null}
                </g>
              );
            })}
          </g>
          {names ? (
            <g className="place-names" aria-hidden="true">
              {overview.places.map((p) => {
                const [x, y] = map.regions[p.region].site;
                const here = placed.byRegion.get(p.region)?.length ?? 0;
                const top = here > 0 ? y - (here / 2) * LINE - 0.06 : y;
                return (
                  <text key={p.region} x={x} y={top} className={here > 0 ? "place-name" : "place-name left"}>
                    {p.names.at(-1)!.spelled}
                  </text>
                );
              })}
            </g>
          ) : null}
          {routes ? (
            <g className="routes">
              {overview.moves.map((m, i) => {
                const mover = overview.communities[m.community];
                const age = generation - m.generation;
                const mine = m.community === chosen.id;
                return (
                  <path
                    key={i}
                    className={`route${m.overseas ? " overseas" : ""}${mine ? " chosen" : ""}`}
                    d={route(map.regions[m.from].site, map.regions[m.to].site)}
                    markerEnd="url(#route-head)"
                    style={{
                      stroke: hue(family(mover)),
                      strokeOpacity: mine ? 1 : 0.3 + 0.6 * Math.max(0, 1 - age / ROUTE_FADE),
                    }}
                  >
                    <title>
                      {`${mover.name} ${m.kind === "split" ? "went out" : "moved"} to ${nameOf(m.to) ?? "new land"}${m.overseas ? " by sea" : ""}, year ${m.generation * YEARS}`}
                    </title>
                  </path>
                );
              })}
            </g>
          ) : null}
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
                  onClick={() => choose(c.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      choose(c.id);
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
            <input type="checkbox" checked={names} onChange={(e) => setNames(e.target.checked)} />
            Names of lands
          </label>
          <label>
            <input type="checkbox" checked={routes} onChange={(e) => setRoutes(e.target.checked)} />
            Roads peoples took
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
            {TERRAIN_NAME[regionOf.terrain].toLowerCase()}
            {nameOf(chosen.region) ? (
              <>
                {" "}
                in <span className="word">{nameOf(chosen.region)}</span>
              </>
            ) : null}
            .
          </p>
          {wanderings.length > 0 ? (
            <ul className="atlas-partners">
              {wanderings.map((m, i) => (
                <li key={i}>
                  Year {m.generation * YEARS}: {m.kind === "split" ? "went out to" : "moved to"}{" "}
                  <button type="button" className="link" onClick={() => setPicked(m.to)}>
                    {nameOf(m.to) ?? "new land"}
                  </button>
                  {m.overseas ? " over the sea" : ""}
                </li>
              ))}
            </ul>
          ) : null}
          {partners.length > 0 ? (
            <ul className="atlas-partners">
              {partners.map((p, i) => (
                <li key={i}>
                  {CONTACT_NAME[p.kind]}:{" "}
                  <button type="button" className="link" onClick={() => choose(p.other.id)}>
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

        <section className="atlas-land">
          <h3 className="word">{landNames.at(-1)?.spelled ?? "A land without a name"}</h3>
          <p className="muted">
            {landRegion.coastal ? "Coastal " : ""}
            {landRegion.coastal
              ? TERRAIN_NAME[landRegion.terrain].toLowerCase()
              : TERRAIN_NAME[landRegion.terrain]}
            {landRegion.island ? ", an island" : ""}.{" "}
            {dwellers.length > 0 ? (
              <>
                Home of{" "}
                {dwellers.map((c, i) => (
                  <span key={c.id}>
                    {i > 0 ? ", " : ""}
                    <button type="button" className="link" onClick={() => choose(c.id)}>
                      {c.name}
                    </button>
                  </span>
                ))}
                .
              </>
            ) : landNames.length > 0 ? (
              "No one lives here now."
            ) : (
              "No one has lived here."
            )}
          </p>
          {landNames.length > 0 ? (
            <ol className="place-history">
              {landNames.map((n, i) => (
                <li key={i}>
                  <span className="word">{n.spelled}</span> <span className="ipa">/{n.ipa}/</span>{" "}
                  <span className="muted">
                    {howNamed(n, landNames[i - 1], overview)}, year {n.since * YEARS}
                    {n.once ? `; once ${n.once}` : ""}
                  </span>
                </li>
              ))}
            </ol>
          ) : null}
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
          {routes ? (
            <ul className="atlas-legend">
              <li>
                <svg className="swatch line" viewBox="0 0 2 1" aria-hidden="true">
                  <line className="route" x1="0" y1="0.5" x2="2" y2="0.5" />
                </svg>
                Road over land
              </li>
              <li>
                <svg className="swatch line" viewBox="0 0 2 1" aria-hidden="true">
                  <line className="route overseas" x1="0" y1="0.5" x2="2" y2="0.5" />
                </svg>
                Voyage over the sea
              </li>
            </ul>
          ) : null}
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

/// How a land came by one of its names, given the name before it.
function howNamed(name: PlaceName, before: PlaceName | undefined, overview: Overview): string {
  const meaning = `“${name.meaning}”`;
  switch (name.origin) {
    case "coined": {
      const by = name.by === null ? null : overview.communities[name.by]?.name;
      return `${meaning}, named in ${name.language}${by ? ` by the ${by}` : ""}`;
    }
    case "borrowed":
      return `${meaning}, ${before ? `${before.spelled} ` : ""}as ${name.language} heard it`;
    case "inherited":
      return `came down into ${name.language}`;
    case "kept":
      return `kept when its people took up ${name.language}`;
  }
}
