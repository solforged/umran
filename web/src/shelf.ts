// The shelf: several books (worlds), each a saved history under its own
// key, plus an index of titles. The older single-world save is copied onto
// the shelf once and otherwise left untouched.

import type { GeographyVersion, HistoryPoint, MapSize, Overview, Subject as Focus } from "./model";

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
  /// Bumped on every save; absent on older shelves means zero.
  revision?: number;
}

/// A people as the shelf's chart shows it: its name at its heart land, in
/// its family's hand, over the lands it holds.
export interface ShelfPeople {
  name: string;
  family: number;
  region: number;
  lands: number[];
}

/// An unfinished founding, until it is begun or explicitly discarded.
export interface DraftEntry {
  id: string;
  title: string | null;
  summary: string;
  step: number;
  updated: number;
  peoples?: ShelfPeople[];
  seed: number;
  size: MapSize;
}

export interface PutAwayEntry extends BookEntry {
  putAway: number;
}

export interface Shelf {
  books: BookEntry[];
  /// The book open when the app was last closed.
  last: string | null;
  drafts: DraftEntry[];
  putAway: PutAwayEntry[];
}

// Stored under the project's first name, langgen, so saves made before the
// rename still open.
const INDEX = "langgen.shelf.v1";
const LEGACY = "langgen.sim.v1";
const bookKey = (id: string) => `langgen.book.${id}`;
const draftKey = (id: string) => `langgen.draft.${id}`;

const indexNumber = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const record = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === "object" && !Array.isArray(value);
const mapSize = (value: unknown): value is MapSize => value === "small" || value === "medium" || value === "large" || value === "vast";

function shelfPeoples(value: unknown): value is ShelfPeople[] {
  return Array.isArray(value) && value.every((people: unknown) => record(people)
    && typeof people.name === "string" && indexNumber(people.family) && indexNumber(people.region)
    && Array.isArray(people.lands) && people.lands.every(indexNumber));
}

