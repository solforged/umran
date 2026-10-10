import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowRight, FileUp, X } from "lucide-react";
import { landMap } from "../engine";
import { YEARS, type WorldMap } from "../model";
import { SAMPLE_LAND } from "../sample";
import { bookLand, type BookEntry, type BookLand, type ShelfPeople } from "../shelf";
import { Miniature } from "./MapView";
import { Modal } from "./Modal";
import { Told } from "./Told";
import "./chartroom.css";

const UNKNOWN: WorldMap = { size: "medium", geography: "continental-v4", radiusKm: 1600, width: 2 * Math.PI * 1600 / 100, height: Math.PI * 1600 / 100, kmPerUnit: 100, regions: [], landmasses: [], rivers: [], lakes: [], climateZones: [] };
/// What founding asks, in order; the title page lists them before the first.
const FOUNDING_STEPS = ["The world", "The homeland", "The peoples", "Their speech", "The title page", "The years"];

function Chart({ land, peoples }: { land: BookLand | null; peoples?: ShelfPeople[] }) {
  const [loaded, setLoaded] = useState<(BookLand & { map: WorldMap }) | null>(null);
  const [seed, size, geography] = [land?.seed, land?.size, land?.geography];
  useEffect(() => {
    if (seed === undefined || size === undefined || geography === undefined) return;
    let live = true;
    landMap(seed, size, geography).then((map) => live && setLoaded({ seed, size, geography, map }), () => undefined);
    return () => { live = false; };
  }, [seed, size, geography]);
  // Unsupported recipes have no map. Never reuse a previously loaded chart
  // or overlay saved region IDs while another recipe's geography is loading.
  const map = loaded && loaded.seed === seed && loaded.size === size && loaded.geography === geography ? loaded.map : null;
  return <Miniature map={map ?? UNKNOWN} peoples={map ? peoples : []} />;
}

/// An author's worlds, with the last-opened chart leading and founding
/// tools kept below the worlds rather than offered as their peers. With no
/// worlds yet, it is a title page: what Umran writes, and two ways in.
export function ChartRoom({ revision, books, onOpen, onBegin, onSample, onImport, onRemove }: {
  revision: number;
  books: BookEntry[];
  onOpen: (id: string) => void;
  onBegin: () => void;
  onSample: () => Promise<void>;
  onImport: (file: File) => void;
  onRemove: (id: string) => void;
}) {
  const [removing, setRemoving] = useState<BookEntry | null>(null);
  const [sampling, setSampling] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);
  const worlds = useMemo(() => [...books].sort((a, b) => b.updated - a.updated)
    .map((book) => ({ book, land: bookLand(book.id, revision) })), [books, revision]);
  const [leading, ...others] = worlds;
  // Writing the sample holds the page for a few seconds; say so first.
  const sample = () => {
    setSampling(true);
    setTimeout(() => void onSample().finally(() => setSampling(false)), 30);
  };
  const sampleLabel = sampling ? "Writing four thousand years…" : "Read a chronicle already written";
  const openFile = <>
    <button type="button" onClick={() => fileInput.current?.click()}><FileUp size={16} aria-hidden="true" /> Open a save file</button>
    <input ref={fileInput} type="file" accept="application/json,.json" hidden onChange={(e) => {
      const file = e.target.files?.[0];
      if (file) onImport(file);
      e.target.value = "";
    }} />
  </>;

  function world({ book, land }: typeof worlds[number], lead: boolean) {
    return <article key={book.id} className={`room-world${lead ? " room-world-leading" : ""}`}>
      <Chart land={land} peoples={book.peoples} />
      <button type="button" className="room-world-open" onClick={() => onOpen(book.id)} aria-label={`${lead ? "Continue" : "Open"} ${book.title}`}>
        <span className="room-world-title">
          <strong>{book.title}</strong>
          {book.author ? <span className="room-world-author">by {book.author}</span> : null}
          <small>year {book.generation * YEARS}</small>
          <span className="room-last-line"><Told text={book.subtitle} /></span>
          {lead ? <span className="room-continue">Continue <ArrowRight size={17} aria-hidden="true" /></span> : null}
        </span>
      </button>
      <button type="button" className="icon room-remove" title={`Remove ${book.title}`} aria-label={`Remove ${book.title}`} onClick={() => setRemoving(book)}>
        <X size={15} />
      </button>
    </article>;
  }

  if (!leading) return <main className="chart-room chart-room-empty" data-engine-revision={revision}>
    <section className="room-title-page">
      <div className="room-title-text">
        <span className="brand"><span className="brand-name"><span>ʿUmrān</span></span></span>
        <h1>The book of a world</h1>
        <p className="room-lede">Found peoples on a world’s coasts, let the centuries run, and read what became of them and of their speech.
          Every sound change is regular, and every borrowed word has a meeting behind it.</p>
        <button type="button" className="primary room-found-first" onClick={onBegin}>Found a new world <ArrowRight size={19} aria-hidden="true" /></button>
        <ol className="room-steps" aria-label="Founding, step by step">{FOUNDING_STEPS.map((step, i) => <li key={step}><span>{i + 1}</span> {step}</li>)}</ol>
        <p className="room-other">{openFile}</p>
      </div>
      <figure className="room-sample">
        <button type="button" className="room-sample-chart" disabled={sampling} onClick={sample} aria-label={sampleLabel}>
          <Chart land={SAMPLE_LAND} />
        </button>
        <figcaption>
          <button type="button" className="link" disabled={sampling} onClick={sample}>{sampleLabel}</button>
          <span>Four thousand years on this chart: three peoples, a conquest, and a new faith.</span>
        </figcaption>
      </figure>
    </section>
    <footer className="room-colophon">Worlds live in this browser; the book page exports them as save files or text.</footer>
  </main>;

  return <main className="chart-room">
    <header className="room-head cartouche">
      <span className="brand"><span className="brand-name"><span>ʿUmrān</span></span></span>
      <h1>The chart room</h1>
    </header>

    <section className="room-worlds" aria-label="Your worlds">
      {world(leading, true)}
      {others.length ? <ul className="room-world-row">
        {others.map((entry) => <li key={entry.book.id}>{world(entry, false)}</li>)}
      </ul> : null}
    </section>

    <section className="room-actions" aria-label="Found or open a world">
      <div className="room-action-list">
        <div className="room-action room-found">
          <button type="button" onClick={onBegin}>Found a new world <ArrowRight size={19} aria-hidden="true" /></button>
        </div>
        <div className="room-action">
          <button type="button" disabled={sampling} onClick={sample}>{sampleLabel}</button>
          <p>Four thousand years, a conquest, and a new faith.</p>
        </div>
        <div className="room-action">{openFile}</div>
      </div>
    </section>

    <footer className="room-colophon" data-engine-revision={revision}>
      Worlds live in this browser; the book page exports them as save files or text.
    </footer>

    <Modal open={removing !== null} title="Remove this world?" onClose={() => setRemoving(null)} footer={<>
      <button type="button" onClick={() => setRemoving(null)}>Keep it</button>
      <button type="button" className="primary" onClick={() => { if (removing) onRemove(removing.id); setRemoving(null); }}>Remove for good</button>
    </>}>
      <p>{removing?.title} will be deleted from this browser. If you might want it again, export it from the book page as a save file first.</p>
    </Modal>
  </main>;
}
