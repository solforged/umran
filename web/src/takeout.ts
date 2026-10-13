// Save files and tabular downloads. Book editions come from edition.ts.

import type { LexiconRow } from "./model";

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

