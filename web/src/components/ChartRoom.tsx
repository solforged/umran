import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowRight, FileUp, X } from "lucide-react";
import { landMap } from "../engine";
import { YEARS, type MapSize, type WorldMap } from "../model";
import { bookLand, type BookEntry, type ShelfPeople } from "../shelf";
import { Miniature } from "./MapView";
import { Modal } from "./Modal";
import { Told } from "./Told";
import "./chartroom.css";

const UNKNOWN: WorldMap = { size: "medium", width: 13.5, height: 8.794229, kmPerUnit: 100, regions: [], landmasses: [], rivers: [], climateZones: [] };

function Chart({ land, peoples }: { land: { seed: number; size: MapSize } | null; peoples?: ShelfPeople[] }) {
  const [map, setMap] = useState<WorldMap | null>(null);
  const [seed, size] = [land?.seed, land?.size];
  useEffect(() => {
    if (seed === undefined || size === undefined) return;
    let live = true;
    landMap(seed, size).then((next) => live && setMap(next), () => undefined);
    return () => { live = false; };
  }, [seed, size]);
  return <Miniature map={map ?? UNKNOWN} peoples={map ? peoples : []} />;
}

/// An author's worlds, with the last-opened chart leading and founding
/// tools kept below the worlds rather than offered as their peers.
export function ChartRoom({ revision, books, onOpen, onBegin, onSample, onImport, onRemove }: {
  revision: number;
  books: BookEntry[];
  onOpen: (id: string) => void;
  onBegin: () => void;
  onSample: () => void;
  onImport: (file: File) => void;
  onRemove: (id: string) => void;
}) {
  const [removing, setRemoving] = useState<BookEntry | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const worlds = useMemo(() => [...books].sort((a, b) => b.updated - a.updated)
    .map((book) => ({ book, land: bookLand(book.id) })), [books]);
  const [leading, ...others] = worlds;

  function world({ book, land }: typeof worlds[number], lead: boolean) {
    return <article key={book.id} className={`room-world${lead ? " room-world-leading" : ""}`}>
      <Chart land={land} peoples={book.peoples} />
      <button type="button" className="room-world-open" onClick={() => onOpen(book.id)} aria-label={`${lead ? "Continue" : "Open"} ${book.title}`}>
        <span className="room-world-title">
          <strong>{book.title}</strong>
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

  return <main className={`chart-room${leading ? "" : " chart-room-empty"}`}>
    <header className="room-head cartouche">
      <span className="brand"><span className="brand-name"><span>ʿUmrān</span></span></span>
      <h1>The chart room</h1>
    </header>

    {leading ? <section className="room-worlds" aria-label="Your worlds">
      {world(leading, true)}
      {others.length ? <ul className="room-world-row">
        {others.map((entry) => <li key={entry.book.id}>{world(entry, false)}</li>)}
      </ul> : null}
    </section> : null}

    <section className="room-actions" aria-label="Found or open a world">
      {!leading ? <div className="room-blank-chart" aria-hidden="true"><Chart land={null} /></div> : null}
      <div className="room-action-list">
        <div className="room-action room-found">
          <button type="button" onClick={onBegin}>Found a new world <ArrowRight size={19} aria-hidden="true" /></button>
          {!leading ? <p>Draw its coasts and choose who lives there.</p> : null}
        </div>
        <div className="room-action">
          <button type="button" onClick={onSample}>Read a chronicle already written</button>
          <p>Four thousand years, a conquest, and a new faith.</p>
        </div>
        <div className="room-action">
          <button type="button" onClick={() => fileInput.current?.click()}><FileUp size={16} aria-hidden="true" /> Open a save file</button>
          <input ref={fileInput} type="file" accept="application/json,.json" hidden onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) onImport(file);
            e.target.value = "";
          }} />
        </div>
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
