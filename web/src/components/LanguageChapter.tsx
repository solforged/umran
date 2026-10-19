import { use, useState, type ReactNode } from "react";
import { Download, Feather } from "lucide-react";
import type { GrammarMarker, HistoryLine, LawOccurrence, LawOccurrenceRef, LexiconRow, LoanCause, Overview, ReadEngine, Subject, Variant, Variety, WorldMap } from "../model";
import { YEARS } from "../model";
import { download, fileName, nameQuarryCsv } from "../takeout";
import { agreementPhrase, AGREEMENT_STEP, CONTACT_NAME, landCardLabel, MARKING_PHRASE, peopleLabel, POSSESSOR_PHRASE, PRONOUN_CASE_PHRASE, stateLabel, STRESS_RULE, WORD_ORDER_PHRASE } from "../lore";
import { individualAnnals, findAnnal } from "../history";
import { Dictionary, INITIAL_DICTIONARY, type DictionaryView } from "./Dictionary";
import { FamilyTree } from "./FamilyTree";
import { Sample } from "./Sample";
import { WordGloss } from "./WordGloss";
import { Explained } from "./Explained";
import { ChangeCauseText, EvidenceForm, LifeRibbon, lawSubject, occurrenceAnchor, occurrenceKey, sameOccurrence } from "./LifeRibbon";
import { Pending } from "./Pending";
import { EditionContext } from "../edition";
import { eras, languageEraChange } from "../eras";
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
const people = (id: number, ctx: ChapterContext) => ctx.link({ kind: "people", id }, peopleLabel(ctx.overview, id));
const year = (at: number) => `year ${at * YEARS}`;

function highRegister(id: number, ctx: ChapterContext) {
  const high = ctx.overview.varieties[id];
  const standing = high?.sacredOf != null ? "sacred tongue"
    : high?.classicalOf != null ? "classical tongue"
    : high?.standardOf != null ? "court standard"
    : "high written tongue";
  return <>{language(id, ctx)}, their {standing}</>;
}

export function Position({ variety: v, ctx }: SectionProps) {
  const daughters = ctx.overview.varieties.filter((d) => d.parent === v.id);
  const kin = use(ctx.engine.kin(ctx.generation, v.id)).filter((k) => k.score >= 0.05);
  return <div className="language-section">
    <p>{v.parent === null ? <>A founding language of its family, first spoken in {year(v.born)}.</> : <>Descended from {language(v.parent, ctx)}, parted in {year(v.forkedAt ?? v.born)}.</>} {v.spoken ? "Spoken now." : `Silent${v.silentSince === null ? " now" : ` since ${year(v.silentSince)}`}.`}</p>
    <p>Family: {language(v.family, ctx)}. Daughters: {daughters.length ? daughters.map((d, i) => <span key={d.id}>{i ? ", " : ""}{language(d.id, ctx)}</span>) : "none recorded"}.</p>
    <FamilyTree overview={ctx.overview} family={v.family} chosen={v.id} onOpen={(id) => ctx.open({ kind: "language", variety: id })} />
    <h4>Core-word sound similarity</h4>
    <p className="muted small">This compares the current forms for the same core meanings, corrects for chance, and is not a count of shared roots.</p>
    {kin.length ? <ul className="roster">{kin.map((k) => <li key={k.other} className="meter-row">{language(k.other, ctx)} <meter min={0} max={1} value={k.score} /> <span>{Math.round(k.score * 100)}%</span></li>)}</ul> : <p className="muted">No other spoken language has a core-word sound similarity of at least 5%.</p>}
    <h4>The language’s fortunes</h4>
    <LanguageFortunes variety={v} ctx={ctx} />
  </div>;
}

export function LanguageFortunes({ variety, ctx }: SectionProps) {
  const calendar = eras(ctx.overview.annals, ctx.generation, ctx.overview).filter((era) => era.end >= variety.born);
  return <ol className="language-fortunes">
    {calendar.map((era) => {
      const change = languageEraChange(era, ctx.overview.annals, variety.id);
      return <li key={era.start}>
        <span className="gen">{era.start * YEARS}–{era.end * YEARS}</span>
        <div><strong>{era.heading}</strong><p>{change ? <><span className="gen">{change.generation * YEARS}</span> {ctx.link({ kind: "event", id: change.id }, change.text.replaceAll("*", ""))}</> : <span className="muted">No recorded turning point for this language in the era.</span>}</p></div>
      </li>;
    })}
  </ol>;
}

