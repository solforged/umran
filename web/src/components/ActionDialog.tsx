import { useState, type FormEvent, type ReactNode } from "react";
import type { Action, Catalog, ContactKind, Naming, Overview } from "../model";
import { NamingSelect } from "./NamingSelect";
import { Modal } from "./Modal";

/// "found" opens the language designer; the rest are here.
export type DialogKind = "found" | "split" | "connect" | "shift";

const TITLES: Record<"split" | "connect" | "shift", string> = {
  split: "A people parts ways",
  connect: "Two peoples meet",
  shift: "A people changes its tongue",
};

/// Forms for splitting, connecting, and shifting. Each validates the
/// basics; the engine has the last word and reports anything it rejects.
export function ActionDialog({
  kind,
  catalog,
  overview,
  selected,
  onClose,
  onAction,
}: {
  kind: "split" | "connect" | "shift";
  catalog: Catalog;
  overview: Overview;
  selected: number;
  onClose: () => void;
  onAction: (action: Action) => void;
}) {
  const communities = overview.communities;
  const others = communities.filter((c) => c.id !== selected);
  const [naming, setNaming] = useState<Naming | null>(null);
  const [community, setCommunity] = useState(selected);
  const [other, setOther] = useState(others[0]?.id ?? 0);
  const [contact, setContact] = useState<ContactKind>("neighbours");
  const [intensity, setIntensity] = useState(kind === "split" ? 0.4 : 0.6);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    switch (kind) {
      case "split":
        onAction({ kind: "split", community, intensity, ...(naming ? { naming } : {}) });
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

  const body: ReactNode = (() => {
    switch (kind) {
      case "split":
        return (
          <>
            {pick("Community", community, setCommunity)}
            <label>
              The new community calls itself
              <NamingSelect
                catalog={catalog}
                value={naming}
                onChange={setNaming}
                daughter
                parent={communities.find((c) => c.id === community)?.name}
              />
              <small>Built from their own words, so it follows their sound changes.</small>
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

  return (
    <Modal
      open
      title={TITLES[kind]}
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" form={`form-${kind}`} className="primary">
            Write it down
          </button>
        </>
      }
    >
      <form id={`form-${kind}`} className="form" onSubmit={submit}>
        {body}
      </form>
    </Modal>
  );
}
