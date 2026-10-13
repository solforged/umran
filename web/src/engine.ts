import init, { Workbench } from "./wasm/umran_web";
import type {
  Naming,
  Action,
  Catalog,
  Engine,
  FoundingPreview,
  GeographyVersion,
  ReadEngine,
  Read,
  Comparison,
  ClimateView,
  ContactKind,
  ContactReach,
  Forming,
  FoundingTraits,
  RiverNamesView,
  LakeNamesView,
  ReadingRef,
  NotebookNote,
  Destination,
  HistoryPoint,
  Kin,
  LanguageDesign,
  LawChoice,
  LexiconRow,
  MapSize,
  Overview,
  Preview,
  SettlementIntent,
  Subject,
  WordDetail,
  WordMap,
  WorldMap,
  SettlementPreview,
  Story,
  DecisionView,
  LawOccurrence,
  WordLife,
  LoanTrail,
  GrammarStory,
  BuilderExample,
} from "./model";
import type { EngineReply, EngineRequest, EngineStatus, Opened, ReadMethod, WriteMethod, Written } from "./engineProtocol";

// Worlds live on the engine's own thread (engine.worker.ts), so a long
// generation never freezes the page. The page keeps a second instance of the
// module only for the pure helpers below, which hold no world: the catalog,
// preset designs, and word previews. Errors from the engine arrive as strings.
let initialized: Promise<unknown> | undefined;

async function ready(): Promise<void> {
  initialized ??= init().catch((error: unknown) => {
    initialized = undefined;
    throw error;
  });
  await initialized;
}

type Request = EngineRequest extends infer R ? R extends unknown ? Omit<R, "id"> : never : never;

let thread: Worker | undefined;
const waiting = new Map<number, { resolve: (value: unknown) => void; reject: (error: string) => void }>();
let nextRequest = 1;

function ask<T>(request: Request): Promise<T> {
  if (!thread) {
    thread = new Worker(new URL("./engine.worker.ts", import.meta.url), { type: "module" });
    thread.onmessage = (event: MessageEvent<EngineReply>) => {
      const reply = event.data;
      const pending = waiting.get(reply.id);
      if (!pending) return;
      waiting.delete(reply.id);
      if (reply.ok) pending.resolve(reply.value);
      else pending.reject(reply.error);
    };
    // A thread that cannot start, or dies, fails everything still waiting;
    // the next request starts a fresh one.
    thread.onerror = (event) => {
      event.preventDefault();
      const reason = event.message || "The engine stopped.";
      waiting.forEach((pending) => pending.reject(reason));
      waiting.clear();
      thread?.terminate();
      thread = undefined;
    };
  }
  const id = nextRequest++;
  return new Promise<T>((resolve, reject) => {
    waiting.set(id, { resolve: resolve as (value: unknown) => void, reject });
    thread!.postMessage({ ...request, id });
  });
}

/// Marks a promise with its settled state, the protocol React's `use` reads
/// so a kept answer renders without suspending.
function tracked<T>(promise: Promise<T>): Read<T> {
  const read = Object.assign(promise, { status: "pending" as const }) as Read<T>;
  promise.then(
    (value) => Object.assign(read, { status: "fulfilled", value }),
    (reason: unknown) => Object.assign(read, { status: "rejected", reason }),
  );
  return read;
}

/// Answers kept by key, so re-rendering asks the thread nothing new. Bounded,
/// oldest first: whole-year views, since scrubbing through the years would
/// otherwise keep every year's, and a card's many small ones more loosely.
class Answers {
  private kept = new Map<string, Read<unknown>>();
  constructor(private limit: number) {}
  get<T>(key: string, load: () => Promise<T>): Read<T> {
    let read = this.kept.get(key);
    if (read) this.kept.delete(key);
    else {
      read = tracked(load());
      if (this.kept.size >= this.limit) this.kept.delete(this.kept.keys().next().value!);
    }
    this.kept.set(key, read);
    return read as Read<T>;
  }
}

const json = (value: unknown) => JSON.parse(value as string) as unknown;

