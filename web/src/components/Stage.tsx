import { useEffect, useMemo, useRef, useState, type ReactNode, type RefObject } from "react";
import { BookOpen, ChevronDown, ChevronUp, Layers, Map as MapIcon, Pause, Play, ScrollText, Search, SkipForward, StepForward, Undo2, X } from "lucide-react";
import type { Annal, Catalog, Craft, Destination, HistoryPoint, ReadEngine, EthosAxis, Overview, SettlementChoice, SettlementPreview, WorldMap } from "../model";
import { YEARS } from "../model";
import { ETHOS_AXES, ETHOS_POLES, EVENT_KIND, hue } from "../lore";
import { PACES } from "../words";
import type { DialogKind, InterventionKind } from "./ActionDialog";
import { message } from "../engine";
import { SettlementDesk, type SettlementDraft } from "./SettlementDesk";
import { concerns, findAnnal, individualAnnals, INITIAL_HISTORY } from "../history";
import { AtlasIndex } from "./AtlasIndex";
import type { DictionaryView } from "./Dictionary";
import { Told } from "./Told";
import { MapView, type MapMotionReading, type Tint } from "./MapView";
import { Pedia, type Focus } from "./Pedia";
import { emphasizeInk, useLiftedValue } from "../motion";
import { LightChoice } from "./LightSwitch";
import { Popover } from "./Popover";
import { Timeline, marksFromAnnals } from "./Timeline";

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
      return annal.kind !== "law" && annal.kind !== "grammar" && annal.kind !== "neighbours" && annal.kind !== "spread";
    case "sounds":
      return annal.kind === "law";
    case "anything":
      return true;
  }
}

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
  sheet,
  canUndo,
  selected,
  onSelect,
  onShelf,
  onExport,
  onRestore,
  onRenameTelling,
  onCompare,
  onScrub,
  onTick,
  onStop,
  onNextEvent,
  onUndo,
  onDialog,
  initialFocus,
  onFocus,
  onSettle,
  onNotebook,
  onKeep,
  onReadPoint,
  comparisonReturn,
  sheetReturn,
  mapMotion,
}: {
  engine: ReadEngine;
  catalog: Catalog;
  map: WorldMap;
  version: number;
  generation: number;
  overview: Overview;
  title: string;
  notices: ReactNode;
  /// A sheet laid over the map: the export page.
  sheet?: ReactNode;
  canUndo: boolean;
  selected: number;
  onSelect: (community: number) => void;
  onShelf: () => void;
  /// Open the page for taking names, glossaries, and the world itself out.
  onExport: () => void;
  /// Tell the history again as a telling set aside told it.
  onRestore: (telling: number) => void;
  onRenameTelling: (telling: number, name: string) => void;
  onCompare: (telling: number) => void;
  onScrub: (generation: number) => HistoryPoint | null;
  /// One generation at the present, while the years pass on their own.
  onTick: () => boolean;
  /// The years stopped passing; a chance to save.
  onStop: () => void;
  onNextEvent: () => void;
  onUndo: () => void;
  onDialog: (kind: DialogKind) => void;
  initialFocus: Focus | null;
  /// The open card changed; the app keeps it as the reader's place.
  onFocus: (focus: Focus) => void;
  onSettle: (choice: SettlementChoice, preview: SettlementPreview) => void;
  onNotebook: () => void;
  onKeep: (subject: Focus, label: string) => void;
  onReadPoint: (point: HistoryPoint) => void;
  comparisonReturn: number;
  sheetReturn: number;
  mapMotion: RefObject<MapMotionReading | null>;
}) {
  const { latest } = overview;
  const atPresent = overview.atTip;

  // The encyclopedia's trail of cards; the last is the one open.
  const [trail, setTrail] = useState<Destination[]>([{ subject: initialFocus ?? { kind: "world" }, reading: { telling: overview.telling, point: overview.point } }]);
  const focus = trail.at(-1)!.subject;
  const focusKey = JSON.stringify(focus);
  useEffect(() => { onFocus(focus); }, [focusKey]); // eslint-disable-line react-hooks/exhaustive-deps
  const destination = (subject: Focus): Destination => ({ subject, reading: { telling: overview.telling, point: overview.point } });
  // The folio page open over the map, by its section's id.
  const [leaf, setLeaf] = useState<string | null>(null);
  const leavesByCard = useRef(new Map<string, string | null>());
  const [instantFolio, setInstantFolio] = useState(false);
  const [folioFromCard, setFolioFromCard] = useState(false);
  const [cardMotion, setCardMotion] = useState({ direction: "none", keyboard: false, ink: false });
  const keyboardActivation = useRef(false);
  const afterLift = useRef<(() => void) | null>(null);
  const comparing = useRef(false);
  const suspendedLeaf = useRef<{ card: string; leaf: string } | null>(null);
  const resumeFolio = () => {
    if (document.querySelector("dialog:modal") || afterLift.current || settlement !== null) return;
    const previous = suspendedLeaf.current;
    suspendedLeaf.current = null;
    if (previous?.card === JSON.stringify(focus)) { comparing.current = false; setInstantFolio(true); setLeaf(previous.leaf); }
  };
  useEffect(resumeFolio, [sheetReturn]);
  const showLeaf = (next: string | null) => {
    setFolioFromCard(false);
    if (next) {
      afterLift.current = null; suspendedLeaf.current = null; comparing.current = false;
      if (liftedSettlement.closing) liftedSettlement.finishLift();
    }
    leavesByCard.current.set(JSON.stringify(focus), next);
    setInstantFolio(false);
    setLeaf(next);
  };
  const [folioHost, setFolioHost] = useState<HTMLDivElement | null>(null);
  const [historyView, setHistoryView] = useState(INITIAL_HISTORY);
  const [storyViews, setStoryViews] = useState<Record<string, typeof INITIAL_HISTORY>>({});
  const [followed, setFollowed] = useState<{ subject: Focus; label: string } | null>(null);
  const [dictionaryViews, setDictionaryViews] = useState<Record<number, DictionaryView>>({});
  const [indexOpen, setIndexOpen] = useState(false);
  useLiftedValue(indexOpen ? true : null, resumeFolio);
  const [pane, setPane] = useState<"map" | "reading">(initialFocus ? "reading" : "map");
  const completeLift = () => {
    const action = afterLift.current;
    afterLift.current = null;
    if (action) action(); else resumeFolio();
  };
  const [settlement, setSettlement] = useState<SettlementDraft | null>(null);
  const settlementOpener = useRef<HTMLElement | null>(null);
  const liftedSettlement = useLiftedValue(settlement, completeLift);
  const desk = liftedSettlement.value;
  const settlementReading = useMemo(() => desk ? engine.overviewAt(desk.point) : overview, [engine, desk?.point, overview]);
  const present = useMemo(() => desk ? engine.overview(engine.latest()) : overview, [engine, desk !== null, overview]);
  const settlementPreview = useMemo(() => {
    if (!desk) return { preview: null, error: null };
    try { return { preview: engine.settlement(desk.point, desk.community, desk.intent, desk.share, desk.destination), error: null }; }
    catch (e) { return { preview: null, error: message(e) }; }
  }, [engine, desk, overview.mutation]);
  const liftThen = (action: () => void, returnTo?: HTMLElement | null) => {
    const caller = returnTo?.closest("details")?.querySelector<HTMLElement>("summary") ?? returnTo;
    const next = () => { caller?.focus({ preventScroll: true }); action(); };
    if (leaf !== null || desk !== null || document.querySelector(".folio")) {
      if (leaf) suspendedLeaf.current = { card: JSON.stringify(focus), leaf };
      afterLift.current = next;
      setLeaf(null); setSettlement(null);
    } else next();
  };
  useEffect(() => {
    if (!comparing.current || document.querySelector("dialog:modal") || afterLift.current || settlement !== null) return;
    comparing.current = false;
    suspendedLeaf.current = null;
    setInstantFolio(true);
    setLeaf("tellings");
  }, [comparisonReturn]);
  // A reader's next choice supersedes any return queued by a lifted sheet.
  const go = (next: Focus) => {
    afterLift.current = null; suspendedLeaf.current = null; comparing.current = false;
    if (liftedSettlement.closing) liftedSettlement.finishLift();
    setFolioFromCard(JSON.stringify(focus) !== JSON.stringify(next));
    setPane("reading");
    leavesByCard.current.set(JSON.stringify(focus), leaf);
    setInstantFolio(false);
    if (JSON.stringify(focus) !== JSON.stringify(next)) {
      const origin = document.activeElement;
      const ink = next.kind === "event" && origin instanceof HTMLElement && !!origin.closest(".moment, .chronicle-latest");
      if (ink && origin instanceof HTMLElement) emphasizeInk(origin);
      setCardMotion({ direction: "forward", keyboard: keyboardActivation.current, ink });
    }
    if (next.kind === "people") onSelect(next.id);
    // The history card is the whole history, so it opens in the folio.
    setLeaf(next.kind === "history" ? "history" : null);
    // Opening the card already open adds nothing to the trail.
    setTrail((t) => {
      const current = [...t.slice(0, -1), destination(t.at(-1)!.subject)];
      return JSON.stringify(t.at(-1)!.subject) === JSON.stringify(next) ? current : [...current.slice(-TRAIL_LENGTH), destination(next)];
    });
  };
  const returnTo = (index: number) => {
    setPlaying(false);
    setFolioFromCard(true);
    setInstantFolio(false);
    setCardMotion({ direction: "back", keyboard: keyboardActivation.current, ink: true });
    setTrail((t) => t.slice(0, index + 1));
    const visit = trail[index];
    setLeaf(leavesByCard.current.get(JSON.stringify(visit.subject)) ?? (visit.subject.kind === "history" ? "history" : null));
    setPane("reading");
    onReadPoint(visit.reading.point);
  };
  // Back navigation and time travel must keep actions attached to the
  // person on the card, just as following a link does.
  useEffect(() => {
    if (focus.kind === "people" && overview.communities[focus.id]) onSelect(focus.id);
    if (focus.kind === "language") {
      const speaker = overview.communities.find((c) => c.variety === focus.variety && c.ended === null);
      if (speaker) onSelect(speaker.id);
    }
  }, [focus, overview.communities, onSelect]);
  // Whether the map is veiled to what the language in view knows. It
  // follows the people or language card open, and lifts on any other.
  const [veiled, setVeiled] = useState(false);
  const knownBy = !veiled ? null
    : focus.kind === "language" ? focus.variety
    : focus.kind === "people" ? (overview.communities[focus.id]?.variety ?? null)
    : null;
  const known = useMemo(
    () => (knownBy === null ? null : overview.varieties[knownBy] ? new Set(overview.varieties[knownBy].knownLands.map((l) => l.region)) : null),
    [knownBy, overview],
  );

  // What the map draws beside the lands and peoples.
  const [layers, setLayers] = useState({ names: true, routes: true, contacts: true, states: true });
  const [landLayer, setLandLayer] = useState<"peoples" | "faiths" | "crafts" | "temper" | "weather">("peoples");
  const [craft, setCraft] = useState<Craft>("metalworking");
  const [axis, setAxis] = useState<EthosAxis>("martial");

  // Time passing on its own.
  const [playing, setPlaying] = useState(false);
  const [pace, setPace] = useState(PACES[1][0]);
  const [pauseOn, setPauseOn] = useState<PauseOn>("peoples");
  const tick = useRef(onTick);
  tick.current = onTick;
  const scrub = (g: number, subject: Focus = focus) => {
    setPlaying(false); setSettlement(null);
    const point = onScrub(g);
    if (point && (point.action !== overview.point.action || point.offset !== overview.point.offset || JSON.stringify(subject) !== JSON.stringify(focus))) {
      leavesByCard.current.set(JSON.stringify(focus), leaf);
      setTrail((t) => [...t.slice(-TRAIL_LENGTH, -1), destination(focus), { subject, reading: { telling: overview.telling, point } }]);
    }
  };
  const openDialog = (kind: InterventionKind, community = selected) => {
    setPlaying(false);
    if (kind === "settlement") {
      settlementOpener.current = document.activeElement instanceof HTMLElement
        ? document.activeElement.closest("details")?.querySelector<HTMLElement>("summary") ?? document.activeElement : null;
      const open = () => {
        setPane("map");
        setSettlement({ community, intent: overview.communities[community].lands.length > 1 ? "partition" : "settlers", share: 0.5, destination: null, point: overview.point });
      };
      if (liftedSettlement.closing) { afterLift.current = null; open(); }
      else liftThen(open, document.activeElement instanceof HTMLElement ? document.activeElement : null);
    } else { liftThen(() => onDialog(kind), document.activeElement instanceof HTMLElement ? document.activeElement : null); }
  };
  const reconsider = (annal: Annal) => {
    if (!annal.settlement || !annal.before) return;
    setPlaying(false);
    settlementOpener.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    liftThen(() => { setPane("map"); setSettlement({ ...annal.settlement!.plan.choice, destination: null, point: annal.before! }); });
  };
  const openIndex = () => {
    setPlaying(false);
    liftThen(() => setIndexOpen(true), document.activeElement instanceof HTMLElement ? document.activeElement : null);
  };
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

  // Shortcuts never steal keys from a form, a dialog, or a focused control.
  useEffect(() => {
    const shortcut = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (target.closest("input, select, textarea, [contenteditable=true], dialog[open]")) return;
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault(); openIndex();
      } else if (!event.metaKey && !event.ctrlKey && !event.altKey && event.key === "/") {
        event.preventDefault(); openIndex();
      }
    };
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  }, []);

  // What happened in the year in view.
  const fresh = useMemo(
    () => overview.annals.filter((a) => a.generation === overview.generation && a.generation > 0),
    [overview],
  );
  // Something worth stopping for stops the years and opens its card.
  useEffect(() => {
    if (!playing) return;
    const stopper = followed
      ? individualAnnals(fresh).find((a) => concerns(a, followed.subject, overview, map))
      : fresh.find((a) => stops(pauseOn, a));
    if (stopper) {
      setPlaying(false);
      go({ kind: "event", id: stopper.id });
    }
    // Only a new year can bring something to stop for.
  }, [fresh]);

  // The map shows whatever the card is about.
  const concept = focus.kind === "word" ? focus.concept : null;
  const focusedEvent = focus.kind === "event" ? findAnnal(overview.annals, focus.id) : undefined;
  const law = focus.kind === "law" ? focus.id : focus.kind === "event" ? (focusedEvent?.laws[0] ?? null) : null;
  const words = useMemo(
    () => (concept ? engine.wordMap(generation, concept) : null),
    // `version` changes whenever the history does.
    [engine, generation, concept, version],
  );
  const chartGeneration = settlement ? settlementReading.generation : generation;
  const climate = useMemo(() => engine.climate(chartGeneration), [engine, chartGeneration, version]);
  const riverNames = useMemo(() => map.rivers.map((river) => engine.river(chartGeneration, river.id)), [engine, map, chartGeneration, version]);
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
    if (landLayer === "temper") return { kind: "temper", axis };
    if (landLayer === "weather") return { kind: "weather" };
    return { kind: "peoples" };
  }, [words, law, overview, focus, landLayer, craft, axis]);

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
        return { chosen, lands: chosen.flatMap((id) => overview.communities[id].lands), point: site(religion?.shrine.region ?? religion?.land) };
      }
      case "continent": {
        const view = overview.continents.find((c) => c.landmass === focus.landmass);
        return { chosen: view?.peoples ?? [], lands: map.landmasses[focus.landmass]?.regions ?? [], point: site(map.landmasses[focus.landmass]?.anchor) };
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
      case "river": {
        const lands = map.rivers[focus.id]?.course ?? [];
        const chosen = overview.communities.filter((c) => c.ended === null && c.lands.some((id) => lands.includes(id))).map((c) => c.id);
        return { chosen, lands, point: site(lands.at(-1)) };
      }
      case "zone": {
        const lands = map.climateZones[focus.id]?.regions ?? [];
        return { chosen: [], lands, point: site(lands[Math.floor(lands.length / 2)]) };
      }
      case "language":
      case "word": {
        const here = overview.communities.filter((c) => c.ended === null && c.variety === focus.variety);
        return { chosen: here.map((c) => c.id), lands: [], point: site(here[0]?.region) };
      }
      case "event": {
        const peoples = (focusedEvent?.peoples ?? []).filter((id) => overview.communities[id]);
        const land = focusedEvent?.lands.at(-1) ?? overview.communities[peoples[0]]?.region;
        return { chosen: peoples, lands: focusedEvent?.lands ?? [], point: site(land) };
      }
      default:
        return { chosen: [], lands: [], point: null };
    }
  }, [focus, overview, map]);
  // Changes to holdings have their own, recorded motion; no decorative pulse.

  // The latest moment written, for the chronicle's line.
  const last = overview.annals.at(-1) ?? null;
  const sameYear = last ? overview.annals.filter((a) => a.generation === last.generation).length : 0;
  const LastIcon = last ? EVENT_KIND[last.kind].icon : null;
  const tellingName = overview.tellings.find((t) => t.id === overview.telling)?.name;
  const { marks, sounds } = useMemo(() => marksFromAnnals(overview.annals), [overview.annals]);

  const layerName = tint.kind === "peoples" ? "Language families"
    : tint.kind === "faiths" ? "Faiths" : tint.kind === "crafts" ? catalog.crafts.find((c) => c.id === tint.craft)?.name
    : tint.kind === "temper" ? `${ETHOS_POLES[tint.axis][0]} — ${ETHOS_POLES[tint.axis][1]}`
    : tint.kind === "weather" ? "Weather"
    : tint.kind === "words" ? `Words for “${concept?.replaceAll("_", " ")}”` : "Sound change";

  return (
    <div className="stage workbench" data-pane={pane} data-settlement={desk !== null}
      onClickCapture={(event) => { keyboardActivation.current = event.detail === 0; }}
      onKeyDownCapture={(event) => { if (event.key === "Enter" || event.key === " ") keyboardActivation.current = true; }}>
      <div className="stage-notices">{notices}{followed ? <p className="following-note">Following {followed.label}. Playback stops when this subject appears in the record. <button type="button" className="link" onClick={() => setFollowed(null)}>Stop following</button></p> : null}</div>

      <section className="stage-map" aria-label="Map">
        <MapView
          map={map}
          overview={settlement ? settlementReading : overview}
          generation={settlement ? settlementReading.generation : generation}
          tint={settlement ? { kind: "peoples" } : tint}
          animateChanges={!playing && !desk}
          motionMemory={mapMotion}
          climate={climate}
          riverNames={riverNames}
          selectedVariety={overview.communities[selected]?.variety}
          names={layers.names}
          routes={!settlement && layers.routes}
          contacts={!settlement && layers.contacts}
          states={!settlement && layers.states}
          chosen={new Set(settlement ? [settlement.community] : highlight.chosen)}
          lands={new Set(highlight.lands)}
          focus={settlement ? map.regions[settlement.destination ?? settlementReading.communities[settlement.community].region].site : highlight.point}
          known={settlement ? null : known}
          settlement={settlementPreview.preview}
          zoomable
          onPeople={(id) => settlement ? setSettlement({ ...settlement, destination: settlementReading.communities[id].region }) : go({ kind: "people", id })}
          onLand={(region) => settlement ? setSettlement({ ...settlement, destination: region }) : go({ kind: "land", region })}
          onContinent={settlement ? undefined : (landmass) => go({ kind: "continent", landmass })}
          onState={(id) => go({ kind: "state", id })}
          onReligion={(id) => go({ kind: "religion", id })}
          onCraft={(id) => go({ kind: "craft", id })}
          onRiver={settlement ? undefined : (id) => go({ kind: "river", id })}
        />
        <div className="cartouche world-cartouche">
          <Popover label="This world" role="menu" side="bottom" align="start"
            trigger={(props) => (
              <button type="button" className="cartouche-open" {...props}>
                <span className="cartouche-kicker">A chart of</span>
                <span className="cartouche-title">{title}</span>
                <span className="cartouche-note">{tellingName} · year {generation * YEARS}</span>
                <ChevronDown size={14} aria-hidden="true" />
              </button>
            )}>
            {(close) => (<>
              <button type="button" onClick={() => { close(); setPlaying(false); go({ kind: "history" }); setLeaf("tellings"); }}>Other tellings</button>
              <button type="button" onClick={() => { close(); setPlaying(false); liftThen(onNotebook, document.querySelector<HTMLElement>(".cartouche-open")); }}>Field notebook</button>
              <button type="button" onClick={() => { close(); onExport(); }}>Export…</button>
              <hr />
              <LightChoice />
              <hr />
              <button type="button" onClick={() => { close(); onShelf(); }}>Back to the shelf</button>
            </>)}
          </Popover>
        </div>
        <div className="map-tools">
          <button type="button" className="map-tool" onClick={openIndex} title="Find anything (⌘K or /)"><Search size={15} aria-hidden="true" /><span className="map-tool-label sr-only">Find</span></button>
          <Popover role="dialog" label="What the map shows" side="bottom" align="end"
            trigger={(props) => <button type="button" className="map-tool" title={layerName} {...props}><Layers size={15} aria-hidden="true" /><span className="map-tool-label sr-only">{layerName}</span><ChevronDown size={12} aria-hidden="true" /></button>}>
            {() => <div className="layers-menu">
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
              <option value="temper">Temper</option>
              <option value="weather">Weather</option>
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
          {tint.kind === "temper" ? (
            <label>
              Leaning
              <select value={tint.axis} onChange={(e) => setAxis(e.target.value as EthosAxis)}>
                {ETHOS_AXES.map((a) => (
                  <option key={a} value={a}>{ETHOS_POLES[a][0]} or {ETHOS_POLES[a][1]}</option>
                ))}
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
          ) : tint.kind === "temper" ? (
            <p className="muted small">
              Orange lands lean {ETHOS_POLES[tint.axis][1]}, blue lands {ETHOS_POLES[tint.axis][0]}; the deeper the colour, the stronger the leaning.
            </p>
          ) : tint.kind === "weather" ? (
            <ul className="atlas-legend weather-legend">
              <li><span className="swatch weather-dry" /> Drier</li>
              <li><span className="swatch weather-wet" /> Wetter</li>
              <li><span className="swatch weather-cold" /> Colder</li>
              <li><span className="swatch weather-usual" /> Near usual</li>
            </ul>
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
            </div>}
          </Popover>
        </div>
        {settlement === null ? <div className="map-caption">
          {last && LastIcon ? (
            <button
              type="button"
              className="chronicle-latest"
              title="Open this moment"
              onClick={() => {
                setPlaying(false);
                go({ kind: "event", id: last.id });
              }}
            >
              <LastIcon size={14} aria-hidden="true" />
              <span className="chronicle-year">{last.generation * YEARS}</span>
              <span className="chronicle-text">
                <Told text={last.text} />
              </span>
            </button>
          ) : (
            <span className="chronicle-latest muted">Nothing is written yet.</span>
          )}
          {sameYear > 1 ? <span className="chronicle-more muted">and {sameYear - 1} more that year</span> : null}
          <button type="button" className="link chronicle-open" onClick={() => go({ kind: "history" })}>
            <ScrollText size={14} aria-hidden="true" /> Chronicle
          </button>
        </div> : null}
        {known !== null ? <div className="map-perspective">
          Known to {overview.varieties[knownBy!]?.name}
          <button type="button" className="icon" aria-label="Show the whole known and unknown world" onClick={() => setVeiled(false)}><X size={14} /></button>
        </div> : null}
      </section>
      <div className="folio-host" ref={setFolioHost} />
      {sheet ? <div className="sheet-host">{sheet}</div> : null}

      {desk ? <SettlementDesk draft={desk} closing={liftedSettlement.closing} preview={settlementPreview.preview} error={settlementPreview.error}
        overview={settlementReading} map={map} catalog={catalog} latest={present} onChange={setSettlement}
        onCancel={() => { afterLift.current = null; setPane("reading"); setSettlement(null); }} onCommit={onSettle} returnFocus={settlementOpener.current} /> : null}
      <Pedia
        hidden={desk !== null && !liftedSettlement.closing}
        cardMotion={cardMotion}
        instantFolio={instantFolio}
        folioFromCard={folioFromCard}
        onFolioLifted={completeLift}
        trail={trail.map((visit) => visit.subject)}
        onReturn={returnTo}
        onIndex={openIndex}
        historyView={historyView}
        onHistoryView={setHistoryView}
        storyView={storyViews[JSON.stringify(focus)] ?? INITIAL_HISTORY}
        onStoryView={(next) => setStoryViews((views) => ({ ...views, [JSON.stringify(focus)]: next }))}
        onKeep={(subject, label) => { setPlaying(false); liftThen(() => onKeep(subject, label), document.activeElement instanceof HTMLElement ? document.activeElement : null); }}
        onFollow={(subject, label) => { setPlaying(false); setFollowed({ subject, label }); }}
        dictionaryViews={dictionaryViews}
        onDictionaryView={(variety, view) => setDictionaryViews((views) => ({ ...views, [variety]: view }))}
        engine={engine}
        catalog={catalog}
        version={version}
        generation={generation}
        overview={overview}
        map={map}
        words={words}
        go={go}
        onScrub={scrub}
        onVisit={(subject, year) => scrub(year, subject)}
        onPlay={() => setPlaying(true)}
        canUndo={canUndo}
        onUndo={onUndo}
        onRestore={onRestore}
        onRenameTelling={onRenameTelling}
        onCompare={(telling) => {
          comparing.current = true;
          liftThen(() => onCompare(telling), document.querySelector<HTMLElement>(".leaf-title[data-leaf=tellings]"));
        }}
        leaf={leaf}
        onLeaf={showLeaf}
        folioHost={folioHost}
        knownBy={knownBy}
        onKnownBy={(v) => setVeiled(v !== null)}
        onDialog={(kind, community) => {
          onSelect(community);
          openDialog(kind, community);
        }}
        onReconsider={reconsider}
      />

      <footer className="timebar stage-bar" inert={settlement !== null}>
        <div className="play-split">
          <button
            type="button"
            className="play primary"
            disabled={!atPresent}
            title={atPresent ? (playing ? "Stop the years" : "Let the years pass") : "Turn to this telling’s latest year to go on"}
            onClick={() => setPlaying(!playing)}
          >
            {playing ? <Pause size={16} aria-hidden="true" /> : <Play size={16} aria-hidden="true" />}
            {playing ? "Pause" : "Play"}
          </button>
          <Popover label="How the years pass" role="dialog" side="top" align="start"
            trigger={(props) => <button type="button" className="play-more primary" aria-label="How the years pass" {...props}><ChevronUp size={14} aria-hidden="true" /></button>}>
            {() => (
              <div className="pace-menu">
                <fieldset><legend>How quickly</legend>{PACES.map(([value, name]) => <label key={value}><input type="radio" name="pace" checked={pace === value} onChange={() => setPace(value)} /> {name}</label>)}</fieldset>
                <fieldset><legend>Stop for</legend>{PAUSE_ON.map(([value, name]) => <label key={value}><input type="radio" name="pause" checked={pauseOn === value} onChange={() => setPauseOn(value)} /> {name}</label>)}</fieldset>
                {followed ? <p className="small">Following {followed.label}. <button type="button" className="link" onClick={() => setFollowed(null)}>Stop following</button></p>
                  : <p className="small muted">To follow one subject, open its card and choose “Follow”.</p>}
              </div>
            )}
          </Popover>
        </div>
        <button type="button" className="icon step-year" disabled={playing}
          title="Advance 25 years" aria-label="Advance 25 years" onClick={() => { onTick(); onStop(); }}>
          <StepForward size={18} aria-hidden="true" />
        </button>
        <button
          type="button"
          className="icon"
          disabled={playing}
          title="Until something happens"
          aria-label="Until something happens"
          onClick={onNextEvent}
        >
          <SkipForward size={18} />
        </button>
        <Timeline marks={marks} sounds={sounds} latest={latest} viewed={generation}
          onScrub={(g) => scrub(g)} onOpen={(mark) => { scrub(mark.generation); go({ kind: "event", id: mark.annal.id }); }} />
        <span className={`stage-year${atPresent ? "" : " in-past"}`}>Year {generation * YEARS}{atPresent ? null : <> · <button type="button" className="link" onClick={() => scrub(latest)}>to the present</button></>}</span>
        <button type="button" className="icon strike-out" disabled={!canUndo} title="Return before the last action" aria-label="Return before the last action" onClick={onUndo}><Undo2 size={16} aria-hidden="true" /></button>
        <nav className="stage-panes" aria-label="Workspace">
          <button type="button" aria-pressed={pane === "map"} onClick={() => { setPane("map"); setLeaf(null); }}><MapIcon size={16} aria-hidden="true" /> Chart</button>
          <button type="button" aria-pressed={pane === "reading"} onClick={() => setPane("reading")}><BookOpen size={16} aria-hidden="true" /> Reading</button>
        </nav>
      </footer>
      <AtlasIndex open={indexOpen} overview={overview} map={map} go={go} onClose={() => setIndexOpen(false)} />
    </div>
  );
}
