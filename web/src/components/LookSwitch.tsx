import { useEffect, useState } from "react";

// Visual directions on trial; see looks.css. The choice is a per-browser
// convenience, so storage failures are ignored.
// Each choice is a look plus a forced theme; "auto" follows the system.
const LOOKS = [
  ["scriptorium", "Scriptorium"],
  ["scriptorium-day", "Scriptorium by day"],
  ["scriptorium-night", "Scriptorium by candlelight"],
  ["workshop", "Plain workshop"],
] as const;

const KEY = "langgen.look";

function saved(): string {
  try {
    const look = localStorage.getItem(KEY);
    return LOOKS.some(([id]) => id === look) ? (look as string) : "scriptorium";
  } catch {
    return "scriptorium";
  }
}

export function LookSwitch() {
  const [look, setLook] = useState(saved);
  useEffect(() => {
    const [base, mode] = look.split("-");
    document.documentElement.dataset.look = base;
    document.documentElement.dataset.theme = mode === "day" ? "light" : mode === "night" ? "dark" : "auto";
    try {
      localStorage.setItem(KEY, look);
    } catch {
      // Not remembered; still applied.
    }
  }, [look]);
  return (
    <label className="look-switch">
      Look
      <select value={look} onChange={(e) => setLook(e.target.value)}>
        {LOOKS.map(([id, name]) => (
          <option key={id} value={id}>
            {name}
          </option>
        ))}
      </select>
    </label>
  );
}
