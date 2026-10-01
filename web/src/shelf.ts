// The shelf: several books (worlds), each a saved history under its own
// key, plus an index of titles. The older single-world save is copied onto
// the shelf once and otherwise left untouched.

import type { Overview } from "./model";
import { YEARS } from "./model";

export interface BookEntry {
  id: string;
  title: string;
  /// The languages spoken at the last save, for the spine.
  subtitle: string;
  generation: number;
  updated: number;
}

export interface Shelf {
  books: BookEntry[];
  /// The book open when the app was last closed.
  last: string | null;
}

const INDEX = "langgen.shelf.v1";
const LEGACY = "langgen.sim.v1";
const bookKey = (id: string) => `langgen.book.${id}`;

function get(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/// The shelf as saved, or an empty one. Throws only when the index exists
/// but cannot be read, so the caller can offer recovery.
export function loadShelf(): Shelf {
  const raw = get(INDEX);
  if (raw !== null) {
    const parsed = JSON.parse(raw) as Partial<Shelf>;
    if (!Array.isArray(parsed.books)) throw new Error("The shelf index is not a list of books.");
    return { books: parsed.books, last: parsed.last ?? null };
  }
  const legacy = get(LEGACY);
  if (legacy === null) return { books: [], last: null };
  const entry: BookEntry = {
    id: "earlier",
    title: "An earlier history",
    subtitle: "Saved before the shelf",
    generation: 0,
    updated: Date.now(),
  };
  const shelf = { books: [entry], last: entry.id };
  // A failed copy leaves the old save where it was; try again next time.
  try {
    localStorage.setItem(bookKey(entry.id), legacy);
    localStorage.setItem(INDEX, JSON.stringify(shelf));
  } catch {
    return { books: [], last: null };
  }
  return shelf;
}

export function readBook(id: string): string | null {
  return get(bookKey(id));
}

export function rawShelf(): string | null {
  return get(INDEX);
}

/// Saves a book and moves it to the front of the shelf. Throws when the
/// browser refuses to store it.
export function saveBook(shelf: Shelf, entry: BookEntry, history: string): Shelf {
  localStorage.setItem(bookKey(entry.id), history);
  const next = { books: [entry, ...shelf.books.filter((b) => b.id !== entry.id)], last: entry.id };
  localStorage.setItem(INDEX, JSON.stringify(next));
  return next;
}

export function setLast(shelf: Shelf, last: string | null): Shelf {
  const next = { ...shelf, last };
  try {
    localStorage.setItem(INDEX, JSON.stringify(next));
  } catch {
    // Only a convenience: the shelf opens instead of the book.
  }
  return next;
}

/// Takes a book off the shelf. Only on the reader's explicit request.
export function removeBook(shelf: Shelf, id: string): Shelf {
  const next = { books: shelf.books.filter((b) => b.id !== id), last: shelf.last === id ? null : shelf.last };
  localStorage.setItem(INDEX, JSON.stringify(next));
  localStorage.removeItem(bookKey(id));
  return next;
}

export function newBookId(): string {
  return `${Date.now().toString(36)}${Math.floor(Math.random() * 1e6).toString(36)}`;
}

/// A book's title and spine, from the world as it stands.
export function describe(id: string, overview: Overview): BookEntry {
  const first = overview.communities[0];
  const spoken = overview.varieties.filter((v) => v.spoken).map((v) => v.name);
  return {
    id,
    title: first ? `The world of the ${first.name}` : "An empty world",
    subtitle: `${spoken.join(", ")} · year ${overview.latest * YEARS}`,
    generation: overview.latest,
    updated: Date.now(),
  };
}
