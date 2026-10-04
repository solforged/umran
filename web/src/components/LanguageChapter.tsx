import { useMemo, useState, type ReactNode } from "react";
import { Feather } from "lucide-react";
import type { Annal, HistoryLine, Law, LexiconRow, LoanCause, Overview, ReadEngine, Subject, Variant, Variety, WorldMap } from "../model";
import { YEARS } from "../model";
import { CONTACT_NAME, MARKING_PHRASE, POSSESSOR_PHRASE, STRESS_RULE, WORD_ORDER_PHRASE } from "../lore";
import { individualAnnals, findAnnal } from "../history";
import { Dictionary, INITIAL_DICTIONARY, type DictionaryView } from "./Dictionary";
import { FamilyTree } from "./FamilyTree";
import { Specimen } from "./Specimen";
import { Sample } from "./Sample";
import { WordGloss } from "./WordGloss";
import { Explained } from "./Explained";
import "./chapter.css";

/** Read-only evidence and navigation shared by a card and the book. */
export interface ChapterContext {
  engine: ReadEngine;
  version: number;
  generation: number;
  overview: Overview;
  map: WorldMap;
  link: (subject: Subject, label: ReactNode) => ReactNode;
  open: (subject: Subject) => void;
  dictionaryView?: DictionaryView;
  onDictionaryView?: (view: DictionaryView) => void;
  onYear?: (generation: number) => void;
  wordAnchor?: (variety: number, concept: string) => string;
}
interface SectionProps { variety: Variety; ctx: ChapterContext }
const PERSON = { 1: "first", 2: "second", 3: "third" } as const;
const CLASS_BASIS = { sex: "by sex", animacy: "by animacy", shape: "by shape", formal: "by ending" } as const;
const HARMONY = { backness: "front with front and back with back", rounding: "rounded with rounded", atr: "tense with tense and lax with lax" } as const;

export interface ParadigmView {
  category: string;
  label: string;
  realizations: { marker: number; kind: string; side: string; form: string; spelled: string; said: string | null; ipa: string; share: number; retired: number | null; history: HistoryLine[] }[];
}

const language = (id: number, ctx: ChapterContext) => ctx.link({ kind: "language", variety: id }, ctx.overview.varieties[id]?.name ?? "An unrecorded language");
const people = (id: number, ctx: ChapterContext) => ctx.link({ kind: "people", id }, ctx.overview.communities[id]?.name ?? "An unrecorded people");
const year = (at: number) => `year ${at * YEARS}`;

export function Position({ variety: v, ctx }: SectionProps) {
  const daughters = ctx.overview.varieties.filter((d) => d.parent === v.id);
  const kin = useMemo(() => ctx.engine.kin(ctx.generation, v.id).filter((k) => k.score >= 0.05), [ctx.engine, ctx.generation, ctx.version, v.id]);
  return <div className="language-section">
    <p>{v.parent === null ? <>A founding language of its family, first spoken in {year(v.born)}.</> : <>Descended from {language(v.parent, ctx)}, parted in {year(v.forkedAt ?? v.born)}.</>} {v.spoken ? "Spoken now." : `Silent${v.silentSince === null ? " now" : ` since ${year(v.silentSince)}`}.`}</p>
    <p>Family: {language(v.family, ctx)}. Daughters: {daughters.length ? daughters.map((d, i) => <span key={d.id}>{i ? ", " : ""}{language(d.id, ctx)}</span>) : "none recorded"}.</p>
    <FamilyTree overview={ctx.overview} family={v.family} chosen={v.id} onOpen={(id) => ctx.open({ kind: "language", variety: id })} />
    <h4>Shared core words</h4>
    {kin.length ? <ul className="roster">{kin.map((k) => <li key={k.other} className="meter-row">{language(k.other, ctx)} <meter min={0} max={1} value={k.score} /> <span>{Math.round(k.score * 100)}%</span></li>)}</ul> : <p className="muted">No other language shares at least 5% of its core words.</p>}
  </div>;
}

