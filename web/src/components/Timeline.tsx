import type { Overview } from "../model";

/// The scrubber: every generation since the founding, with markers where
/// someone acted or the world changed on its own.
export function Timeline({
  overview,
  generation,
  onScrub,
}: {
  overview: Overview;
  generation: number;
  onScrub: (generation: number) => void;
}) {
  const { latest } = overview;
  const span = Math.max(latest, 1);
  const markers = overview.timeline.filter((m) => m.kind !== "run");
  const here = overview.timeline.filter((m) => m.generation === generation && m.kind !== "run");
  return (
    <section className="timeline" aria-label="Timeline">
      <div className="track">
        <input
          type="range"
          min={0}
          max={latest}
          value={generation}
          disabled={latest === 0}
          aria-label="Generation"
          aria-valuetext={`Generation ${generation} of ${latest}`}
          onChange={(e) => onScrub(Number(e.target.value))}
        />
        <div className="ticks" aria-hidden="true">
          {markers.map((m, i) => (
            <button
              key={i}
              type="button"
              tabIndex={-1}
              className={`tick tick-${m.kind}`}
              style={{ left: `${(m.generation / span) * 100}%` }}
              title={`Generation ${m.generation}: ${m.label}`}
              onClick={() => onScrub(m.generation)}
            />
          ))}
        </div>
      </div>
      <div className="timeline-foot">
        <span>
          {here.length > 0 ? here.map((m) => m.label).join(" · ") : `Generation ${generation} of ${latest}`}
        </span>
        {generation < latest ? (
          <button type="button" className="link" onClick={() => onScrub(latest)}>
            Back to the present
          </button>
        ) : null}
      </div>
    </section>
  );
}
