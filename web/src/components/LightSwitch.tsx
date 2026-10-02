import { Monitor, Moon, Sun, type LucideIcon } from "lucide-react";
import { useEffect, useSyncExternalStore } from "react";

// The chart by day or by lamplight, or as the system sets it. The choice is
// a per-browser convenience, so storage failures are ignored.
const LIGHTS: readonly (readonly [string, string, LucideIcon])[] = [
  ["auto", "As the system sets it", Monitor],
  ["light", "By day", Sun],
  ["dark", "By lamplight", Moon],
];
/// The order the switch cycles through: day, lamplight, then the system.
const CYCLE = [LIGHTS[1], LIGHTS[2], LIGHTS[0]];

const KEY = "umran.light";

function saved(): string {
  try {
    const light = localStorage.getItem(KEY);
    return LIGHTS.some(([id]) => id === light) ? (light as string) : "auto";
  } catch {
    return "auto";
  }
}

let choice: string | undefined;
const listeners = new Set<() => void>();
const snapshot = () => choice ??= saved();
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
};
const choose = (light: string) => {
  if (snapshot() === light) return;
  choice = light;
  listeners.forEach((listener) => listener());
};

export function useLight(): [string, (light: string) => void] {
  const light = useSyncExternalStore(subscribe, snapshot);
  useEffect(() => {
    document.documentElement.dataset.theme = light;
    try {
      localStorage.setItem(KEY, light);
    } catch {
      // Not remembered; still applied.
    }
  }, [light]);
  return [light, choose];
}

export function LightChoice() {
  const [light, setLight] = useLight();
  return <>{CYCLE.map(([id, name, Icon]) => (
    <button key={id} type="button" role="menuitemradio" aria-checked={light === id} onClick={() => setLight(id)}>
      <Icon size={15} aria-hidden="true" /> {name}
    </button>
  ))}</>;
}

/// One button showing the light in use; each press moves to the next.
export function LightSwitch() {
  const [light, setLight] = useLight();
  const at = Math.max(0, CYCLE.findIndex(([id]) => id === light));
  const [, name, Icon] = CYCLE[at];
  const [nextId, nextName] = CYCLE[(at + 1) % CYCLE.length];
  const title = `${name}. Switch to ${nextName.toLowerCase()}`;
  return (
    <button type="button" className="light-switch icon" title={title} aria-label={title} onClick={() => setLight(nextId)}>
      <Icon size={16} aria-hidden="true" />
    </button>
  );
}