export function Sounds({ variety: v }: SectionProps) {
  return <div className="language-section">
    <h4>Consonants · {v.consonants.length}</h4><p className="segments">{v.consonants.join(" ")}</p>
    <h4>Vowels · {v.vowels.length}</h4><p className="segments">{v.vowels.join(" ")}</p>
    {v.harmony ? <p>{v.harmony.lost === null ? <>Vowel harmony: within a word, vowels agree {HARMONY[v.harmony.feature]}, since {year(v.harmony.since)}.</> : <>Vowel harmony, {HARMONY[v.harmony.feature]}, from {year(v.harmony.since)} until {year(v.harmony.lost)}.</>}</p> : null}
    {v.tonal ? <p>{v.tonal.lost === null ? <>Tone: {v.tones} pitch {v.tones === 1 ? "pattern tells" : "patterns tell"} words apart, since {year(v.tonal.since)}. Tones are written with Chao tone letters after the vowel.</> : <>Tonal from {year(v.tonal.since)} until {year(v.tonal.lost)}.</>}</p> : null}
    <dl className="chapter-facts"><div><dt><Explained term="minimal word">Minimal word</Explained></dt><dd>{v.minimalWord.includes("heavy") ? <Explained term="heavy syllable">{v.minimalWord}</Explained> : v.minimalWord}; sound change does not wear a word below it.</dd></div><div><dt><Explained term="stress">Stress</Explained></dt><dd>{v.stress === "weight" ? <>on a <Explained term="heavy syllable">heavy</Explained> next-to-last syllable, otherwise the syllable before it; on the first syllable in a two-syllable word</> : STRESS_RULE[v.stress]}.</dd></div><div><dt><Explained term="geminate">Geminates</Explained></dt><dd>{v.geminates ? "Long consonants occur in living words." : "No long consonants in living words."}</dd></div></dl>
    <p className="muted small"><Explained term="IPA">IPA</Explained>: International Phonetic Alphabet, symbols for speech sounds.</p>
  </div>;
}

function lawAnnal(v: Variety, law: Law, ctx: ChapterContext): Annal | undefined {
  const ancestors = new Set<number>();
  let ancestor: Variety | undefined = v;
  while (ancestor) { ancestors.add(ancestor.id); ancestor = ancestor.parent === null ? undefined : ctx.overview.varieties[ancestor.parent]; }
  return individualAnnals(ctx.overview.annals).find((a) => a.generation === law.generation && a.laws.includes(law.id) && a.variety !== null && ancestors.has(a.variety));
}

/** Exact forms in a word's recorded ledger, not a browser application of a law. */
export function LawWords({ variety, law, ctx }: SectionProps & { law: Law }) {
  const changed = useMemo(() => {
    const examples: { concept: string; gloss: string; text: string }[] = [];
    for (const row of ctx.engine.lexicon(ctx.generation, variety.id)) {
      const detail = ctx.engine.word(ctx.generation, variety.id, row.concept);
      const seen = new Set<string>();
      for (const variant of detail.variants) for (const line of variant.history) {
        if (line.generation !== law.generation || !line.text.startsWith(`${law.label}: /`) || seen.has(line.text)) continue;
        seen.add(line.text);
        examples.push({ concept: row.concept, gloss: row.gloss, text: line.text.slice(law.label.length + 2) });
      }
    }
    return examples;
  }, [ctx.engine, ctx.generation, ctx.version, variety.id, law.id, law.label, law.generation]);
  return changed.length ? <table className="law-words"><thead><tr><th>Meaning</th><th>Before → after</th></tr></thead><tbody>{changed.map((w, i) => <tr key={i}><td>{ctx.link({ kind: "word", variety: variety.id, concept: w.concept }, w.gloss)}</td><td className="ipa">{w.text}</td></tr>)}</tbody></table> : <p className="muted small">No surviving lexical variant records this law. It may have reached names, grammatical forms, or words that later fell out of use.</p>;
}

