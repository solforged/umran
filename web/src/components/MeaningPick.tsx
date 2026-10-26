import { useId, useState } from "react";
import type { ConceptView } from "../model";

type Pickable = Pick<ConceptView, "id" | "gloss" | "pronoun" | "custom">;

/// Finds a meaning among the world's, by short name or gloss, and hands its
/// short name on, so a list of meanings need not be written from memory.
export function MeaningPick({ meanings, label = "Find a meaning", onPick }: {
  meanings: Pickable[];
  label?: string;
  onPick: (concept: Pickable) => void;
}) {
  const [query, setQuery] = useState("");
  const [missed, setMissed] = useState<string | null>(null);
  const list = useId();
  // As the quarry resolves a list: id, gloss, then one part of a gloss
  // that lists several ("chief, king" answers to "king").
  const pick = (text: string) => {
    const wanted = text.trim().toLowerCase();
    if (!wanted) return false;
    const parts = meanings.filter((m) => m.gloss.toLowerCase().split(", ").includes(wanted));
    const concept = meanings.find((m) => m.id === wanted) ?? meanings.find((m) => m.gloss.toLowerCase() === wanted) ?? (parts.length === 1 ? parts[0] : undefined);
    if (!concept) { setMissed(text.trim()); return false; }
    onPick(concept);
    setQuery("");
    setMissed(null);
    return true;
  };
  return <label className="meaning-pick">
    <span className="muted small">{label}</span>
    <input type="search" list={list} value={query} spellCheck={false} placeholder={`${meanings.length} meanings: water, king, to see…`}
      onChange={(event) => { setQuery(event.target.value); setMissed(null); if ((event.nativeEvent as InputEvent).inputType === "insertReplacementText" || !(event.nativeEvent instanceof InputEvent)) pick(event.target.value); }}
      onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); pick(query); } }} />
    {missed ? <span className="muted small" role="status">No one meaning is called “{missed}”; choose one from the list as you type.</span> : null}
    <datalist id={list}>{meanings.filter((m) => !m.pronoun).map((m) => <option key={m.id} value={m.id}>{m.gloss}{m.custom ? " · yours" : ""}</option>)}</datalist>
  </label>;
}
