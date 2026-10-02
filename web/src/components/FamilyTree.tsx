import type { Overview, Variety } from "../model";
import { YEARS } from "../model";
import { hue } from "../lore";

/// Room for the names, and the height of one row, in the chart's own units.
const LABEL = 112;
const ROW = 20;
const WIDTH = 320;
const RIGHT = 8;
const AXIS = 16;

/// One line of descent: a language in its family, or a faith among the
/// branches that broke from it.
export interface Lineage {
  id: number;
  name: string;
  parent: number | null;
  /// The generation it arose.
  born: number;
  /// When it fell silent or lost its last followers, if it has.
  ended: number | null;
  tone: string;
  /// How it reads to a screen reader and in its tooltip.
  label: string;
}

/// Members born by now, with roots and daughters eldest first.
export function orderRows(lineages: Lineage[], now: number): Lineage[] {
  const members = lineages.filter((l) => l.born <= now);
  const eldestFirst = (list: Lineage[]) => [...list].sort((a, b) => a.born - b.born);
  const rows: Lineage[] = [];
  const visit = (l: Lineage) => {
    rows.push(l);
    for (const d of eldestFirst(members.filter((m) => m.parent === l.id))) visit(d);
  };
  for (const root of eldestFirst(members.filter((l) => !members.some((m) => m.id === l.parent)))) visit(root);
  return rows;
}

/// Regular years far enough apart to read, followed by the present year.
export function axisTicks(start: number, now: number): number[] {
  const span = Math.max(now - start, 1) * YEARS;
  const plot = WIDTH - LABEL - 6 - RIGHT;
  const steps = [100, 200, 500, 1000, 2000, 5000];
  const step = steps.find((s) => (s / span) * plot >= 40) ?? steps[steps.length - 1];
  const end = now * YEARS;
  const ticks: number[] = [];
  for (let year = Math.ceil((start * YEARS) / step) * step; year <= end; year += step) ticks.push(year);
  // The present is always labelled; a regular tick crowding it gives way.
  const last = ticks[ticks.length - 1];
  if (last !== undefined && last !== end && ((end - last) / span) * plot < 40) ticks.pop();
  if (ticks[ticks.length - 1] !== end) ticks.push(end);
  return ticks;
}

function timeScale(start: number, now: number) {
  const span = Math.max(now - start, 1);
  return (g: number) => LABEL + 6 + ((g - start) / span) * (WIDTH - LABEL - 6 - RIGHT);
}

function Rows({
  rows,
  now,
  chosen,
  x,
  onOpen,
}: {
  rows: Lineage[];
  now: number;
  chosen?: number;
  x: (generation: number) => number;
  onOpen: (id: number) => void;
}) {
  const y = (i: number) => ROW * i + ROW / 2;
  const row = new Map(rows.map((l, i) => [l.id, i]));
  return rows.map((l, i) => {
    const end = l.ended ?? now;
    const parent = l.parent === null ? undefined : row.get(l.parent);
    return (
      <g key={l.id} className={`${l.ended === null ? "spoken" : "silent"}${l.id === chosen ? " chosen" : ""}`}>
        {parent !== undefined ? (
          <line className="join" x1={x(l.born)} y1={y(parent)} x2={x(l.born)} y2={y(i)} stroke={l.tone} />
        ) : null}
        <line className="bar" x1={x(l.born)} y1={y(i)} x2={Math.max(x(end), x(l.born) + 2)} y2={y(i)} stroke={l.tone} />
        <circle cx={x(l.born)} cy={y(i)} r={2.5} fill={l.tone} />
        <text
          x={LABEL}
          y={y(i)}
          role="button"
          tabIndex={0}
          aria-label={l.label}
          onClick={() => onOpen(l.id)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              onOpen(l.id);
            }
          }}
        >
          <title>{l.label}</title>
          {l.name}
        </text>
      </g>
    );
  });
}