export function LawEvidence({ variety, law, ctx }: SectionProps & { law: Law }) {
  const annal = lawAnnal(variety, law, ctx);
  const [expanded, setExpanded] = useState(false);
  const changed = annal?.specimen.filter((w) => w.was !== null || w.wasIpa !== null) ?? [];
  return <div className="law-evidence">
    {law.from !== null ? <p className="muted small">A wave from {language(law.from, ctx)}.</p> : null}
    {changed.length ? <><p className="muted small">{annal!.laws.length > 1 ? "Specimen across this year’s recorded laws" : "Before and after in the specimen"}</p><Specimen words={changed} changes onWord={(concept) => ctx.open({ kind: "word", variety: variety.id, concept })} /></> : <p className="muted small">{annal ? "This law did not change a recorded specimen word." : "No before-and-after specimen is recorded for this law here."}</p>}
    <details open={expanded} onToggle={(e) => setExpanded(e.currentTarget.open)}><summary>Words changed · before and after</summary>{expanded ? <LawWords variety={variety} law={law} ctx={ctx} /> : null}</details>
  </div>;
}

export function SoundLaws({ variety, ctx }: SectionProps) {
  return <div className="language-section">
    <p className="muted small">A dated ledger of regular changes, from earliest to latest. Forms are the engine’s recorded evidence, not reconstructed here.</p>
    {variety.laws.length ? <ol className="law-ledger">{variety.laws.map((law, i) => <li key={`${law.id}:${law.generation}:${i}`} data-law={law.id}><div className="law-ledger-head"><span className="gen">{law.generation * YEARS}</span> {law.decision != null ? <span className="pen" title="The author's decision"><Feather size={12} aria-hidden="true" /></span> : null}{ctx.link({ kind: "law", id: law.id }, law.label)}</div><LawEvidence variety={variety} law={law} ctx={ctx} /></li>)}</ol> : <p className="muted">No sound laws yet.</p>}
  </div>;
}

export function WordBuilding({ variety: v }: SectionProps) {
  return <div className="language-section">
    <p>Words are built with <Explained term={v.wordBuilding.toLowerCase().includes("pattern") ? "root and pattern" : "affix"}>{v.wordBuilding}</Explained>.</p>
    <dl className="builders">{v.builders.map((b) => <div key={b.relation}><dt>{b.relation}</dt><dd className="ipa">{b.shape}</dd></div>)}</dl>
  </div>;
}