class Reading implements ReadEngine {
  private years = new Answers(12);
  private answers = new Answers(4096);
  constructor(private world: World, private scope: { telling: number; point: string } | null) {}
  private ask<T>(method: ReadMethod, ...args: unknown[]): Read<T> {
    return this.answers.get(`${method}:${JSON.stringify(args)}`, () => this.world.call(method, args, this.scope).then(json) as Promise<T>);
  }
  private year<T>(method: ReadMethod, ...args: unknown[]): Read<T> {
    return this.years.get(`${method}:${JSON.stringify(args)}`, () => this.world.call(method, args, this.scope).then(json) as Promise<T>);
  }
  /// One answer per id, asked of the thread at once.
  private each<T>(method: ReadMethod, generation: number, count: (map: WorldMap) => number): Read<T[]> {
    return this.years.get(`${method}s:${generation}`, async () => {
      const ids = Array.from({ length: count(await this.map()) }, (_, id) => [generation, id]);
      const answers = await this.world.callEach(method, ids, this.scope);
      return answers.map(json) as T[];
    });
  }
  overviewAt(point: HistoryPoint) { return this.year<Overview>("overviewAt", JSON.stringify(point)); }
  settlement(point: HistoryPoint, community: number, intent: SettlementIntent, share: number, destination: number | null) {
    return this.ask<SettlementPreview>("settlement", JSON.stringify(point), community, intent, share, destination ?? -1);
  }
  contactReach(point: HistoryPoint, a: number, b: number, contact: ContactKind) {
    return this.ask<ContactReach>("contactReach", JSON.stringify(point), a, b, contact);
  }
  lawChoices(point: HistoryPoint, variety: number) { return this.ask<LawChoice[]>("lawChoices", JSON.stringify(point), variety); }
  latest() { return this.answers.get("latest", () => this.world.call("latest", [], this.scope) as Promise<number>); }
  overview(generation: number) { return this.year<Overview>("overview", generation); }
  lexicon(generation: number, variety: number) { return this.ask<LexiconRow[]>("lexicon", generation, variety); }
  kin(generation: number, variety: number) { return this.ask<Kin[]>("kin", generation, variety); }
  word(generation: number, variety: number, concept: string) { return this.ask<WordDetail>("word", generation, variety, concept); }
  lawOccurrences(generation: number, variety: number, law: string) { return this.ask<LawOccurrence[]>("lawOccurrences", generation, variety, law); }
  specimenLives(generation: number, variety: number) { return this.ask<WordLife[]>("specimenLives", generation, variety); }
  wordLife(generation: number, variety: number, concept: string) { return this.ask<WordLife>("wordLife", generation, variety, concept); }
  loanTrail(generation: number, variety: number, concept: string) { return this.ask<LoanTrail | null>("loanTrail", generation, variety, concept); }
  grammarStory(generation: number, variety: number, category: string) { return this.ask<GrammarStory>("grammarStory", generation, variety, category); }
  builderExamples(generation: number, variety: number) { return this.ask<BuilderExample[]>("builderExamples", generation, variety); }
  story(generation: number, subject: Subject) { return this.ask<Story>("story", generation, JSON.stringify(subject)); }
  climate(generation: number) { return this.year<ClimateView>("climate", generation); }
  river(generation: number, id: number) { return this.ask<RiverNamesView>("river", generation, id); }
  lake(generation: number, id: number) { return this.ask<LakeNamesView>("lake", generation, id); }
  rivers(generation: number) { return this.each<RiverNamesView>("river", generation, (map) => map.rivers.length); }
  lakes(generation: number) { return this.each<LakeNamesView>("lake", generation, (map) => map.lakes.length); }
  map() { return this.world.map(); }
  wordMap(generation: number, concept: string) { return this.ask<WordMap>("wordMap", generation, concept); }
  save() { return this.world.save(); }
  decisions() { return this.ask<DecisionView[]>("decisions"); }
}

class World implements Engine {
  private status!: EngineStatus;
  private notes!: NotebookNote[];
  private present!: Reading;
  private readings = new Map<string, Reading>();
  private answers = new Answers(256);
  private drawn: Read<WorldMap> | undefined;
  private closed = false;

  constructor(private id: number, status: EngineStatus) {
    this.apply(status);
  }

  /// A change makes every earlier reading of this world history; new
  /// readings start from the world as it now stands.
  private apply(status: EngineStatus) {
    this.status = status;
    this.notes = JSON.parse(status.notebook) as NotebookNote[];
    this.present = new Reading(this, null);
    this.readings = new Map();
    this.answers = new Answers(256);
  }

  call(method: ReadMethod, args: unknown[], scope: { telling: number; point: string } | null): Promise<unknown> {
    if (this.closed) return Promise.reject("This world has been closed.");
    return ask({ op: "read", world: this.id, scope, method, args });
  }

  callEach(method: ReadMethod, calls: unknown[][], scope: { telling: number; point: string } | null): Promise<unknown[]> {
    if (this.closed) return Promise.reject("This world has been closed.");
    return ask({ op: "readEach", world: this.id, scope, method, calls });
  }

  private async write(method: WriteMethod, ...args: unknown[]): Promise<unknown> {
    if (this.closed) throw "This world has been closed.";
    const written = await ask<Written>({ op: "write", world: this.id, method, args });
    this.apply(written.status);
    return written.value;
  }

  private own<T>(method: ReadMethod, ...args: unknown[]): Read<T> {
    return this.answers.get(`${method}:${JSON.stringify(args)}`, () => this.call(method, args, null).then(json) as Promise<T>);
  }