export function Sounds({ variety: v, ctx }: SectionProps) {
  return <div className="language-section">
    <h4>{v.consonants.length === 1 ? "Consonant" : "Consonants"} · {v.consonants.length}</h4><p className="segments">{v.consonants.join(" ")}</p>
    <h4>{v.vowels.length === 1 ? "Vowel" : "Vowels"} · {v.vowels.length}</h4><p className="segments">{v.vowels.join(" ")}</p>
    {v.harmony ? <p>{v.harmony.lost === null ? <>Vowel harmony: within a word, vowels agree {HARMONY[v.harmony.feature]}, since {year(v.harmony.since)}.</> : <>Vowel harmony, {HARMONY[v.harmony.feature]}, from {year(v.harmony.since)} until {year(v.harmony.lost)}.</>}</p> : null}
    {v.tonal ? <p>{v.tonal.lost === null ? <>Tone: {v.tones} pitch {v.tones === 1 ? "pattern tells" : "patterns tell"} words apart, since {year(v.tonal.since)}. Spelling marks tones with vowel diacritics; IPA uses Chao tone letters after the vowel.</> : <>Tonal from {year(v.tonal.since)} until {year(v.tonal.lost)}.</>}</p> : null}
    <dl className="chapter-facts"><div><dt><Explained term="minimal word">Minimal word</Explained></dt><dd>{v.minimalWord.includes("heavy") ? <Explained term="heavy syllable">{v.minimalWord}</Explained> : v.minimalWord}; words founded short or borrowed may sit below it, but sound change never wears a word below it or further below it.</dd></div><div><dt><Explained term="stress">Stress</Explained></dt><dd>{v.stress === "weight" ? <>on a <Explained term="heavy syllable">heavy</Explained> next-to-last syllable, otherwise the syllable before it; on the first syllable in a two-syllable word</> : STRESS_RULE[v.stress]}.</dd></div><div><dt><Explained term="geminate">Geminates</Explained></dt><dd>{v.geminates ? "Long consonants occur in living words." : "No long consonants in living words."}</dd></div></dl>
    <p className="muted small"><Explained term="IPA">IPA</Explained>: International Phonetic Alphabet, symbols for speech sounds.</p>
    <LifeRibbon variety={v.id} ctx={ctx} />
  </div>;
}

/** Exact evidence for one application, supplied together by the engine. */
export function LawEvidence({ variety, occurrence, ctx }: SectionProps & { occurrence: LawOccurrence }) {
  const [expanded, setExpanded] = useState(false);
  const { complete } = use(EditionContext);
  const arrival = occurrence.arrival;
  return <div className="law-evidence">
    <p className="muted small">{arrival.kind === "wave" ? <>A <Explained term="wave">wave</Explained> from {language(arrival.from, ctx)}.</>
      : arrival.kind === "inherited" ? <>Inherited from {language(arrival.ancestor, ctx)}.</>
      : arrival.kind === "authored" ? <>The author’s decision {arrival.decision + 1}.</>
      : arrival.kind === "substrate" ? <><Explained term="substrate">Substrate</Explained>: traces of the speakers’ earlier tongue.</>
      : arrival.kind === "unrecorded" ? "How it arrived is not recorded."
      : "This language’s own change."}
      {occurrence.annal ? <> {ctx.link({ kind: "event", id: occurrence.annal }, "Read its chronicle entry")}.</> : null}
    </p>
    <details open={complete || expanded} onToggle={(event) => setExpanded(event.currentTarget.open)}>
      <summary>{occurrence.words.length} {occurrence.words.length === 1 ? "word" : "words"}, {occurrence.markers.length} grammatical {occurrence.markers.length === 1 ? "form" : "forms"}, {occurrence.names.length} {occurrence.names.length === 1 ? "name" : "names"} · before and after</summary>
      {complete || expanded ? <>
        {occurrence.words.length ? <div className="table-wrap"><table className="law-words">
          <thead><tr><th>Meaning</th><th>Before → after</th></tr></thead>
          <tbody>{occurrence.words.map((word, i) => <tr key={i}>
            <td>{ctx.link({ kind: "word", variety: variety.id, concept: word.concept }, word.gloss)}{!word.dominant ? <span className="muted"> · competing word</span> : null}</td>
            <td><EvidenceForm form={word.before} /> → <EvidenceForm form={word.after} /></td>
          </tr>)}</tbody>
        </table></div> : <p className="muted small">No word records a changed form in this application.</p>}
        {occurrence.markers.length ? <><h5>Grammatical forms</h5><ul className="roster">{occurrence.markers.map((marker, i) => <li key={i}>{marker.category} · marker {marker.marker}: <EvidenceForm form={marker.before} /> → <EvidenceForm form={marker.after} /></li>)}</ul></> : null}
        {occurrence.names.length ? <><h5>Names</h5><ul className="roster">{occurrence.names.map((name, i) => <li key={i}>{ctx.link(name.subject, <><span className="word">{name.before}</span> → <span className="word">{name.after}</span></>)}</li>)}</ul></> : null}
      </> : null}
    </details>
  </div>;
}

