// Draws the ʿUmrān astrolabe and writes every brand asset in place:
// the favicon, the app icons, the CSS mask for the brand lettering, and
// the outlined logos for the README. Run with `bun run brand`.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import opentype from "opentype.js";
import { Resvg } from "@resvg/resvg-js";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

// The logo's lettering is set from the app's own faces, then outlined, so
// the SVGs need no fonts. Both are OFL; they are fetched, not committed.
const FONTS = {
  "IMFeENsc28P.ttf": "https://github.com/google/fonts/raw/main/ofl/imfellenglishsc/IMFeENsc28P.ttf",
  "EBGaramond-Italic.ttf": "https://github.com/google/fonts/raw/main/ofl/ebgaramond/EBGaramond-Italic%5Bwght%5D.ttf",
};
async function font(name) {
  const path = join(here, "fonts", name);
  if (!existsSync(path)) {
    const res = await fetch(FONTS[name]);
    if (!res.ok) throw new Error(`fetching ${name}: ${res.status}`);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, Buffer.from(await res.arrayBuffer()));
  }
  const b = readFileSync(path);
  return opentype.parse(b.buffer.slice(b.byteOffset, b.byteOffset + b.length));
}

// The palettes of styles.css, by day and by lamplight.
const LIGHT = { bg: "#ece0c2", disc: "#f4ead2", ink: "#2b2117", soft: "#6b5a43", gold: "#a8812c", red: "#9b2c1c", tag: "#6b5a43" };
const DARK = { bg: "#2a2219", disc: "#33291c", ink: "#eadbb8", soft: "#8a7656", gold: "#d6aa48", red: "#e07a52", tag: "#b09c7a" };
const f = n => +n.toFixed(2);

// The astrolabe in a 512 box. From the top: the shackle ring and throne it
// hangs by, the limb (the graduated brass rim), the plate engraved with
// almucantars (circles of equal altitude, one plate per clime), and the
// rete, here a khatam star, with its vermilion pointer. Content spans x
// 60–452 and y 20–492. `small` drops the fine lines and thickens strokes
// for favicon sizes.
const C = 256, CY = 292, H = 132, S = H / Math.SQRT2;
const RETE = [
  `${C},${CY - H} ${C + H},${CY} ${C},${CY + H} ${C - H},${CY}`,
  `${f(C - S)},${f(CY - S)} ${f(C + S)},${f(CY - S)} ${f(C + S)},${f(CY + S)} ${f(C - S)},${f(CY + S)}`,
];
const THRONE = "M190 106C200 78 226 62 256 60C286 62 312 78 322 106Z";

function mark(p, small = false) {
  const w = small ? 1.6 : 1, rete = small ? 15 : 8;
  let ticks = "", alm = "";
  if (!small) {
    for (let d = 0; d < 360; d += 6) {
      const a = d * Math.PI / 180, r1 = d % 30 ? 178 : 169, r2 = 188;
      ticks += `M${f(C + r1 * Math.sin(a))} ${f(CY - r1 * Math.cos(a))}L${f(C + r2 * Math.sin(a))} ${f(CY - r2 * Math.cos(a))}`;
    }
    for (const t of [0, 0.18, 0.36, 0.54, 0.72, 0.88]) {
      alm += `<circle cx="${C}" cy="${f((CY + 24) * (1 - t) + (CY - 62) * t)}" r="${f(150 * (1 - t) + 12 * t)}"/>`;
    }
  }
  return `<g stroke-linejoin="round">
<circle cx="${C}" cy="40" r="17" fill="none" stroke="${p.gold}" stroke-width="${9 * w}"/>
<path d="${THRONE}" fill="${p.gold}" stroke="${p.ink}" stroke-width="${5 * w}"/>
${small ? "" : `<circle cx="${C}" cy="80" r="9" fill="${p.disc}" stroke="${p.ink}" stroke-width="3"/>`}
<circle cx="${C}" cy="${CY}" r="196" fill="${p.gold}" stroke="${p.ink}" stroke-width="${6 * w}"/>
${small ? "" : `<path d="${ticks}" stroke="${p.ink}" stroke-width="2.5"/>`}
<circle cx="${C}" cy="${CY}" r="160" fill="${p.disc}" stroke="${p.ink}" stroke-width="${4 * w}"/>
${small ? "" : `<g fill="none" stroke="${p.soft}" stroke-width="2.2">${alm}</g><path d="M${C} ${CY - 160}V${CY + 160}" stroke="${p.soft}" stroke-width="2"/>`}
${small ? "" : `<circle cx="${C}" cy="${CY - 30}" r="92" fill="none" stroke="${p.ink}" stroke-width="7"/>`}
${RETE.map(pts => `<polygon points="${pts}" fill="none" stroke="${p.ink}" stroke-width="${rete}"/>`).join("")}
<path d="M${C} ${CY - H - 6}l${small ? 18 : 12} ${small ? 40 : 30}h-${small ? 36 : 24}Z" fill="${p.red}" stroke="${p.ink}" stroke-width="3"/>
<circle cx="${C}" cy="${CY}" r="${small ? 22 : 15}" fill="${p.gold}" stroke="${p.ink}" stroke-width="${4 * w}"/>
</g>`;
}

