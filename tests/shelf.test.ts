import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import {
  bookLand, bringBack, discardDraft, loadShelf, onShelfChange, putAway, readBook, readDraft,
  readPlace, rebuildShelf, removeForGood, saveBook, saveDraft, savePlace, scanBooks, setLast, StaleSave,
  type BookEntry, type DraftEntry, type ReadingPlace,
} from "../web/src/shelf";

let stored: Map<string, string>;
let originalStorage: PropertyDescriptor | undefined;
beforeEach(() => {
  stored = new Map();
  originalStorage = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  const storage: Storage = {
    get length() { return stored.size; },
    clear: () => stored.clear(),
    getItem: (key) => stored.get(key) ?? null,
    key: (index) => [...stored.keys()][index] ?? null,
    removeItem: (key) => { stored.delete(key); },
    setItem: (key, value) => { stored.set(key, value); },
  };
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: storage });
});
afterEach(() => {
  if (originalStorage) Object.defineProperty(globalThis, "localStorage", originalStorage);
  else Reflect.deleteProperty(globalThis, "localStorage");
});

function save(id: string, recipe: Record<string, unknown>): string {
  const original = JSON.stringify(recipe, null, 2);
  stored.set(`langgen.book.${id}`, original);
  return original;
}

describe("saved shelf geography", () => {
  test("revision 33 without geography previews its original spherical map without rewriting recovery data", () => {
    const original = save("old", { revision: 33, seed: 5, map: "medium", title: "Original", tellings: [] });
    expect(bookLand("old", 34)).toEqual({ seed: 5, size: "medium", geography: "spherical-v1" });
    expect(readBook("old")).toBe(original);
  });

  test("the same seed and size retain separate old and new recipe geographies after re-saving", () => {
    save("retained", { revision: 34, seed: 5, map: "large", geography: "spherical-v1" });
    save("new", { revision: 34, seed: 5, map: "large", geography: "continental-v2" });
    expect(bookLand("retained", 34)).toEqual({ seed: 5, size: "large", geography: "spherical-v1" });
    expect(bookLand("new", 34)).toEqual({ seed: 5, size: "large", geography: "continental-v2" });
    save("wind", { revision: 35, seed: 5, map: "large", geography: "continental-v3" });
    expect(bookLand("wind", 48)).toEqual({ seed: 5, size: "large", geography: "continental-v3" });
    save("lakes", { revision: 42, seed: 5, map: "large", geography: "continental-v4" });
    expect(bookLand("lakes", 48)).toEqual({ seed: 5, size: "large", geography: "continental-v4" });
    save("cratons", { revision: 48, seed: 5, map: "large", geography: "continental-v5" });
    expect(bookLand("cratons", 54)).toEqual({ seed: 5, size: "large", geography: "continental-v5" });
    save("current", { revision: 54, seed: 5, map: "large", geography: "continental-v6" });
    expect(bookLand("current", 54)).toEqual({ seed: 5, size: "large", geography: "continental-v6" });
  });

  test("implicit geography follows the recipe migration default, not the current engine default", () => {
    save("implicit", { revision: 34, seed: 7, map: "small" });
    expect(bookLand("implicit", 34)).toEqual({ seed: 7, size: "small", geography: "spherical-v1" });
  });

  test("unrenderable recipes keep their originals rather than drawing current region identities", () => {
    for (const [id, recipe] of [
      ["flat", { revision: 32, seed: 5, map: "medium" }],
      ["future", { revision: 35, seed: 5, map: "medium", geography: "continental-v2" }],
      ["unknown", { revision: 34, seed: 5, map: "medium", geography: "unknown-v3" }],
    ] as const) {
      const original = save(id, recipe);
      expect(bookLand(id, 34)).toBeNull();
      expect(readBook(id)).toBe(original);
    }
  });
});

const INDEX = "langgen.shelf.v1";
function entry(id: string): BookEntry {
  return { id, title: `World ${id}`, subtitle: "Nothing is written yet", generation: 0, updated: 1 };
}
function draft(id: string): DraftEntry {
  return { id, title: null, summary: "One people · their speech", step: 3, updated: 2, seed: 21, size: "medium" };
}
function history(title = "An intact world"): Record<string, unknown> {
  return {
    format: "langgen-sim-recipe", revision: 54, seed: 21, map: "medium", geography: "continental-v6",
    title, author: "An author", active: 0,
    tellings: [{ id: 0, name: "The first telling", parent: null, actions: [{ kind: "run", generations: 12 }] }],
    notebook: [],
  };
}

