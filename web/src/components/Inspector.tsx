import { useMemo, useState } from "react";
import type { Community, FormationOption, Lexeme, Sense, Snapshot, Variety } from "../model";

export function Inspector({
  snapshot,
  community,
  variety,
  lexeme,
  sense,
  options,
  readOnly,
  onSelectSense,
  onOpenLexeme,
  onSelectCommunity,
  onPreviewDerive,
  onCompose,
  onPreviewLexicalize,
}: {
  snapshot: Snapshot;
  community: Community | null;
  variety: Variety | null;
  lexeme: Lexeme | null;
  sense: Sense | null;
  options: FormationOption[];
  readOnly: boolean;
  onSelectSense: (id: number) => void;
  onOpenLexeme: (args: { variety: number; lexeme: string; sense?: number; checkpoint?: number }) => void;
  onSelectCommunity: (id: number) => void;
  onPreviewDerive: (option: FormationOption) => void;
  onCompose: (kind: "Sense" | "Replace" | "Remodel") => void;
  onPreviewLexicalize: () => void;
}) {
  const [tab, setTab] = useState<"word" | "sound">("word");

  const relatives = useMemo(() => {
    if (!variety || !lexeme) {
      return { base: null as Lexeme | null, children: [] as Lexeme[], siblings: [] as Lexeme[] };
    }
    const base = lexeme.analysis
      ? (variety.lexicon.find((item) => item.id === lexeme.analysis?.base) ?? null)
      : null;
    const children = variety.lexicon.filter((item) => item.analysis?.base === lexeme.id);
    const siblings = lexeme.analysis
      ? variety.lexicon.filter(
          (item) => item.analysis?.base === lexeme.analysis?.base && item.id !== lexeme.id,
        )
      : [];
    return { base, children, siblings };
  }, [variety, lexeme]);

  const speakers = variety
    ? snapshot.communities.filter((item) => item.uses.some((use) => use.variety === variety.id))
    : [];
  const lexicalClass =
    variety && lexeme ? (variety.classes.find((item) => item.id === lexeme.classId) ?? null) : null;
  const compatible =
    variety && lexeme
      ? variety.lexicon.filter(
          (item) => !item.retired && item.id !== lexeme.id && item.classId === lexeme.classId,
        )
      : [];
  const canAuthor = Boolean(variety && lexeme && !readOnly && !lexeme.retired);
  const frameExplain = !sense
    ? ""
    : "Entity" in sense.frame
      ? sense.frame.Entity.countable
        ? "Countable entity"
        : "Mass entity"
      : sense.frame.Event.roles.length > 0
        ? `Event with ${sense.frame.Event.roles.join(", ")}`
        : "Event";

  if (!community) {
    return (
      <aside className="inspector" aria-label="Inspector">
        <div className="empty-block">
          <p>No community at this checkpoint.</p>
          <p className="muted">Found a community to start a lexicon.</p>
        </div>
      </aside>
    );
  }

  if (!variety) {
    return (
      <aside className="inspector" aria-label="Inspector">
        <div className="empty-block">
          <p>{community.name} has no language use yet.</p>
          <p className="muted">Attach a language, or found one with this community.</p>
        </div>
      </aside>
    );
  }

  return (
    <aside className="inspector" aria-label="Inspector">
      <div className="inspector-tabs" role="tablist" aria-label="Inspector views">
        <button type="button" role="tab" aria-selected={tab === "word"} onClick={() => setTab("word")}>
          Word
        </button>
        <button type="button" role="tab" aria-selected={tab === "sound"} onClick={() => setTab("sound")}>
          Sound
        </button>
      </div>

      {tab === "sound" ? (
        <div className="inspector-scroll" role="tabpanel">
          <h3 className="inspect-kicker">{variety.name}</h3>
          <p className="lead">
            {variety.aesthetic.name}. {variety.aesthetic.description}
          </p>
          <p className="muted">Stress {variety.stress}.</p>
          <h4>Spoken by</h4>
          {speakers.length === 0 ? (
            <p className="muted">No community uses this language yet.</p>
          ) : (
            <ul className="relatives">
              {speakers.map((item) => (
                <li key={item.id}>
                  {item.id === community.id ? (
                    <strong>{item.name}</strong>
                  ) : (
                    <button type="button" className="text-link" onClick={() => onSelectCommunity(item.id)}>
                      {item.name}
                    </button>
                  )}
                  <span className="muted">
                    {" "}
                    {item.uses
                      .filter((use) => use.variety === variety.id)
                      .map((use) => use.domain)
                      .join(", ")}
                  </span>
                </li>
              ))}
            </ul>
          )}
          <h4>Inventory</h4>
          <p className="ipa-line">
            <span className="muted">C</span> {variety.inventory.consonants.join(" ") || "—"}
          </p>
          <p className="ipa-line">
            <span className="muted">V</span> {variety.inventory.vowels.join(" ") || "—"}
          </p>
          <h4>Lexical classes</h4>
          {variety.classes.length === 0 ? (
            <p className="muted">No classes at this checkpoint.</p>
          ) : (
            <ul className="suffix-list">
              {variety.classes.map((item) => (
                <li key={item.id}>
                  <strong>{item.label}</strong>
                  <span className="muted">{item.kind}</span>
                </li>
              ))}
            </ul>
          )}
          <h4>Constructions</h4>
          {variety.constructions.length === 0 ? (
            <p className="muted">No constructions at this checkpoint.</p>
          ) : (
            <ul className="suffix-list">
              {variety.constructions.map((rule) => (
                <li key={rule.id}>
                  <strong>{rule.label}</strong>
                  <span className="muted">
                    {rule.inputClass} → {rule.outputClass} · {rule.operation}
                  </span>
                  <span className="form">{rule.exponents.join(" ") || "—"}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : lexeme ? (
        <div className="inspector-scroll" role="tabpanel">
          <p className="inspect-kicker">
            {sense?.gloss ?? "No sense"}
            {lexeme.retired ? " · archived" : ""}
          </p>
          <p className="display-form">{lexeme.form}</p>
          <p className="display-ipa">/{lexeme.ipa}/</p>
          <p className="muted">
            {lexeme.classLabel}
            {lexicalClass ? ` · ${lexicalClass.kind}` : ""}
          </p>

          <h4>Senses</h4>
          {lexeme.senses.length === 0 ? (
            <p className="muted">This word has no senses.</p>
          ) : (
            <div className="filters senses" role="radiogroup" aria-label="Selected sense">
              {lexeme.senses.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  className={item.id === sense?.id ? "chip selected" : "chip"}
                  aria-pressed={item.id === sense?.id}
                  onClick={() => onSelectSense(item.id)}
                >
                  {item.gloss}
                </button>
              ))}
            </div>
          )}
          {sense ? <p className="frame-explain">{frameExplain}</p> : null}

          <OriginBlock
            snapshot={snapshot}
            community={community}
            variety={variety}
            lexeme={lexeme}
            onOpenLexeme={onOpenLexeme}
          />

          <h4>Relatives</h4>
          {relatives.base || relatives.children.length || relatives.siblings.length ? (
            <ul className="relatives">
              {relatives.base && lexeme.analysis ? (
                <li>
                  <button
                    type="button"
                    className="text-link"
                    onClick={() =>
                      onOpenLexeme({
                        variety: variety.id,
                        lexeme: relatives.base!.id,
                        sense: lexeme.analysis?.sense,
                      })
                    }
                  >
                    Base {relatives.base.form}
                    <span className="muted">
                      {" "}
                      {relatives.base.senses.find((item) => item.id === lexeme.analysis?.sense)?.gloss ??
                        relatives.base.senses[0]?.gloss ??
                        relatives.base.id}
                    </span>
                  </button>
                </li>
              ) : null}
              {relatives.siblings.map((item) => (
                <li key={item.id}>
                  <button
                    type="button"
                    className="text-link"
                    onClick={() => onOpenLexeme({ variety: variety.id, lexeme: item.id })}
                  >
                    {item.analysis?.label ?? "Related"} {item.form}
                    <span className="muted"> {item.senses[0]?.gloss ?? item.id}</span>
                    {item.retired ? <span className="muted"> · archived</span> : null}
                  </button>
                </li>
              ))}
              {relatives.children.map((item) => (
                <li key={item.id}>
                  <button
                    type="button"
                    className="text-link"
                    onClick={() => onOpenLexeme({ variety: variety.id, lexeme: item.id })}
                  >
                    {item.analysis?.label ?? "Derived"} {item.form}
                    <span className="muted"> {item.senses[0]?.gloss ?? item.id}</span>
                    {item.retired ? <span className="muted"> · archived</span> : null}
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted">No current analysis relatives.</p>
          )}

          <h4>Formations</h4>
          {!sense ? (
            <p className="muted">Select a sense to see eligible constructions.</p>
          ) : options.length === 0 ? (
            <p className="muted">No formation options for this sense.</p>
          ) : (
            <ul className="formation-cards">
              {options.map((option) =>
                option.existing ? (
                  <li key={`${option.construction}-${option.existing}`}>
                    <button
                      type="button"
                      className="formation-card existing"
                      onClick={() => onOpenLexeme({ variety: variety.id, lexeme: option.existing as string })}
                    >
                      <span className="formation-kind">{option.label}</span>
                      <span className="formation-gloss">{option.gloss}</span>
                      <span className="formation-shape">
                        <em>{option.form}</em>
                      </span>
                      <span className="ipa">/{option.ipa}/</span>
                      <span className="formation-in-lexicon">In lexicon · View</span>
                    </button>
                  </li>
                ) : (
                  <li key={option.construction}>
                    <button
                      type="button"
                      className="formation-card"
                      disabled={readOnly || lexeme.retired}
                      onClick={() => onPreviewDerive(option)}
                    >
                      <span className="formation-kind">{option.label}</span>
                      <span className="formation-gloss">{option.gloss}</span>
                      <span className="formation-shape">
                        <em>{option.form}</em>
                      </span>
                      <span className="muted">Base {lexeme.form}; marker {option.exponent || "∅"}</span>
                      <span className="ipa">/{option.ipa}/</span>
                      {readOnly ? <span className="muted">Return to latest to form words</span> : null}
                    </button>
                  </li>
                ),
              )}
            </ul>
          )}

          {canAuthor ? (
            <div className="authoring" role="group" aria-label="Lexicon events">
              <button type="button" className="btn" onClick={() => onCompose("Sense")}>
                Change sense
              </button>
              {compatible.length > 0 ? (
                <button type="button" className="btn" onClick={() => onCompose("Replace")}>
                  Replace
                </button>
              ) : null}
              {lexeme.analysis ? (
                <button type="button" className="btn" onClick={onPreviewLexicalize}>
                  Lexicalize
                </button>
              ) : null}
              <button type="button" className="btn" onClick={() => onCompose("Remodel")}>
                Remodel
              </button>
            </div>
          ) : lexeme.retired ? (
            <p className="muted">Archived words stay inspectable but cannot be used in new events.</p>
          ) : null}

          <h4>History of this word</h4>
          {lexeme.traces.length === 0 ? (
            <p className="muted">No traces recorded.</p>
          ) : (
            <ol className="traces">
              {lexeme.traces.map((trace, index) => (
                <li
                  key={`${trace.checkpoint}-${index}`}
                  className={trace.checkpoint === snapshot.checkpoint ? "current" : undefined}
                >
                  <span className="trace-head">
                    ck{trace.checkpoint}{" "}
                    {snapshot.varieties.find((item) => item.id === trace.variety)?.name ??
                      `Variety ${trace.variety}`}{" "}
                    · {trace.before ?? "—"} → {trace.after}
                  </span>
                  <span className="muted">{trace.explanation}</span>
                </li>
              ))}
            </ol>
          )}
        </div>
      ) : (
        <div className="inspector-scroll empty-block" role="tabpanel">
          <p>Select a word to inspect its senses, analysis, and origin.</p>
        </div>
      )}
    </aside>
  );
}

function OriginBlock({
  snapshot,
  community,
  variety,
  lexeme,
  onOpenLexeme,
}: {
  snapshot: Snapshot;
  community: Community;
  variety: Variety;
  lexeme: Lexeme;
  onOpenLexeme: (args: { variety: number; lexeme: string; sense?: number; checkpoint?: number }) => void;
}) {
  const chain: Variety[] = [];
  let cursor: Variety | undefined = variety;
  const guard = new Set<number>();
  while (cursor && !guard.has(cursor.id)) {
    guard.add(cursor.id);
    chain.push(cursor);
    cursor = cursor.parent == null ? undefined : snapshot.varieties.find((item) => item.id === cursor!.parent);
  }
  chain.reverse();

  const analyzed = lexeme.analysis
    ? (variety.lexicon.find((item) => item.id === lexeme.analysis?.base) ?? null)
    : null;
  const sourceRef = lexeme.origin.source;
  const sourceVariety = sourceRef
    ? (snapshot.varieties.find((item) => item.id === sourceRef.variety) ?? null)
    : null;

  return (
    <section className="lineage" aria-label="Analysis and origin">
      <h4>Current analysis</h4>
      {lexeme.analysis ? (
        <p>
          {lexeme.analysis.label}
          {analyzed ? (
            <>
              {" · "}
              <button
                type="button"
                className="text-link"
                onClick={() =>
                  onOpenLexeme({
                    variety: variety.id,
                    lexeme: lexeme.analysis!.base,
                    sense: lexeme.analysis!.sense,
                  })
                }
              >
                {analyzed.form}
              </button>
              <span className="muted">
                {" "}
                {analyzed.senses.find((item) => item.id === lexeme.analysis?.sense)?.gloss ??
                  analyzed.senses[0]?.gloss ??
                  analyzed.id}
              </span>
            </>
          ) : (
            <span className="muted"> · {lexeme.analysis.base}</span>
          )}
        </p>
      ) : (
        <p className="muted">No current analysis.</p>
      )}

      <h4>Historical origin</h4>
      <p>{lexeme.origin.label}</p>
      <p className="muted">
        {lexeme.origin.kind} · ck{lexeme.origin.checkpoint}
      </p>
      {sourceRef ? (
        <p>
          <button
            type="button"
            className="text-link"
            onClick={() =>
              onOpenLexeme({
                variety: sourceRef.variety,
                lexeme: sourceRef.lexeme,
                sense: lexeme.origin.sense ?? undefined,
                checkpoint: sourceRef.checkpoint,
              })
            }
          >
            Open source at ck{sourceRef.checkpoint} · {sourceVariety?.name ?? `Variety ${sourceRef.variety}`}
          </button>
        </p>
      ) : null}

      <h4>Language lineage</h4>
      <p>
        {chain.map((item, index) => (
          <span key={item.id}>
            {index > 0 ? <span className="muted"> → </span> : null}
            {item.id === variety.id ? (
              <strong>{item.name}</strong>
            ) : (
              <button
                type="button"
                className="text-link"
                disabled={
                  !item.lexicon.length ||
                  !snapshot.communities.some((host) => host.uses.some((use) => use.variety === item.id))
                }
                onClick={() =>
                  onOpenLexeme({
                    variety: item.id,
                    lexeme: item.lexicon.find((word) => !word.retired)?.id ?? item.lexicon[0]?.id ?? "",
                  })
                }
              >
                {item.name}
              </button>
            )}
          </span>
        ))}
      </p>
      <p className="muted">
        Spoken here by {community.name}
        {community.group ? ` · ${community.group}` : ""}
        {community.location ? ` · ${community.location}` : ""}
      </p>
    </section>
  );
}