function get(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/// The shelf index is untrusted saved data, including its optional chart.
/// Reject the whole index rather than quietly losing an unreadable entry.
export function isBookEntry(value: unknown): value is BookEntry {
  if (!record(value)) return false;
  return typeof value.id === "string" && value.id.length > 0
    && typeof value.title === "string" && typeof value.subtitle === "string"
    && indexNumber(value.generation) && indexNumber(value.updated)
    && (value.author === undefined || typeof value.author === "string")
    && (value.named === undefined || typeof value.named === "boolean")
    && (value.revision === undefined || indexNumber(value.revision))
    && (value.peoples === undefined || shelfPeoples(value.peoples));
}

function isDraftEntry(value: unknown): value is DraftEntry {
  return record(value) && typeof value.id === "string" && value.id.length > 0
    && (value.title === null || typeof value.title === "string")
    && typeof value.summary === "string" && indexNumber(value.step) && indexNumber(value.updated)
    && indexNumber(value.seed) && mapSize(value.size)
    && (value.peoples === undefined || shelfPeoples(value.peoples));
}

/// The shelf as saved, or an empty one. Throws only when the index exists
/// but cannot be read, so the caller can offer recovery.
export function loadShelf(): Shelf {
  const raw = get(INDEX);
  if (raw !== null) {
    const parsed: unknown = JSON.parse(raw);
    if (!record(parsed)) throw new Error("The list of saved worlds could not be read.");
    const index = parsed;
    if (!Array.isArray(index.books)) throw new Error("The list of saved worlds is not a list.");
    if (!index.books.every(isBookEntry)) throw new Error("A saved world's shelf entry could not be read.");
    if (index.last !== undefined && index.last !== null && typeof index.last !== "string") {
      throw new Error("The last saved world could not be read.");
    }
    if (index.drafts !== undefined && (!Array.isArray(index.drafts) || !index.drafts.every(isDraftEntry))) {
      throw new Error("An unfinished world's shelf entry could not be read.");
    }
    if (index.putAway !== undefined && (!Array.isArray(index.putAway)
      || !index.putAway.every((entry: unknown) => isBookEntry(entry) && record(entry) && indexNumber(entry.putAway)))) {
      throw new Error("A put-away world's shelf entry could not be read.");
    }
    const books = index.books as BookEntry[];
    const drafts = (index.drafts ?? []) as DraftEntry[];
    const putAway = (index.putAway ?? []) as PutAwayEntry[];
    const ids = [...books, ...putAway].map((entry) => entry.id);
    const draftIds = drafts.map((entry) => entry.id);
    // Beginning a world saves its book before removing the draft of that id.
    if (new Set(ids).size !== ids.length || new Set(draftIds).size !== draftIds.length) {
      throw new Error("A world is listed more than once on the shelf.");
    }
    return { books, drafts, putAway, last: (index.last ?? null) as string | null };
  }
  const legacy = get(LEGACY);
  if (legacy === null) return { books: [], drafts: [], putAway: [], last: null };
  const entry: BookEntry = {
    id: "earlier",
    title: "An earlier history",
    subtitle: "Saved before the shelf",
    generation: 0,
    updated: Date.now(),
  };
  const shelf: Shelf = { books: [entry], drafts: [], putAway: [], last: entry.id };
  // A failed copy leaves the old save where it was; try again next time.
  try {
    if (get(bookKey(entry.id)) === null) localStorage.setItem(bookKey(entry.id), legacy);
    localStorage.setItem(INDEX, JSON.stringify(shelf));
  } catch {
    return { books: [], drafts: [], putAway: [], last: null };
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
    if (geography !== "spherical-v1" && geography !== "continental-v2" && geography !== "continental-v3" && geography !== "continental-v4" && geography !== "continental-v5" && geography !== "continental-v6") return null;
    return { seed: recipe.seed, size: recipe.map as MapSize, geography };
  } catch {
    return null;
  }
}

export function rawShelf(): string | null {
  return get(INDEX);
}

export class StaleSave extends Error {
  constructor(public readonly stored: number, public readonly base: number) {
    super("Another tab has saved a newer version of this world.");
    this.name = "StaleSave";
  }
}

function writeShelf(shelf: Shelf): Shelf {
  localStorage.setItem(INDEX, JSON.stringify(shelf));
  return shelf;
}

/// Read the stored shelf for every change, not an older copy held by a tab.
/// A world put away elsewhere stays put away even if an open tab saves it.
export function saveBook(entry: BookEntry, history: string, base: number): { shelf: Shelf; revision: number } {
  const shelf = loadShelf();
  const current = [...shelf.books, ...shelf.putAway].find((book) => book.id === entry.id);
  const stored = current?.revision ?? 0;
  if (stored > base) throw new StaleSave(stored, base);
  const revision = Math.max(stored, base) + 1;
  const saved = { ...entry, revision };
  const away = shelf.putAway.find((book) => book.id === entry.id);
  const next: Shelf = away
    ? { ...shelf, putAway: shelf.putAway.map((book) => book.id === entry.id ? { ...saved, putAway: book.putAway } : book) }
    : { ...shelf, books: [saved, ...shelf.books.filter((book) => book.id !== entry.id)], last: entry.id };
  localStorage.setItem(bookKey(entry.id), history);
  return { shelf: writeShelf(next), revision };
}

export function saveDraft(entry: DraftEntry, design: string): Shelf {
  const shelf = loadShelf();
  const next = { ...shelf, drafts: [entry, ...shelf.drafts.filter((draft) => draft.id !== entry.id)] };
  localStorage.setItem(draftKey(entry.id), design);
  return writeShelf(next);
}

export function readDraft(id: string): string | null {
  return get(draftKey(id));
}

export function discardDraft(id: string): Shelf {
  const shelf = loadShelf();
  const next = writeShelf({ ...shelf, drafts: shelf.drafts.filter((draft) => draft.id !== id) });
  localStorage.removeItem(draftKey(id));
  return next;
}

export function setLast(last: string | null): Shelf {
  const shelf = loadShelf();
  const next = { ...shelf, last };
  try {
    return writeShelf(next);
  } catch {
    // Only a convenience: the chart room opens instead of the world.
    return shelf;
  }
}

/// Put away keeps both the original history and the reader's place.
export function putAway(id: string): Shelf {
  const shelf = loadShelf();
  const entry = shelf.books.find((book) => book.id === id);
  if (!entry) return shelf;
  return writeShelf({
    ...shelf,
    books: shelf.books.filter((book) => book.id !== id),
    putAway: [{ ...entry, putAway: Date.now() }, ...shelf.putAway],
    last: shelf.last === id ? null : shelf.last,
  });
}

export function bringBack(id: string): Shelf {
  const shelf = loadShelf();
  const entry = shelf.putAway.find((book) => book.id === id);
  if (!entry) return shelf;
  const { putAway: _putAway, ...book } = entry;
  return writeShelf({
    ...shelf,
    books: [book, ...shelf.books],
    putAway: shelf.putAway.filter((book) => book.id !== id),
  });
}

/// Permanent removal is only available for a world already put away.
export function removeForGood(id: string): Shelf {
  const shelf = loadShelf();
  if (!shelf.putAway.some((book) => book.id === id)) return shelf;
  const next = writeShelf({
    ...shelf,
    putAway: shelf.putAway.filter((book) => book.id !== id),
    last: shelf.last === id ? null : shelf.last,
  });
  localStorage.removeItem(bookKey(id));
  localStorage.removeItem(placeKey(id));
  return next;
}

function bookDetails(raw: string): { title: string | null; generation: number | null; readable: boolean; author?: string } {
  try {
    const recipe: unknown = JSON.parse(raw);
    if (!record(recipe)) return { title: null, generation: null, readable: false };
    const title = typeof recipe.title === "string" && recipe.title.length ? recipe.title : null;
    const author = typeof recipe.author === "string" ? recipe.author : undefined;
    const telling = Array.isArray(recipe.tellings)
      ? recipe.tellings.find((telling: unknown) => record(telling) && telling.id === recipe.active) : null;
    // Earlier recipes kept one action list instead of named tellings.
    const actions = record(telling) ? telling.actions : recipe.actions;
    let generation: number | null = null;
    if (Array.isArray(actions)) {
      generation = 0;
      for (const action of actions) {
        if (!record(action) || typeof action.kind !== "string") { generation = null; break; }
        if (action.kind === "run") {
          if (!indexNumber(action.generations)) { generation = null; break; }
          generation += action.generations;
          if (!Number.isSafeInteger(generation)) { generation = null; break; }
        }
      }
    }
    const readable = recipe.format === "langgen-sim-recipe"
      && indexNumber(recipe.revision) && indexNumber(recipe.seed) && generation !== null
      && (recipe.map === undefined || mapSize(recipe.map));
    return { title, generation, readable, author };
  } catch {
    return { title: null, generation: null, readable: false };
  }
}

export interface ScannedBook {
  id: string;
  title: string | null;
  generation: number | null;
  bytes: number;
  raw: string;
}

/// Include unreadable originals as well, so recovery can download everything.
export function scanBooks(): ScannedBook[] {
  const keys: string[] = [];
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (key?.startsWith("langgen.book.")) keys.push(key);
  }
  return keys.sort().flatMap((key) => {
    const raw = localStorage.getItem(key);
    if (raw === null) return [];
    const { title, generation } = bookDetails(raw);
    return [{ id: key.slice("langgen.book.".length), title, generation, bytes: new TextEncoder().encode(raw).length, raw }];
  });
}

