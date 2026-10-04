import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { bookLand, readBook } from "../web/src/shelf";

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
    save("current", { revision: 35, seed: 5, map: "large", geography: "continental-v3" });
    expect(bookLand("current", 35)).toEqual({ seed: 5, size: "large", geography: "continental-v3" });
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
