import { useEffect, useMemo, useState } from "react";
import type { Action, AuthoredLaw, LawPreview, LawRequest, Meaning, MeaningClass, MeaningField, MeaningRelation, MeaningTier, Overview, ReadEngine, Variety } from "../model";
import { YEARS } from "../model";
import { attempt, checkLaw } from "../engine";
import { MEANING_CLASS, MEANING_RELATION, MEANING_TIER, NAME_KIND, RULE_TROUBLE, STRESS_RULE } from "../lore";
import { Specimen } from "./Specimen";
import { useAnswer } from "./Pending";

/// The author's decisions that write language rather than history: a sound
/// law, a meaning, a word. Each form stages its action for the dialog's
/// footer and clears it while the engine refuses.

const NONE: never[] = [];
/// Meaning sets that are not fields of the catalog.
const NOT_FIELDS = ["core", "specimen", "custom"];

/// A value once it has stopped changing for `ms`, so a full-lexicon preview
/// waits for the typing to settle.
function useSettled<T>(value: T, ms: number): T {
  const [settled, setSettled] = useState(value);
  useEffect(() => {
    const timer = window.setTimeout(() => setSettled(value), ms);
    return () => window.clearTimeout(timer);
  }, [value, ms]);
  return settled;
}

function LanguagePick({ languages, variety, onChange }: { languages: Variety[]; variety: number; onChange: (id: number) => void }) {
  return <label>
    Language
    <select value={variety} onChange={(e) => onChange(Number(e.target.value))}>
      {languages.map((v) => <option key={v.id} value={v.id}>{v.name}</option>)}
    </select>
  </label>;
}

/// Every word and name the law would change, through the engine's own path.
function LawPreviewTable({ preview }: { preview: LawPreview }) {
  const [all, setAll] = useState(false);
  const shown = all ? preview.words : preview.words.slice(0, 24);
  const changed = preview.words.length + preview.names.length;
  return <div className="law-preview">
    <p className={changed ? "muted small" : "notice"}>
      {changed ? <>{preview.words.length} {preview.words.length === 1 ? "word" : "words"} and {preview.names.length} {preview.names.length === 1 ? "name" : "names"} change; {preview.unchangedWords} words and {preview.unchangedNames} names stay as they are.</>
        : "This law would change nothing that is said now."}
      {preview.stress ? <> Stress moves from {STRESS_RULE[preview.stress.before]} to {STRESS_RULE[preview.stress.after]}.</> : null}
    </p>
    {preview.rules.some((rule) => rule.unrealisable) ? <p className="notice">
      {preview.rules.filter((rule) => rule.unrealisable).map((rule) => <span key={rule.source}><code>{rule.source}</code> matched {rule.unrealisable} {rule.unrealisable === 1 ? "sound" : "sounds"} it cannot turn into any catalog sound; those stay. </span>)}
    </p> : null}
    {preview.rules.length > 1 ? <ul className="law-preview-rules">{preview.rules.map((rule) => <li key={rule.source}><code>{rule.source}</code> <span className="muted small">{rule.changed} {rule.changed === 1 ? "change" : "changes"}</span></li>)}</ul> : null}
    {preview.words.length ? <div className="table-wrap"><table className="law-words">
      <thead><tr><th>Meaning</th><th>Before → after</th></tr></thead>
      <tbody>{shown.map((word) => <tr key={word.concept}>
        <td>{word.gloss}</td>
        <td><span className="word">{word.spelledBefore}</span> <span className="ipa">/{word.before}/</span> → <span className="word">{word.spelledAfter}</span> <span className="ipa">/{word.after}/</span></td>
      </tr>)}</tbody>
    </table></div> : null}
    {preview.words.length > shown.length ? <button type="button" className="link" onClick={() => setAll(true)}>All {preview.words.length} words</button> : null}
    {preview.names.length ? <ul className="law-preview-names">{preview.names.map((name, i) => <li key={i}>
      <span className="muted small">{NAME_KIND[name.kind]}</span> <span className="word">{name.before}</span> → <span className="word">{name.after}</span>
    </li>)}</ul> : null}
  </div>;
}

