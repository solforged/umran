import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Layers, Pause, Play, SkipForward } from "lucide-react";
import type { Annal, Catalog, Craft, Engine, Overview, WorldMap } from "../model";
import { YEARS } from "../model";
import { EVENT_KIND, hue } from "../lore";
import { PACES, year } from "../words";
import type { DialogKind } from "./ActionDialog";
import { Told } from "./Told";
import { MapView, type Tint } from "./MapView";
import { Pedia, type Focus } from "./Pedia";
import { SpecimenChanges } from "./Specimen";

/// What stops the years passing on their own.
type PauseOn = "nothing" | "peoples" | "sounds" | "anything";

const PAUSE_ON: [PauseOn, string][] = [
  ["nothing", "nothing"],
  ["peoples", "peoples meeting, parting, moving"],
  ["sounds", "sound changes"],
  ["anything", "anything at all"],
];

function stops(on: PauseOn, annal: Annal): boolean {
  switch (on) {
    case "nothing":
      return false;
    case "peoples":
      // Neighbours and gradual spread happen too often to stop the years for.
      return annal.kind !== "law" && annal.kind !== "neighbours" && annal.kind !== "spread";
    case "sounds":
      return annal.kind === "law";
    case "anything":
      return true;
  }
}

/// Most entries the feed shows.
const FEED_LENGTH = 8;
/// Most cards the back button remembers.
const TRAIL_LENGTH = 40;