export function SoundLaws({ variety, ctx }: SectionProps) {
  return <Pending reading={ctx.engine}><LawLedger variety={variety} ctx={ctx} /></Pending>;
}

function LawLedger({ variety, ctx }: SectionProps) {
  const [selected, setSelected] = useState<LawOccurrenceRef | null>(null);
  const ids = [...new Set(variety.laws.map((law) => law.id))];
  const reads = ids.map((id) => ctx.engine.lawOccurrences(ctx.generation, variety.id, id));
  const occurrences = reads.flatMap((read) => use(read)).sort((a, b) => a.ref.generation - b.ref.generation || a.ref.index - b.ref.index || a.ref.law.localeCompare(b.ref.law));
  return <div className="language-section">
    <LifeRibbon variety={variety.id} ctx={ctx} selected={selected} />
    <p className="muted small">A dated ledger of regular changes, from earliest to latest. Choose an application to highlight its ticks above. Forms are the engine’s recorded evidence, not reconstructed here.</p>
    {occurrences.length ? <ol className="law-ledger">{occurrences.map((occurrence) => <li key={occurrenceKey(occurrence.ref)} id={occurrence.ref.variety === variety.id ? occurrenceAnchor(occurrence.ref) : undefined} data-law={occurrence.ref.law} data-selected={sameOccurrence(selected, occurrence.ref) || undefined}>
      <div className="law-ledger-head">
        <span className="gen">{occurrence.ref.generation * YEARS}</span>
        {occurrence.arrival.kind === "authored" ? <span className="pen" title="The author’s decision"><Feather size={12} aria-hidden="true" /></span> : null}
        <button type="button" className="link law-select" aria-pressed={sameOccurrence(selected, occurrence.ref)} onClick={() => setSelected(sameOccurrence(selected, occurrence.ref) ? null : occurrence.ref)}>{occurrence.label}</button>
        {ctx.link(lawSubject(occurrence.ref), "Read this application")}
      </div>
      <LawEvidence variety={variety} occurrence={occurrence} ctx={ctx} />
    </li>)}</ol> : <p className="muted">No sound laws yet.</p>}
  </div>;
}