export function rebuildShelf(): Shelf {
  const original = localStorage.getItem(INDEX);
  const found = scanBooks();
  // Rebuilding is recovery, not an alternate way to clear a readable shelf.
  if (original !== null) {
    try { return loadShelf(); } catch { /* Keep the unreadable index first. */ }
    let backup = `${INDEX}.broken.${Date.now()}`;
    while (localStorage.getItem(backup) !== null) backup += ".kept";
    localStorage.setItem(backup, original);
  }
  const books = found.flatMap((book): BookEntry[] => {
    const details = bookDetails(book.raw);
    return book.id.length > 0 && details.readable ? [{
      id: book.id, title: details.title ?? "A recovered world", author: details.author,
      named: details.title !== null, subtitle: "Recovered from its saved history",
      generation: details.generation!, updated: Date.now(), revision: 0,
    }] : [];
  });
  return writeShelf({ books, last: null, drafts: [], putAway: [] });
}

export function onShelfChange(listener: (change: { book: string | null }) => void): () => void {
  const changed = (event: StorageEvent) => {
    if (event.storageArea !== null && event.storageArea !== localStorage) return;
    if (event.key === INDEX || event.key === null) listener({ book: null });
    else if (event.key.startsWith("langgen.book.")) listener({ book: event.key.slice("langgen.book.".length) });
  };
  window.addEventListener("storage", changed);
  return () => window.removeEventListener("storage", changed);
}

/// Where the reader was in a book: the telling and exact reading, the
/// year, and the open card. Only a convenience; a bad place is ignored.
export interface ReadingPlace {
  telling: number | null;
  point: HistoryPoint | null;
  viewing: number | null;
  focus: Focus;
  /// A writing session's intended year; reopening offers it, never runs it.
  runTo?: number;
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

/// A world's entry from its present telling at its latest year, using the
/// author's title or the fixed continent name.
export function describe(id: string, overview: Overview, title: string | null, author: string | null): BookEntry {
  const last = overview.latest === 0 ? null : overview.annals.at(-1);
  const peoples = overview.communities
    .filter((c) => c.ended === null)
    .sort((a, b) => b.size - a.size)
    .map((c) => ({ name: c.name, family: overview.varieties[c.variety].family, region: c.region, lands: c.lands }));
  return {
    id,
    title: title ?? worldName(overview) ?? "Unknown waters",
    named: title != null,
    author: author ?? undefined,
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