  latest() { return this.status.latest; }
  save() { return this.status.save; }
  title() { return this.status.title; }
  author() { return this.status.author; }
  notebook() { return this.notes; }
  tip(): ReadEngine { return this.present; }
  read(telling: number, point?: HistoryPoint | null): ReadEngine {
    const encoded = point ? JSON.stringify(point) : "";
    const key = `${telling}:${encoded}`;
    let reading = this.readings.get(key);
    if (!reading) { reading = new Reading(this, { telling, point: encoded }); this.readings.set(key, reading); }
    return reading;
  }
  /// The land never changes with history, so it is drawn once per world.
  map() {
    this.drawn ??= tracked(this.call("map", [], null).then(json) as Promise<WorldMap>);
    return this.drawn;
  }
  async foundingDraft(): Promise<Engine> {
    if (this.closed) throw "This world has been closed.";
    const draft = opened(await ask<Opened>({ op: "draft", world: this.id }));
    // A draft shares its base's geography, so it shares the drawn map too.
    draft.drawn = this.drawn;
    return draft;
  }
  foundingPreview() { return this.own<FoundingPreview>("foundingPreview"); }
  foundingSites(region: number, count: number) { return this.own<number[]>("foundingSites", region, count); }
  foundingTraits(community: number) { return this.own<FoundingTraits>("foundingTraits", community); }
  forming() {
    return this.answers.get("forming", () => this.call("forming", [], null).then((value) => json(value) as Forming, () => null));
  }
  async saveNote(note: NotebookNote) { await this.write("saveNote", JSON.stringify(note)); }
  async resolveNote(id: string) { return json(await this.call("resolveNote", [id], null)) as Destination | null; }
  async removeNote(id: string) { await this.write("removeNote", id); }
  async setTitle(title: string) { await this.write("setTitle", title); }
  async setAuthor(author: string) { await this.write("setAuthor", author); }
  async previous(reading: ReadingRef) { return json(await this.call("previous", [JSON.stringify(reading)], null)) as ReadingRef; }
  compare(left: number, right: number, generation: number) { return this.own<Comparison>("compare", left, right, generation); }
  async rename(telling: number, name: string) { await this.write("rename", telling, name); }
  async actAt(reading: ReadingRef, mutation: number, action: Action) {
    await this.write("actAt", JSON.stringify(reading), mutation, JSON.stringify(action));
  }
  async untilAt(reading: ReadingRef, mutation: number, limit: number) {
    return await this.write("untilAt", JSON.stringify(reading), mutation, limit) as number;
  }
  async act(action: Action) { await this.write("act", JSON.stringify(action)); }
  async runUntilEvent(limit: number) { return await this.write("runUntilEvent", limit) as number; }
  async branch(generation: number) { await this.write("branch", generation); }
  async restore(telling: number) { await this.write("restore", telling); }
  dispose() {
    if (this.closed) return;
    this.closed = true;
    ask({ op: "free", world: this.id }).catch(() => undefined);
  }
}

function opened(world: Opened): World {
  return new World(world.world, world.status);
}

/// A read that always settles, for views that show their failure in place.
export type Outcome<T> = { value: T; error: null } | { value: null; error: string };
const outcomes = new WeakMap<Read<unknown>, Read<Outcome<unknown>>>();
export function attempt<T>(read: Read<T>): Read<Outcome<T>> {
  let outcome = outcomes.get(read);
  if (!outcome) {
    outcome = tracked(read.then((value) => ({ value, error: null }), (error: unknown) => ({ value: null, error: message(error) })));
    outcomes.set(read, outcome);
  }
  return outcome as Read<Outcome<T>>;
}

export async function createEngine(seed: number, size: MapSize): Promise<Engine> {
  if (!Number.isInteger(seed) || seed < 0 || seed > 0xffff_ffff) {
    throw new Error("Choose a whole-number seed between 0 and 4294967295.");
  }
  return opened(await ask<Opened>({ op: "create", seed, size }));
}

export async function loadEngine(save: string): Promise<Engine> {
  return opened(await ask<Opened>({ op: "load", json: save }));
}

// Seed, size and the recipe's geography version determine a map, so the shelf
// can draw a saved world without replaying its history. Kept once drawn.
const maps = new Map<string, Promise<WorldMap>>();

export function landMap(seed: number, size: MapSize, geography: GeographyVersion): Promise<WorldMap> {
  const key = `${seed}:${size}:${geography}`;
  let map = maps.get(key);
  if (!map) {
    map = ask<string>({ op: "geography", seed, size, geography }).then((drawn) => JSON.parse(drawn) as WorldMap);
    maps.set(key, map);
    map.catch(() => maps.delete(key));
  }
  return map;
}

export async function loadCatalog(): Promise<Catalog> {
  await ready();
  return JSON.parse(Workbench.catalog()) as Catalog;
}

// Design helpers; call only after `loadCatalog` has resolved, which
// initializes the page's module.
export function presetDesign(preset: string, seed: number): LanguageDesign {
  return JSON.parse(Workbench.design(preset, seed)) as LanguageDesign;
}

/// Sounds drawn by how common they are worldwide, in the numbers given.
export function frequencyDesign(seed: number, consonants: number, vowels: number): LanguageDesign {
  return JSON.parse(Workbench.frequencyDesign(seed, consonants, vowels)) as LanguageDesign;
}

/// Sample words for a design, or the reason it cannot found a language.
export function preview(design: LanguageDesign, seed: number, naming: Naming): Preview | string {
  try {
    return JSON.parse(Workbench.preview(JSON.stringify(design), seed, JSON.stringify(naming))) as Preview;
  } catch (error) {
    return message(error);
  }
}

export function message(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
