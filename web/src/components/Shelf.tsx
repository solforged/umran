import { useEffect, useMemo, useState } from "react";
import { FileUp, X } from "lucide-react";
import { landMap } from "../engine";
import type { MapSize, WorldMap } from "../model";
import { SAMPLE_LAND } from "../sample";
import { bookLand, type BookEntry, type ShelfPeople } from "../shelf";
import { Miniature } from "./MapView";
import { Modal } from "./Modal";

/// A sheet with no land on it yet: compass lines over open sea, the size
/// of a middling world.
const UNKNOWN: WorldMap = { size: "medium", width: 13.5, height: 8.794229, regions: [] };

/// A world's chart, drawn once its map is ready; a blank sheet until then
/// or if its save cannot be read.
function Chart({ land, peoples }: { land: { seed: number; size: MapSize } | null; peoples?: ShelfPeople[] }) {
  const [map, setMap] = useState<WorldMap | null>(null);
  const [seed, size] = [land?.seed, land?.size];
  useEffect(() => {
    if (seed === undefined || size === undefined) return;
    let live = true;
    landMap(seed, size).then(
      (next) => live && setMap(next),
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [seed, size]);
  return <Miniature map={map ?? UNKNOWN} peoples={map ? peoples : []} />;
}

/// The chart room: every saved world as a sheet of its own chart, most
/// recently opened first, beside a blank sheet for a new world and the
/// sample chronicle.
export function Shelf({
  revision,
  books,
  onOpen,
  onBegin,
  onSample,
  onImport,
  onRemove,
}: {
  /// The engine revision, for the note at the foot.
  revision: number;
  books: BookEntry[];
  onOpen: (id: string) => void;
  onBegin: () => void;
  onSample: () => void;
  onImport: (file: File) => void;
  onRemove: (id: string) => void;
}) {
  const [removing, setRemoving] = useState<BookEntry | null>(null);
  const worlds = useMemo(
    () => [...books].sort((a, b) => b.updated - a.updated).map((book) => ({ book, land: bookLand(book.id) })),
    [books],
  );
  return (
    <main className="shelf">
      <header className="shelf-head">
        <span className="brand">‘Umrān</span>
        <h1>The chart room</h1>
        <p>Every world charted so far, the last one opened on top.</p>
      </header>

      <ul className="sheets">
        <li className="sheet blank">
          <Chart land={null} />
          <button type="button" className="sheet-open" onClick={onBegin}>
            <span className="sheet-title">
              <strong>Unknown waters</strong>
              <span>Chart a new world: draw its coasts and choose who lives there.</span>
            </span>
          </button>
        </li>
        <li className="sheet">
          <Chart land={SAMPLE_LAND} />
          <button type="button" className="sheet-open" onClick={onSample}>
            <span className="sheet-title">
              <strong>A chronicle already written</strong>
              <span>Four thousand years, a conquest, and a new faith.</span>
            </span>
          </button>
        </li>
        {worlds.map(({ book, land }) => (
          <li key={book.id} className="sheet">
            <Chart land={land} peoples={book.peoples} />
            <button type="button" className="sheet-open" onClick={() => onOpen(book.id)}>
              <span className="sheet-title">
                <strong>{book.title}</strong>
                <span>{book.subtitle}</span>
                <small>
                  opened {new Date(book.updated).toLocaleDateString(undefined, { day: "numeric", month: "long" })}
                </small>
              </span>
            </button>
            <button
              type="button"
              className="icon put-away"
              title={`Remove ${book.title}`}
              aria-label={`Remove ${book.title}`}
              onClick={() => setRemoving(book)}
            >
              <X size={15} />
            </button>
          </li>
        ))}
      </ul>

      <footer className="colophon">
        <label className="file-link">
          <FileUp size={16} aria-hidden="true" /> Open a save file…
          <input
            type="file"
            accept="application/json,.json"
            hidden
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) onImport(file);
              e.target.value = "";
            }}
          />
        </label>
        <p>
          Worlds are kept in this browser only. To keep one safe or move it elsewhere, open it and choose Export, then
          “As a save file”. A save opened on the same engine revision (now {revision}) replays the same history word
          for word.
        </p>
      </footer>

      <Modal
        open={removing !== null}
        title="Remove this world?"
        onClose={() => setRemoving(null)}
        footer={
          <>
            <button type="button" onClick={() => setRemoving(null)}>
              Keep it
            </button>
            <button
              type="button"
              className="primary"
              onClick={() => {
                if (removing) onRemove(removing.id);
                setRemoving(null);
              }}
            >
              Remove for good
            </button>
          </>
        }
      >
        <p>
          {removing?.title} will be deleted from this browser. If you might want it again, open it and choose Export,
          then "As a save file", first.
        </p>
      </Modal>
    </main>
  );
}
