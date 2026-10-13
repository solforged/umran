/// Hash destinations work unchanged beneath GitHub Pages' /umran/ path.
export type Route =
  | { kind: "shelf" }
  | { kind: "setup"; draft?: string }
  | { kind: "world"; id: string; page: "stage" | "book"; chapter?: string };

/// A legacy chapter hash belongs to the last opened world, never a new one.
export function parse(hash: string, last: string | null = null): Route | null {
  if (hash === "" || hash === "#" || hash === "#/") return { kind: "shelf" };
  try {
    if (hash.startsWith("#book-")) {
      const chapter = decodeURIComponent(hash.slice(6));
      return last && chapter ? { kind: "world", id: last, page: "book", chapter } : null;
    }
    if (!hash.startsWith("#/")) return null;
    const parts = hash.slice(2).split("/").map(decodeURIComponent);
    if (parts.some((part) => !part)) return null;
    if (parts[0] === "found" && parts.length <= 2) return { kind: "setup", ...(parts[1] ? { draft: parts[1] } : {}) };
    if (parts[0] !== "world" || !parts[1]) return null;
    if (parts.length === 2) return { kind: "world", id: parts[1], page: "stage" };
    if (parts[2] === "book" && parts.length <= 4) return { kind: "world", id: parts[1], page: "book", ...(parts[3] ? { chapter: parts[3] } : {}) };
    return null;
  } catch {
    return null;
  }
}

export function format(route: Route): string {
  switch (route.kind) {
    case "shelf": return "#/";
    case "setup": return `#/found${route.draft ? `/${encodeURIComponent(route.draft)}` : ""}`;
    case "world": return `#/world/${encodeURIComponent(route.id)}${route.page === "book" ? `/book${route.chapter ? `/${encodeURIComponent(route.chapter)}` : ""}` : ""}`;
  }
}

export function push(route: Route): void {
  window.history.pushState({ umran: true }, "", format(route));
}

export function replace(route: Route): void {
  window.history.replaceState({ umran: true }, "", format(route));
}
