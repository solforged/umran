import { useState, type FormEvent, type ReactNode } from "react";
import type { Action, Catalog, ContactKind, Overview } from "../model";
import { Modal } from "./Modal";

export type DialogKind = "world" | "found" | "split" | "connect" | "shift";

const TITLES: Record<DialogKind, string> = {
  world: "New world",
  found: "Found a community",
  split: "Split a community",
  connect: "Bring two communities into contact",
  shift: "Shift a community's language",
};

function randomSeed(): number {
  return Math.floor(Math.random() * 0xffff_ffff);
}

/// Forms for every action, plus the new-world form. Each validates the
/// basics; the engine has the last word and reports anything it rejects.
export function ActionDialog({
  kind,
  inline = false,
  catalog,
  overview,
  selected,
  onClose,
  onAction,
  onWorld,
}: {
  kind: DialogKind;
  inline?: boolean;
  catalog: Catalog;
  overview: Overview | null;
  selected: number;
  onClose: () => void;
  onAction: (action: Action) => void;
  onWorld: (seed: number, founding: Action) => void;
}) {
  const communities = overview?.communities ?? [];
  const others = communities.filter((c) => c.id !== selected);
  const [seed, setSeed] = useState(() => randomSeed());
  const [name, setName] = useState(kind === "split" ? `${communities[selected]?.name ?? ""} 2` : "");
  const [profile, setProfile] = useState(catalog.profiles[0]?.id ?? "neutral");
  const [flavors, setFlavors] = useState<string[]>([]);
  const [power, setPower] = useState(0.5);
  const [openness, setOpenness] = useState(0.5);
  const [community, setCommunity] = useState(selected);
  const [other, setOther] = useState(others[0]?.id ?? 0);
  const [contact, setContact] = useState<ContactKind>("neighbours");
  const [intensity, setIntensity] = useState(kind === "split" ? 0.4 : 0.6);

  const founding = (): Action => ({ kind: "found", name, profile, flavors, power, openness });

  const submit = (event: FormEvent) => {
    event.preventDefault();
    switch (kind) {
      case "world":
        onWorld(seed, founding());
        return;
      case "found":
        onAction(founding());
        return;
      case "split":
        onAction({ kind: "split", community, name, intensity });
        return;
      case "connect":
        onAction({ kind: "connect", a: community, b: other, intensity, contact });
        return;
      case "shift":
        onAction({ kind: "shift", community, toward: other });
    }
  };

  const pick = (label: string, value: number, set: (n: number) => void, exclude?: number) => (
    <label>
      {label}
      <select value={value} onChange={(e) => set(Number(e.target.value))}>
        {communities
          .filter((c) => c.id !== exclude)
          .map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
      </select>
    </label>
  );

  const slider = (label: string, value: number, set: (n: number) => void, hint: string) => (
    <label>
      {label} <output>{value.toFixed(2)}</output>
      <input type="range" min={0} max={1} step={0.05} value={value} onChange={(e) => set(Number(e.target.value))} />
      <small>{hint}</small>
    </label>
  );

  const culture: ReactNode = (
    <>
      <label>
        Name
        <input required value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Hill" />
      </label>
      <fieldset>
        <legend>Sound preferences</legend>
        {catalog.profiles.map((p) => (
          <label key={p.id} className="choice">
            <input type="radio" name="profile" checked={profile === p.id} onChange={() => setProfile(p.id)} />
            <span>
              <strong>{p.name}</strong> {p.description}
            </span>
          </label>
        ))}
      </fieldset>
      <fieldset>
        <legend>Flavors (optional)</legend>
        {catalog.flavors.map((f) => (
          <label key={f.id} className="choice">
            <input
              type="checkbox"
              checked={flavors.includes(f.id)}
              onChange={(e) => setFlavors(e.target.checked ? [...flavors, f.id] : flavors.filter((x) => x !== f.id))}
            />
            <span>
              <strong>{f.name}</strong> “{f.description}”
            </span>
          </label>
        ))}
      </fieldset>
      {slider("Power", power, setPower, "Standing apart from size: wealth, arms, sacred status.")}
      {slider("Openness", openness, setOpenness, "Readiness to take in foreign words.")}
    </>
  );

  const body: ReactNode = (() => {
    switch (kind) {
      case "world":
        return (
          <>
            <label>
              Seed
              <span className="row">
                <input
                  type="number"
                  min={0}
                  max={4294967295}
                  required
                  value={seed}
                  onChange={(e) => setSeed(Number(e.target.value))}
                />
                <button type="button" onClick={() => setSeed(randomSeed())}>
                  Random
                </button>
              </span>
              <small>The same seed and choices always give the same language.</small>
            </label>
            <h3>Founding community</h3>
            {culture}
          </>
        );
      case "found":
        return culture;
      case "split":
        return (
          <>
            {pick("Community", community, setCommunity)}
            <label>
              New community's name
              <input required value={name} onChange={(e) => setName(e.target.value)} />
            </label>
            {slider("Contact afterwards", intensity, setIntensity, "0 means the two lose touch entirely.")}
            <p className="muted">Half the people leave; from now on their speech changes on its own.</p>
          </>
        );
      case "connect":
        return (
          <>
            {pick("Community", community, setCommunity)}
            {pick("With", other, setOther, community)}
            <fieldset>
              <legend>Kind of contact</legend>
              {catalog.contacts.map((c) => (
                <label key={c.id} className="choice">
                  <input
                    type="radio"
                    name="contact"
                    checked={contact === c.id}
                    onChange={() => setContact(c.id as ContactKind)}
                  />
                  <span>
                    <strong>{c.name}</strong> {c.description}
                  </span>
                </label>
              ))}
            </fieldset>
            {slider("Intensity", intensity, setIntensity, "How much they deal with each other.")}
          </>
        );
      case "shift":
        return (
          <>
            {pick("Community", community, setCommunity)}
            {pick("Adopts the language of", other, setOther, community)}
            <p className="muted">
              They keep their own sound preferences and some old words, mostly about the land around them.
            </p>
          </>
        );
    }
  })();

  const form = (
    <form id={`form-${kind}`} className="form" onSubmit={submit}>
      {body}
      {inline ? (
        <button type="submit" className="primary">
          Create world
        </button>
      ) : null}
    </form>
  );

  if (inline) return form;
  return (
    <Modal
      open
      title={TITLES[kind]}
      onClose={onClose}
      wide={kind === "world" || kind === "found"}
      footer={
        <>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" form={`form-${kind}`} className="primary">
            {kind === "world" ? "Replace current world" : "Do it"}
          </button>
        </>
      }
    >
      {form}
    </Modal>
  );
}
