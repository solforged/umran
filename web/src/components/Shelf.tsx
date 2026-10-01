import { useState } from "react";
import type { BookEntry } from "../shelf";
import { YEARS } from "../model";
import { Modal } from "./Modal";

/// The books, newest first, with ways to begin, read the sample, or bring
/// one in from a file.
export function Shelf({
  books,
  onOpen,
  onBegin,
  onSample,
  onImport,
  onRemove,
}: {
  books: BookEntry[];
  onOpen: (id: string) => void;
  onBegin: () => void;
  onSample: () => void;
  onImport: (file: File) => void;
  onRemove: (id: string) => void;
}) {
  const [removing, setRemoving] = useState<BookEntry | null>(null);
  return (
    <main className="shelf">
      <header>
        <h1>Langgen</h1>
        <p className="muted">Each book is a world: its peoples, their languages, and how both change over the years.</p>
      </header>

      <ul className="books">
        <li>
          <button type="button" className="book-card begin" onClick={onBegin}>
            <strong>Begin a new book</strong>
            <span>Choose a people and how they sound.</span>
          </button>
        </li>
        <li>
          <button type="button" className="book-card sample" onClick={onSample}>
            <strong>A sample chronicle</strong>
            <span>A few centuries already written, to read and continue.</span>
          </button>
        </li>
        {books.map((book) => (
          <li key={book.id}>
            <button type="button" className="book-card" onClick={() => onOpen(book.id)}>
              <strong>{book.title}</strong>
              <span>{book.subtitle}</span>
              <small className="muted">
                {book.generation > 0 ? `${book.generation * YEARS} years written · ` : ""}last opened{" "}
                {new Date(book.updated).toLocaleDateString()}
              </small>
            </button>
            <button type="button" className="link put-away" onClick={() => setRemoving(book)}>
              Remove
            </button>
          </li>
        ))}
      </ul>

      <p>
        <label className="link file-link">
          Bring in a book from a file…
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
      </p>

      <Modal
        open={removing !== null}
        title="Remove this book?"
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
          {removing?.title} will be deleted from this browser. If you might want it again, open it and take
          “The book itself” from its Appendix first.
        </p>
      </Modal>
    </main>
  );
}
