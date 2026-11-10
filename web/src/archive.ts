// The archive: IndexedDB beside the shelf, for records too large for the
// shelf's own storage. It keeps each world's frozen record (the world as an
// engine revision resolved it, megabytes of data) so a later engine can say
// what it reads differently. The shelf's saves stay where they are: they are
// small, and saving them must not wait.
//
// Everything here is a convenience over the save. A failure is reported to
// the caller and never blocks or undoes a save.

const DATABASE = "umran";
const FROZEN = "frozen";

/// One world as one engine revision last read it.
export interface FrozenMeta {
  id: string;
  /// The engine revision that resolved the record.
  revision: number;
  generation: number;
  /// The shelf revision of the save it was taken from.
  saved: number;
  taken: number;
}
interface Stored extends FrozenMeta {
  bytes: ArrayBuffer;
  gzip: boolean;
}

let opened: Promise<IDBDatabase> | undefined;

function database(): Promise<IDBDatabase> {
  return opened ??= new Promise<IDBDatabase>((resolve, reject) => {
    if (typeof indexedDB === "undefined") { reject(new Error("This browser has no archive storage.")); return; }
    const request = indexedDB.open(DATABASE, 1);
    request.onupgradeneeded = () => {
      const store = request.result.createObjectStore(FROZEN, { keyPath: ["id", "revision"] });
      store.createIndex("id", "id");
    };
    request.onsuccess = () => {
      // Another tab with a newer app may need to upgrade; do not hold it up.
      request.result.onversionchange = () => { request.result.close(); opened = undefined; };
      resolve(request.result);
    };
    request.onerror = () => reject(request.error ?? new Error("The archive could not be opened."));
    request.onblocked = () => reject(new Error("The archive is in use by another tab."));
  }).catch((error: unknown) => { opened = undefined; throw error; });
}

function settled<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error("The archive could not be read."));
  });
}

function finished(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = transaction.onabort = () => reject(transaction.error ?? new Error("The archive could not be written."));
  });
}

async function pack(text: string): Promise<{ bytes: ArrayBuffer; gzip: boolean }> {
  const plain = new Blob([text]);
  if (typeof CompressionStream === "undefined") return { bytes: await plain.arrayBuffer(), gzip: false };
  return { bytes: await new Response(plain.stream().pipeThrough(new CompressionStream("gzip"))).arrayBuffer(), gzip: true };
}

async function unpack(stored: Stored): Promise<string> {
  const packed = new Blob([stored.bytes]);
  if (!stored.gzip) return packed.text();
  return new Response(packed.stream().pipeThrough(new DecompressionStream("gzip"))).text();
}

const meta = ({ id, revision, generation, saved, taken }: FrozenMeta): FrozenMeta => ({ id, revision, generation, saved, taken });

/// Keep `text`, a frozen file, as how `revision` last read world `id`. It
/// replaces that revision's earlier record and no other revision's.
export async function keepFrozen(record: Omit<FrozenMeta, "taken">, text: string): Promise<void> {
  const packed = await pack(text);
  const transaction = (await database()).transaction(FROZEN, "readwrite");
  transaction.objectStore(FROZEN).put({ ...meta({ ...record, taken: Date.now() }), ...packed } satisfies Stored);
  await finished(transaction);
}

/// What is kept of one world, oldest revision first, or of every world.
export async function frozenRecords(id?: string): Promise<FrozenMeta[]> {
  const store = (await database()).transaction(FROZEN).objectStore(FROZEN);
  const all = await settled<Stored[]>(id === undefined ? store.getAll() : store.index("id").getAll(id));
  return all.map(meta).sort((a, b) => a.id.localeCompare(b.id) || a.revision - b.revision);
}

/// The frozen file kept for a world under one revision.
export async function frozenText(id: string, revision: number): Promise<string | null> {
  const store = (await database()).transaction(FROZEN).objectStore(FROZEN);
  const stored = await settled<Stored | undefined>(store.get([id, revision]));
  return stored ? unpack(stored) : null;
}

/// A world removed for good takes its records with it.
export async function removeFrozen(id: string): Promise<void> {
  const transaction = (await database()).transaction(FROZEN, "readwrite");
  const store = transaction.objectStore(FROZEN);
  for (const key of await settled(store.index("id").getAllKeys(id))) store.delete(key);
  await finished(transaction);
}

/// Whether the browser has agreed to keep this site's data. Without that it
/// may clear saved worlds when the device runs short of space.
export type Keeping = "kept" | "may-clear" | "unknown";

export async function keeping(): Promise<Keeping> {
  try {
    if (!navigator.storage?.persisted) return "unknown";
    return await navigator.storage.persisted() ? "kept" : "may-clear";
  } catch {
    return "unknown";
  }
}

/// Ask the browser to keep this site's data. Some browsers decide silently
/// and some ask the reader, so call this when there is a world worth keeping.
export async function askToKeep(): Promise<Keeping> {
  try {
    if (!navigator.storage?.persist) return "unknown";
    if (await navigator.storage.persisted()) return "kept";
    return await navigator.storage.persist() ? "kept" : "may-clear";
  } catch {
    return "unknown";
  }
}