export function GrammarSketch({ variety: v, ctx }: SectionProps) {
  const grammar = v.grammar;
  const object = grammar.markers.find((marker) => marker.category === "object" && marker.productive && marker.retired === null && marker.kind !== "none" && marker.ipa.length > 0);
  const objectMarker = object ? object.kind === "particle"
    ? <>the separate word <span className="word">{object.spelled}</span> {object.side === "prefix" ? "before" : "after"} it</>
    : object.kind === "bound"
      ? <>{object.side === "prefix" ? "the prefix" : "the suffix"} <span className="word">{object.side === "prefix" ? `${object.spelled}-` : `-${object.spelled}`}</span></>
      : <>the vowel pattern <span className="word">{object.spelled}</span></> : null;
  return <div className="language-section grammar-sketch">
    <p>{WORD_ORDER_PHRASE[grammar.order].prose}; {MARKING_PHRASE[grammar.marking].prose}
      {objectMarker ? grammar.marking === "case" ? <> with {objectMarker}</> : <>, though {objectMarker} still marks the object in some uses</> : null}; {POSSESSOR_PHRASE[grammar.possessor].prose}.</p>
    {grammar.sample ? <>
      <Sample rendering={grammar.sample.sentence} label="Sample sentence" />
      <Sample rendering={grammar.sample.possession} label="Sample possession" />
      <p className="muted small">“Child — fish” and “fish — child” both mean “the child’s fish”; possessor placement sets the order, while genitive marking is shown separately.</p>
      <Sample rendering={grammar.sample.future} label="Sample future" />
    </> : null}
    <p className="muted small">Grammatical forms can change through <Explained term="grammaticalization">grammaticalization</Explained>, <Explained term="fusion">fusion</Explained>, and <Explained term="analogy">analogy</Explained>.</p>
    {grammar.categories.map((category) => <section key={category.category} className="grammar-category">
      <h5>{category.label}</h5>
      <p>{category.description} {category.eligible} words can take this category. In {Math.round(category.howSynthetic * 100)}% of uses, it is distinguished within one word; in {Math.round(category.contrastRetention * 100)}% of uses, it remains <Explained term="grammatical contrast">audibly distinct</Explained>, including separate grammatical words.</p>
      <ul className="roster">{grammar.markers.filter((m) => m.category === category.category).map((m) => <li key={m.id}>
        <span className="word">{m.spelled || "∅"}</span>{m.said !== null ? <> · said <span className="word">{m.said}</span></> : null} <span className="ipa">/{m.ipa}/</span>
        {m.alternants?.length ? <> · by vowel harmony also {m.alternants.map((a) => a.spelled).join(", ")}</> : null} · {m.kind === "pattern" ? <>vowel pattern <span className="ipa">{m.template}</span></> : m.kind === "none" ? "no overt marker" : m.kind === "particle" ? <Explained term="particle">separate word {m.side === "prefix" ? "before" : "after"} the stem</Explained> : <Explained term="affix">{m.side === "prefix" ? "prefix" : "suffix"}</Explained>}, {Math.round(m.share * 100)}% of uses; <Explained term="productive">{m.productive ? "used to form new words in this category" : "not used to form new words in this category"}</Explained>
        {m.retired !== null ? `; retired in ${year(m.retired)}` : `; since ${year(m.born)}`}.
        {m.origin.kind === "grammaticalized" ? <> Through <Explained term="grammaticalization">grammaticalization</Explained>, from the word “{m.origin.gloss}”.</> : m.origin.kind === "imported" ? <> Imported from {language(m.origin.from, ctx)}.</> : m.origin.kind === "fused" ? <> Through <Explained term="fusion">fusion</Explained> from a <Explained term="particle">particle</Explained>.</> : " Founding marker."}
        <details><summary>Recorded changes</summary><ol className="history">{m.history.map((line, i) => <li key={i}><span className="gen">{line.generation * YEARS}</span><span>{line.text}</span></li>)}</ol></details>
      </li>)}</ul>
    </section>)}
    {v.pronouns.length ? <section className="grammar-category"><h5>Pronouns</h5><ul className="roster">{v.pronouns.map((p) => <li key={p.cell}><span className="word">{p.spelled}</span> <span className="ipa">/{p.ipa}/</span> · {PERSON[p.person]} person {p.number === "sg" ? "singular" : "plural"}{p.polite ? `, respectful address since ${year(p.since)}` : p.origin === "renewed" ? `; renewed in ${year(p.since)}` : p.origin === "borrowed" ? `; borrowed in ${year(p.since)}` : ""}.</li>)}</ul></section> : null}
    {grammar.classes.some((c) => c.mergedInto === null && c.members > 0) ? <section className="grammar-category"><h5>Noun classes</h5><p>Every noun belongs to a class, and the word for “this” agrees with it.</p><ul className="roster">{grammar.classes.filter((c) => c.mergedInto === null && c.members > 0).map((c) => <li key={c.id}>{c.marker ? <><span className="word">{c.marker.spelled}</span> <span className="ipa">/{c.marker.ipa}/</span> · </> : null}{CLASS_BASIS[c.basis]}, {c.members} nouns; since {year(c.born)}.</li>)}</ul></section> : null}
    <h4>Not yet modelled</h4>
    <p className="muted">Agreement beyond the word for “this”, and tenses beyond past and future.</p>
  </div>;
}

export function LoanCauseText({ cause, ctx }: { cause: LoanCause; ctx: ChapterContext }) {
  let text: ReactNode;
  switch (cause.kind) {
    case "unrecorded": return <span className="muted"> Contact not recorded.</span>;
    case "contact": text = <>{CONTACT_NAME[cause.contact].toLowerCase()} between {people(cause.donor, ctx)} and {people(cause.recipient, ctx)}, since {year(cause.since)}</>; break;
    case "rule": text = <>the rule of {people(cause.ruler, ctx)} over {people(cause.ruled, ctx)}</>; break;
    case "faith": text = <>the faith of {ctx.link({ kind: "religion", id: cause.religion }, ctx.overview.religions[cause.religion]?.name ?? "an unrecorded faith")}{cause.teacher !== null ? <>, taught by {people(cause.teacher, ctx)}</> : null}</>; break;
    case "shift": text = <>{people(cause.community, ctx)} taking up another tongue, keeping words from {language(cause.fromVariety, ctx)}</>; break;
    case "city": text = <>contact in {ctx.overview.cities[cause.city]?.name.name ?? "a great city"}</>; break;
    case "coinage": text = <>a new idea among {cause.peoples.map((id, i) => <span key={id}>{i ? ", " : ""}{people(id, ctx)}</span>)}</>; break;
    case "classical": text = <>learning from {language(cause.classical, ctx)} by {people(cause.recipient, ctx)}</>; break;
  }
  const event = cause.event ? findAnnal(ctx.overview.annals, cause.event) : undefined;
  return <span className="loan-cause"> Through {text}.{event ? <> After {ctx.link({ kind: "event", id: event.id }, <>{event.kind} in {year(event.generation)}</>)}.</> : null}</span>;
}