describe("the manuscript shelf", () => {
  test("an old index receives empty draft and put-away lists without changing its original", () => {
    const raw = JSON.stringify({ books: [entry("old")], last: "old" });
    stored.set(INDEX, raw);
    expect(loadShelf()).toEqual({ books: [entry("old")], last: "old", drafts: [], putAway: [] });
    expect(stored.get(INDEX)).toBe(raw);
  });

  test("unreadable JSON and structurally invalid entries keep their originals for recovery", () => {
    for (const raw of [
      "{broken", "null", JSON.stringify({ books: [null] }),
      JSON.stringify({ books: [entry("x")], drafts: [null] }),
      JSON.stringify({ books: [], putAway: [entry("x")] }),
      JSON.stringify({ books: [{ ...entry("x"), revision: -1 }] }),
    ]) {
      stored.set(INDEX, raw);
      expect(() => loadShelf()).toThrow();
      expect(stored.get(INDEX)).toBe(raw);
    }
  });

  test("legacy migration copies rather than deletes and does not overwrite an existing original", () => {
    stored.set("langgen.sim.v1", "legacy original");
    stored.set("langgen.book.earlier", "already kept");
    const shelf = loadShelf();
    expect(shelf.books.map((book) => book.id)).toEqual(["earlier"]);
    expect(shelf.drafts).toEqual([]);
    expect(shelf.putAway).toEqual([]);
    expect(readBook("earlier")).toBe("already kept");
    expect(stored.get("langgen.sim.v1")).toBe("legacy original");
  });

  test("every book save reads the current index and bumps only that world's revision", () => {
    const first = saveBook(entry("a"), "a at first", 0);
    const otherTab = saveBook(entry("b"), "b at first", 0);
    const next = saveBook(entry("a"), "a at second", first.revision);
    expect(first.revision).toBe(1);
    expect(otherTab.revision).toBe(1);
    expect(next.revision).toBe(2);
    expect(next.shelf.books.map((book) => [book.id, book.revision])).toEqual([["a", 2], ["b", 1]]);
    expect(next.shelf.last).toBe("a");
    expect(readBook("a")).toBe("a at second");
    expect(readBook("b")).toBe("b at first");
  });

  test("a stale save cannot replace either the history or the index", () => {
    saveBook(entry("a"), "a at first", 0);
    saveBook(entry("a"), "a at second", 1);
    const rawIndex = stored.get(INDEX);
    let error: unknown;
    try { saveBook(entry("a"), "a from a stale tab", 1); } catch (e) { error = e; }
    expect(error).toBeInstanceOf(StaleSave);
    expect((error as StaleSave).stored).toBe(2);
    expect((error as StaleSave).base).toBe(1);
    expect(readBook("a")).toBe("a at second");
    expect(stored.get(INDEX)).toBe(rawIndex);
  });

  test("an old entry starts at revision zero", () => {
    stored.set(INDEX, JSON.stringify({ books: [entry("old")], last: "old" }));
    expect(saveBook(entry("old"), "updated", 0).revision).toBe(1);
  });

  test("draft changes preserve worlds and drafts added by another tab", () => {
    saveDraft(draft("a"), "first design");
    saveBook(entry("world"), "intact history", 0);
    saveDraft(draft("b"), "second design");
    const updated = saveDraft({ ...draft("a"), step: 4 }, "revised design");
    expect(updated.books.map((book) => book.id)).toEqual(["world"]);
    expect(updated.drafts.map((item) => item.id)).toEqual(["a", "b"]);
    expect(readDraft("a")).toBe("revised design");
    expect(readDraft("b")).toBe("second design");
    const discarded = discardDraft("a");
    expect(discarded.drafts.map((item) => item.id)).toEqual(["b"]);
    expect(readDraft("a")).toBeNull();
    expect(readBook("world")).toBe("intact history");
  });

  test("beginning saves a book before discarding its draft of the same id", () => {
    saveDraft(draft("a"), "founding choices");
    const saved = saveBook(entry("a"), "finished founding", 0);
    expect(saved.shelf.drafts[0].id).toBe("a");
    expect(loadShelf().books[0].id).toBe("a");
    expect(discardDraft("a").drafts).toEqual([]);
    expect(readBook("a")).toBe("finished founding");
  });

  test("put away and bring back preserve history, place, drafts, and other tabs' additions", () => {
    saveBook(entry("a"), "a history", 0);
    const place = { telling: null, point: null, viewing: 4, focus: { kind: "world" as const } };
    savePlace("a", place);
    saveDraft(draft("unfinished"), "founding choices");
    const away = putAway("a");
    expect(away.books).toEqual([]);
    expect(away.last).toBeNull();
    expect(away.putAway[0].id).toBe("a");
    expect(away.putAway[0].putAway).toBeGreaterThan(0);
    expect(readBook("a")).toBe("a history");
    expect(readPlace("a")).toEqual(place);
    saveBook(entry("b"), "b history", 0);
    const restored = bringBack("a");
    expect(restored.books.map((book) => book.id)).toEqual(["a", "b"]);
    expect(restored.putAway).toEqual([]);
    expect(restored.drafts.map((item) => item.id)).toEqual(["unfinished"]);
    expect(readPlace("a")).toEqual(place);
  });

  test("an open tab saving a put-away world does not bring it back", () => {
    saveBook(entry("a"), "first", 0);
    putAway("a");
    const saved = saveBook({ ...entry("a"), generation: 12 }, "second", 1);
    expect(saved.shelf.books).toEqual([]);
    expect(saved.shelf.putAway[0].revision).toBe(2);
    expect(saved.shelf.putAway[0].generation).toBe(12);
    expect(saved.shelf.last).toBeNull();
    expect(readBook("a")).toBe("second");
  });

  test("permanent removal only deletes an explicitly put-away world and its place", () => {
    saveBook(entry("active"), "keep", 0);
    saveBook(entry("away"), "remove", 0);
    savePlace("away", { telling: null, point: null, viewing: null, focus: { kind: "world" } });
    removeForGood("active");
    expect(readBook("active")).toBe("keep");
    putAway("away");
    saveDraft(draft("draft"), "keep this design");
    const removed = removeForGood("away");
    expect(removed.books.map((book) => book.id)).toEqual(["active"]);
    expect(removed.putAway).toEqual([]);
    expect(readBook("away")).toBeNull();
    expect(readPlace("away")).toBeNull();
    expect(readDraft("draft")).toBe("keep this design");
  });

  test("setLast reads the latest shelf rather than an older tab's copy", () => {
    saveBook(entry("a"), "a", 0);
    saveBook(entry("b"), "b", 0);
    const shelf = setLast("a");
    expect(shelf.last).toBe("a");
    expect(shelf.books.map((book) => book.id)).toEqual(["b", "a"]);
    expect(setLast(null).books).toEqual(shelf.books);
  });

  test("no shelf mutation replaces an unreadable index or deletes its originals", () => {
    const broken = "{keep the unreadable index";
    stored.set(INDEX, broken);
    stored.set("langgen.book.a", "keep the history");
    stored.set("langgen.draft.a", "keep the choices");
    for (const change of [
      () => saveBook(entry("a"), "replace", 0),
      () => saveDraft(draft("a"), "replace"),
      () => discardDraft("a"), () => putAway("a"), () => bringBack("a"),
      () => removeForGood("a"), () => setLast(null),
    ]) expect(change).toThrow();
    expect(stored.get(INDEX)).toBe(broken);
    expect(readBook("a")).toBe("keep the history");
    expect(readDraft("a")).toBe("keep the choices");
  });

  test("a refused index write cannot discard a draft or permanently delete a world", () => {
    saveDraft(draft("draft"), "keep choices");
    saveBook(entry("book"), "keep history", 0);
    putAway("book");
    const original = localStorage.setItem;
    localStorage.setItem = (key, value) => {
      if (key === INDEX) throw new Error("Storage full");
      original(key, value);
    };
    expect(() => discardDraft("draft")).toThrow("Storage full");
    expect(() => removeForGood("book")).toThrow("Storage full");
    expect(readDraft("draft")).toBe("keep choices");
    expect(readBook("book")).toBe("keep history");
  });
});

