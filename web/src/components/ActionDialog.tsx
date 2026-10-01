import { useState, type FormEvent, type ReactNode } from "react";
import type { Action, Catalog, ContactKind, Craft, Naming, Overview } from "../model";
import { NamingSelect } from "./NamingSelect";
import { Modal } from "./Modal";

/// "found" opens the language designer; the rest are here.
export type DialogKind = "found" | "split" | "connect" | "shift" | "state" | "religion" | "craft";

const TITLES: Record<Exclude<DialogKind, "found">, string> = {
  split: "A people parts ways",
  connect: "Two peoples meet",
  shift: "A people takes up another language",
  state: "Found a state",
  religion: "Found a religion",
  craft: "Teach a craft",
};

/// Forms for shaping a people's history. Each validates the basics;
/// the engine has the last word and reports anything it rejects.
export function ActionDialog({
  kind,
  catalog,
  overview,
  selected,
  onClose,
  onAction,
}: {
  kind: Exclude<DialogKind, "found">;
  catalog: Catalog;
  overview: Overview;
  selected: number;
  onClose: () => void;
  onAction: (action: Action) => void;
}) {
  const communities = overview.communities.filter((c) => c.ended === null &&
    (kind !== "state" || !overview.states.some((s) => s.fell === null &&
      (s.rulers === c.id || s.members.some((m) => m.community === c.id && m.left === null)))) &&
    (kind !== "craft" || catalog.crafts.some((craft) => !c.crafts.includes(craft.id as Craft))));
  const first = communities.find((c) => c.id === selected)?.id ?? communities[0]?.id ?? -1;
  const others = communities.filter((c) => c.id !== first);
  const [naming, setNaming] = useState<Naming | null>(null);
  const [community, setCommunity] = useState(first);
  const [other, setOther] = useState(others[0]?.id ?? -1);
  const [contact, setContact] = useState<ContactKind>("neighbours");
  const [intensity, setIntensity] = useState(kind === "split" ? 0.4 : 0.6);
  const [capital, setCapital] = useState(overview.communities[first]?.region ?? -1);
  const [craft, setCraft] = useState<Craft>((catalog.crafts.find((c) =>
    !overview.communities[first]?.crafts.includes(c.id as Craft))?.id ?? "metalworking") as Craft);
  const people = communities.find((c) => c.id === community);
  const canSubmit = !!people && (kind === "state" ? people.lands.includes(capital) :
    kind === "craft" ? !people.crafts.includes(craft) :
    kind === "split" || kind === "religion" || communities.some((c) => c.id === other && c.id !== community));

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!canSubmit) return;
    switch (kind) {
      case "split":
        onAction({ kind: "split", community, intensity, ...(naming ? { naming } : {}) });
        return;
      case "connect":
        onAction({ kind: "connect", a: community, b: other, intensity, contact });
        return;
      case "shift":
        onAction({ kind: "shift", community, toward: other });
        return;
      case "state":
        onAction({ kind: "state", community, capital });
        return;
      case "religion":
        onAction({ kind: "religion", community });
        return;
      case "craft":
        onAction({ kind: "craft", community, craft });
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
      case "religion":
        return communities.length === 0 ? (
          <p className="muted">No living people can found a religion.</p>
        ) : (
          <>
            {pick("Founder's people", community, setCommunity)}
            <p className="muted">A founder teaches a new faith. Its sacred language keeps the words and sounds used now.</p>
          </>
        );
      case "craft":
        return communities.length === 0 ? (
          <p className="muted">No living people lacks a craft.</p>
        ) : (
          <>
            {pick("People", community, (id) => {
              setCommunity(id);
              setCraft(catalog.crafts.find((c) => !overview.communities[id].crafts.includes(c.id as Craft))!.id as Craft);
            })}
            <label>
              Craft
              <select value={craft} onChange={(e) => setCraft(e.target.value as Craft)}>
                {catalog.crafts.filter((c) => !people?.crafts.includes(c.id as Craft)).map((c) => (
                  <option key={c.id} value={c.id}>{c.name}</option>
                ))}
              </select>
              <small>{catalog.crafts.find((c) => c.id === craft)?.description}</small>
            </label>
            <p className="muted">They come upon this craft and find words for it.</p>
          </>
        );
      case "state":
        return (
          <>
            {communities.length === 0 ? (
              <p className="muted">No living people is outside a standing state. A ruler or subject cannot found another state.</p>
            ) : (
              <>
                {pick("Ruling people", community, (id) => {
                  setCommunity(id);
                  setCapital(overview.communities[id].region);
                })}
                <label>
                  Capital
                  <select value={capital} onChange={(e) => setCapital(Number(e.target.value))}>
                    {people?.lands.map((region) => (
                      <option key={region} value={region}>
                        {overview.places.find((p) => p.region === region)?.names.at(-1)?.spelled ?? `Land ${region + 1}`}
                        {region === people.region ? " (heart land)" : ""}
                      </option>
                    ))}
                  </select>
                </label>
                <p className="muted">They organize a realm around this capital. Tribute will feed its city, and in time its court’s speech may become a standard language.</p>
              </>
            )}
          </>
        );
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
          <button
            type="submit"
            form={`form-${kind}`}
            className="primary"
            disabled={!canSubmit}
          >
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
