//! Offline surface diagnostics, deliberately outside the replay map schema.
use super::*;
use std::fmt::Write;

fn turn(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn hull(mut points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    let mut result = Vec::with_capacity(points.len());
    for &p in &points {
        while result.len() > 1 && turn(result[result.len() - 2], result[result.len() - 1], p) <= 0.0
        {
            result.pop();
        }
        result.push(p);
    }
    let lower = result.len();
    for &p in points.iter().rev().skip(1) {
        while result.len() > lower
            && turn(result[result.len() - 2], result[result.len() - 1], p) <= 0.0
        {
            result.pop();
        }
        result.push(p);
    }
    result.pop();
    result
}

struct Projection {
    centre: Point,
    east: Point,
    north: Point,
}
impl Projection {
    fn new(mesh: &sphere::Mesh, members: &[Option<usize>], id: usize) -> Self {
        let centre = unit(
            mesh.cells
                .iter()
                .enumerate()
                .filter(|&(r, _)| members[r] == Some(id))
                .fold([0.0; 3], |sum, (_, c)| {
                    add(sum, sphere::moment(&c.boundary))
                }),
        );
        let (east, north) = sphere::tangent(centre);
        Self {
            centre,
            east,
            north,
        }
    }
    fn at(&self, p: Point) -> Option<[f64; 2]> {
        let facing = dot(p, self.centre);
        (facing > 0.0).then(|| [dot(p, self.east) / facing, dot(p, self.north) / facing])
    }
}

/// Spherical convexity: the fraction of sampled pairs whose minor arc stays
/// on this same body. Unlike centroid-gnomonic hulls this is defined even for
/// legacy trefoils crossing their centroid's horizon. Every body participates.
pub(super) fn continent_convexity(
    mesh: &sphere::Mesh,
    members: &[Option<usize>],
    areas: &[f64],
    cutoff: f64,
) -> Vec<f64> {
    let mut ids: Vec<_> = (0..areas.len()).filter(|&id| areas[id] >= cutoff).collect();
    ids.sort_by(|&a, &b| areas[b].total_cmp(&areas[a]).then(a.cmp(&b)));
    let interval = 0.8 / (mesh.cells.len() as f64).sqrt();
    ids.into_iter()
        .map(|id| {
            let land: Vec<_> = members
                .iter()
                .enumerate()
                .filter_map(|(r, &m)| (m == Some(id)).then_some(r))
                .collect();
            let pairs = land.len() * (land.len() - 1) / 2;
            let samples = pairs.min(512);
            if samples == 0 {
                return 1.0;
            }
            let mut stayed = 0;
            let mut row = 0;
            let mut offset = 0;
            for sample in 0..samples {
                let pair = (2 * sample + 1) * pairs / (2 * samples);
                while pair >= offset + land.len() - row - 1 {
                    offset += land.len() - row - 1;
                    row += 1;
                }
                let a = land[row];
                let b = land[row + 1 + pair - offset];
                let (start, end) = (mesh.cells[a].position, mesh.cells[b].position);
                let angle = sphere::angle(start, end);
                let steps = (angle / interval) as usize + 1;
                let mut at = a;
                let mut inside = true;
                for step in 1..steps {
                    let t = step as f64 / steps as f64;
                    let left = math::sin((1.0 - t) * angle);
                    let right = math::sin(t * angle);
                    let p = unit(std::array::from_fn(|i| start[i] * left + end[i] * right));
                    at = continental::v6::nearest(mesh, p, at);
                    let same = |r: usize| members[r] == Some(id) && contains(&mesh.cells[r], p);
                    if !(members[at] == Some(id)
                        && mesh.cells[at]
                            .neighbours
                            .iter()
                            .all(|&n| members[n] == Some(id)))
                        && !same(at)
                        && !mesh.cells[at].neighbours.iter().any(|&n| same(n))
                    {
                        inside = false;
                        break;
                    }
                }
                stayed += usize::from(inside);
            }
            stayed as f64 / samples as f64
        })
        .collect()
}

fn contains(cell: &sphere::Cell, p: Point) -> bool {
    cell.boundary
        .iter()
        .zip(cell.boundary.iter().cycle().skip(1))
        .any(|(&a, &b)| {
            dot(sphere::cross(cell.position, a), p) <= 1e-12
                && dot(sphere::cross(a, b), p) <= 1e-12
                && dot(sphere::cross(b, cell.position), p) <= 1e-12
        })
}

/// A gulf is a connected patch of sea inside a landmass's spherical convex
/// hull. Its rim includes the open mouth as well as land borders. Count it
/// only when at least 70% of that rim belongs to this same landmass, and at
/// least two regional centres lie inside. The open ocean is never a gulf.
pub(super) fn enclosed_seas(mesh: &sphere::Mesh, members: &[Option<usize>]) -> usize {
    let count = members.iter().flatten().max().map_or(0, |id| id + 1);
    let mut result = 0;
    for id in 0..count {
        if members.iter().filter(|&&m| m == Some(id)).count() < 8 {
            continue;
        }
        let projection = Projection::new(mesh, members, id);
        let points: Vec<_> = mesh
            .cells
            .iter()
            .enumerate()
            .filter(|&(r, _)| members[r] == Some(id))
            .flat_map(|(_, c)| c.boundary.iter().filter_map(|&p| projection.at(p)))
            .collect();
        let outline = hull(points);
        let mut inside: Vec<_> = mesh
            .cells
            .iter()
            .enumerate()
            .map(|(r, c)| {
                members[r].is_none()
                    && projection.at(c.position).is_some_and(|p| {
                        outline
                            .iter()
                            .zip(outline.iter().cycle().skip(1))
                            .all(|(&a, &b)| turn(a, b, p) >= 0.0)
                    })
            })
            .collect();
        let mut stack = Vec::new();
        for start in 0..inside.len() {
            if !inside[start] {
                continue;
            }
            let mut body = Vec::new();
            inside[start] = false;
            stack.push(start);
            while let Some(r) = stack.pop() {
                body.push(r);
                for &n in &mesh.cells[r].neighbours {
                    if inside[n] {
                        inside[n] = false;
                        stack.push(n);
                    }
                }
            }
            if body.len() < 2 {
                continue;
            }
            body.sort_unstable();
            let mut rim = 0.0;
            let mut land = 0.0;
            for &r in &body {
                for &n in &mesh.cells[r].neighbours {
                    if body.binary_search(&n).is_ok() {
                        continue;
                    }
                    let boundary = &mesh.cells[r].boundary;
                    let other = &mesh.cells[n].boundary;
                    let length: f64 = boundary
                        .iter()
                        .zip(boundary.iter().cycle().skip(1))
                        .filter(|(a, b)| other.contains(a) && other.contains(b))
                        .map(|(&a, &b)| sphere::angle(a, b))
                        .sum();
                    rim += length;
                    if members[n] == Some(id) {
                        land += length;
                    }
                }
            }
            result += usize::from(land >= 0.70 * rim);
        }
    }
    result
}

/// Terrain and history proof only, not a facade view or a second map model.
pub fn continental_svg(seed: u64, size: MapSize) -> String {
    let mut mesh = sphere::mesh(size.subdivisions());
    let ((terrain, _, _), evidence) =
        continental::v6::diagnosed_surface(seed, &mut mesh, size.radius_km());
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"660\" viewBox=\"0 0 1200 660\"><rect width=\"1200\" height=\"660\" fill=\"#f3eddf\"/>",
    );
    writeln!(svg,"<text x=\"30\" y=\"32\" font-family=\"sans-serif\" font-size=\"20\">ContinentalV6 seed {seed} {size:?}: plate boundaries (red), initial coast (faint)</text>").unwrap();
    for hemisphere in 0..2 {
        let sign = if hemisphere == 0 { 1.0 } else { -1.0 };
        let cx = 300.0 + 600.0 * hemisphere as f64;
        let project = |p: Point| [cx + sign * p[1] * 275.0, 345.0 - p[2] * 275.0];
        writeln!(
            svg,
            "<circle cx=\"{cx}\" cy=\"345\" r=\"275\" fill=\"#82acbb\"/>"
        )
        .unwrap();
        for (r, cell) in mesh.cells.iter().enumerate() {
            let color = match terrain[r] {
                Terrain::Sea => "#82acbb",
                Terrain::Plains => "#d3cc85",
                Terrain::Forest => "#75986d",
                Terrain::Steppe => "#b9b28a",
                Terrain::Hills => "#ac946d",
                Terrain::Mountains => "#766c62",
                Terrain::Desert => "#e1c291",
            };
            // Clip each convex cell to the view hemisphere in 3D. Coast
            // polygons are locally star shaped, so this also clips shores.
            let mut clipped = Vec::new();
            for (&a, &b) in cell
                .boundary
                .iter()
                .zip(cell.boundary.iter().cycle().skip(1))
            {
                let (da, db) = (sign * a[0], sign * b[0]);
                if da >= 0.0 {
                    clipped.push(a);
                }
                if (da >= 0.0) != (db >= 0.0) {
                    let t = da / (da - db);
                    clipped.push(unit(std::array::from_fn(|i| a[i] + t * (b[i] - a[i]))));
                }
            }
            if clipped.len() < 3 {
                continue;
            }
            write!(
                svg,
                "<path fill=\"{color}\" stroke=\"{color}\" stroke-width=\"0.35\" d=\""
            )
            .unwrap();
            for (i, p) in clipped.into_iter().enumerate() {
                let [x, y] = project(p);
                write!(svg, "{}{x:.2},{y:.2} ", if i == 0 { "M" } else { "L" }).unwrap();
            }
            svg.push_str("Z\"/>");
        }
        for (segments, color, opacity) in [
            (&evidence.initial_coast, "#ffffff", 0.5),
            (&evidence.boundaries, "#ab413a", 0.55),
        ] {
            for &[mut a, mut b] in segments {
                let (da, db) = (sign * a[0], sign * b[0]);
                if da < 0.0 && db < 0.0 {
                    continue;
                }
                if (da >= 0.0) != (db >= 0.0) {
                    let t = da / (da - db);
                    let p = unit(std::array::from_fn(|i| a[i] + t * (b[i] - a[i])));
                    if da < 0.0 {
                        a = p;
                    } else {
                        b = p;
                    }
                }
                let [x, y] = project(a);
                let [u, v] = project(b);
                write!(svg,"<path d=\"M{x:.2},{y:.2} L{u:.2},{v:.2}\" fill=\"none\" stroke=\"{color}\" opacity=\"{opacity}\" stroke-width=\"0.8\"/>").unwrap();
            }
        }
    }
    svg.push_str("</svg>");
    svg
}