describe("manuscript recovery", () => {
  test("scanning finds every book without trusting the shelf and keeps malformed raw data", () => {
    const intact = save("intact", history("The seas 🌊"));
    const broken = "{unreadable history";
    stored.set(INDEX, "{unreadable index");
    stored.set("langgen.book.broken", broken);
    stored.set("langgen.draft.draft", "not a book");
    const found = scanBooks();
    expect(found.map((book) => book.id)).toEqual(["broken", "intact"]);
    expect(found.find((book) => book.id === "intact")).toEqual({
      id: "intact", title: "The seas 🌊", generation: 12, bytes: new TextEncoder().encode(intact).length, raw: intact,
    });
    expect(found.find((book) => book.id === "broken")).toEqual({
      id: "broken", title: null, generation: null, bytes: broken.length, raw: broken,
    });
    expect(stored.get(INDEX)).toBe("{unreadable index");
  });

  test("scanning reads the active telling rather than summing alternate histories", () => {
    save("telling", {
      ...history(), active: 2,
      tellings: [
        { id: 0, actions: [{ kind: "run", generations: 100 }] },
        { id: 2, actions: [{ kind: "run", generations: 5 }, { kind: "state", community: 1 }, { kind: "run", generations: 3 }] },
      ],
    });
    expect(scanBooks()[0].generation).toBe(8);
  });

  test("rebuilding keeps the unreadable index and all books, listing readable originals", () => {
    const intact = save("intact", history());
    const broken = "{unreadable history";
    const index = JSON.stringify({ books: [null], last: "intact" });
    stored.set("langgen.book.broken", broken);
    stored.set(INDEX, index);
    stored.set("langgen.draft.draft", "kept choices");
    const rebuilt = rebuildShelf();
    expect(rebuilt.books.map((book) => book.id)).toEqual(["intact"]);
    expect(rebuilt.books[0].generation).toBe(12);
    expect(rebuilt.books[0].title).toBe("An intact world");
    expect(loadShelf()).toEqual(rebuilt);
    const backups = [...stored.keys()].filter((key) => key.startsWith(`${INDEX}.broken.`));
    expect(backups).toHaveLength(1);
    expect(stored.get(backups[0])).toBe(index);
    expect(readBook("intact")).toBe(intact);
    expect(readBook("broken")).toBe(broken);
    expect(readDraft("draft")).toBe("kept choices");
  });

  test("a readable shelf is not replaced by rebuilding", () => {
    saveDraft(draft("draft"), "choices");
    saveBook(entry("away"), JSON.stringify(history()), 0);
    putAway("away");
    const original = stored.get(INDEX);
    expect(rebuildShelf()).toEqual(loadShelf());
    expect(stored.get(INDEX)).toBe(original);
    expect([...stored.keys()].some((key) => key.startsWith(`${INDEX}.broken.`))).toBe(false);
  });

  test("the original index is not overwritten when its recovery backup cannot be stored", () => {
    const original = "{unreadable original index";
    stored.set(INDEX, original);
    save("intact", history());
    localStorage.setItem = () => { throw new Error("Storage full"); };
    expect(() => rebuildShelf()).toThrow("Storage full");
    expect(stored.get(INDEX)).toBe(original);
    expect(readBook("intact")).not.toBeNull();
  });

  test("storage listeners identify book changes, ignore other storage, and unsubscribe", () => {
    const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
    const events = new EventTarget();
    Object.defineProperty(globalThis, "window", { configurable: true, value: events });
    try {
      const changes: { book: string | null }[] = [];
      const unsubscribe = onShelfChange((change) => changes.push(change));
      const emit = (key: string | null, area: unknown = localStorage) => {
        events.dispatchEvent(Object.assign(new Event("storage"), { key, storageArea: area }));
      };
      emit(INDEX);
      emit("langgen.book.a");
      emit("langgen.place.a");
      emit("langgen.draft.a");
      emit(INDEX, {});
      emit(null, null);
      expect(changes).toEqual([{ book: null }, { book: "a" }, { book: null }]);
      unsubscribe();
      emit(INDEX);
      expect(changes).toHaveLength(3);
    } finally {
      if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
      else Reflect.deleteProperty(globalThis, "window");
    }
  });
});