export function WordBuilding({ variety: v, ctx }: SectionProps) {
  const examples = use(ctx.engine.builderExamples(ctx.generation, v.id));
  const patterns = v.wordBuilding.toLowerCase().includes("pattern");
  return <div className="language-section">
    <p>Words are built with <Explained term={patterns ? "root and pattern" : "affix"}>{v.wordBuilding}</Explained>.</p>
    {patterns ? <p className="muted small">C1, C2, and C3 are the root’s first, second, and third consonants; the pattern fits vowels and other sounds around them.</p> : null}
    <dl className="builders">{v.builders.map((builder) => {
      const worked = examples.filter((example) => example.builder === builder.relation);
      return <div key={builder.relation}><dt>{builder.relation}</dt><dd><span className="ipa">{builder.shape}</span>
        {worked.map((example, i) => <p className="builder-example" key={i}>
          {ctx.link({ kind: "word", variety: v.id, concept: example.base.concept }, <EvidenceForm form={example.base.form} />)} “{example.base.gloss}”
          {" "}→ {ctx.link({ kind: "word", variety: v.id, concept: example.built.concept }, <EvidenceForm form={example.built.form} />)} “{example.built.gloss}”
        </p>)}
        {!worked.length ? <p className="muted small">No surviving built word records an example of this pattern.</p> : null}
      </dd></div>;
    })}</dl>
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
      {grammar.sample.pronoun ? <>
        <Sample rendering={grammar.sample.pronoun.first} label="Sample with pronouns" />
        <Sample rendering={grammar.sample.pronoun.second} label="Sample with pronouns reversed" />
      </> : null}
    </> : null}
    {PRONOUN_CASE_PHRASE[grammar.pronounCase] ? <p className="muted small">{PRONOUN_CASE_PHRASE[grammar.pronounCase]}.</p> : null}
    {agreementPhrase(grammar.agreement.stage, grammar.agreement.proDrop) ? <p className="muted small">{agreementPhrase(grammar.agreement.stage, grammar.agreement.proDrop)}{grammar.agreement.since !== null ? `, since ${year(grammar.agreement.since)}` : ""}.</p> : null}
    <p className="muted small">Grammatical forms can change through <Explained term="grammaticalization">grammaticalization</Explained>, <Explained term="fusion">fusion</Explained>, and <Explained term="analogy">analogy</Explained>.</p>
    {grammar.categories.map((category) => <GrammarCategory key={category.category} category={category} variety={v} ctx={ctx} />)}
    {v.pronouns.length ? <section className="grammar-category"><h5>Pronouns</h5><ul className="roster">{v.pronouns.map((p) => <li key={p.cell}><span className="word">{p.spelled}</span> <span className="ipa">/{p.ipa}/</span>{p.object ? <>, as object <span className="word">{p.object.spelled}</span> <span className="ipa">/{p.object.ipa}/</span></> : null} · {PERSON[p.person]} person {p.number === "sg" ? "singular" : "plural"}{p.clusivity ? <>, <Explained term="clusivity">{p.clusivity}</Explained></> : null}{p.polite ? `, respectful address since ${year(p.since)}` : p.origin === "renewed" ? `; renewed in ${year(p.since)}` : p.origin === "borrowed" ? `; borrowed in ${year(p.since)}` : ""}.</li>)}</ul></section> : null}
    {grammar.classes.some((c) => c.mergedInto === null && c.members > 0) ? <section className="grammar-category"><h5>Noun classes</h5><p>Every noun belongs to a class, and the word for “this” agrees with it.</p><ul className="roster">{grammar.classes.filter((c) => c.mergedInto === null && c.members > 0).map((c) => <li key={c.id}>{c.marker ? <><span className="word">{c.marker.spelled}</span> <span className="ipa">/{c.marker.ipa}/</span> · </> : null}{CLASS_BASIS[c.basis]}, {c.members} {c.members === 1 ? "noun" : "nouns"}; since {year(c.born)}.</li>)}</ul></section> : null}
    {grammar.agreement.stage !== "none" || grammar.agreement.history.length ? <section className="grammar-category"><h5>Person on the verb</h5>
      {grammar.agreement.stage !== "none" ? <ul className="roster">{grammar.agreement.cells.map((cell) => <li key={cell.cell}>
        <span className="word">{grammar.agreement.side === "prefix" ? `${cell.spelled}${grammar.agreement.stage === "clitic" ? "=" : "-"}` : `${grammar.agreement.stage === "clitic" ? "=" : "-"}${cell.spelled}`}</span> <span className="ipa">/{cell.ipa}/</span> · {PERSON[cell.person]} person {cell.number === "sg" ? "singular" : "plural"}{cell.cell.endsWith("-inclusive") ? ", inclusive" : cell.cell.endsWith("-exclusive") ? ", exclusive" : ""}, from <span className="word">{cell.source.spelled}</span>{cell.distinct ? "" : "; no longer distinct"}.
      </li>)}</ul> : null}
      {grammar.agreement.history.length ? <ol className="history">{grammar.agreement.history.map((step, i) => <li key={i}><span className="gen">{step.generation * YEARS}</span><span>{AGREEMENT_STEP[step.kind].replace(/^./, (c) => c.toUpperCase())}.</span></li>)}</ol> : null}
    </section> : null}
    <h4>Not yet modelled</h4>
    <p className="muted">Object clitics on the verb, possessive pronouns, and tenses beyond past and future.</p>
  </div>;
}