function LawPreviewRead({ engine, overview, variety, law }: { engine: ReadEngine; overview: Overview; variety: number; law: LawRequest }) {
  const outcome = useAnswer(attempt(engine.lawPreview(overview.point, variety, law)), true);
  if (!outcome) return <p className="pending" role="status">Applying the law to every word…</p>;
  if (outcome.error !== null) return <p className="notice error" role="alert">{outcome.error}</p>;
  return <LawPreviewTable preview={outcome.value} />;
}

export function LawDecision({ open, overview, engine, languages, onStage }: {
  open: boolean; overview: Overview; engine: ReadEngine; languages: Variety[];
  onStage: (action: Action | null) => void;
}) {
  const [variety, setVariety] = useState(languages[0]?.id ?? -1);
  const [mode, setMode] = useState<"catalog" | "written">("catalog");
  const [catalog, setCatalog] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [source, setSource] = useState("");
  const choices = useAnswer(open && mode === "catalog" && variety >= 0 ? engine.lawChoices(overview.point, variety) : null) ?? NONE;
  const report = useMemo(() => title.trim() || source.trim() ? checkLaw(title, source) : null, [title, source]);
  // The full preview applies the law to every word; wait for the typing to settle.
  const settled = useSettled(report?.law ?? null, 350);
  const written: LawRequest | null = settled ? { authored: settled } : null;
  const outcome = useAnswer(open && mode === "written" && variety >= 0 && written ? attempt(engine.lawPreview(overview.point, variety, written)) : null, true);
  const writtenChanges = outcome && outcome.error === null && outcome.value.words.length + outcome.value.names.length > 0;
  useEffect(() => {
    if (mode === "catalog") onStage(catalog && choices.some((choice) => choice.id === catalog) ? { kind: "law", variety, law: catalog } : null);
    else onStage(writtenChanges && settled ? { kind: "authored", variety, law: settled } : null);
  }, [mode, catalog, choices, variety, settled, writtenChanges, onStage]);
  const trouble = report?.rules.filter((rule) => rule.status !== "ok" && rule.status !== "empty") ?? NONE;
  return <>
    <LanguagePick languages={languages} variety={variety} onChange={(id) => { setVariety(id); setCatalog(null); }} />
    <div className="segmented" role="tablist" aria-label="Where the law comes from">
      <button type="button" role="tab" aria-selected={mode === "catalog"} onClick={() => setMode("catalog")}>From the catalog</button>
      <button type="button" role="tab" aria-selected={mode === "written"} onClick={() => setMode("written")}>In your own hand</button>
    </div>
    {mode === "catalog" ? <>
      {choices.length ? <fieldset className="law-choices">
        <legend>Sound change</legend>
        {choices.map((choice) => <div className="law-choice" key={choice.id}>
          <input type="radio" name="law" id={`law-choice-${choice.id}`} checked={catalog === choice.id}
            onChange={() => setCatalog(choice.id)} />
          <div>
            <label htmlFor={`law-choice-${choice.id}`}><strong>{choice.label}</strong><span className="muted small">{choice.words} {choice.words === 1 ? "word" : "words"}{choice.names ? `, ${choice.names} ${choice.names === 1 ? "name" : "names"}` : ""}</span></label>
            {catalog === choice.id ? <LawPreviewRead engine={engine} overview={overview} variety={variety} law={{ catalog: choice.id }} /> : <Specimen words={choice.specimen} changes />}
            {choice.recent ? <small>happened here lately</small> : null}
          </div>
        </div>)}
      </fieldset> : <p className="muted">Nothing in the catalog would change a word of this language now.</p>}
    </> : <>
      <label>
        Name of the law
        <input type="text" value={title} maxLength={60} placeholder="The great lenition" onChange={(e) => setTitle(e.target.value)} />
      </label>
      <label>
        Rules, one to a line
        <textarea className="law-source" value={source} rows={Math.min(10, Math.max(3, source.split("\n").length + 1))} spellCheck={false} autoCapitalize="off" autoCorrect="off"
          placeholder={"p > f / _ #\n[voiceless stop] > [voiced] / V _ V\na > e / _ .. [front]"} onChange={(e) => setSource(e.target.value)} />
        <small>Sounds are the catalog’s IPA. <code>C</code> and <code>V</code> are any consonant or vowel; <code>[voiceless stop]</code>, <code>[front]</code>, <code>[long]</code>, <code>[stressed]</code> name classes; <code>#</code> is the edge of the word; <code>∅</code> deletes; <code>_ .. [front]</code> looks to the next syllable’s vowel; <code>stress &gt; initial</code> moves the stress. One sound on each side of the blank.</small>
      </label>
      {report ? <ul className="law-rules-check">{report.rules.filter((rule) => rule.status !== "empty").map((rule) => <li key={rule.line} className={rule.status === "ok" ? "ok" : "trouble"}>
        <code>{rule.source}</code>
        <span className="small">{rule.status === "ok" ? rule.reading : <>{RULE_TROUBLE[rule.status]}{rule.message ? `: ${rule.message}` : ""}</>}</span>
      </li>)}</ul> : null}
      {report && !trouble.length && report.law && written ? <LawPreviewRead engine={engine} overview={overview} variety={variety} law={written} /> : null}
    </>}
  </>;
}