describe("the chart reading place", () => {
  test("keeps both cameras, the globe's turn and a world's projection beside its reading", () => {
    const place: ReadingPlace = {
      telling: 2, point: { action: 3, offset: 4 }, viewing: 40, focus: { kind: "world" },
      chart: { camera: [12, 4, 40, 20], globeCamera: [5, 5, 10, 10], projection: "globe", rotation: [-80, 30] },
      book: { chapter: "lands", anchor: null, scroll: 12, open: ["lands"] },
    };
    savePlace("a", place);
    savePlace("b", { ...place, chart: { camera: [0, 0, 80, 40], projection: "chart" } });
    expect(readPlace("a")).toEqual(place);
    expect(readPlace("b")?.chart).toEqual({ camera: [0, 0, 80, 40], projection: "chart" });
  });

  test("older reading places retain their exact reading without a chart preference", () => {
    const place: ReadingPlace = { telling: null, point: null, viewing: 4, focus: { kind: "world" } };
    savePlace("old", place);
    expect(readPlace("old")).toEqual(place);
  });

  test("bad optional camera fields do not lose or rewrite the reading or original", () => {
    const raw = JSON.stringify({
      telling: 1, point: { action: 2, offset: 0 }, viewing: 20, focus: { kind: "world" },
      chart: { camera: [0, 0, 0, 10], globeCamera: [0, "5", 10, 10], projection: "unknown", rotation: [0, 91] },
    });
    stored.set("langgen.place.bad", raw);
    expect(readPlace("bad")).toEqual({
      telling: 1, point: { action: 2, offset: 0 }, viewing: 20, focus: { kind: "world" }, chart: {},
    });
    expect(stored.get("langgen.place.bad")).toBe(raw);
  });
});
