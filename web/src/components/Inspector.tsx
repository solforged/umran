import { useMemo, useState } from "react";
import type { Community, FormationOption, Lexeme, Snapshot } from "../model";

export function Inspector({
  snapshot,
  community,
  lexeme,
  options,
  readOnly,
  onSelectLexeme,
  onSelectCommunity,
  onPreviewDerive,
}: {
  snapshot: Snapshot;
  community: Community | null;
  lexeme: Lexeme | null;
  options: FormationOption[];
  readOnly: boolean;
  onSelectLexeme: (id: string) => void;
  onSelectCommunity: (id: number) => void;
  onPreviewDerive: (option: FormationOption) => void;
}) {
  const [tab, setTab] = useState<"word" | "sound">("word");

  const relatives = useMemo(() => {
    if (!community || !lexeme) {
      return { base: null as Lexeme | null, children: [] as Lexeme[], siblings: [] as Lexeme[] };
    }
    const base = lexeme.baseLexeme ? (community.lexicon.find((item) => item.id === lexeme.baseLexeme) ?? null) : null;
    const children = community.lexicon.filter((item) => item.baseLexeme === lexeme.id);
    const siblings = lexeme.baseLexeme
      ? community.lexicon.filter(
          (item) => item.baseLexeme === lexeme.baseLexeme && item.id !== lexeme.id,
        )
      : [];
    return { base, children, siblings };
  }, [community, lexeme]);

  if (!community) {
    return (
      <aside className="inspector" aria-label="Inspector">
        <div className="empty-block">
          <p>No community at this checkpoint.</p>
          <p className="muted">Found a speech community to mint a lexicon.</p>
        </div>
      </aside>
    );
  }

  return (
    <aside className="inspector" aria-label="Inspector">
      <div className="inspector-tabs" role="tablist" aria-label="Inspector views">
        <button
          type="button"
          role="tab"
          aria-selected={tab === "word"}
          onClick={() => setTab("word")}
        >
          Word
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === "sound"}
          onClick={() => setTab("sound")}
        >
          Sound
        </button>
      </div>

      {tab === "sound" ? (
        <div className="inspector-scroll" role="tabpanel">
          <h3 className="inspect-kicker">{community.name}</h3>
          <p className="lead">
            {community.aesthetic.name}. {community.aesthetic.description}
          </p>
          <h4>Inventory</h4>
          <p className="ipa-line">
            <span className="muted">C</span> {community.inventory.consonants.join(" ") || "—"}
          </p>
          <p className="ipa-line">
            <span className="muted">V</span> {community.inventory.vowels.join(" ") || "—"}
          </p>
          <h4>Productive suffixes</h4>
          {community.formations.length === 0 ? (
            <p className="muted">No productive suffixes at this checkpoint.</p>
          ) : (
            <ul className="suffix-list">
              {community.formations.map((rule) => (
                <li key={rule.kind}>
                  <strong>{rule.label}</strong>
                  <span className="form">{rule.form}</span>
                  <span className="ipa">/{rule.ipa}/</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : lexeme ? (
        <div className="inspector-scroll" role="tabpanel">
          <p className="inspect-kicker">{lexeme.gloss}</p>
          <p className="display-form">{lexeme.form}</p>
          <p className="display-ipa">/{lexeme.ipa}/</p>

          <Lineage snapshot={snapshot} community={community} lexeme={lexeme} onSelectCommunity={onSelectCommunity} />

          <h4>Relatives</h4>
          {relatives.base || relatives.children.length || relatives.siblings.length ? (
            <ul className="relatives">
              {relatives.base ? (
                <li>
                  <button type="button" className="text-link" onClick={() => onSelectLexeme(relatives.base!.id)}>
                    Base {relatives.base.form}
                    <span className="muted"> {relatives.base.gloss}</span>
                  </button>
                </li>
              ) : null}
              {relatives.siblings.map((item) => (
                <li key={item.id}>
                  <button type="button" className="text-link" onClick={() => onSelectLexeme(item.id)}>
                    {item.formation ?? "Related"} {item.form}
                    <span className="muted"> {item.gloss}</span>
                  </button>
                </li>
              ))}
              {relatives.children.map((item) => (
                <li key={item.id}>
                  <button type="button" className="text-link" onClick={() => onSelectLexeme(item.id)}>
                    {item.formation ?? "Derived"} {item.form}
                    <span className="muted"> {item.gloss}</span>
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted">No linked relatives yet.</p>
          )}

          <h4>Formations</h4>
          {options.length === 0 ? (
            <p className="muted">No formation options for this word.</p>
          ) : (
            <ul className="formation-cards">
              {options.map((option) =>
                option.existing ? (
                  <li key={option.kind}>
                    <button
                      type="button"
                      className="formation-card existing"
                      onClick={() => {
                        if (option.existing) onSelectLexeme(option.existing);
                      }}
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
                  <li key={option.kind}>
                    <button
                      type="button"
                      className="formation-card"
                      disabled={readOnly}
                      onClick={() => onPreviewDerive(option)}
                    >
                      <span className="formation-kind">{option.label}</span>
                      <span className="formation-gloss">{option.gloss}</span>
                      <span className="formation-shape">
                        {lexeme.form} + {option.exponent} → <em>{option.form}</em>
                      </span>
                      <span className="ipa">/{option.ipa}/</span>
                      {readOnly ? <span className="muted">Return to latest to form words</span> : null}
                    </button>
                  </li>
                ),
              )}
            </ul>
          )}

          <h4>History of this word</h4>
          {lexeme.traces.length === 0 ? (
            <p className="muted">No traces recorded.</p>
          ) : (
            <ol className="traces">
              {lexeme.traces.map((trace, index) => (
                <li key={`${trace.checkpoint}-${index}`} className={trace.checkpoint === snapshot.checkpoint ? "current" : undefined}>
                  <span className="trace-head">
                    ck{trace.checkpoint} {snapshot.communities.find((item) => item.id === trace.community)?.name ?? `Community ${trace.community}`} · {trace.before ?? "—"} → {trace.after}
                  </span>
                  <span className="muted">{trace.explanation}</span>
                </li>
              ))}
            </ol>
          )}
        </div>
      ) : (
        <div className="inspector-scroll empty-block" role="tabpanel">
          <p>Select a word to inspect its form, relatives, and formations.</p>
        </div>
      )}
    </aside>
  );
}

function Lineage({
  snapshot,
  community,
  lexeme,
  onSelectCommunity,
}: {
  snapshot: Snapshot;
  community: Community;
  lexeme: Lexeme;
  onSelectCommunity: (id: number) => void;
}) {
  const chain: Community[] = [];
  let cursor: Community | undefined = community;
  const guard = new Set<number>();
  while (cursor && !guard.has(cursor.id)) {
    guard.add(cursor.id);
    chain.push(cursor);
    cursor = cursor.parent == null ? undefined : snapshot.communities.find((item) => item.id === cursor!.parent);
  }
  chain.reverse();
  const source = lexeme.originCommunity !== community.id
    ? snapshot.communities.find((item) => item.id === lexeme.originCommunity)
    : null;
  const sourceWord = source?.lexicon.find((item) => item.id === lexeme.originLexeme);

  return (
    <section className="lineage" aria-label="Source and community lineage">
      <h4>Lineage</h4>
      <p>
        {chain.map((item, index) => (
          <span key={item.id}>
            {index > 0 ? <span className="muted"> → </span> : null}
            {item.id === community.id ? (
              <strong>{item.name}</strong>
            ) : (
              <button type="button" className="text-link" onClick={() => onSelectCommunity(item.id)}>
                {item.name}
              </button>
            )}
          </span>
        ))}
      </p>
      {lexeme.originCommunity !== community.id ? (
        <p className="muted">
          Ultimate source {source?.name ?? `Community ${lexeme.originCommunity}`}
          {sourceWord ? ` · ${sourceWord.form} (${sourceWord.gloss})` : ""}
        </p>
      ) : null}
    </section>
  );
}
