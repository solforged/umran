import { useMemo, useState, type ReactNode } from "react";
import { presetDesign, preview, typicalDesign } from "../engine";
import type { Catalog, LanguageDesign, LongVowelStyle, SoundInfo } from "../model";

export interface Founding {
  name: string;
  design: LanguageDesign;
  /// The language's own seed, the one previewed.
  seed: number;
  power: number;
  openness: number;
  /// Only when starting a new world.
  worldSeed: number;
}

function randomSeed(): number {
  return Math.floor(Math.random() * 0xffff_ffff);
}

const SECONDARY_ORDER = ["plain", "aspirated", "breathy", "labialized", "rounded"];

/// Design a language before founding it: pick its sounds on a chart, set a
/// few knobs, and watch the words it would have.
export function Designer({
  catalog,
  newWorld,
  onFound,
  onCancel,
}: {
  catalog: Catalog;
  newWorld: boolean;
  onFound: (founding: Founding) => void;
  onCancel?: () => void;
}) {
  const [worldSeed, setWorldSeed] = useState(() => randomSeed());
  const [seed, setSeed] = useState(() => randomSeed());
  const [consonants, setConsonants] = useState(18);
  const [vowels, setVowels] = useState(5);
  const [design, setDesign] = useState<LanguageDesign>(() => typicalDesign(seed, 18, 5));
  const [name, setName] = useState("");
  const [power, setPower] = useState(0.5);
  const [openness, setOpenness] = useState(0.5);
  const result = useMemo(() => preview(design, seed), [design, seed]);
  const set = (patch: Partial<LanguageDesign>) => setDesign((d) => ({ ...d, ...patch }));

  const state = (ipa: string) => design.sounds.find((s) => s.ipa === ipa);
  const cycle = (ipa: string) => {
    const current = state(ipa);
    const sounds = design.sounds.filter((s) => s.ipa !== ipa);
    if (!current) sounds.push({ ipa, favoured: false });
    else if (!current.favoured) sounds.push({ ipa, favoured: true });
    set({ sounds });
  };

  const chip = (sound: SoundInfo) => {
    const s = state(sound.ipa);
    const label = s ? (s.favoured ? "favoured" : "used") : "not used";
    return (
      <button
        key={sound.ipa}
        type="button"
        className={`chip ${s ? (s.favoured ? "favoured" : "used") : "off"}`}
        title={`${sound.ipa} · written ${sound.roman} · in ${Math.round(sound.share * 100)}% of languages · ${label}`}
        aria-pressed={Boolean(s)}
        onClick={() => cycle(sound.ipa)}
      >
        {sound.ipa}
      </button>
    );
  };

  const order = (a: SoundInfo, b: SoundInfo) =>
    SECONDARY_ORDER.indexOf(a.secondary) - SECONDARY_ORDER.indexOf(b.secondary) ||
    Number(a.voiced) - Number(b.voiced);
  const consonantSounds = catalog.sounds.filter((s) => !s.vowel);
  const vowelSounds = catalog.sounds.filter((s) => s.vowel);
  const places = catalog.places.filter((p) => consonantSounds.some((s) => s.column === p));
  const manners = catalog.manners.filter((m) => consonantSounds.some((s) => s.row === m));
  const heights = catalog.heights.filter((h) => vowelSounds.some((s) => s.row === h));
  const backs = ["front", "central", "back"];

  const knob = (label: string, value: number, update: (v: number) => void, low: string, high: string) => (
    <label className="knob">
      <span>{label}</span>
      <span className="knob-scale">
        <small>{low}</small>
        <input type="range" min={0} max={1} step={0.05} value={value} onChange={(e) => update(Number(e.target.value))} />
        <small>{high}</small>
      </span>
    </label>
  );

  const letters = (ipa: string) => design.spelling.overrides.find(([from]) => from === ipa)?.[1] ?? "";
  const setLetters = (ipa: string, to: string) => {
    const overrides = design.spelling.overrides.filter(([from]) => from !== ipa);
    if (to.trim() !== "") overrides.push([ipa, to.trim()]);
    set({ spelling: { ...design.spelling, overrides } });
  };

  const sounds = design.sounds.length;
  const chosenVowels = design.sounds.filter((s) => vowelSounds.some((v) => v.ipa === s.ipa)).length;

  return (
    <form
      className="designer"
      onSubmit={(e) => {
        e.preventDefault();
        if (typeof result === "string" || name.trim() === "") return;
        onFound({ name: name.trim(), design, seed, power, openness, worldSeed });
      }}
    >
      <div className="designer-controls">
        <section className="designer-row">
          <label>
            Start from
            <select
              defaultValue=""
              onChange={(e) => {
                if (e.target.value) setDesign(presetDesign(e.target.value, seed));
                e.target.value = "";
              }}
            >
              <option value="" disabled>
                Choose a starting point…
              </option>
              {catalog.presets.map((p) => (
                <option key={p.id} value={p.id} title={p.description}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            Consonants <output>{consonants}</output>
            <input type="range" min={6} max={30} value={consonants} onChange={(e) => setConsonants(Number(e.target.value))} />
          </label>
          <label>
            Vowels <output>{vowels}</output>
            <input type="range" min={2} max={10} value={vowels} onChange={(e) => setVowels(Number(e.target.value))} />
          </label>
          <button type="button" onClick={() => setDesign(typicalDesign(seed, consonants, vowels))}>
            Fill typical
          </button>
        </section>

        <Section title={`Consonants · ${sounds - chosenVowels} chosen`}>
          <p className="muted small">Click a sound: not used → used → favoured. Favoured sounds appear more often and sound changes lean toward them.</p>
          <div className="chart-wrap">
            <table className="chart">
              <thead>
                <tr>
                  <th />
                  {places.map((p) => (
                    <th key={p}>{p}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {manners.map((m) => (
                  <tr key={m}>
                    <th>{m}</th>
                    {places.map((p) => (
                      <td key={p}>
                        {consonantSounds
                          .filter((s) => s.row === m && s.column === p)
                          .sort(order)
                          .map(chip)}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Section>

        <Section title={`Vowels · ${chosenVowels} chosen`}>
          <table className="chart vowels">
            <thead>
              <tr>
                <th />
                {backs.map((b) => (
                  <th key={b}>{b}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {heights.map((h) => (
                <tr key={h}>
                  <th>{h}</th>
                  {backs.map((b) => (
                    <td key={b}>
                      {vowelSounds
                        .filter((s) => s.row === h && s.column === b)
                        .sort(order)
                        .map(chip)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </Section>

        <Section title="Shape">
          {knob("Word length", design.wordLength, (v) => set({ wordLength: v }), "short", "long")}
          {knob("Final consonants", design.finalConsonants, (v) => set({ finalConsonants: v }), "open", "closed")}
          {knob("Long vowels", design.longVowels, (v) => set({ longVowels: v }), "none", "many")}
          {knob("Repeated consonants", design.repetition, (v) => set({ repetition: v }), "avoided", "common")}
          <label className="check">
            <input type="checkbox" checked={design.innerClusters} onChange={(e) => set({ innerClusters: e.target.checked })} />
            Consonants may meet inside words (kasta, not only kasata)
          </label>
        </Section>

        <Section title="Word building">
          <div className="row">
            {(
              [
                ["concatenative", "Affixes"],
                ["root-pattern", "Root and pattern (k-t-b)"],
              ] as const
            ).map(([id, label]) => (
              <label key={id} className="check">
                <input type="radio" name="building" checked={design.building === id} onChange={() => set({ building: id })} />
                {label}
              </label>
            ))}
          </div>
          {design.building === "concatenative"
            ? knob("Affix position", design.suffixing, (v) => set({ suffixing: v }), "prefixes", "suffixes")
            : null}
          {knob("Word families", design.derivation, (v) => set({ derivation: v }), "few", "many")}
        </Section>

        <details className="section">
          <summary>Spelling</summary>
          <label>
            Long vowels written
            <select
              value={design.spelling.long_vowels}
              onChange={(e) => set({ spelling: { ...design.spelling, long_vowels: e.target.value as LongVowelStyle } })}
            >
              <option value="Macron">ā (macron)</option>
              <option value="Acute">á (acute)</option>
              <option value="Double">aa (doubled)</option>
              <option value="Unmarked">a (unmarked)</option>
            </select>
          </label>
          <div className="letters">
            {design.sounds.map((s) => {
              const info = catalog.sounds.find((c) => c.ipa === s.ipa);
              return (
                <label key={s.ipa}>
                  <span className="ipa">{s.ipa}</span>
                  <input value={letters(s.ipa)} placeholder={info?.roman ?? s.ipa} onChange={(e) => setLetters(s.ipa, e.target.value)} />
                </label>
              );
            })}
          </div>
        </details>

        <Section title="Community">
          <label>
            Name
            <input required value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Hill" />
          </label>
          {knob("Power", power, setPower, "weak", "strong")}
          {knob("Openness to foreign words", openness, setOpenness, "closed", "open")}
          {newWorld ? (
            <label>
              World seed
              <span className="row">
                <input type="number" min={0} max={4294967295} value={worldSeed} onChange={(e) => setWorldSeed(Number(e.target.value))} />
                <button type="button" onClick={() => setWorldSeed(randomSeed())}>
                  Random
                </button>
              </span>
              <small>Decides how history unfolds once time runs.</small>
            </label>
          ) : null}
        </Section>
      </div>

      <aside className="designer-preview">
        <div className="row spread">
          <h3>Preview</h3>
          <button type="button" onClick={() => setSeed(randomSeed())} title="Same design, a different draw of words">
            Reroll words
          </button>
        </div>
        {typeof result === "string" ? (
          <p className="error">{result}</p>
        ) : (
          <>
            <p className="muted small">
              {result.homophones} words share a form · {result.syllables.toFixed(1)} syllables per word · seed {seed}
            </p>
            <table className="preview">
              <tbody>
                {result.words.map((w) => (
                  <tr key={w.gloss}>
                    <td className="muted">{w.gloss}</td>
                    <td className="word">{w.spelled}</td>
                    <td className="ipa">/{w.ipa}/</td>
                  </tr>
                ))}
              </tbody>
            </table>
            {result.families.length > 0 ? (
              <>
                <h4>Word families</h4>
                <table className="preview">
                  <tbody>
                    {result.families.map((w) => (
                      <tr key={w.gloss}>
                        <td className="muted">{w.gloss}</td>
                        <td className="word">{w.spelled}</td>
                        <td className="muted small">from {w.from}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </>
            ) : null}
          </>
        )}
        <div className="row designer-actions">
          {onCancel ? (
            <button type="button" onClick={onCancel}>
              Cancel
            </button>
          ) : null}
          <button type="submit" className="primary" disabled={typeof result === "string" || name.trim() === ""}>
            {newWorld ? "Create world" : "Found community"}
          </button>
        </div>
      </aside>
    </form>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="section">
      <h3>{title}</h3>
      {children}
    </section>
  );
}
