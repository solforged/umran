/// <reference lib="webworker" />
// The engine's thread. Every world, founding draft, and reading lives here,
// so a long generation never blocks the page. The page sends one request at a
// time per call; this thread answers them in order, so a read sent after a
// change always sees it.
import init, { ReadView, Workbench } from "./wasm/umran_web";
import type { EngineReply, EngineRequest, EngineStatus } from "./engineProtocol";
import { READ_METHODS, WRITE_METHODS } from "./engineProtocol";

declare const self: DedicatedWorkerGlobalScope;

interface World {
  bench: Workbench;
  /// Readings of other tellings or fixed points, newest last. A change to the
  /// world frees them; a later read opens them again from the new history.
  scopes: Map<string, ReadView>;
}

const SCOPES = 4;
const worlds = new Map<number, World>();
let nextWorld = 1;
const ready = init();

function world(id: number): World {
  const found = worlds.get(id);
  if (!found) throw "This world has been closed.";
  return found;
}

function clearScopes(entry: World) {
  entry.scopes.forEach((scope) => scope.free());
  entry.scopes.clear();
}

function scope(entry: World, telling: number, point: string): ReadView {
  const key = `${telling}:${point}`;
  let view = entry.scopes.get(key);
  if (view) entry.scopes.delete(key);
  else {
    view = entry.bench.read(telling, point);
    if (entry.scopes.size >= SCOPES) {
      const [oldest, stale] = entry.scopes.entries().next().value!;
      stale.free(); entry.scopes.delete(oldest);
    }
  }
  entry.scopes.set(key, view);
  return view;
}

function status(bench: Workbench): EngineStatus {
  return {
    latest: bench.latest(),
    save: bench.save(),
    title: bench.title() ?? null,
    author: bench.author() ?? null,
    notebook: bench.notebook(),
  };
}

function open(bench: Workbench) {
  const id = nextWorld++;
  worlds.set(id, { bench, scopes: new Map() });
  return { world: id, status: status(bench) };
}

type Callable = Record<string, (...args: unknown[]) => unknown>;

function answer(request: EngineRequest): unknown {
  switch (request.op) {
    case "create": return open(new Workbench(request.seed, request.size));
    case "load": return open(Workbench.load(request.json));
    case "draft": return open(world(request.world).bench.foundingDraft());
    case "geography": {
      const bench = Workbench.withGeography(request.seed, request.size, request.geography);
      try { return bench.map(); } finally { bench.free(); }
    }
    case "free": {
      const entry = worlds.get(request.world);
      if (entry) { clearScopes(entry); entry.bench.free(); worlds.delete(request.world); }
      return null;
    }
    case "read":
    case "readEach": {
      if (!Object.hasOwn(READ_METHODS, request.method)) throw `Unknown reading: ${request.method}`;
      const entry = world(request.world);
      const target = (request.scope ? scope(entry, request.scope.telling, request.scope.point) : entry.bench) as unknown as Callable;
      if (request.op === "read") return target[request.method](...request.args) ?? null;
      return request.calls.map((args) => target[request.method](...args) ?? null);
    }
    case "write": {
      if (!Object.hasOwn(WRITE_METHODS, request.method)) throw `Unknown change: ${request.method}`;
      const entry = world(request.world);
      clearScopes(entry);
      const value = (entry.bench as unknown as Callable)[request.method](...request.args) ?? null;
      return { value, status: status(entry.bench) };
    }
  }
}

self.onmessage = async (event: MessageEvent<EngineRequest>) => {
  const request = event.data;
  let reply: EngineReply;
  try {
    await ready;
    reply = { id: request.id, ok: true, value: answer(request) };
  } catch (error) {
    reply = { id: request.id, ok: false, error: typeof error === "string" ? error : error instanceof Error ? error.message : String(error) };
  }
  self.postMessage(reply);
};