function GrammarCategory({ category, variety, ctx }: SectionProps & { category: Variety["grammar"]["categories"][number] }) {
  const { complete } = use(EditionContext);
  const markers = variety.grammar.markers.filter((marker) => marker.category === category.category);
  const current = markers.filter((marker) => marker.retired === null);
  const retired = markers.filter((marker) => marker.retired !== null).sort((a, b) => a.retired! - b.retired!);
  return <section className="grammar-category">
    <h5>{category.label}</h5>
    <p>{category.description} {category.eligible} {category.eligible === 1 ? "word" : "words"} can take this category. In {Math.round(category.howSynthetic * 100)}% of uses, it is distinguished within one word; in {Math.round(category.contrastRetention * 100)}% of uses, it remains <Explained term="grammatical contrast">audibly distinct</Explained>, including separate grammatical words.</p>
    <h6 className="grammar-story-heading">How the marking changed</h6>
    <Pending reading={ctx.engine}><GrammarStoryList category={category} variety={variety} ctx={ctx} /></Pending>
    <h6>Current inventory</h6>
    {current.length ? <ul className="roster">{current.map((marker) => <GrammarInventoryMarker key={marker.id} marker={marker} variety={variety.id} ctx={ctx} />)}</ul> : <p className="muted small">No current marker is recorded.</p>}
    {retired.length ? <details className="grammar-appendix" open={complete || undefined}>
      <summary>Retired markers · dated appendix ({retired.length})</summary>
      <ul className="roster">{retired.map((marker) => <GrammarInventoryMarker key={marker.id} marker={marker} variety={variety.id} ctx={ctx} />)}</ul>
    </details> : null}
  </section>;
}

function GrammarStoryList({ category, variety, ctx }: SectionProps & { category: Variety["grammar"]["categories"][number] }) {
  const story = use(ctx.engine.grammarStory(ctx.generation, variety.id, category.category));
  const events: { generation: number; key: string; text: ReactNode }[] = [];
  for (const marker of story.markers) {
    const first = marker.forms[0];
    events.push({ generation: marker.born, key: `${marker.marker}:born`, text: <>
      {marker.source ? <>{ctx.link({ kind: "word", variety: variety.id, concept: marker.source.concept }, <EvidenceForm form={marker.source.form} />)} “{marker.source.gloss}” → </> : null}
      {first ? <EvidenceForm form={first.form} /> : <span>Marker {marker.marker}</span>}, {marker.kind === "none" ? "no overt marker" : marker.kind === "particle" ? "a separate grammatical word" : marker.kind === "pattern" ? "a vowel pattern" : "a bound marker"}.
      {first ? <> <ChangeCauseText cause={first.cause} ctx={ctx} />.</> : null}
    </> });
    marker.forms.slice(1).forEach((point, i) => events.push({ generation: point.generation, key: `${marker.marker}:form:${i}`, text: <>
      <EvidenceForm form={marker.forms[i].form} /> → <EvidenceForm form={point.form} />. <ChangeCauseText cause={point.cause} ctx={ctx} />.
    </> }));
    marker.shares.forEach((point, i) => events.push({ generation: point.generation, key: `${marker.marker}:share:${i}`, text: <>
      Marker {marker.marker}{first ? <> (<EvidenceForm form={first.form} />)</> : null}: {Math.round(point.share * 100)}% of uses in this category.
    </> }));
    if (marker.retired !== null) events.push({ generation: marker.retired, key: `${marker.marker}:retired`, text: <>Marker {marker.marker}{first ? <> (<EvidenceForm form={first.form} />)</> : null} retired; its dated forms are kept in the appendix.</> });
  }
  events.sort((a, b) => a.generation - b.generation);
  return events.length ? <ol className="history grammar-story">{events.map((event) => <li key={event.key}><span className="gen">{event.generation * YEARS}</span><span>{event.text}</span></li>)}</ol> : <p className="muted small">No dated marker story is recorded.</p>;
}

