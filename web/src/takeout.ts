// Formats for taking a world's contents elsewhere. These only arrange what
// the engine reports; they never make or change words.

import type { LexiconRow, Overview } from "./model";
import { YEARS } from "./model";

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
  return `${slug || "langgen"}.${extension}`;
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
  const lines = [["word", "ipa", "meaning", "field", "origin"]];
  for (const r of rows) lines.push([r.spelled, r.ipa, r.gloss, r.field, origin(r)]);
  return lines.map((l) => l.map(cell).join(",")).join("\n") + "\n";
}

/// Who is who: each people's name, what it means, how it was once said,
/// and what others call them.
export function peopleLines(overview: Overview): string[] {
  const name = (id: number) => overview.communities[id]?.name ?? "?";
  return overview.communities.map((c) => {
    const tongue = overview.varieties[c.variety];
    const parts = [`${c.name}, “${c.meaning}”, speaking ${tongue.name}${tongue.meaning ? ` (“${tongue.meaning}”)` : ""}`];
    if (c.once) parts.push(`once ${c.once}`);
    if (c.exonyms.length > 0) parts.push(`called ${c.exonyms.map((e) => `${e.name} by the ${name(e.by)}`).join(", ")}`);
    return parts.join("; ");
  });
}

/// The whole world as Markdown: the peoples, everything that happened, and
/// a glossary for every language still spoken.
export function bookMarkdown(title: string, overview: Overview, glossaries: [string, LexiconRow[]][]): string {
  const out = [`# ${title}`, "", `As it stands in year ${overview.generation * YEARS}.`, "", "## The peoples", ""];
  for (const line of peopleLines(overview)) out.push(`- ${line}`);
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
    for (const r of sorted) out.push(`- **${r.spelled}** /${r.ipa}/ ${r.gloss}`);
  }
  return out.join("\n") + "\n";
}
