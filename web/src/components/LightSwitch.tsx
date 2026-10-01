import { useEffect, useState } from "react";

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

export function LightSwitch() {
  const [light, setLight] = useState(saved);
  useEffect(() => {
    document.documentElement.dataset.theme = light;
    try {
      localStorage.setItem(KEY, light);
    } catch {
      // Not remembered; still applied.
    }
  }, [light]);
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