function slug(gloss: string): string {
  return gloss.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/^to /, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 32);
}

export function MeaningDecision({ open, overview, engine, onStage }: {
  open: boolean; overview: Overview; engine: ReadEngine; onStage: (action: Action | null) => void;
}) {
  const [gloss, setGloss] = useState("");
  const [id, setId] = useState("");
  const [idTouched, setIdTouched] = useState(false);
  const [field, setField] = useState<MeaningField>("house");
  const [shape, setShape] = useState<"count" | "mass" | "event" | "property">("count");
  const [tier, setTier] = useState<MeaningTier>("everyday");
  const [base, setBase] = useState("");
  const [relation, setRelation] = useState<MeaningRelation>("place");
  const sets = useAnswer(open ? engine.meaningSets() : null) ?? NONE;
  const meanings = useAnswer(open ? engine.meanings(overview.generation) : null) ?? NONE;
  const name = idTouched ? id : slug(gloss);
  const meaning: Meaning = useMemo(() => ({
    id: name, gloss: gloss.trim(), field,
    class: (shape === "event" ? "event" : shape === "property" ? "property" : "entity") as MeaningClass,
    countable: shape === "count", tier,
    from: base ? { base, relation } : null,
  }), [name, gloss, field, shape, tier, base, relation]);
  const check = useAnswer(open && meaning.gloss && meaning.id ? attempt(engine.checkMeaning(overview.point, meaning)) : null, true);
  const ok = check?.error === null && check.value.status === "ok";
  useEffect(() => { onStage(ok ? { kind: "meaning", meaning } : null); }, [ok, meaning, onStage]);
  const baseMeaning = meanings.find((concept) => concept.id === base);
  return <>
    <label>
      Meaning
      <input type="text" value={gloss} maxLength={40} placeholder="hearth" onChange={(e) => setGloss(e.target.value)} />
      <small>In English, as the catalog glosses its meanings: <i>hearth</i>, <i>to forge</i>, <i>sacred</i>.</small>
    </label>
    <label>
      Short name
      <input type="text" value={name} maxLength={32} spellCheck={false} onChange={(e) => { setIdTouched(true); setId(e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, "-")); }} />
      <small>How tables, roots, and exports refer to it; letters, digits, and hyphens.</small>
    </label>
    <div className="form-row">
      <label>
        Kind
        <select value={shape} onChange={(e) => setShape(e.target.value as typeof shape)}>
          {(["count", "mass", "event", "property"] as const).map((kind) => <option key={kind} value={kind}>{MEANING_CLASS[kind]}</option>)}
        </select>
      </label>
      <label>
        Field
        <select value={field} onChange={(e) => setField(e.target.value as MeaningField)}>
          {sets.filter((set) => !NOT_FIELDS.includes(set.id)).map((set) => <option key={set.id} value={set.id}>{set.label}</option>)}
        </select>
      </label>
      <label>
        How bound to a way of life
        <select value={tier} onChange={(e) => setTier(e.target.value as MeaningTier)}>
          {(["basic", "everyday", "specialized"] as const).map((t) => <option key={t} value={t}>{MEANING_TIER[t].name}</option>)}
        </select>
        <small>{MEANING_TIER[tier].drives}</small>
      </label>
    </div>
    <div className="form-row">
      <label>
        Built from
        <input type="text" list="meaning-bases" value={base} spellCheck={false} placeholder="none; a fresh root" onChange={(e) => setBase(e.target.value.trim())} />
        <datalist id="meaning-bases">{meanings.filter((concept) => !concept.pronoun).map((concept) => <option key={concept.id} value={concept.id}>{concept.gloss}</option>)}</datalist>
        <small>{baseMeaning ? <>Languages that build words will derive it from their word for “{baseMeaning.gloss}”.</> : "Another meaning’s short name, where a language should derive this one from it."}</small>
      </label>
      {base ? <label>
        As its
        <select value={relation} onChange={(e) => setRelation(e.target.value as MeaningRelation)}>
          {(Object.keys(MEANING_RELATION) as MeaningRelation[]).map((r) => <option key={r} value={r}>{MEANING_RELATION[r]}</option>)}
        </select>
      </label> : null}
    </div>
    {check ? <p className={ok ? "muted small" : "notice"} role="status">
      {check.error !== null ? check.error : ok ? `Every living language will find a word for “${meaning.gloss}” in the years after ${overview.generation * YEARS}${overview.generation === 0 ? ", or carry one from its founding" : ""}.` : <>{check.value.message}{check.value.suggestions.length ? <> Perhaps you mean {check.value.suggestions.map((s, i) => <span key={s}>{i ? ", " : ""}<code>{s}</code></span>)}.</> : null}</>}
    </p> : null}
  </>;
}