// A square icon on paper, the mark centred at scale k, leaving the margin
// that maskable icons need.
const icon = (p, small, k) => `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
<rect width="512" height="512" fill="${p.bg}"/>
<g transform="translate(256 ${f(256 - 256 * k)}) scale(${k}) translate(-256 0)">${mark(p, small)}</g>
</svg>
`;
const bare = p => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="60 20 392 472">\n${mark(p)}\n</svg>\n`;

// One colour, for the CSS mask before the brand lettering, which inks it
// in the theme's gold: the limb as a ring, the rete as heavy strokes.
const maskMark = () => `<svg xmlns="http://www.w3.org/2000/svg" width="392" height="472" viewBox="60 20 392 472"><g fill="#000" stroke="#000" stroke-linejoin="round">
<circle cx="${C}" cy="40" r="17" fill="none" stroke-width="14"/>
<path d="${THRONE}"/>
<path fill-rule="evenodd" stroke="none" d="M${C - 199} ${CY}a199 199 0 1 0 398 0a199 199 0 1 0-398 0ZM${C - 150} ${CY}a150 150 0 1 1 300 0a150 150 0 1 1-300 0Z"/>
${RETE.map(pts => `<polygon fill="none" stroke-width="20" points="${pts}"/>`).join("\n")}
<circle cx="${C}" cy="${CY}" r="26" stroke="none"/>
</g></svg>
`;

// Lettering with tracking, as path data; returns the data and its width.
function setText(face, text, size, tracking, x, y) {
  let d = "", cx = x;
  const glyphs = face.stringToGlyphs(text);
  glyphs.forEach((g, i) => {
    d += g.getPath(cx, y, size).toPathData(2);
    cx += g.advanceWidth * size / face.unitsPerEm + (i < glyphs.length - 1 ? tracking * size : 0);
  });
  return { d, width: cx - x };
}

// Fell SC has no ʿ (U+02BF); its opening quote is the same shape, set a
// little smaller and higher so it sits as a mark rather than a letter.
function logo(p, caps, italic) {
  const markH = 150, tx = markH * 392 / 472 + 34;
  const ayn = setText(caps, "\u2018", 70, 0, tx, 80);
  const name = setText(caps, "Umr\u0101n", 92, 0.16, tx + ayn.width + 6, 92);
  const tag = setText(italic, "A chronicle of peoples and their tongues", 30, 0.01, tx + 2, 140);
  const W = Math.ceil(tx + Math.max(ayn.width + 6 + name.width, tag.width + 2) + 8);
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="170" viewBox="0 0 ${W} 170">
<g transform="translate(0 10) scale(${f(markH / 472)}) translate(-60 -20)">${mark(p)}</g>
<path d="${ayn.d}${name.d}" fill="${p.ink}"/>
<path d="${tag.d}" fill="${p.tag}"/>
</svg>
`;
}

const png = (svg, size) => new Resvg(svg, { fitTo: { mode: "width", value: size } }).render().asPng();

const [caps, italic] = await Promise.all([font("IMFeENsc28P.ttf"), font("EBGaramond-Italic.ttf")]);
const appIcon = icon(LIGHT, false, 0.86);
const outputs = {
  "web/public/icon.svg": icon(LIGHT, true, 0.9),
  "web/public/icon-512.png": png(appIcon, 512),
  "web/public/icon-192.png": png(appIcon, 192),
  "web/public/apple-touch-icon.png": png(appIcon, 180),
  "web/src/assets/mark-mask.svg": maskMark(),
  "docs/images/mark.svg": bare(LIGHT),
  "docs/images/logo.svg": logo(LIGHT, caps, italic),
  "docs/images/logo-dark.svg": logo(DARK, caps, italic),
};
for (const [path, data] of Object.entries(outputs)) {
  writeFileSync(join(root, path), data);
  console.log(path);
}