export function WordOrigin({ word: { origin, coined }, ctx }: { word: Pick<Variant, "origin" | "coined">; ctx: ChapterContext }) {
  return <span className={`origin origin-${origin.kind}`}>
    {coined ? coined.kind === "calque"
      ? <>Translated part by part from {language(coined.from!, ctx)} in {year(coined.generation)}.</>
      : <>{coined.kind === "compound" ? "Made" : "Derived"} in {year(coined.generation)} from {coined.parts.map((part, i) => <span key={i}>{i ? " and " : ""}<span className="word">{part.spelled}</span> ‘{part.gloss}’</span>)}.</>
      : <>{origin.kind === "borrowed" ? <>Borrowed from {origin.fromVariety !== null ? language(origin.fromVariety, ctx) : origin.from ?? "an unrecorded source"}</> : origin.kind === "kept" ? <>Kept from {origin.fromVariety !== null ? language(origin.fromVariety, ctx) : origin.from ?? "an earlier tongue"}</> : origin.kind === "derived" ? <>Built from “{origin.from}”</> : origin.kind === "coined" ? "Coined" : "Inherited"}, {year(origin.generation)}.</>}
    {coined?.opaqueSince !== null && coined?.opaqueSince !== undefined ? <> Its parts were no longer heard in it after {year(coined.opaqueSince)}.</> : null}
    {origin.cause ? <LoanCauseText cause={origin.cause} ctx={ctx} /> : null}
  </span>;
}

function LexicalEvidence({ row, variety, ctx }: SectionProps & { row: LexiconRow }) {
  const [open, setOpen] = useState(false);
  return <details className="lexical-evidence" open={open} onToggle={(e) => setOpen(e.currentTarget.open)}>
    <summary>{row.competitors ? `${row.competitors} competing ${row.competitors === 1 ? "word" : "words"} · ` : ""}Origins, senses, and family cognates</summary>
    {open ? <WordGloss engine={ctx.engine} version={ctx.version} generation={ctx.generation}
      variety={variety.id} concept={row.concept} onScrub={ctx.onYear}
      onOpenVariety={(id) => ctx.open({ kind: "word", variety: id, concept: row.concept })}
      renderCause={(cause) => <LoanCauseText cause={cause} ctx={ctx} />}
      renderOrigin={(word) => <WordOrigin word={word} ctx={ctx} />}
      family={variety.family} overview={ctx.overview} /> : null}
  </details>;
}

export function Lexicon({ variety, ctx }: SectionProps) {
  const [view, setView] = useState(INITIAL_DICTIONARY);
  return <div className="language-section">
    <p className="muted small">Every current meaning, with spelling, speech, and origin. Open a word’s evidence for competitors, stretched senses, <Explained term="paradigm">paradigms</Explained>, and family cognates.</p>
    <Dictionary engine={ctx.engine} version={ctx.version} generation={ctx.generation} variety={variety.id}
      view={ctx.dictionaryView ?? view} onView={ctx.onDictionaryView ?? setView}
      onConcept={(concept) => ctx.open({ kind: "word", variety: variety.id, concept })}
      renderOrigin={(word) => <WordOrigin word={word} ctx={ctx} />}
      renderEvidence={(row) => <LexicalEvidence row={row} variety={variety} ctx={ctx} />}
      rowId={ctx.wordAnchor ? (row) => ctx.wordAnchor!(variety.id, row.concept) : undefined} />
  </div>;
}

