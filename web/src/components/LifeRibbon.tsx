import { use, useId, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import type { ChangeCause, FormView, LawOccurrenceRef, Subject, WordLife } from "../model";
import { YEARS } from "../model";
import type { ChapterContext } from "./LanguageChapter";
import { Pending } from "./Pending";
import { Explained } from "./Explained";
import "./chapter.css";

export function lawSubject(ref: LawOccurrenceRef): Subject {
  return { kind: "law", id: ref.law, variety: ref.variety, generation: ref.generation, index: ref.index };
}

export function occurrenceKey(ref: LawOccurrenceRef): string {
  return `${ref.variety}:${ref.generation}:${ref.law}:${ref.index}`;
}

export function occurrenceAnchor(ref: LawOccurrenceRef): string {
  return `law-${ref.variety}-${ref.generation}-${ref.law}-${ref.index}`;
}

export function sameOccurrence(a: LawOccurrenceRef | null, b: LawOccurrenceRef): boolean {
  return a !== null && occurrenceKey(a) === occurrenceKey(b);
}

export function EvidenceForm({ form }: { form: FormView }) {
  return <><span className="word">{form.spelled || "∅"}</span> <span className="ipa">/{form.ipa}/</span></>;
}

export function ChangeCauseText({ cause, ctx }: { cause: ChangeCause | { kind: "analogy" }; ctx: Pick<ChapterContext, "overview" | "link"> }) {
  const language = (variety: number) => ctx.link({ kind: "language", variety }, ctx.overview.varieties[variety]?.name ?? "an unrecorded language");
  switch (cause.kind) {
    case "law": return <>{cause.occurrence === null ? cause.label : ctx.link(lawSubject(cause.occurrence), cause.label)}</>;
    case "loan": return <>Borrowed{cause.cause.kind !== "unrecorded" && cause.cause.event ? <> through {ctx.link({ kind: "event", id: cause.cause.event }, "this meeting")}</> : " through an unrecorded meeting"}</>;
    case "coined": return <>{cause.coined.kind === "calque" ? "Translated part by part" : cause.coined.kind === "compound" ? "Made" : "Derived"}{cause.coined.parts.length ? <> from {cause.coined.parts.map((part, i) => <span key={i}>{i ? " and " : ""}<span className="word">{part.spelled}</span> “{part.gloss}”</span>)}</> : null}</>;
    case "inherited": return <>Inherited from {language(cause.from)}</>;
    case "kept": return <>Kept through a change of tongue from {language(cause.from)}</>;
    case "extended": return <>Meaning stretched from “{cause.from.replaceAll("_", " ")}”</>;
    case "expressive": return <>A fresh expressive root</>;
    case "marker": {
      const origin = cause.origin;
      return origin.kind === "grammaticalized" ? <>The word “{origin.gloss}” gained a grammatical job through <Explained term="grammaticalization">grammaticalization</Explained></>
        : origin.kind === "fused" ? <>A separate <Explained term="particle">particle</Explained> became attached through <Explained term="fusion">fusion</Explained></>
        : origin.kind === "imported" ? <>A marker imported from {language(origin.from)}</>
        : <>A founding grammatical marker</>;
    }
    case "founding": return <>Founding form</>;
    case "analogy": return <>Changed to follow another pattern through <Explained term="analogy">analogy</Explained></>;
  }
}

type Point = WordLife["holders"][number]["forms"][number];
interface Tick { concept: string; gloss: string; holder: number; point: Point; before: FormView | null; key: string }
interface LabelPosition { left: number; row: number }
interface RibbonProps {
  variety: number;
  ctx: ChapterContext;
  selected?: LawOccurrenceRef | null;
  compact?: boolean;
  onExpand?: () => void;
}

/** The engine's six word lives, on one calendar; no forms are reconstructed here. */
export function LifeRibbon(props: RibbonProps) {
  return <Pending reading={props.ctx.engine} label="Reading the six word lives…"><Ribbon {...props} /></Pending>;
}

function Ribbon({ variety, ctx, selected = null, compact = false, onExpand }: RibbonProps) {
  const lives = use(ctx.engine.specimenLives(ctx.generation, variety));
  const [active, setActive] = useState<Tick | null>(null);
  const description = useId();
  const chart = useRef<HTMLDivElement>(null);
  const [labels, setLabels] = useState<Record<string, LabelPosition>>({});
  const start = Math.min(ctx.generation, ...lives.flatMap((life) => life.holders.flatMap((holder) => [holder.entered, ...holder.forms.map((point) => point.generation)])));
  const span = Math.max(1, ctx.generation - start);
  const position = (generation: number) => `${Math.max(0, Math.min(1, (generation - start) / span)) * 100}%`;
  const ticks = (life: WordLife): Tick[] => life.holders.flatMap((holder, holderIndex) => holder.forms.map((point, index) => ({
    concept: life.concept, gloss: life.gloss, holder: holderIndex, point,
    before: index > 0 ? holder.forms[index - 1].form : life.holders[holderIndex - 1]?.forms.at(-1)?.form ?? null,
    key: `${life.concept}:${holderIndex}:${index}`,
  })));
  // Measure the rendered spelling, not its character count: diacritics and
  // self-hosted fonts make estimated widths unreliable at the right edge.
  useLayoutEffect(() => {
    const node = chart.current;
    if (!node || compact) return;
    const place = () => {
      const next: Record<string, LabelPosition> = {};
      for (const lane of node.querySelectorAll<HTMLElement>(".life-ribbon-lane")) {
        const width = lane.clientWidth;
        const occupied: { left: number; right: number }[][] = [];
        for (const label of lane.querySelectorAll<HTMLElement>(".life-ribbon-tick-form")) {
          const tick = label.parentElement!;
          const x = Number(tick.dataset.position) * width;
          const size = label.getBoundingClientRect().width;
          const left = Math.max(0, Math.min(width - size, x - size / 2));
          let row = occupied.findIndex((intervals) => intervals.every((other) => left >= other.right + 8 || left + size + 8 <= other.left));
          if (row < 0) { row = occupied.length; occupied.push([]); }
          occupied[row].push({ left, right: left + size });
          next[tick.dataset.tick!] = { left: left - x, row };
        }
      }
      setLabels((previous) => JSON.stringify(previous) === JSON.stringify(next) ? previous : next);
    };
    place();
    const resize = new ResizeObserver(place);
    resize.observe(node);
    let live = true;
    void document.fonts.ready.then(() => { if (live) place(); });
    return () => { live = false; resize.disconnect(); };
  }, [lives, compact, span]);
  const highlighted = lives.flatMap(ticks).filter((tick) => tick.point.cause.kind === "law" && tick.point.cause.occurrence !== null && sameOccurrence(selected, tick.point.cause.occurrence)).length;
  const shown = lives.flatMap(ticks).find((tick) => tick.key === active?.key) ?? null;
  return <figure className={`life-ribbon${compact ? " compact" : ""}`}>
    <figcaption>Six meanings through the years</figcaption>
    <p className="muted small">{compact ? "Bands trace the recorded holders; focus a tick to read its change." : "Each band is a word that held the meaning; ticks show its recorded forms. Focus or hover a tick to read its change. A faded beginning means the onset of dominance was not recorded."}</p>
    {compact && onExpand ? <p className="small"><button type="button" className="link" onClick={onExpand}>Read the ribbon in Sound laws →</button></p> : null}
    <div className="life-ribbon-scroll">
      <div className="life-ribbon-chart" ref={chart}>
        <div className="life-ribbon-axis"><span>Meaning</span><div><span>Year {start * YEARS}</span><span>{ctx.generation * YEARS}</span></div></div>
        {lives.map((life) => <div className="life-ribbon-row" key={life.concept}>
          <div className="life-ribbon-meaning">{ctx.link({ kind: "word", variety, concept: life.concept }, life.gloss)}</div>
          <div className="life-ribbon-track">
            {life.holders.map((holder, i) => {
              const from = holder.dominantFrom ?? holder.entered;
              const first = holder.forms[0];
              const until = holder.until ?? ctx.generation;
              const label = holder.dominantFrom === null
                ? `Held this meaning by year ${(first?.generation ?? holder.entered) * YEARS}; onset of dominance not recorded`
                : `Held this meaning from year ${holder.dominantFrom * YEARS}`;
              const holderTicks = ticks(life).filter((tick) => tick.holder === i);
              const rows = compact ? 0 : Math.max(1, ...holderTicks.map((tick) => (labels[tick.key]?.row ?? 0) + 1));
              const ending = holder.until === null ? "; no later replacement recorded" : ` until year ${holder.until * YEARS}`;
              return <div className="life-ribbon-lane" key={i} style={{ "--label-rows": rows } as CSSProperties}>
                <span data-holder={i % 2} className={`life-ribbon-band${holder.dominantFrom === null ? " unknown-onset" : ""}${holder.until === null && i < life.holders.length - 1 ? " unknown-end" : ""}`} style={{ left: position(from), width: `${Math.max(0, (until - from) / span) * 100}%` }} title={`${label}${ending}`} aria-label={`${label}${ending}`} />
                {holderTicks.map((tick) => {
                  const cause = tick.point.cause;
                  const chosen = cause.kind === "law" && cause.occurrence !== null && sameOccurrence(selected, cause.occurrence);
                  const changed = tick.before === null || tick.before.spelled !== tick.point.form.spelled;
                  const text = `${life.gloss}, year ${tick.point.generation * YEARS}: ${tick.before ? `${tick.before.spelled}, /${tick.before.ipa}/ → ` : ""}${tick.point.form.spelled}, /${tick.point.form.ipa}/`;
                  const placement = labels[tick.key];
                  return <button type="button" key={tick.key} data-tick={tick.key} data-position={Math.max(0, Math.min(1, (tick.point.generation - start) / span))} className={`life-ribbon-tick${!compact && changed ? " labeled" : ""}${chosen ? " selected" : ""}${shown?.key === tick.key ? " active" : ""}`} style={{ left: position(tick.point.generation), "--tick-row": placement?.row ?? 0, "--label-left": `${placement?.left ?? 0}px` } as CSSProperties}
                    title={text} aria-label={text}
                    aria-describedby={shown?.key === tick.key ? description : undefined}
                    onPointerEnter={() => setActive(tick)} onFocus={() => setActive(tick)}
                    onClick={() => { setActive(tick); if (cause.kind === "law" && cause.occurrence !== null) ctx.open(lawSubject(cause.occurrence)); }}>
                    <span className="life-ribbon-tick-mark" aria-hidden="true" />
                    {!compact && changed ? <span className="word life-ribbon-tick-form" aria-hidden="true">{tick.point.form.spelled || "∅"}</span> : null}
                  </button>;
                })}
              </div>;
            })}
            {!life.holders.length ? <span className="muted small">No word recorded yet.</span> : null}
          </div>
          {compact ? <span className="word life-ribbon-current">{life.holders.at(-1)?.forms.at(-1)?.form.spelled || "∅"}</span> : null}
        </div>)}
      </div>
    </div>
    {selected ? <p className="muted small" role="status">{highlighted ? `${highlighted} ${highlighted === 1 ? "tick belongs" : "ticks belong"} to the selected application.` : "This application changed none of the six recorded word lives."}</p> : null}
    {!compact && lives.some((life) => life.holders.some((holder) => holder.dominantFrom === null)) ? <ul className="life-ribbon-unknown muted small">
      {lives.flatMap((life) => life.holders.flatMap((holder, i) => holder.dominantFrom === null ? <li key={`${life.concept}:${i}`}>“{life.gloss}”: {holder.forms[0] ? <EvidenceForm form={holder.forms[0].form} /> : "a word"} held this meaning by year {(holder.forms[0]?.generation ?? holder.entered) * YEARS}; the onset of dominance was not recorded.</li> : []))}
    </ul> : null}
    {!compact || shown ? <div className="life-ribbon-reading" id={description}>
      {shown ? <><span className="gen">{shown.point.generation * YEARS}</span><span>“{shown.gloss}”: {shown.before ? <><EvidenceForm form={shown.before} /> → </> : null}<EvidenceForm form={shown.point.form} />. <ChangeCauseText cause={shown.point.cause} ctx={ctx} />.</span></> : <p className="muted small">Choose a tick to read before → after and what caused it.</p>}
    </div> : null}
  </figure>;
}