function Axis({ start, now, height }: { start: number; now: number; height: number }) {
  const x = timeScale(start, now);
  const ticks = axisTicks(start, now);
  const baseline = height - AXIS + 2;
  return (
    <g className="axis">
      <line x1={LABEL + 6} y1={baseline} x2={WIDTH - RIGHT} y2={baseline} />
      {ticks.map((year, i) => {
        const position = x(year / YEARS);
        const last = i === ticks.length - 1;
        const firstOverflows = i === 0 && position - String(year).length * 2.5 < LABEL + 6;
        return (
          <g key={year}>
            <line x1={position} y1={baseline} x2={position} y2={baseline + 3} />
            <text x={position} y={height - 3} textAnchor={last ? "end" : firstOverflows ? "start" : "middle"}>
              {last ? `year ${year}` : year}
            </text>
          </g>
        );
      })}
    </g>
  );
}

/// Lineages as a chart of descent over time: one row each, a bar from when
/// it arose to now or to its end, and a join from its parent's bar at the
/// year they parted. Daughters sit under their parents, eldest first.
export function DescentChart({
  lineages,
  now,
  start: sharedStart,
  chosen,
  what,
  onOpen,
}: {
  lineages: Lineage[];
  now: number;
  start?: number;
  chosen: number;
  /// What the rows are, for the chart's label: "languages", "faiths".
  what: string;
  onOpen: (id: number) => void;
}) {
  const rows = orderRows(lineages, now);
  if (rows.length < 2) return null;
  const start = sharedStart ?? Math.min(...rows.map((l) => l.born));
  const height = ROW * rows.length + AXIS;
  return (
    <svg
      className="family-tree"
      viewBox={`0 0 ${WIDTH} ${height}`}
      role="img"
      aria-label={`${rows.length} ${what}, from year ${start * YEARS} to year ${now * YEARS}`}
    >
      <Rows rows={rows} now={now} chosen={chosen} x={timeScale(start, now)} onOpen={onOpen} />
      <Axis start={start} now={now} height={height} />
    </svg>
  );
}

function familyLineages(overview: Overview, family: number): Lineage[] {
  const tone = hue(family);
  return overview.varieties
    .filter((v) => v.family === family)
    .map((v: Variety): Lineage => ({
      id: v.id,
      name: v.name,
      parent: v.parent,
      born: v.born,
      ended: v.silentSince,
      tone,
      label: `${v.name}: arose in year ${v.born * YEARS}${
        v.silentSince !== null ? `, silent since year ${v.silentSince * YEARS}` : ", spoken now"
      }`,
    }));
}

/// A language family as a chart of descent.
export function FamilyTree({
  overview,
  family,
  chosen,
  onOpen,
}: {
  overview: Overview;
  family: number;
  chosen: number;
  onOpen: (variety: number) => void;
}) {
  return <DescentChart lineages={familyLineages(overview, family)} now={overview.generation} chosen={chosen} what="languages" onOpen={onOpen} />;
}

/// Families sharing a time axis, ordered by their earliest birth.
export function FamilyForest({
  overview,
  families,
  onOpen,
}: {
  overview: Overview;
  families: number[];
  onOpen: (variety: number) => void;
}) {
  const now = overview.generation;
  const groups = families
    .map((family) => ({ family, rows: orderRows(familyLineages(overview, family), now) }))
    .filter(({ rows }) => rows.length >= 2)
    .map((group) => ({ ...group, start: Math.min(...group.rows.map((l) => l.born)) }))
    .sort((a, b) => a.start - b.start);
  if (groups.length === 0) return null;
  const start = groups[0].start;
  const count = groups.reduce((total, { rows }) => total + rows.length, 0);
  const height = count * ROW + (groups.length - 1) * ROW / 2 + AXIS;
  const x = timeScale(start, now);
  let offset = 0;
  return (
    <svg
      className="family-tree family-forest"
      viewBox={`0 0 ${WIDTH} ${height}`}
      role="img"
      aria-label={`${groups.length} families, ${count} languages, from year ${start * YEARS} to year ${now * YEARS}`}
    >
      {groups.map(({ family, rows }) => {
        const top = offset;
        offset += rows.length * ROW + ROW / 2;
        return (
          <g key={family} className="family" transform={`translate(0 ${top})`}>
            <Rows rows={rows} now={now} x={x} onOpen={onOpen} />
          </g>
        );
      })}
      <Axis start={start} now={now} height={height} />
    </svg>
  );
}