/// The world as a stage: the map in the middle, what just happened beside
/// it, the encyclopedia card for whatever is in focus to the side, and
/// time along the bottom.
export function Stage({
  engine,
  catalog,
  map,
  version,
  generation,
  overview,
  title,
  notices,
  canUndo,
  selected,
  onSelect,
  onShelf,
  onExport,
  onRestore,
  onScrub,
  onTick,
  onStop,
  onNextEvent,
  onUndo,
  onDialog,
}: {
  engine: Engine;
  catalog: Catalog;
  map: WorldMap;
  version: number;
  generation: number;
  overview: Overview;
  title: string;
  notices: ReactNode;
  canUndo: boolean;
  selected: number;
  onSelect: (community: number) => void;
  onShelf: () => void;
  /// Open the page for taking names, glossaries, and the world itself out.
  onExport: () => void;
  /// Tell the history again as a telling set aside told it.
  onRestore: (telling: number) => void;
  onScrub: (generation: number) => void;
  /// One generation at the present, while the years pass on their own.
  onTick: () => boolean;
  /// The years stopped passing; a chance to save.
  onStop: () => void;
  onNextEvent: () => void;
  onUndo: () => void;
  onDialog: (kind: DialogKind) => void;
}) {
  const { latest } = overview;
  const atPresent = generation === latest;

  // The encyclopedia's trail of cards; the last is the one open.
  const [trail, setTrail] = useState<Focus[]>([{ kind: "world" }]);
  const focus = trail.at(-1)!;
  const go = (next: Focus) => {
    if (next.kind === "people") onSelect(next.id);
    // Opening the card already open adds nothing to the trail.
    setTrail((t) => (JSON.stringify(t.at(-1)) === JSON.stringify(next) ? t : [...t.slice(-TRAIL_LENGTH), next]));
  };

  // What the map draws beside the lands and peoples.
  const [layers, setLayers] = useState({ names: true, routes: true, contacts: true, states: true });
  const [landLayer, setLandLayer] = useState<"peoples" | "faiths" | "crafts">("peoples");
  const [craft, setCraft] = useState<Craft>("metalworking");

  // Time passing on its own.
  const [playing, setPlaying] = useState(false);
  const [pace, setPace] = useState(PACES[1][0]);
  const [pauseOn, setPauseOn] = useState<PauseOn>("peoples");
  const tick = useRef(onTick);
  tick.current = onTick;
  const stop = useRef(onStop);
  stop.current = onStop;
  useEffect(() => {
    if (!playing) return;
    const timer = window.setInterval(() => {
      if (!tick.current()) setPlaying(false);
    }, 1000 / pace);
    return () => {
      window.clearInterval(timer);
      stop.current();
    };
  }, [playing, pace]);
  // Turning back to an earlier year stops the years passing.
  useEffect(() => {
    if (!atPresent) setPlaying(false);
  }, [atPresent]);

  // What happened in the year in view.
  const fresh = useMemo(
    () => overview.annals.filter((a) => a.generation === overview.generation && a.generation > 0),
    [overview],
  );
  // Something worth stopping for stops the years and opens its card.
  useEffect(() => {
    if (!playing) return;
    const stopper = fresh.find((a) => stops(pauseOn, a));
    if (stopper) {
      setPlaying(false);
      go({ kind: "event", annal: stopper });
    }
    // Only a new year can bring something to stop for.
  }, [fresh]);

  // The map shows whatever the card is about.
  const concept = focus.kind === "word" ? focus.concept : null;
  const law = focus.kind === "law" ? focus.id : focus.kind === "event" ? (focus.annal.laws[0] ?? null) : null;
  const words = useMemo(
    () => (concept ? engine.wordMap(generation, concept) : null),
    // `version` changes whenever the history does.
    [engine, generation, concept, version],
  );
  const tint: Tint = useMemo(() => {
    if (words) return { kind: "words", words };
    if (law) {
      const had = overview.communities.filter((c) => c.ended === null && overview.varieties[c.variety].laws.some((l) => l.id === law));
      return { kind: "change", had: new Set(had.map((c) => c.id)) };
    }
    if (focus.kind === "religion") return { kind: "faiths" };
    if (focus.kind === "craft") return { kind: "crafts", craft: focus.id };
    if (landLayer === "faiths") return { kind: "faiths" };
    if (landLayer === "crafts") return { kind: "crafts", craft };
    return { kind: "peoples" };
  }, [words, law, overview, focus, landLayer, craft]);

  const highlight = useMemo(() => {
    const site = (region: number | undefined) => (region === undefined ? null : map.regions[region].site);
    switch (focus.kind) {
      case "people": {
        const c = overview.communities[focus.id];
        return { chosen: c?.ended === null ? [focus.id] : [], lands: c ? c.lands : [], point: site(c?.region) };
      }
      case "state": {
        const state = overview.states[focus.id];
        const chosen = state?.fell === null ? [state.rulers, ...state.members.filter((m) => m.left === null).map((m) => m.community)] : [];
        return { chosen, lands: state?.lands ?? [], point: site(state?.capital) };
      }
      case "religion": {
        const religion = overview.religions[focus.id];
        const chosen = religion?.followers ?? [];
        return { chosen, lands: chosen.flatMap((id) => overview.communities[id].lands), point: site(religion?.land) };
      }
      case "craft": {
        const craft = overview.crafts.find((c) => c.id === focus.id);
        const chosen = craft?.holders ?? [];
        return { chosen, lands: chosen.flatMap((id) => overview.communities[id].lands), point: null };
      }
      case "land": {
        const here = overview.communities.filter((c) => c.ended === null && c.lands.includes(focus.region)).map((c) => c.id);
        return { chosen: here, lands: [focus.region], point: site(focus.region) };
      }
      case "language":
      case "word": {
        const here = overview.communities.filter((c) => c.ended === null && c.variety === focus.variety);
        return { chosen: here.map((c) => c.id), lands: [], point: site(here[0]?.region) };
      }
      case "event": {
        const peoples = focus.annal.peoples.filter((id) => overview.communities[id]);
        const land = focus.annal.lands.at(-1) ?? overview.communities[peoples[0]]?.region;
        return { chosen: peoples, lands: focus.annal.lands, point: site(land) };
      }
      default:
        return { chosen: [], lands: [], point: null };
    }
  }, [focus, overview, map]);
  // Peoples something just happened to pulse, sound changes aside.
  const beacons = fresh.filter((a) => a.kind !== "law").flatMap((a) => a.peoples);

  const people = overview.communities[selected];

  return (
    <div className="stage">
      <header className="stage-head">
        <nav>
          <button type="button" className="link brand" onClick={onShelf} title="Back to the shelf">
            Umran
          </button>
          <button type="button" className="link" onClick={onExport}>
            Export
          </button>
        </nav>
        <span className="stage-title">{title}</span>
      </header>
      <div className="stage-notices">{notices}</div>

      <section className="stage-map" aria-label="Map">
        <MapView
          map={map}
          overview={overview}
          generation={generation}
          tint={tint}
          names={layers.names}
          routes={layers.routes}
          contacts={layers.contacts}
          states={layers.states}
          chosen={new Set(highlight.chosen)}
          lands={new Set(highlight.lands)}
          beacons={beacons}
          focus={highlight.point}
          zoomable
          onPeople={(id) => go({ kind: "people", id })}
          onLand={(region) => go({ kind: "land", region })}
          onState={(id) => go({ kind: "state", id })}
          onReligion={(id) => go({ kind: "religion", id })}
          onCraft={(id) => go({ kind: "craft", id })}
        />
        <details className="map-layers">
          <summary title="What the map shows">
            <Layers size={16} aria-hidden="true" /> Show
          </summary>
          <label>
            Colour lands by
            <select value={tint.kind}
              onChange={(e) => {
                setLandLayer(e.target.value as typeof landLayer);
                if (focus.kind === "religion" || focus.kind === "craft" || focus.kind === "word" || law) go({ kind: "world" });
              }}>
              <option value="peoples">Language families</option>
              <option value="faiths">Faiths</option>
              <option value="crafts">Crafts</option>
              {tint.kind === "words" ? <option value="words" disabled>Word roots</option> : null}
              {tint.kind === "change" ? <option value="change" disabled>Sound change</option> : null}
            </select>
          </label>
          {tint.kind === "crafts" ? (
            <label>
              Craft
              <select value={tint.craft} onChange={(e) => {
                setCraft(e.target.value as Craft);
                setLandLayer("crafts");
                if (focus.kind === "craft") go({ kind: "craft", id: e.target.value as Craft });
              }}>
                {catalog.crafts.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
              </select>
            </label>
          ) : null}
          {tint.kind === "faiths" ? (
            <>
              <p className="muted small">Grey lands keep their own gods. Diamonds mark founding lands.</p>
              <ul className="atlas-legend">
                {overview.religions.map((religion) => (
                  <li key={religion.id}>
                    <span className="swatch" style={{ background: hue(religion.id) }} />
                    <button type="button" className="link word" onClick={() => go({ kind: "religion", id: religion.id })}>{religion.name}</button>
                  </li>
                ))}
              </ul>
            </>
          ) : tint.kind === "crafts" ? (
            <p className="muted small">Orange lands hold this craft. Squares mark inventors at their heart lands.</p>
          ) : null}
          {(
            [
              ["names", "Names of lands"],
              ["routes", "Roads peoples took"],
              ["contacts", "Rule, and the chosen peoples' dealings"],
              ["states", "States"],
            ] as const
          ).map(([key, label]) => (
            <label key={key}>
              <input
                type="checkbox"
                checked={layers[key]}
                onChange={(e) => setLayers({ ...layers, [key]: e.target.checked })}
              />
              {label}
            </label>
          ))}
        </details>
        <Feed
          annals={overview.annals}
          overview={overview}
          onState={(id) => go({ kind: "state", id })}
          onReligion={(id) => go({ kind: "religion", id })}
          onCraft={(id) => go({ kind: "craft", id })}
          generation={overview.generation}
          open={focus.kind === "event" ? focus.annal : null}
          onPick={(annal) => {
            setPlaying(false);
            go({ kind: "event", annal });
          }}
        />
      </section>

      <Pedia
        trail={trail}
        onReturn={(i) => setTrail((t) => t.slice(0, i + 1))}
        engine={engine}
        catalog={catalog}
        version={version}
        generation={generation}
        overview={overview}
        map={map}
        words={words}
        go={go}
        onScrub={onScrub}
        onPlay={() => setPlaying(true)}
        onRestore={onRestore}
        onDialog={(kind, community) => {
          onSelect(community);
          onDialog(kind);
        }}
      />

      <footer className="timebar">
        <div className="transport">
          <button
            type="button"
            className="play primary"
            disabled={!atPresent}
            title={atPresent ? (playing ? "Stop the years" : "Let the years pass") : "Turn to the present to go on"}
            onClick={() => setPlaying(!playing)}
          >
            {playing ? <Pause size={16} aria-hidden="true" /> : <Play size={16} aria-hidden="true" />}
            {playing ? "Pause" : "Play"}
          </button>
          <select value={pace} aria-label="How quickly" onChange={(e) => setPace(Number(e.target.value))}>
            {PACES.map(([value, name]) => (
              <option key={value} value={value}>
                {name}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="icon"
            disabled={!atPresent || playing}
            title="Until something happens"
            onClick={onNextEvent}
          >
            <SkipForward size={18} />
          </button>
        </div>
        <div className="track scrub">
          <input
            type="range"
            min={0}
            max={latest}
            value={generation}
            disabled={latest === 0}
            aria-label="Year"
            aria-valuetext={year(generation)}
            onChange={(e) => onScrub(Number(e.target.value))}
          />
          <div className="ticks" aria-hidden="true">
            {overview.timeline
              .filter((m) => m.kind !== "run")
              .map((m, i) => (
                <button
                  key={i}
                  type="button"
                  tabIndex={-1}
                  className={`tick tick-${m.kind}`}
                  style={{ left: `${(m.generation / Math.max(latest, 1)) * 100}%` }}
                  title={`In ${year(m.generation)}: ${m.label}`}
                  onClick={() => onScrub(m.generation)}
                />
              ))}
          </div>
        </div>
        <span className="stage-year">
          Year {generation * YEARS}
          {atPresent ? null : (
            <>
              {" · "}
              <button type="button" className="link" onClick={() => onScrub(latest)}>
                to the present
              </button>
            </>
          )}
        </span>
        <label className="pause-on">
          Stop for{" "}
          <select value={pauseOn} onChange={(e) => setPauseOn(e.target.value as PauseOn)}>
            {PAUSE_ON.map(([value, name]) => (
              <option key={value} value={value}>
                {name}
              </option>
            ))}
          </select>
        </label>
        <details className="act">
          <summary>Shape history</summary>
          <div className="act-menu" onClick={(e) => (e.currentTarget.parentElement as HTMLDetailsElement).removeAttribute("open")}>
            {people?.ended === null ? (
              <>
                <button type="button" onClick={() => onDialog("split")}>
                  Some of the {people.name} go their own way
                </button>
                <button type="button" onClick={() => onDialog("connect")}>
                  The {people.name} meet another people
                </button>
                <button type="button" onClick={() => onDialog("shift")}>
                  The {people.name} take up another language
                </button>
              </>
            ) : null}
            <button type="button" onClick={() => onDialog("found")}>
              A new people arrives
            </button>
            <button type="button" onClick={() => onDialog("state")}>
              Found a state
            </button>
            <button type="button" onClick={() => onDialog("religion")}>
              Found a religion
            </button>
            <button type="button" onClick={() => onDialog("craft")}>
              Teach a craft
            </button>
            <button type="button" disabled={!canUndo} onClick={onUndo}>
              Strike out what was last written
            </button>
          </div>
        </details>
      </footer>
    </div>
  );
}

/// What happened most recently, newest first, over the map.
function Feed({
  annals,
  overview,
  onState,
  onReligion,
  onCraft,
  generation,
  open,
  onPick,
}: {
  annals: Annal[];
  overview: Overview;
  onState: (id: number) => void;
  onReligion: (id: number) => void;
  onCraft: (id: Craft) => void;
  generation: number;
  open: Annal | null;
  onPick: (annal: Annal) => void;
}) {
  const recent = annals.slice(-FEED_LENGTH).reverse();
  if (recent.length === 0) return null;
  return (
    <ol className="feed" aria-label="What happened">
      {recent.map((a) => {
        const Icon = EVENT_KIND[a.kind].icon;
        const same = open !== null && open.generation === a.generation && open.text === a.text;
        return (
          <li key={`${a.generation}-${a.kind}-${a.text}`} className={a.generation === generation ? "fresh" : undefined}>
            <button type="button" className="feed-entry" aria-current={same} onClick={() => onPick(a)}>
              <Icon size={14} aria-hidden="true" />
              <span className="feed-year">{a.generation * YEARS}</span>
              <span className="feed-text">
                <Told text={a.text} />
              </span>
              {a.kind === "law" ? <SpecimenChanges words={a.specimen} /> : null}
            </button>
            {a.states.length + a.religions.length + a.crafts.length > 0 ? (
              <div className="feed-states">
                {a.states.filter((id) => overview.states[id]).map((id) => (
                  <button key={`state-${id}`} type="button" className="link word" style={{ color: hue(id) }} onClick={() => onState(id)}>
                    {overview.states[id].name}
                  </button>
                ))}
                {a.religions.filter((id) => overview.religions[id]).map((id) => (
                  <button key={`religion-${id}`} type="button" className="link word" style={{ color: hue(id) }} onClick={() => onReligion(id)}>
                    {overview.religions[id].name}
                  </button>
                ))}
                {a.crafts.map((id) => (
                  <button key={id} type="button" className="link" onClick={() => onCraft(id)}>
                    {overview.crafts.find((c) => c.id === id)?.name}
                  </button>
                ))}
              </div>
            ) : null}
          </li>
        );
      })}
    </ol>
  );
}
