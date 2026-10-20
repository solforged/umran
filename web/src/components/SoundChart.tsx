import { use, useState, type CSSProperties } from "react";
import type { SoundInfo, SoundUse, Variety } from "../model";
import { YEARS } from "../model";
import type { ChapterContext } from "./LanguageChapter";
import { EditionContext } from "../edition";

const SECONDARY_ORDER = ["plain", "aspirated", "breathy", "labialized", "rounded"];
const BACKS = ["front", "central", "back"];

/// A language's sounds on the catalog's chart, consonants by place and
/// manner and vowels by height and backness, each chip darker the more
/// words say it, with the sounds gained and lost since the language arose.
export function SoundChart({ variety, ctx }: { variety: Variety; ctx: ChapterContext }) {
  const chart = use(ctx.engine.soundChart(ctx.generation, variety.id));
  const { complete } = use(EditionContext);
  const [selected, setSelected] = useState<string | null>(null);
  const info = new Map(ctx.catalog.sounds.map((sound) => [sound.ipa, sound]));
  const uses = new Map<string, { use: SoundUse; lost: boolean }>();
  for (const use of chart.segments) uses.set(use.ipa, { use, lost: false });
  for (const use of chart.lost) uses.set(use.ipa, { use, lost: true });
  const placed = [...uses.values()].flatMap(({ use, lost }) => {
    const sound = info.get(use.ipa);
    return sound ? [{ sound, use, lost }] : [];
  });
  const order = (a: SoundInfo, b: SoundInfo) =>
    SECONDARY_ORDER.indexOf(a.secondary) - SECONDARY_ORDER.indexOf(b.secondary) || Number(a.voiced) - Number(b.voiced);
  const consonants = placed.filter(({ sound }) => !sound.vowel);
  const vowels = placed.filter(({ sound }) => sound.vowel);
  const places = ctx.catalog.places.filter((place) => consonants.some(({ sound }) => sound.column === place));
  const manners = ctx.catalog.manners.filter((manner) => consonants.some(({ sound }) => sound.row === manner));
  const heights = ctx.catalog.heights.filter((height) => vowels.some(({ sound }) => sound.row === height));
  const backs = BACKS.filter((back) => vowels.some(({ sound }) => sound.column === back));
  const gained = chart.segments.filter((use) => use.new).length;
  const chosen = selected === null ? null : uses.get(selected) ?? null;

  const chip = ({ sound, use, lost }: { sound: SoundInfo; use: SoundUse; lost: boolean }) => {
    // Square-root scaling keeps rare sounds visible beside the commonest.
    const share = lost ? 0 : Math.sqrt(use.words / Math.max(chart.words, 1));
    const standing = lost ? "lost" : use.new ? "gained" : "kept";
    const title = lost
      ? `${use.ipa} · lost; once in ${use.example.spelled} “${use.example.gloss}”`
      : `${use.ipa} · in ${use.words} ${use.words === 1 ? "word" : "words"} · ${use.example.spelled}${use.example.concept === null ? `, ${use.example.gloss}` : ` “${use.example.gloss}”`}${use.new ? " · gained" : ""}`;
    return complete
      ? <span key={sound.ipa} className={`chip sound ${standing}`} style={{ "--share": share } as CSSProperties} title={title}>{sound.ipa}</span>
      : <button
        key={sound.ipa}
        type="button"
        className={`chip sound ${standing}`}
        style={{ "--share": share } as CSSProperties}
        title={title}
        aria-pressed={selected === sound.ipa}
        onClick={() => setSelected(selected === sound.ipa ? null : sound.ipa)}
      >{sound.ipa}</button>;
  };

  const grid = (
    className: string,
    columns: string[],
    rows: string[],
    members: { sound: SoundInfo; use: SoundUse; lost: boolean }[],
  ) => <div className="chart-wrap"><table className={`chart sound-grid ${className}`}>
    <thead><tr><th />{columns.map((column) => <th key={column}>{column}</th>)}</tr></thead>
    <tbody>{rows.map((row) => <tr key={row}>
      <th>{row}</th>
      {columns.map((column) => <td key={column}>
        {members.filter(({ sound }) => sound.row === row && sound.column === column).sort((a, b) => order(a.sound, b.sound)).map(chip)}
      </td>)}
    </tr>)}</tbody>
  </table></div>;

  return <div className="sound-chart">
    <h4>{chart.segments.filter((use) => !use.vowel).length === 1 ? "Consonant" : "Consonants"} · {chart.segments.filter((use) => !use.vowel).length}</h4>
    {consonants.length ? grid("consonants", places, manners, consonants) : <p className="muted">None.</p>}
    <h4>{chart.segments.filter((use) => use.vowel).length === 1 ? "Vowel" : "Vowels"} · {chart.segments.filter((use) => use.vowel).length}</h4>
    {vowels.length ? grid("vowels", backs, heights, vowels) : <p className="muted">None.</p>}
    <p className="muted small sound-legend">
      Darker chips sound in more of the {chart.words.toLocaleString()} words.
      {gained || chart.lost.length ? <> Since year {chart.since * YEARS}: {gained ? <><span className="chip sound gained legend">{gained}</span> gained</> : null}{gained && chart.lost.length ? ", " : ""}{chart.lost.length ? <><span className="chip sound lost legend">{chart.lost.length}</span> lost</> : null}.</> : <> Unchanged since year {chart.since * YEARS}.</>}
      {complete ? null : " Choose a chip for a word that says it."}
    </p>
    {chosen ? <p className="sound-caption" aria-live="polite">
      <span className="ipa">{chosen.use.ipa}</span>{chosen.lost
        ? <> is lost: no word says it now. It was once in </>
        : chosen.use.words === 0
          ? <> is in no main word{chosen.use.new ? ", gained since the language arose" : ""}; it is heard in </>
          : <> is in {chosen.use.words} of {chart.words.toLocaleString()} words{chosen.use.new ? ", gained since the language arose" : ""}, as in </>}
      {chosen.use.example.concept === null
        ? <span className="word">{chosen.use.example.spelled}</span>
        : ctx.link({ kind: "word", variety: variety.id, concept: chosen.use.example.concept }, <span className="word">{chosen.use.example.spelled}</span>)} <span className="ipa">/{chosen.use.example.ipa}/</span>{chosen.use.example.concept === null ? `, ${chosen.use.example.gloss}` : ` “${chosen.use.example.gloss}”`}.
    </p> : null}
  </div>;
}