export function Names({ variety: v, ctx }: SectionProps) {
  const lands = ctx.overview.places.flatMap((p) => p.names.filter((n) => n.variety === v.id).map((n) => ({ region: p.region, name: n })));
  const peoples = ctx.overview.communities.filter((c) => c.variety === v.id);
  const otherNames = ctx.overview.communities.flatMap((c) => c.exonyms
    .filter((name) => ctx.overview.communities[name.by]?.variety === v.id)
    .map((name) => ({ community: c.id, name })));
  return <div className="language-section">
    <p>{v.nameStyle === "double" ? "Two-part given names, built from two themes." : "One-word given names."}</p>
    <h4>Given names</h4>
    {v.names.length ? <ul className="roster given-names">{v.names.map((n, i) => <li key={i}>
      <span className="word">{n.name}</span> <span className="ipa">/{n.ipa}/</span> “{n.meaning}”
      {n.from !== null ? <> · from {language(n.from, ctx)} (sacred)</> : null}
    </li>)}</ul> : <p className="muted">No given names recorded.</p>}
    <h4>Lands named in this tongue</h4>
    {lands.length ? <ul className="roster">{lands.map(({ region, name: n }, i) => <li key={i}>
      {ctx.link({ kind: "land", region }, <span className="word">{n.spelled}</span>)} <span className="ipa">/{n.ipa}/</span>
      {" "}“{n.meaning}” · {n.origin}, since {year(n.since)}
      {n.by !== null ? <>; coined by {people(n.by, ctx)}</> : null}{n.once ? `; once ${n.once}` : ""}.
    </li>)}</ul> : <p className="muted">No land names recorded in this tongue.</p>}
    <h4>Peoples named in this tongue</h4>
    {peoples.length ? <ul className="roster">{peoples.map((c) => <li key={c.id}>
      {people(c.id, ctx)} <span className="ipa">/{c.ipa}/</span> “{c.meaning}” · self-name coined in {year(c.coined)}
      {c.once ? `; once ${c.once}` : ""}.
    </li>)}</ul> : <p className="muted">No people’s self-name is recorded in this tongue.</p>}
    {otherNames.length ? <><h4>Names for other peoples</h4><ul className="roster">
      {otherNames.map(({ community, name }, i) => <li key={i}>
        <span className="word">{name.name}</span> for {people(community, ctx)}, as {people(name.by, ctx)} call them.
      </li>)}
    </ul></> : null}
  </div>;
}

export function Standing({ variety: v, ctx }: SectionProps) {
  const speakers = ctx.overview.communities.filter((c) => c.ended === null && c.variety === v.id);
  const writers = ctx.overview.varieties.filter((w) => w.spoken && w.high === v.id && w.vernacular === null);
  const city = ctx.overview.cities.find((c) => c.townsfolk !== null && ctx.overview.communities[c.townsfolk]?.variety === v.id);
  const standing = individualAnnals(ctx.overview.annals).filter((a) =>
    (a.variety === v.id || a.languages.includes(v.id)) &&
    (a.kind === "standard" || a.kind === "classical" || a.kind === "vernacular" ||
      a.kind === "respelling" || a.kind === "koine" || a.kind === "faith" || (a.kind === "craft" && a.crafts.includes("writing"))));
  return <div className="language-section">
    <dl className="chapter-facts">
      <div><dt>Speech</dt><dd>{speakers.length ? speakers.map((c, i) => <span key={c.id}>
        {i ? ", " : ""}{people(c.id, ctx)}
      </span>) : "No living speakers"}.</dd></div>
      {v.standardOf !== null ? <div><dt>Standard</dt><dd>Of {ctx.link({ kind: "state", id: v.standardOf }, ctx.overview.states[v.standardOf]?.name)}.</dd></div> : null}
      {v.classicalOf !== null ? <div><dt>Classical</dt><dd>
        Kept by {ctx.link({ kind: "state", id: v.classicalOf }, ctx.overview.states[v.classicalOf]?.name)}
        {writers.length ? <>; written by speakers of {writers.map((w, i) => <span key={w.id}>{i ? ", " : ""}{language(w.id, ctx)}</span>)}</> : null}.
      </dd></div> : null}
      {v.sacredOf !== null ? <div><dt>Sacred</dt><dd>To {ctx.link({ kind: "religion", id: v.sacredOf }, ctx.overview.religions[v.sacredOf]?.name)}.</dd></div> : null}
      <div><dt>Writing</dt><dd>{v.high !== null && v.vernacular === null
        ? <>Its speakers write {language(v.high, ctx)}, their classical tongue.</>
        : v.written !== null ? <>Written or last respelled in {year(v.written)}.</> : "Unwritten."}
      </dd></div>
      {v.vernacular !== null ? <div><dt>Vernacular</dt><dd>
        Its own speech written since {year(v.vernacular)}{v.high !== null ? <>, in place of {language(v.high, ctx)}</> : null}.
      </dd></div> : null}
      {v.purism.length ? <div><dt>Purism</dt><dd>
        {v.purism.map((p, i) => <span key={p.since}>{i ? "; " : ""}in {year(p.since)} its keepers wrote {p.replaced.slice(0, 3).map((w, j) => <span key={w.concept}>{j ? ", " : ""}<span className="word">{w.native}</span> for <span className="word">{w.loan}</span></span>)}{p.replaced.length > 3 ? `, and ${p.replaced.length - 3} more` : ""}</span>)}.
      </dd></div> : null}
      {v.koineOf !== null ? <div><dt>City speech</dt><dd>
        A koiné{city ? ` of ${city.name.name}` : ""}, levelled from {v.koineOf.map((s, i) => <span key={s.variety}>
          {i ? ", " : ""}{language(s.variety, ctx)} {Math.round(s.share * 100)}%
        </span>)}.
      </dd></div> : null}
    </dl>
    {standing.length ? <><h4>How it came to stand</h4><ol className="history">
      {standing.map((a) => <li key={a.id}><span className="gen">{a.generation * YEARS}</span>
        <span>{ctx.link({ kind: "event", id: a.id }, a.text.replaceAll("*", ""))}</span>
      </li>)}
    </ol></> : null}
  </div>;
}

