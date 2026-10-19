// Save files and tabular downloads. Book editions come from edition.ts.

import { YEARS, type LexiconRow, type NameQuarry, type WordTable } from "./model";

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
  if (row.origin.authored) return "the author's root";
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

/// A word table as CSV: one row per meaning, two columns per language (the
/// word and its sounds), then the root number of each, so that cognates can
/// be told from same-meaning strangers after the sheet leaves the app.
export function wordTableCsv(table: WordTable): string {
  const names = table.varieties.map((variety) => variety.name);
  const lines = [["meaning", ...names.flatMap((name) => [name, `${name} (IPA)`]), ...names.map((name) => `${name} root`), ...names.map((name) => `${name} origin`)]];
  for (const row of table.rows) {
    lines.push([
      row.gloss,
      ...row.cells.flatMap((word) => word ? [word.spelled, word.ipa] : ["", ""]),
      ...row.cells.map((word) => word ? String(word.root + 1) : ""),
      ...row.cells.map((word) => word ? (word.authored ? "the author's root" : word.origin.kind) : ""),
    ]);
  }
  return lines.map((l) => l.map(cell).join(",")).join("\n") + "\n";
}

/// Every name a language uses as CSV: what kind of thing it names, the name,
/// its sounds, what its parts meant, and since when.
export function nameQuarryCsv(quarry: NameQuarry): string {
  const lines = [["kind", "name", "ipa", "meaning", "since", "origin"]];
  for (const name of quarry.given) lines.push(["given name", name.name, name.ipa, name.meaning, "", name.from === null ? "" : "sacred"]);
  for (const name of quarry.peoples) lines.push(["people", name.name, name.ipa, name.meaning, String(name.since * YEARS), ""]);
  for (const name of quarry.places) lines.push([name.feature, name.name, name.ipa, name.meaning, String(name.since * YEARS), name.origin]);
  for (const name of quarry.states) lines.push(["state", name.name, name.ipa, name.meaning, "", ""]);
  for (const name of quarry.faiths) lines.push(["faith", name.name, name.ipa, name.meaning, "", ""]);
  return lines.map((l) => l.map(cell).join(",")).join("\n") + "\n";
}

