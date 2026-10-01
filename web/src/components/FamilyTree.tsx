import type { Overview, Variety } from "../model";
import { YEARS } from "../model";
import { hue } from "../lore";

/// Room for the names, and the height of one language's row, in the
/// chart's own units.
const LABEL = 112;
const ROW = 20;
const WIDTH = 320;
const RIGHT = 8;
const AXIS = 16;

/// A language family as a chart of descent over time: one row for each
/// language, a bar from when it arose to now or to when its last speakers
/// left it, and a join from its parent's bar at the year they parted.
/// Daughters sit under their parents, eldest first.
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
  const members = overview.varieties.filter((v) => v.family === family && v.born <= overview.generation);
  const eldestFirst = (list: Variety[]) => [...list].sort((a, b) => a.born - b.born);
  const rows: Variety[] = [];
  const visit = (v: Variety) => {
    rows.push(v);
    for (const d of eldestFirst(members.filter((m) => m.parent === v.id))) visit(d);
  };
  for (const root of eldestFirst(members.filter((v) => !members.some((m) => m.id === v.parent)))) visit(root);
  if (rows.length < 2) return null;

  const now = overview.generation;
  const start = Math.min(...rows.map((v) => v.born));
  const span = Math.max(now - start, 1);
  const x = (g: number) => LABEL + 6 + ((g - start) / span) * (WIDTH - LABEL - 6 - RIGHT);
  const y = (i: number) => ROW * i + ROW / 2;
  const row = new Map(rows.map((v, i) => [v.id, i]));
  const height = ROW * rows.length + AXIS;
  const tone = hue(family);

  return (
    <svg
      className="family-tree"
      viewBox={`0 0 ${WIDTH} ${height}`}
      role="img"
      aria-label={`The family's ${rows.length} languages, from year ${start * YEARS} to year ${now * YEARS}`}
    >
      {rows.map((v, i) => {
        const end = v.silentSince ?? now;
        const parent = v.parent === null ? undefined : row.get(v.parent);
        const label = `${v.name}: arose in year ${v.born * YEARS}${
          v.silentSince !== null ? `, silent since year ${v.silentSince * YEARS}` : ", spoken now"
        }`;
        return (
          <g key={v.id} className={`${v.spoken ? "spoken" : "silent"}${v.id === chosen ? " chosen" : ""}`}>
            {parent !== undefined ? (
              <line className="join" x1={x(v.born)} y1={y(parent)} x2={x(v.born)} y2={y(i)} stroke={tone} />
            ) : null}
            <line className="bar" x1={x(v.born)} y1={y(i)} x2={Math.max(x(end), x(v.born) + 2)} y2={y(i)} stroke={tone} />
            <circle cx={x(v.born)} cy={y(i)} r={2.5} fill={tone} />
            <text
              x={LABEL}
              y={y(i)}
              role="button"
              tabIndex={0}
              aria-label={label}
              onClick={() => onOpen(v.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onOpen(v.id);
                }
              }}
            >
              <title>{label}</title>
              {v.name}
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
