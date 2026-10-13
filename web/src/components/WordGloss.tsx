import { use, type ReactNode } from "react";
import type { LoanCause, LoanTrail, Overview, ReadEngine, Subject, Variant } from "../model";
import { YEARS } from "../model";
import type { ParadigmView } from "./LanguageChapter";
import { Pending } from "./Pending";
import { ChangeCauseText, EvidenceForm } from "./LifeRibbon";
import { EditionContext } from "../edition";

interface GlossProps {
  engine: ReadEngine;
  generation: number;
  variety: number;
  concept: string | null;
  onScrub?: (generation: number) => void;
  onOpenVariety: (variety: number) => void;
  /// How the loan on this line came, drawn by the card that knows the world.
  renderCause?: (cause: LoanCause, generation: number) => ReactNode;
  renderOrigin?: (variant: Variant) => ReactNode;
  renderLink: (subject: Subject, label: ReactNode) => ReactNode;
  family?: number;
  overview: Overview;
}

/// A marginal note on one word: its forms, how it came to be, and its kin
/// in related languages. It waits on its own, so opening one word never
/// blanks the card around it.
export function WordGloss(props: GlossProps) {
  return <Pending reading={props.engine}><Gloss {...props} /></Pending>;
}

function Gloss({
  engine,
  generation,
  variety,
  concept,
  onScrub,
  onOpenVariety,
  renderCause,
  renderOrigin,
  renderLink,
  family,
  overview,
}: GlossProps) {
  // Start the independent evidence together rather than one Suspense retry
  // per request, especially when every lexical entry is opened for an edition.
  const reads = concept ? [
    engine.word(generation, variety, concept),
    engine.loanTrail(generation, variety, concept),
    engine.wordLife(generation, variety, concept),
  ] as const : null;
  const detail = reads ? use(reads[0]) : null;
  const trail = reads ? use(reads[1]) : null;
  const life = reads ? use(reads[2]) : null;
  const { complete } = use(EditionContext);
  if (!detail) {
    return <p className="muted gloss-hint">Choose a word, and its chronicle will be noted here.</p>;
  }
  const cognates = detail.cognates.filter((c) => family === undefined || overview?.varieties[c.variety]?.family === family);

  return (
    <article className="gloss">
      <p className="eyebrow">
        {detail.field}
        {detail.rank !== null ? ` · core meaning #${detail.rank}` : ""}
      </p>
      <h3>{detail.gloss}</h3>
      {trail ? <LoanRoad trail={trail} variety={variety} overview={overview} renderLink={renderLink} /> : null}
      {detail.variants.map((v, i) => {
        const extensions = i === 0 ? life?.holders.at(-1)?.forms.filter((point) => point.cause.kind === "extended") ?? [] : [];
        return (
        <section key={i} className="variant">
          <div className="variant-head">
            <span className="word big">{v.spelled}</span>
            {v.said !== null ? <span> <span className="muted">· said</span> <span className="word">{v.said}</span></span> : null}
            <span className="ipa">/{v.ipa}/</span>
            {detail.variants.length > 1 ? (
              <meter min={0} max={1} value={v.share} title={`${Math.round(v.share * 100)}% of uses`} />
            ) : null}
          </div>
          {renderOrigin ? <p>{renderOrigin(v)}</p> : null}
          {extensions.map((point, j) => <p className="word-extension" key={j}>
            Stretched to “{detail.gloss}” in year {point.generation * YEARS}. <ChangeCauseText cause={point.cause} ctx={{ overview, link: renderLink }} />.
          </p>)}
          {(v as Variant & { paradigms?: ParadigmView[] }).paradigms?.map((p) => (
            <section className="word-paradigm" key={p.category}>
              <h4>{p.label}</h4>
              <ul className="roster">
                {p.realizations.filter((form) => form.retired === null).map((form, k) => <li key={k}>
                  <span className="word">{form.spelled || "∅"}</span>
                  {form.said !== null ? <> · said <span className="word">{form.said}</span></> : null}
                  {" "}<span className="ipa">/{form.ipa}/</span> · {Math.round(form.share * 100)}% of uses.
                  <details open={complete || undefined}><summary>Recorded changes</summary><ol className="history">
                    {form.history.map((line, j) => <li key={j}><span className="gen">{line.generation * YEARS}</span><span>{line.text}</span></li>)}
                  </ol></details>
                </li>)}
              </ul>
              {p.realizations.some((form) => form.retired !== null) ? <details className="grammar-appendix" open={complete || undefined}>
                <summary>Retired realizations · dated appendix</summary>
                <ul className="roster">{p.realizations.filter((form) => form.retired !== null).sort((a, b) => a.retired! - b.retired!).map((form, k) => <li key={k}>
                  <span className="gen">{form.retired! * YEARS}</span> <span className="word">{form.spelled || "∅"}</span> <span className="ipa">/{form.ipa}/</span> · retired.
                  <ol className="history">{form.history.map((line, j) => <li key={j}><span className="gen">{line.generation * YEARS}</span><span>{line.text}</span></li>)}</ol>
                </li>)}</ul>
              </details> : null}
            </section>
          ))}
          {v.senses.length > 1 ? (
            <p className="muted">Also means: {v.senses.filter((s) => s !== detail.gloss).join(", ")}</p>
          ) : null}
          <ol className="history">
            {v.history.filter((line, j) => !(renderOrigin && v.origin.cause && (j === 0 || (i === 0 && trail && line.cause)))).map((line, j) => (
              <li key={j}>
                {onScrub ? <button
                  type="button"
                  className="gen"
                  title={`See the world in year ${line.generation * YEARS}`}
                  onClick={() => onScrub(line.generation)}
                >
                  {line.generation * YEARS}
                </button> : <span className="gen">{line.generation * YEARS}</span>}
                <span>
                  {line.text}
                  {/* A loan's later adaptation lines repeat its cause; tell it once. */}
                  {line.cause && renderCause && !(renderOrigin && v.origin.cause) && !v.history.slice(0, j).some((l) => l.cause)
                    ? renderCause(line.cause, line.generation)
                    : null}
                </span>
              </li>
            ))}
          </ol>
        </section>
        );
      })}
      {cognates.length > 0 ? (
        <>
          <h4>In related languages</h4>
          <ul className="cognates">
            {cognates.map((c) => (
              <li key={c.variety}>
                <button type="button" className="link" onClick={() => onOpenVariety(c.variety)}>
                  {c.name}
                </button>{" "}
                <span className="word">{c.spelled}</span> <span className="ipa">/{c.ipa}/</span>
              </li>
            ))}
          </ul>
        </>
      ) : null}
      {detail.related.length > 0 ? (
        <p className="muted small">A word can spread to: {detail.related.join(", ")}</p>
      ) : null}
    </article>
  );
}

