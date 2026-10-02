import type { FoundingPreview, Overview, WorldMap } from "../model";
import type { MapCamera } from "./MapView";

/// These strokes show possible encounters, not a road or a measured route.
/// Eligibility and effort come only from the engine's founding preview.
export function ReachOverlay({ map, overview, preview, people, camera }: {
  map: WorldMap;
  overview: Overview;
  preview: FoundingPreview;
  people: number;
  camera?: MapCamera;
}) {
  const founder = overview.communities.find((community) => community.id === people);
  if (!founder) return null;
  const homeland = map.regions[founder.region];
  const encounters = preview.pairs.filter((pair) => (pair.a === people || pair.b === people) && pair.reach !== "apart");
  return <svg className="reach-overlay" viewBox={(camera ?? [0, 0, map.width, map.height]).join(" ")}
    role="img" aria-label={`Possible first encounters of the ${founder.name}`}>
    <polygon className="reach-homeland" points={homeland.outline.map((point) => point.join(",")).join(" ")} />
    {encounters.map((pair) => {
      const other = overview.communities.find((community) => community.id === (pair.a === people ? pair.b : pair.a));
      if (!other) return null;
      const destination = map.regions[other.region];
      const [x, y] = homeland.site;
      const [endX, endY] = destination.site;
      const dx = endX - x, dy = endY - y;
      const distance = Math.hypot(dx, dy);
      let stroke = `M ${x} ${y} L ${endX} ${endY}`;
      if (distance === 0) {
        stroke = `M ${x} ${y} c -.3 -.4 .3 -.4 0 0`;
      } else if (pair.reach === "sea") {
        // Wave-dashes distinguish a voyage without implying an engine road.
        stroke = `M ${x} ${y}`;
        const waves = 12;
        for (let i = 0; i < waves; i++) {
          const middle = (i + 0.5) / waves;
          const bend = (i % 2 === 0 ? 1 : -1) * Math.min(0.06, distance / 20);
          stroke += ` Q ${x + dx * middle - dy / distance * bend} ${y + dy * middle + dx / distance * bend} ${x + dx * (i + 1) / waves} ${y + dy * (i + 1) / waves}`;
        }
      }
      const effort = pair.reach === "sea" ? pair.voyage : pair.walk;
      const evidence = `${founder.name} and ${other.name}: ${pair.reach === "sea" ? "would need boats" : "may meet on foot"}${effort === null ? "" : ` · ${Math.round(effort).toLocaleString()} effort-km`}`;
      return <g key={`${pair.a}:${pair.b}`}>
        <polygon className="reach-neighbour" points={destination.outline.map((point) => point.join(",")).join(" ")} />
        <path className={`reach-line ${pair.reach === "sea" ? "reach-sea" : "reach-foot"}`} d={stroke}
          data-people={other.id} aria-label={evidence}><title>{evidence}</title></path>
      </g>;
    })}
  </svg>;
}
