import { useEffect, useSyncExternalStore } from "react";

// The chart by day or by lamplight, or as the system sets it. The choice is
// a per-browser convenience, so storage failures are ignored.
const LIGHTS = [
  ["auto", "As the system sets it"],
  ["light", "By day"],
  ["dark", "By lamplight"],
] as const;

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
  return <>{[LIGHTS[1], LIGHTS[2], LIGHTS[0]].map(([id, name]) => (
    <button key={id} type="button" role="menuitemradio" aria-checked={light === id} onClick={() => setLight(id)}>{name}</button>
  ))}</>;
}

export function LightSwitch() {
  const [light, setLight] = useLight();
  return (
    <label className="light-switch">
      Light
      <select value={light} onChange={(e) => setLight(e.target.value)}>
        {LIGHTS.map(([id, name]) => (
          <option key={id} value={id}>
            {name}
          </option>
        ))}
      </select>
    </label>
  );
}