function GrammarInventoryMarker({ marker: m, variety, ctx }: { marker: GrammarMarker; variety: number; ctx: ChapterContext }) {
  const alternants = [...new Set(m.alternants?.map((form) => form.spelled).filter((spelled) => spelled !== m.spelled))];
  return <li>
    <span className="word">{m.spelled || "∅"}</span>{m.said !== null ? <> · said <span className="word">{m.said}</span></> : null} <span className="ipa">/{m.ipa}/</span>
    {alternants.length ? <> · by vowel harmony also {alternants.join(", ")}</> : null} · {m.kind === "pattern" ? <>vowel pattern <span className="ipa">{m.template}</span></> : m.kind === "none" ? "no overt marker" : m.kind === "particle" ? <Explained term="particle">separate word {m.side === "prefix" ? "before" : "after"} the stem</Explained> : <Explained term="affix">{m.side === "prefix" ? "prefix" : "suffix"}</Explained>}, {Math.round(m.share * 100)}% of uses; {m.scope === "pronouns" ? "on pronouns only" : <Explained term="productive">{m.productive ? "used to form new words in this category" : "not used to form new words in this category"}</Explained>}.
    {" "}Since {year(m.born)}{m.retired !== null ? `; retired in ${year(m.retired)}` : ""}.
    {m.origin.kind === "grammaticalized" ? <> From {ctx.link({ kind: "word", variety, concept: m.origin.concept }, `“${m.origin.gloss}”`)} through <Explained term="grammaticalization">grammaticalization</Explained>.</> : m.origin.kind === "imported" ? <> Imported from {language(m.origin.from, ctx)}.</> : m.origin.kind === "fused" ? <> Through <Explained term="fusion">fusion</Explained> from a <Explained term="particle">particle</Explained>.</> : " Founding marker."}
    {m.retired !== null ? <ol className="history">{m.history.map((line, i) => <li key={i}><span className="gen">{line.generation * YEARS}</span><span>{line.text}</span></li>)}</ol> : null}
  </li>;
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
  return <span className="loan-cause"> Through {text}{event ? <> ({ctx.link({ kind: "event", id: event.id }, year(event.generation))})</> : null}.</span>;
}

export function WordOrigin({ word: { origin, coined }, ctx }: { word: Pick<Variant, "origin" | "coined">; ctx: ChapterContext }) {
  return <span className={`origin origin-${origin.kind}`}>
    {coined ? coined.kind === "calque"
      ? <>Translated part by part from {language(coined.from!, ctx)} in {year(coined.generation)}.</>
      : <>{coined.kind === "compound" ? "Made" : "Derived"} in {year(coined.generation)} from {coined.parts.map((part, i) => <span key={i}>{i ? " and " : ""}<span className="word">{part.spelled}</span> “{part.gloss}”</span>)}.</>
      : <>{origin.kind === "borrowed" ? <>Borrowed from {origin.fromVariety !== null ? language(origin.fromVariety, ctx) : origin.from ?? "an unrecorded source"}</> : origin.kind === "kept" ? <>Kept from {origin.fromVariety !== null ? language(origin.fromVariety, ctx) : origin.from ?? "an earlier tongue"}</> : origin.kind === "derived" ? <>Built from “{origin.from}”</> : origin.kind === "coined" ? "Coined" : origin.authored ? "The author’s own root" : "Inherited"}, {year(origin.generation)}.</>}
    {coined?.opaqueSince !== null && coined?.opaqueSince !== undefined ? <> Its parts were no longer heard in it after {year(coined.opaqueSince)}.</> : null}
    {origin.cause ? <LoanCauseText cause={origin.cause} ctx={ctx} /> : null}
  </span>;
}

function LexicalEvidence({ row, variety, ctx }: SectionProps & { row: LexiconRow }) {
  const [open, setOpen] = useState(false);
  const { complete } = use(EditionContext);
  return <details className="lexical-evidence" open={complete || open} onToggle={(e) => setOpen(e.currentTarget.open)}>
    <summary>{row.competitors ? `${row.competitors} competing ${row.competitors === 1 ? "word" : "words"} · ` : ""}Origins, senses, and family cognates</summary>
    {complete || open ? <WordGloss engine={ctx.engine} generation={ctx.generation}
      variety={variety.id} concept={row.concept} onScrub={ctx.onYear}
      onOpenVariety={(id) => ctx.open({ kind: "word", variety: id, concept: row.concept })}
      renderCause={(cause) => <LoanCauseText cause={cause} ctx={ctx} />}
      renderOrigin={(word) => <WordOrigin word={word} ctx={ctx} />}
      renderLink={ctx.link}
      family={variety.family} overview={ctx.overview} /> : null}
  </details>;
}

