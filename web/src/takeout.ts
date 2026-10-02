// Formats for taking a world's contents elsewhere. These only arrange what
// the engine reports; they never make or change words.

import type { Catalog, LexiconRow, Overview, RenderingRow, Variety } from "./model";
import { YEARS } from "./model";
import { FAITH_HOW, renderingOrigin } from "./lore";

export function download(name: string, text: string, type = "application/json") {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  URL.revokeObjectURL(url);
}

/// A file name from a title: lowercase words joined by hyphens.
export function fileName(title: string, extension: string): string {
  const slug = title
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return `${slug || "umran"}.${extension}`;
}

export function origin(row: LexiconRow): string {
  switch (row.origin.kind) {
    case "borrowed":
    case "derived":
      return `from ${row.origin.from}`;
    case "kept":
      return `kept from ${row.origin.from}`;
    default:
      return row.origin.kind;
  }
}

function cell(text: string): string {
  return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

/// A glossary as CSV: one row per meaning, with the word, its sounds, its
/// semantic field, and where it came from.
export function glossaryCsv(rows: LexiconRow[]): string {
  const lines = [["word", "said", "ipa", "meaning", "field", "origin"]];
  for (const r of rows) lines.push([r.spelled, r.said ?? "", r.ipa, r.gloss, r.field, origin(r)]);
  return lines.map((l) => l.map(cell).join(",")).join("\n") + "\n";
}

/// Who is who now: each living people's name, what it means, how it was
/// once said, and what others call them.
export function peopleLines(overview: Overview): string[] {
  const name = (id: number) => overview.communities[id]?.name ?? "?";
  return overview.communities.filter((c) => c.ended === null).map((c) => {
    const tongue = overview.varieties[c.variety];
    const parts = [`${c.name}, “${c.meaning}”, speaking ${tongue.name}${tongue.meaning ? ` (“${tongue.meaning}”)` : ""}`];
    if (c.once) parts.push(`once ${c.once}`);
    if (c.exonyms.length > 0) parts.push(`called ${c.exonyms.map((e) => `${e.name} by the ${name(e.by)}`).join(", ")}`);
    parts.push(c.faith === null ? "keeps its own gods" : `holds ${overview.religions[c.faith].name}`);
    if (c.crafts.length > 0) parts.push(`crafts: ${c.crafts.map((id) => overview.crafts.find((c) => c.id === id)!.name).join(", ")}`);
    return parts.join("; ");
  });
}

/// Every realm's name, including those that have fallen, with its rulers.
export function stateLines(overview: Overview): string[] {
  return overview.states.map((s) => {
    const parts = [`${s.name}, “${s.meaning}”, ruled by the ${overview.communities[s.rulers].name}`];
    if (s.once) parts.push(`once ${s.once}`);
    parts.push(`founded by ${s.founder.name}, “${s.founder.meaning}”, /${s.founder.ipa}/`);
    if (s.fell !== null) parts.push(`fell in year ${s.fell * YEARS}`);
    return parts.join("; ");
  });
}

/// Every founded faith, with the founder and the traits of its teaching.
export function religionLines(overview: Overview): string[] {
  return overview.religions.map((r) => [
    `${r.name}, “${r.meaning}”, /${r.ipa}/`,
    `founded in year ${r.founded * YEARS}, ${FAITH_HOW[r.how]}`,
    `founder: ${r.founder.name}, “${r.founder.meaning}”, /${r.founder.ipa}/`,
    `people: ${overview.communities[r.people].name}`,
    `land: ${overview.places.find((p) => p.region === r.land)?.names.at(-1)?.spelled ?? `Land ${r.land + 1}`}`,
    `sacred language: ${overview.varieties[r.sacred].name}`,
    r.converts ? "seeks converts" : "keeps to its own",
    r.translates ? "followers translate its words" : "followers borrow its words",
    r.scripture ? "written scripture; brings writing" : "unwritten teaching",
    `followers: ${r.followers.map((id) => overview.communities[id].name).join(", ") || "none now"}`,
  ].join("; "));
}

export function craftLines(overview: Overview, catalog: Catalog): string[] {
  return overview.crafts.map((c) => [
    `${c.name}: ${catalog.crafts.find((choice) => choice.id === c.id)!.description}`,
    c.first === null ? "not yet held" : `first held in year ${c.first * YEARS}`,
    `inventors: ${c.inventors.map((id) => overview.communities[id].name).join(", ") || "none yet"}`,
    `holders: ${c.holders.map((id) => overview.communities[id].name).join(", ") || "none now"}`,
  ].join("; "));
}

export function givenNameLines(variety: Variety, overview: Overview): string[] {
  return variety.names.map((name) => `${name.name} /${name.ipa}/ “${name.meaning}”${
    name.from === null ? "" : `; from ${overview.varieties[name.from].name} (sacred)`
  }`);
}

export function renderingLines(rows: RenderingRow[], overview: Overview): string[] {
  return rows.flatMap((row) => row.renderings.map((r) =>
    `${row.gloss}: ${r.spelled} /${r.ipa}/ in ${overview.varieties[r.variety].name}; ${renderingOrigin(r)}`
  ));
}


/// One column per language, with the sacred language first for a faith.
function renderingMarkdown(rows: RenderingRow[], overview: Overview, sacred?: number): string[] {
  if (rows.length === 0) return ["No words yet."];
  const languages = [...new Set(rows.flatMap((row) => row.renderings.map((r) => r.variety)))];
  if (sacred !== undefined) {
    const at = languages.indexOf(sacred);
    if (at >= 0) languages.splice(at, 1);
    languages.unshift(sacred);
  }
  const line = (cells: string[]) => `| ${cells.map((text) => text.replaceAll("|", "\\|").replaceAll("\n", " ")).join(" | ")} |`;
  return [
    line(["Meaning", ...languages.map((id) => overview.varieties[id].name)]),
    line(languages.map(() => "---").concat("---")),
    ...rows.map((row) => line([row.gloss, ...languages.map((id) => {
      const r = row.renderings.find((r) => r.variety === id);
      return r ? `*${r.spelled}* /${r.ipa}/ · ${renderingOrigin(r)}` : "—";
    })])),
  ];
}

/// The world as Markdown: peoples, realms, faiths, crafts, names, history,
/// and a glossary for each spoken or sacred language.
export function bookMarkdown(title: string, overview: Overview, glossaries: [string, LexiconRow[]][], catalog: Catalog, notes: import("./model").NotebookNote[] = []): string {
  const out = [`# ${title}`, "", `As it stands in year ${overview.generation * YEARS}.`, "", "## The peoples", ""];
  for (const line of peopleLines(overview)) out.push(`- ${line}`);
  if (overview.states.length > 0) {
    out.push("", "## The states", "");
    for (const line of stateLines(overview)) out.push(`- ${line}`);
  }
  if (overview.religions.length > 0) {
    out.push("", "## Religions", "");
    const lines = religionLines(overview);
    for (const r of overview.religions) {
      out.push(`### ${r.name}`, "", lines[r.id], "", ...renderingMarkdown(r.words, overview, r.sacred), "");
    }
  }
  out.push("", "## Crafts", "");
  const crafts = craftLines(overview, catalog);
  overview.crafts.forEach((craft, i) => {
    out.push(`### ${craft.name}`, "", crafts[i], "", ...renderingMarkdown(craft.words, overview), "");
  });
  out.push("", "## Given names", "");
  for (const v of overview.varieties) {
    out.push(`### ${v.name}`, "", v.nameStyle === "double" ? "Two-part names." : "One-word names.", "");
    if (v.written === null) out.push("Unwritten.");
    else out.push(`Written or last respelled in year ${v.written * YEARS}.`);
    if (v.sacredOf !== null) out.push(`Sacred to ${overview.religions[v.sacredOf].name}.`);
    out.push("", ...givenNameLines(v, overview).map((line) => `- ${line}`), "");
  }
  out.push("", "## History");
  let year = -1;
  for (const a of overview.annals) {
    if (a.generation !== year) {
      year = a.generation;
      out.push("", `### ${year === 0 ? "In the beginning" : `Year ${year * YEARS}`}`, "");
    }
    out.push(a.kind === "law" ? `- *${a.text}*` : `- ${a.text}`);
  }
  for (const [tongue, rows] of glossaries) {
    out.push("", `## Glossary of ${tongue}`, "");
    const sorted = [...rows].sort((a, b) => a.spelled.localeCompare(b.spelled));
    for (const r of sorted) out.push(`- **${r.spelled}**${r.said === null ? "" : ` · said *${r.said}*`} /${r.ipa}/ ${r.gloss}`);
  }
  if (notes.length) {
    out.push("", "## The field notebook", "");
    for (const note of notes) {
      const telling = overview.tellings.find((t) => t.id === note.target?.reading.telling);
      out.push(`### ${note.title}`, "", `${note.kind}${note.archived ? " · archived" : ""} · year ${note.generation * YEARS}${note.target ? ` · ${telling?.name ?? "Unavailable telling"}` : ""}`, "");
      if (note.target) out.push(`Reference: ${note.label}.`, "");
      if (note.revision !== overview.revision) out.push("The original reference needs checking after an engine change.", "");
      out.push(note.body, "");
    }
  }
  return out.join("\n") + "\n";
}
