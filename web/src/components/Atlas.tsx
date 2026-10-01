import { useMemo, useState } from "react";
import type { Community, Engine, Law, Overview, WorldMap } from "../model";
import { YEARS } from "../model";
import { CONTACT_NAME, howCame, howNamed, hue, TERRAIN_NAME } from "../lore";
import { MapView, peoplesByRegion, type Tint } from "./MapView";

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
  const [show, setShow] = useState<"peoples" | "words" | "change">("peoples");
  const [law, setLaw] = useState<string | null>(null);
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

  const byRegion = useMemo(() => peoplesByRegion(overview), [overview]);
  const placeOf = useMemo(() => new Map(overview.places.map((p) => [p.region, p.names])), [overview.places]);
  const family = (c: Community) => overview.varieties[c.variety].family;

  // Every sound change some living people has undergone, with who has
  // it, the most widespread first. The substrate merges of a language
  // shift are each their own, so they are left out.
  const changes = useMemo(() => {
    const byId = new Map<string, { id: string; label: string; had: Map<number, Law> }>();
    for (const c of overview.communities) {
      for (const l of overview.varieties[c.variety].laws) {
        if (l.id === "substrate") continue;
        const entry = byId.get(l.id) ?? { id: l.id, label: l.label, had: new Map() };
        entry.had.set(c.id, l);
        byId.set(l.id, entry);
      }
    }
    return [...byId.values()].sort((a, b) => b.had.size - a.had.size || a.label.localeCompare(b.label));
  }, [overview]);
  // By default the most widespread change that stopped short of someone.
  const change =
    changes.find((c) => c.id === law) ??
    changes.find((c) => c.had.size < overview.communities.length) ??
    changes[0] ??
    null;

  const tint: Tint =
    show === "words" && words
      ? { kind: "words", words }
      : show === "change" && change
        ? { kind: "change", had: new Set(change.had.keys()) }
        : { kind: "peoples" };

  const groups = useMemo(() => {
    if (!words) return [];
    const seen = new Map<number, string[]>();
    for (const w of words.words) seen.set(w.group, [...(seen.get(w.group) ?? []), w.spelled]);
    return [...seen.entries()].map(([group, forms]) => ({ group, forms: [...new Set(forms)] }));
  }, [words]);

  const regionOf = map.regions[chosen.region];
  const partners = overview.contacts
    .filter((k) => k.a === chosen.id || k.b === chosen.id)
    .map((k) => ({ kind: k.kind, other: overview.communities[k.a === chosen.id ? k.b : k.a] }));
  const nameOf = (region: number): string | null => placeOf.get(region)?.at(-1)?.spelled ?? null;
  const wanderings = overview.moves.filter((m) => m.community === chosen.id);

  const land = picked ?? chosen.region;
  const landRegion = map.regions[land];
  const landNames = placeOf.get(land) ?? [];
  const dwellers = byRegion.get(land) ?? [];

  return (
    <main className="atlas">
      <figure className="atlas-map">
        <MapView
          map={map}
          overview={overview}
          generation={generation}
          tint={tint}
          names={names}
          routes={routes}
          contacts={contacts}
          chosen={new Set([chosen.id])}
          lands={new Set([land])}
          onPeople={choose}
          onLand={setPicked}
        />
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
            <input
              type="radio"
              name="show"
              checked={show === "change"}
              disabled={!change}
              onChange={() => setShow("change")}
            />
            A sound change{" "}
            <select
              value={change?.id ?? ""}
              disabled={!change}
              onChange={(e) => {
                setLaw(e.target.value);
                setShow("change");
              }}
            >
              {changes.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.label} ({c.had.size})
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

        {show === "change" && change ? (
          <section>
            <h3>{change.label}</h3>
            <p className="muted">
              Lines mark where the change stopped: an isogloss. Changes spread most readily between close kin
              living side by side.
            </p>
            <ul className="atlas-partners">
              {overview.communities.map((c) => {
                const had = change.had.get(c.id);
                return (
                  <li key={c.id}>
                    <button type="button" className="link" onClick={() => choose(c.id)}>
                      {c.name}
                    </button>
                    : {had ? howCame(had, c, overview) : <span className="muted">not undergone</span>}
                  </li>
                );
              })}
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