function LoanRoad({ trail, variety, overview, renderLink }: { trail: LoanTrail; variety: number; overview: Overview; renderLink: GlossProps["renderLink"] }) {
  const event = trail.cause.kind === "unrecorded" ? null : trail.cause.event;
  return <section className="loan-trail" aria-label="The road of the loan">
    <h4>The road of the loan</h4>
    <ol className="loan-trail-strip">
      <li><span className="loan-trail-label">Donor</span>
        {renderLink({ kind: "language", variety: trail.donor.variety }, overview.varieties[trail.donor.variety]?.name ?? "An unrecorded language")}
        <span><EvidenceForm form={trail.donor.form} /></span>
      </li>
      <li><span className="loan-trail-label">Meeting · year {trail.since * YEARS}</span>
        {event ? renderLink({ kind: "event", id: event }, "Read the meeting") : <span className="muted">Meeting not recorded</span>}
      </li>
      <li><span className="loan-trail-label">Heard</span><EvidenceForm form={trail.heard} /></li>
      <li><span className="loan-trail-label">First said here</span><EvidenceForm form={trail.adapted} /></li>
      {trail.after.map((point, i) => <li key={i}>
        <span className="loan-trail-label">Year {point.generation * YEARS}</span>
        <EvidenceForm form={point.form} />
        <span><ChangeCauseText cause={point.cause} ctx={{ overview, link: renderLink }} /></span>
      </li>)}
    </ol>
    {trail.sameMeeting.length ? <p className="small loan-trail-companions">Other words from this meeting: {trail.sameMeeting.map((word, i) => <span key={word.concept}>{i ? "; " : ""}{renderLink({ kind: "word", variety, concept: word.concept }, <EvidenceForm form={word.form} />)} “{word.gloss}”</span>)}.</p> : <p className="muted small">No other surviving word records this same meeting.</p>}
  </section>;
}
