import { useState } from "react";
import { FileUp, Globe, Plus, Sparkles, Trash2 } from "lucide-react";
import type { BookEntry } from "../shelf";
import { YEARS } from "../model";
import { Modal } from "./Modal";

/// The saved worlds, most recently opened first, with ways to begin a new
/// one, watch the sample, or open a save file.
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
  const worlds = [...books].sort((a, b) => b.updated - a.updated);
  return (
    <main className="shelf">
      <header>
        <h1>Langgen</h1>
        <p className="muted">Worlds of peoples and their languages, and how both change over the years.</p>
      </header>

      <ul className="worlds">
        <li>
          <button type="button" className="world-card begin" onClick={onBegin}>
            <Plus size={20} aria-hidden="true" />
            <strong>A new world</strong>
            <span>Draw a map and choose who lives in it.</span>
          </button>
        </li>
        <li>
          <button type="button" className="world-card" onClick={onSample}>
            <Sparkles size={20} aria-hidden="true" />
            <strong>A sample world</strong>
            <span>A few centuries already played, to watch and go on with.</span>
          </button>
        </li>
        {worlds.map((book) => (
          <li key={book.id}>
            <button type="button" className="world-card" onClick={() => onOpen(book.id)}>
              <Globe size={20} aria-hidden="true" />
              <strong>{book.title}</strong>
              <span>{book.subtitle}</span>
              <small className="muted">
                {book.generation > 0 ? `${book.generation * YEARS} years played · ` : ""}opened{" "}
                {new Date(book.updated).toLocaleDateString()}
              </small>
            </button>
            <button
              type="button"
              className="icon put-away"
              title={`Remove ${book.title}`}
              aria-label={`Remove ${book.title}`}
              onClick={() => setRemoving(book)}
            >
              <Trash2 size={15} />
            </button>
          </li>
        ))}
      </ul>

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

      <footer className="colophon">
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
          then “As a save file”, first.
        </p>
      </Modal>
    </main>
  );
}
