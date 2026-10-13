import { useEffect, useMemo, useRef, useState } from "react";
import { Archive, ArrowRight, FileUp } from "lucide-react";
import { landMap } from "../engine";
import { YEARS, type WorldMap } from "../model";
import { SAMPLE_LAND } from "../sample";
import { bookLand, isBookEntry, type BookEntry, type BookLand, type DraftEntry, type Shelf, type ShelfPeople } from "../shelf";
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
export function ChartRoom({ revision, shelf, opening, onOpen, onBegin, onSample, onImport, onPutAway, onBringBack, onRemoveForGood, onDownload, onResumeDraft, onDiscardDraft, onFailure }: {
  revision: number;
  shelf: Shelf;
  opening: string | null;
  onOpen: (id: string) => void;
  onBegin: () => void;
  onSample: () => Promise<void>;
  onImport: (file: File) => void;
  onPutAway: (id: string) => boolean;
  onBringBack: (id: string) => boolean;
  onRemoveForGood: (id: string) => boolean;
  onDownload: (id: string) => void;
  onResumeDraft: (id: string) => void;
  onDiscardDraft: (id: string) => boolean;
  onFailure: (error: Error) => void;
}) {
  const [removing, setRemoving] = useState<BookEntry | null>(null);
  const [discarding, setDiscarding] = useState<DraftEntry | null>(null);
  const [undone, setUndone] = useState<BookEntry | null>(null);
  const [sampling, setSampling] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);
  const worlds = useMemo(() => shelf.books.every(isBookEntry)
    ? [...shelf.books].sort((a, b) => a.id === shelf.last ? -1 : b.id === shelf.last ? 1 : b.updated - a.updated)
      .map((book) => ({ book, land: bookLand(book.id, revision) }))
    : null, [shelf.books, shelf.last, revision]);
  useEffect(() => {
    if (worlds === null) onFailure(new Error("A saved world's shelf entry could not be read."));
  }, [worlds, onFailure]);
  const [leading, ...others] = worlds ?? [];
  const busy = sampling || opening !== null;
  const empty = !leading && !shelf.drafts.length && !shelf.putAway.length;
  const sample = async () => {
    setSampling(true);
    try { await onSample(); }
    finally { setSampling(false); }
  };
  const sampleLabel = sampling ? "Opening the chronicle…" : "Read a chronicle already written";
  const openFile = <>
    <button type="button" disabled={busy} onClick={() => fileInput.current?.click()}><FileUp size={16} aria-hidden="true" /> Open a save file</button>
    <input ref={fileInput} type="file" accept="application/json,.json" hidden onChange={(e) => {
      const file = e.target.files?.[0];
      if (file) onImport(file);
      e.target.value = "";
    }} />
  </>;

  function world({ book, land }: NonNullable<typeof worlds>[number], lead: boolean) {
    const isOpening = opening === book.id;
    return <article key={book.id} className={`room-world${lead ? " room-world-leading" : ""}`} aria-busy={isOpening}>
      <Chart land={land} peoples={book.peoples} />
      <button type="button" className="room-world-open" disabled={busy} onClick={() => onOpen(book.id)} aria-label={`${lead ? "Continue" : "Open"} ${book.title}`}>
        <span className="room-world-title">
          <strong>{book.title}</strong>
          {book.author ? <span className="room-world-author">by {book.author}</span> : null}
          <small>year {book.generation * YEARS}</small>
          <span className="room-last-line"><Told text={book.subtitle} /></span>
          {isOpening ? <span className="room-continue" role="status">Opening this world…</span>
            : lead ? <span className="room-continue">Continue <ArrowRight size={17} aria-hidden="true" /></span> : null}
        </span>
      </button>
      <button type="button" className="room-put-away" disabled={busy} aria-label={`Put away ${book.title}`} onClick={() => {
        if (onPutAway(book.id)) setUndone(book);
      }}><Archive size={15} aria-hidden="true" /> Put away</button>
    </article>;
  }

  if (worlds === null) return <main className="chart-room">
    <p role="alert">The shelf could not be read. Its saved data has not been changed or deleted.</p>
  </main>;

  return <main className={`chart-room${empty ? " chart-room-empty" : ""}`} data-engine-revision={revision}>
    {empty ? <section className="room-title-page">
      <div className="room-title-text">
        <span className="brand"><span className="brand-name"><span>ʿUmrān</span></span></span>
        <h1>The book of a world</h1>
        <p className="room-lede">Found peoples on a world’s coasts, let the centuries run, and read what became of them and of their speech.
          Every sound change is regular, and every borrowed word has a meeting behind it.</p>
        <button type="button" className="primary room-found-first" disabled={busy} onClick={onBegin}>Found a new world <ArrowRight size={19} aria-hidden="true" /></button>
        <ol className="room-steps" aria-label="Founding, step by step">{FOUNDING_STEPS.map((step, i) => <li key={step}><span>{i + 1}</span> {step}</li>)}</ol>
        <p className="room-other">{openFile}</p>
      </div>
      <figure className="room-sample">
        <button type="button" className="room-sample-chart" disabled={busy} onClick={() => void sample()} aria-label={sampleLabel}>
          <Chart land={SAMPLE_LAND} />
        </button>
        <figcaption>
          <button type="button" className="link" disabled={busy} onClick={() => void sample()}>{sampleLabel}</button>
          <span>Four thousand years on this chart: three peoples, a conquest, and a new faith.</span>
        </figcaption>
      </figure>
    </section> : <>
      <header className="room-head cartouche">
        <span className="brand"><span className="brand-name"><span>ʿUmrān</span></span></span>
        <h1>The chart room</h1>
      </header>

      {undone && shelf.putAway.some((book) => book.id === undone.id) ? <p className="room-undo" role="status">
        {undone.title} is put away. <button type="button" className="link" disabled={busy} onClick={() => {
          if (onBringBack(undone.id)) setUndone(null);
        }}>Undo</button>
      </p> : null}

      {leading ? <section className="room-worlds" aria-label="Your worlds">
        {world(leading, true)}
        {others.length ? <ul className="room-world-row">
          {others.map((entry) => <li key={entry.book.id}>{world(entry, false)}</li>)}
        </ul> : null}
      </section> : null}

      {shelf.drafts.length ? <section className="room-unfinished" aria-labelledby="unfinished-worlds">
        <h2 id="unfinished-worlds">Unfinished worlds</h2>
        <ul className="room-world-row">
          {[...shelf.drafts].sort((a, b) => b.updated - a.updated).map((draft) => <li key={draft.id}>
            <article className="room-world room-draft">
              <Chart land={{ seed: draft.seed, size: draft.size, geography: "continental-v6" }} peoples={draft.peoples} />
              <button type="button" className="room-world-open" disabled={busy} onClick={() => onResumeDraft(draft.id)} aria-label={`Resume ${draft.title || "an unfinished world"}`}>
                <span className="room-world-title">
                  <strong>{draft.title || "An unfinished world"}</strong>
                  <span className="room-last-line">{draft.summary}</span>
                  <span className="room-continue">Resume <ArrowRight size={17} aria-hidden="true" /></span>
                </span>
              </button>
              <button type="button" className="room-put-away" disabled={busy} onClick={() => setDiscarding(draft)}>Discard</button>
            </article>
          </li>)}
        </ul>
      </section> : null}

      <section className="room-actions" aria-label="Found or open a world">
        <div className="room-action-list">
          <div className="room-action room-found">
            <button type="button" disabled={busy} onClick={onBegin}>Found a new world <ArrowRight size={19} aria-hidden="true" /></button>
          </div>
          <div className="room-action">
            <button type="button" disabled={busy} onClick={() => void sample()}>{sampleLabel}</button>
            <p>Four thousand years, a conquest, and a new faith.</p>
          </div>
          <div className="room-action">{openFile}</div>
        </div>
      </section>

      <details className="room-put-away-list">
        <summary>Put away ({shelf.putAway.length})</summary>
        {shelf.putAway.length ? <ul>
          {[...shelf.putAway].sort((a, b) => b.putAway - a.putAway).map((book) => <li key={book.id}>
            <div><strong>{book.title}</strong><span>year {book.generation * YEARS}</span></div>
            <div className="room-kept-actions">
              <button type="button" disabled={busy} onClick={() => onBringBack(book.id)}>Bring back</button>
              <button type="button" disabled={busy} onClick={() => setRemoving(book)}>Remove for good</button>
            </div>
          </li>)}
        </ul> : <p>No worlds are put away.</p>}
      </details>
    </>}

    <footer className="room-colophon">Worlds live in this browser; the book page exports them as save files or text.</footer>

    <Modal open={removing !== null} title="Remove this world for good?" onClose={() => setRemoving(null)} footer={<>
      <button type="button" onClick={() => setRemoving(null)}>Keep it put away</button>
      <button type="button" className="primary" disabled={busy} onClick={() => {
        if (removing && onRemoveForGood(removing.id)) setRemoving(null);
      }}>Remove for good</button>
    </>}>
      <p>{removing?.title} will be deleted from this browser, including its reading place. This cannot be undone.</p>
      <p>Keep a save file if you might want this world again.</p>
      <button type="button" disabled={busy} onClick={() => { if (removing) onDownload(removing.id); }}>Download {removing?.title} first</button>
    </Modal>

    <Modal open={discarding !== null} title="Discard this unfinished world?" onClose={() => setDiscarding(null)} footer={<>
      <button type="button" onClick={() => setDiscarding(null)}>Keep it</button>
      <button type="button" className="primary" disabled={busy} onClick={() => {
        if (discarding && onDiscardDraft(discarding.id)) setDiscarding(null);
      }}>Discard</button>
    </>}>
      <p>{discarding?.title || "This unfinished world"} and its founding choices will be deleted from this browser. This cannot be undone.</p>
    </Modal>
  </main>;
}
