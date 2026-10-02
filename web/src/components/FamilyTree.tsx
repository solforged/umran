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

/// Lineages as a chart of descent over time: one row each, a bar from when
/// it arose to now or to its end, and a join from its parent's bar at the
/// year they parted. Daughters sit under their parents, eldest first.
export function DescentChart({
  lineages,
  now,
  chosen,
  what,
  onOpen,
}: {
  lineages: Lineage[];
  now: number;
  chosen: number;
  /// What the rows are, for the chart's label: "languages", "faiths".
  what: string;
  onOpen: (id: number) => void;
}) {
  const members = lineages.filter((l) => l.born <= now);
  const eldestFirst = (list: Lineage[]) => [...list].sort((a, b) => a.born - b.born);
  const rows: Lineage[] = [];
  const visit = (l: Lineage) => {
    rows.push(l);
    for (const d of eldestFirst(members.filter((m) => m.parent === l.id))) visit(d);
  };
  for (const root of eldestFirst(members.filter((l) => !members.some((m) => m.id === l.parent)))) visit(root);
  if (rows.length < 2) return null;

  const start = Math.min(...rows.map((l) => l.born));
  const span = Math.max(now - start, 1);
  const x = (g: number) => LABEL + 6 + ((g - start) / span) * (WIDTH - LABEL - 6 - RIGHT);
  const y = (i: number) => ROW * i + ROW / 2;
  const row = new Map(rows.map((l, i) => [l.id, i]));
  const height = ROW * rows.length + AXIS;

  return (
    <svg
      className="family-tree"
      viewBox={`0 0 ${WIDTH} ${height}`}
      role="img"
      aria-label={`${rows.length} ${what}, from year ${start * YEARS} to year ${now * YEARS}`}
    >
      {rows.map((l, i) => {
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
      })}
      <g className="axis">
        <text x={x(start)} y={height - 3}>
          {start * YEARS}
        </text>
        <text x={x(now)} y={height - 3} textAnchor="end">
          year {now * YEARS}
        </text>
      </g>
    </svg>
  );
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
  const tone = hue(family);
  const lineages = overview.varieties
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
  return <DescentChart lineages={lineages} now={overview.generation} chosen={chosen} what="languages" onOpen={onOpen} />;
}
