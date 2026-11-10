// Downloads: save files, and the files the engine writes for taking out.
// Book editions come from edition.ts.

import type { Taken } from "./model";

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

/// Save a file the engine wrote, named after what it holds. A CSV gets a
/// byte-order mark here so that a spreadsheet program reads its IPA as
/// Unicode; the engine's own text has none, for programs that read it.
export function saveTaken(taken: Taken, title?: string) {
  const text = taken.media.startsWith("text/csv") ? `\uFEFF${taken.text}` : taken.text;
  download(fileName(title ? `${title} ${taken.name}` : taken.name, taken.extension), text, taken.media);
}