export function WordDecision({ open, overview, engine, languages, onStage }: {
  open: boolean; overview: Overview; engine: ReadEngine; languages: Variety[]; onStage: (action: Action | null) => void;
}) {
  const [variety, setVariety] = useState(languages[0]?.id ?? -1);
  const [concept, setConcept] = useState("");
  const [ipa, setIpa] = useState("");
  const meanings = useAnswer(open ? engine.meanings(overview.generation) : null) ?? NONE;
  const chosen = meanings.find((m) => m.id === concept);
  const check = useAnswer(open && variety >= 0 && concept && ipa.trim() ? attempt(engine.checkWord(overview.point, variety, concept, ipa.trim())) : null, true);
  const ok = check?.error === null && check.value.status === "ok";
  useEffect(() => { onStage(ok ? { kind: "word", variety, concept, ipa: ipa.trim() } : null); }, [ok, variety, concept, ipa, onStage]);
  const current = check?.error === null ? check.value.current : null;
  return <>
    <LanguagePick languages={languages} variety={variety} onChange={setVariety} />
    <label>
      Meaning
      <input type="text" list="word-meanings" value={concept} spellCheck={false} placeholder="water" onChange={(e) => setConcept(e.target.value.trim())} />
      <datalist id="word-meanings">{meanings.filter((m) => !m.pronoun).map((m) => <option key={m.id} value={m.id}>{m.gloss}</option>)}</datalist>
      <small>{chosen ? current ? <>They say <span className="word">{current.spelled}</span> <span className="ipa">/{current.form}/</span> for “{chosen.gloss}” now; yours takes its place and the old word may linger beside it.</> : <>“{chosen.gloss}”: they have no word for it yet.</> : "A meaning’s short name from the catalog or one you added."}</small>
    </label>
    <label>
      Their word
      <input type="text" value={ipa} spellCheck={false} autoCapitalize="off" autoCorrect="off" placeholder="kaːr" onChange={(e) => setIpa(e.target.value)} />
      <small>In the catalog’s IPA, using sounds this language has. Sound change wears it like any word from here on.</small>
    </label>
    {check ? <p className={ok ? "muted small" : "notice"} role="status">
      {check.error !== null ? check.error : ok ? <>Spelled <span className="word">{check.value.spelled}</span> in their hand.</>
        : <>{WORD_TROUBLE[check.value.status]}{check.value.detail ? ` (${check.value.detail})` : ""}{check.value.suggestions.length ? <>; try {check.value.suggestions.map((s, i) => <span key={s}>{i ? ", " : ""}<code>{s}</code></span>)}</> : null}.</>}
    </p> : null}
  </>;
}

const WORD_TROUBLE = {
  ok: "",
  "unknown-meaning": "No meaning by that name",
  "bad-ipa": "The form cannot be read as catalog IPA",
  "no-vowel": "The form has no vowel",
  "outside-inventory": "The form uses a sound this language does not have",
  pronoun: "Pronouns are not given this way",
  dead: "No one speaks this language now",
} as const;
