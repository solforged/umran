// The shelf: several books (worlds), each a saved history under its own
// key, plus an index of titles. The older single-world save is copied onto
// the shelf once and otherwise left untouched.

import type { Engine, GeographyVersion, HistoryPoint, MapSize, Overview, Subject as Focus } from "./model";

export interface BookEntry {
  id: string;
  title: string;
  author?: string;
  /// True when the author gave the title; otherwise it is the world's own
  /// name and may be re-derived on each save.
  named?: boolean;
  /// The last chronicle line at the latest save.
  subtitle: string;
  generation: number;
  updated: number;
  /// The living peoples at the last save, largest first, for the chart on
  /// the shelf. Missing from books last saved before charts were drawn.
  peoples?: ShelfPeople[];
}

/// A people as the shelf's chart shows it: its name at its heart land, in
/// its family's hand, over the lands it holds.
export interface ShelfPeople {
  name: string;
  family: number;
  region: number;
  lands: number[];
}

export interface Shelf {
  books: BookEntry[];
  /// The book open when the app was last closed.
  last: string | null;
}

// Stored under the project's first name, langgen, so saves made before the
// rename still open.
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
    if (!Array.isArray(parsed.books)) throw new Error("The list of saved worlds is not a list.");
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

export interface BookLand {
  seed: number;
  size: MapSize;
  geography: GeographyVersion;
}

/// Preview only supported spherical recipes. Revision 33 predates the explicit
/// geography field; its region IDs always belong to spherical-v1.
/// Reading a miniature never migrates or rewrites the saved original.
export function bookLand(id: string, revision: number): BookLand | null {
  try {
    const recipe = JSON.parse(get(bookKey(id)) ?? "null") as {
      seed?: unknown; map?: unknown; revision?: unknown; geography?: unknown;
    } | null;
    if (!recipe || typeof recipe.revision !== "number"
      || recipe.revision < 33 || recipe.revision > revision
      || typeof recipe.seed !== "number" || typeof recipe.map !== "string") return null;
    const geography = recipe.geography === undefined ? "spherical-v1" : recipe.geography;
    if (geography !== "spherical-v1" && geography !== "continental-v2" && geography !== "continental-v3" && geography !== "continental-v4") return null;
    return { seed: recipe.seed, size: recipe.map as MapSize, geography };
  } catch {
    return null;
  }
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
  localStorage.removeItem(placeKey(id));
  return next;
}

/// Where the reader was in a book: the telling and exact reading, the
/// year, and the open card. Only a convenience; a bad place is ignored.
export interface ReadingPlace {
  telling: number | null;
  point: HistoryPoint | null;
  viewing: number | null;
  focus: Focus;
}
const placeKey = (id: string) => `langgen.place.${id}`;

export function readPlace(id: string): ReadingPlace | null {
  try {
    return JSON.parse(get(placeKey(id)) ?? "null") as ReadingPlace | null;
  } catch {
    return null;
  }
}

export function savePlace(id: string, place: ReadingPlace) {
  try {
    localStorage.setItem(placeKey(id), JSON.stringify(place));
  } catch {
    // Losing the place only means reopening at the latest year.
  }
}

export function newBookId(): string {
  return `${Date.now().toString(36)}${Math.floor(Math.random() * 1e6).toString(36)}`;
}

/// A world's entry, using the author's title or the fixed continent name.
export function describe(id: string, engine: Engine): BookEntry {
  const overview = engine.overview(engine.latest());
  const title = engine.title();
  const last = overview.latest === 0 ? null : overview.annals.at(-1);
  const peoples = overview.communities
    .filter((c) => c.ended === null)
    .sort((a, b) => b.size - a.size)
    .map((c) => ({ name: c.name, family: overview.varieties[c.variety].family, region: c.region, lands: c.lands }));
  return {
    id,
    title: title ?? worldName(overview) ?? "Unknown waters",
    named: title != null,
    author: engine.author() ?? undefined,
    subtitle: last?.text ?? "Nothing is written yet",
    generation: overview.latest,
    updated: Date.now(),
    peoples,
  };
}

/// The earliest-named continent, tied by landmass id. A continent's name
/// is fixed once given, so people's changing names never rename the world.
export function worldName(overview: Overview): string | null {
  let first: Overview["continents"][number] | null = null;
  for (const continent of overview.continents) {
    if (!continent.name) continue;
    if (!first || continent.name.since < first.name!.since ||
      (continent.name.since === first.name!.since && continent.landmass < first.landmass)) {
      first = continent;
    }
  }
  return first?.name?.name ?? null;
}
