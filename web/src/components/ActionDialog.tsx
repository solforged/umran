import { useMemo, useState, type FormEvent, type ReactNode } from "react";
import type { Action, Catalog, ContactKind, Craft, EthosAxis, Overview, ReadEngine } from "../model";
import { YEARS } from "../model";
import { ETHOS_AXES, ETHOS_POLES, temperament } from "../lore";
import { Modal } from "./Modal";
import { Specimen } from "./Specimen";

/// "found" opens the language designer; the rest are here.
export type DialogKind = "found" | "connect" | "shift" | "state" | "religion" | "craft" | "temper" | "law";
export type InterventionKind = DialogKind | "settlement";

const TITLES: Record<Exclude<DialogKind, "found">, string> = {
  connect: "Two peoples meet",
  shift: "A people takes up another language",
  state: "Found a state",
  religion: "Found a religion",
  craft: "Teach a craft",
  temper: "A people's temper turns",
  law: "A sound change",
};

/// Forms for shaping a people's history. Each validates the basics;
/// the engine has the last word and reports anything it rejects.
export function ActionDialog({
  open,
  kind,
  catalog,
  overview,
  engine,
  selected,
  onClose,
  onAction,
}: {
  open: boolean;
  kind: Exclude<DialogKind, "found">;
  catalog: Catalog;
  overview: Overview;
  engine: ReadEngine;
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
  const [community, setCommunity] = useState(first);
  const [other, setOther] = useState(others[0]?.id ?? -1);
  const [contact, setContact] = useState<ContactKind>("neighbours");
  const [intensity, setIntensity] = useState(0.6);
  const [capital, setCapital] = useState(overview.communities[first]?.region ?? -1);
  const [craft, setCraft] = useState<Craft>((catalog.crafts.find((c) =>
    !overview.communities[first]?.crafts.includes(c.id as Craft))?.id ?? "metalworking") as Craft);
  const people = communities.find((c) => c.id === community);
  const [axis, setAxis] = useState<EthosAxis>("martial");
  const [toward, setToward] = useState<1 | -1>(1);
  const [amount, setAmount] = useState(0.3);
  const languages = overview.varieties.filter((v) => communities.some((c) => c.variety === v.id));
  const [variety, setVariety] = useState(overview.communities[first]?.variety ?? languages[0]?.id ?? -1);
  const [law, setLaw] = useState<string | null>(null);
  const lawChoices = useMemo(() => open && kind === "law" && variety >= 0
    ? engine.lawChoices(overview.point, variety) : [], [engine, open, kind, overview.point, variety]);
  const canSubmit = kind === "law" ? lawChoices.some((choice) => choice.id === law) :
    !!people && (kind === "state" ? people.lands.includes(capital) :
      kind === "craft" ? !people.crafts.includes(craft) :
      kind === "temper" ? amount > 0 :
      kind === "religion" || communities.some((c) => c.id === other && c.id !== community));

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!canSubmit) return;
    switch (kind) {
      case "law":
        onAction({ kind: "law", variety, law: law! });
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
        return;
      case "temper":
        onAction({ kind: "temper", community, axis, amount: toward * amount });
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
      case "law":
        return <>
          <label>
            Language
            <select value={variety} onChange={(e) => { setVariety(Number(e.target.value)); setLaw(null); }}>
              {languages.map((v) => <option key={v.id} value={v.id}>{v.name}</option>)}
            </select>
          </label>
          {lawChoices.length ? <fieldset className="law-choices">
            <legend>Sound change</legend>
            {lawChoices.map((choice) => <div className="law-choice" key={choice.id}>
              <input type="radio" name="law" id={`law-choice-${choice.id}`} checked={law === choice.id}
                onChange={() => setLaw(choice.id)} />
              <div>
                <label htmlFor={`law-choice-${choice.id}`}><strong>{choice.label}</strong><span className="muted small">{choice.words} {choice.words === 1 ? "word" : "words"}</span></label>
                <Specimen words={choice.specimen} changes />
                {choice.recent ? <small>happened here lately</small> : null}
              </div>
            </div>)}
          </fieldset> : <p className="muted">Nothing in the catalog would change a word of this language now.</p>}
        </>;
      case "temper":
        return communities.length === 0 ? (
          <p className="muted">No living people is left to temper.</p>
        ) : (
          <>
            {pick("People", community, setCommunity)}
            <label>
              They grow more
              <select value={`${axis} ${toward}`} onChange={(e) => {
                const [a, t] = e.target.value.split(" ");
                setAxis(a as EthosAxis);
                setToward(Number(t) as 1 | -1);
              }}>
                {ETHOS_AXES.flatMap((a) => [1, -1].map((t) => (
                  <option key={`${a} ${t}`} value={`${a} ${t}`}>{ETHOS_POLES[a][t > 0 ? 1 : 0]}</option>
                )))}
              </select>
              <small>
                Now {people ? temperament(people.ethos).join(", ") || "even-tempered" : ""}.
              </small>
            </label>
            {slider("How far", amount, setAmount, "How far along the line toward that end; 1 is half its length.")}
            <p className="muted">
              Fate's hand, not a law: their temper still drifts, follows their neighbours', and turns again with what befalls them.
            </p>
          </>
        );
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
      case "connect":
        return communities.length < 2 ? (
          <p className="muted">There is no other living people to meet. Let a people part, or wait for one to.</p>
        ) : (
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
        return communities.length < 2 ? (
          <p className="muted">There is no other living people whose language they could take up.</p>
        ) : (
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
      open={open}
      wide={kind === "law"}
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
            {kind === "law" ? "Let it happen" : "Make this decision"}
          </button>
        </>
      }
    >
      <p className="telling-note">A decision in year {overview.generation * YEARS}.
        {!overview.atTip ? <> This begins another telling. The years through {overview.latest * YEARS} stay in the chronicle.</> : null}
      </p>
      <form id={`form-${kind}`} className="form" onSubmit={submit}>
        {body}
      </form>
    </Modal>
  );
}