export const CHAPTER = [
  { id: "position", title: "Position", Section: Position },
  { id: "sounds", title: "Sounds", Section: Sounds },
  { id: "laws", title: "Sound laws", Section: SoundLaws },
  { id: "building", title: "Word building", Section: WordBuilding },
  { id: "grammar", title: "Grammar", Section: GrammarSketch },
  { id: "lexicon", title: "Lexicon", Section: Lexicon },
  { id: "names", title: "Names", Section: Names },
  { id: "standing", title: "Standing", Section: Standing },
];

export function ChapterSummary({ variety: v, section, ctx }: SectionProps & { section: string }) {
  switch (section) {
    case "position": {
      const size = ctx.overview.varieties.filter((d) => d.family === v.family).length;
      return <p>{v.parent === null ? "A founding tongue" : <>Descended from {language(v.parent, ctx)}</>};
        {" "}{size === 1 ? "the only language of its family" : `the ${size} languages of its family`}.
      </p>;
    }
    case "sounds": return <p>{v.consonants.length} consonants, {v.vowels.length} vowels; stress {STRESS_RULE[v.stress]}.</p>;
    case "laws": return <p>{v.laws.length ? <>{v.laws.length} laws; the latest in {year(v.laws.at(-1)!.generation)}: {v.laws.at(-1)!.label}.</> : "No sound laws yet."}</p>;
    case "building": return <p>{v.wordBuilding}.</p>;
    case "grammar": return <p>{WORD_ORDER_PHRASE[v.grammar.order].summary}; {MARKING_PHRASE[v.grammar.marking].summary}; {POSSESSOR_PHRASE[v.grammar.possessor].summary}.</p>;
    case "lexicon": return <p>{v.words.toLocaleString()} words, with origins, competitors, and cognates.</p>;
    case "names": return <p>{v.names.slice(0, 3).map((n) => n.name).join(", ") || "No given names recorded"}; {v.nameStyle === "double" ? "two-part" : "one-word"} names.</p>;
    default: return <p>{v.spoken ? "Spoken" : "Silent"}{v.standardOf !== null ? "; standard" : ""}{v.classicalOf !== null ? "; classical" : ""}{v.sacredOf !== null ? "; sacred" : ""}; {v.high !== null && v.vernacular === null ? <>its speakers write {language(v.high, ctx)}</> : v.written !== null ? "written" : "unwritten"}.</p>;
  }
}