export function Lexicon({ variety, ctx }: SectionProps) {
  const [view, setView] = useState(INITIAL_DICTIONARY);
  const { complete, apparatus } = use(EditionContext);
  return <div className="language-section">
    <p className="muted small">Every current meaning, with spelling, speech, and origin. Search and origin filters include every current competing word; matching forms appear beneath their meaning. Open a word’s evidence for competitors, stretched senses, <Explained term="paradigm">paradigms</Explained>, and family cognates.</p>
    <Dictionary engine={ctx.engine} version={ctx.version} generation={ctx.generation} variety={variety.id}
      view={complete ? INITIAL_DICTIONARY : ctx.dictionaryView ?? view} onView={ctx.onDictionaryView ?? setView}
      onConcept={(concept) => ctx.open({ kind: "word", variety: variety.id, concept })}
      renderOrigin={(word) => <WordOrigin word={word} ctx={ctx} />}
      renderEvidence={complete && !apparatus ? undefined : (row) => <LexicalEvidence row={row} variety={variety} ctx={ctx} />}
      evidenceBelow={complete}
      rowId={ctx.wordAnchor ? (row) => ctx.wordAnchor!(variety.id, row.concept) : undefined} />
  </div>;
}

export function Names({ variety: v, ctx }: SectionProps) {
  const { complete } = use(EditionContext);
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
      {ctx.link({ kind: "land", region }, <span className="word">{landCardLabel(ctx.overview, ctx.map, region, n.spelled, lands.filter((other) => other.name.spelled === n.spelled).map((other) => other.region))}</span>)} <span className="ipa">/{n.ipa}/</span>
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
    {!complete ? <p className="compared-take" data-edition-omit><button type="button" className="link" onClick={() => void ctx.engine.nameQuarry(ctx.generation, v.id).then((quarry) => download(fileName(`${v.name}-names`, "csv"), nameQuarryCsv(quarry), "text/csv;charset=utf-8"))}>
      <Download size={13} aria-hidden="true" /> Take every name as CSV</button> <span className="muted small">Given names, peoples, lands, rivers, lakes, cities, states, and faiths, with what each meant.</span></p> : null}
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
      {v.standardOf !== null ? <div><dt>Standard</dt><dd>Of {ctx.link({ kind: "state", id: v.standardOf }, stateLabel(ctx.overview, v.standardOf))}.</dd></div> : null}
      {v.classicalOf !== null ? <div><dt>Classical</dt><dd>
        Kept by {ctx.link({ kind: "state", id: v.classicalOf }, stateLabel(ctx.overview, v.classicalOf))}
        {writers.length ? <>; written by speakers of {writers.map((w, i) => <span key={w.id}>{i ? ", " : ""}{language(w.id, ctx)}</span>)}</> : null}.
      </dd></div> : null}
      {v.sacredOf !== null ? <div><dt>Sacred</dt><dd>To {ctx.link({ kind: "religion", id: v.sacredOf }, ctx.overview.religions[v.sacredOf]?.name)}.</dd></div> : null}
      <div><dt>Writing</dt><dd>{v.high !== null && v.vernacular === null
        ? <>Its speakers {v.spoken ? "write" : "wrote"} {highRegister(v.high, ctx)}.</>
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
    case "sounds": return <p>{v.consonants.length} {v.consonants.length === 1 ? "consonant" : "consonants"}, {v.vowels.length} {v.vowels.length === 1 ? "vowel" : "vowels"}; stress {STRESS_RULE[v.stress]}.</p>;
    case "laws": return <p>{v.laws.length ? <>{v.laws.length} {v.laws.length === 1 ? "law" : "laws"}; the latest in {year(v.laws.at(-1)!.generation)}: {v.laws.at(-1)!.label}.</> : "No sound laws yet."}</p>;
    case "building": return <p>{v.wordBuilding}.</p>;
    case "grammar": return <p>{WORD_ORDER_PHRASE[v.grammar.order].summary}; {MARKING_PHRASE[v.grammar.marking].summary}; {POSSESSOR_PHRASE[v.grammar.possessor].summary}.</p>;
    case "lexicon": return <p>{v.words.toLocaleString()} {v.words === 1 ? "word" : "words"}, with origins, competitors, and cognates.</p>;
    case "names": return <p>{v.names.slice(0, 3).map((n) => n.name).join(", ") || "No given names recorded"}; {v.nameStyle === "double" ? "two-part" : "one-word"} names.</p>;
    default: return <p>{v.spoken ? "Spoken" : "Silent"}{v.standardOf !== null ? "; standard" : ""}{v.classicalOf !== null ? "; classical" : ""}{v.sacredOf !== null ? "; sacred" : ""}; {v.high !== null && v.vernacular === null ? <>its speakers {v.spoken ? "write" : "wrote"} {highRegister(v.high, ctx)}</> : v.written !== null ? "written" : "unwritten"}.</p>;
  }
}
