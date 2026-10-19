import { createContext } from "react";
import bookCss from "./components/book.css?raw";
import chapterCss from "./components/chapter.css?raw";
import printCss from "./print.css?raw";
import { parse } from "./route";
import type { ReadEngine } from "./model";

/// `complete` opens every chapter for an edition; `apparatus` also opens
/// every word's evidence, which multiplies the text by the lexicon.
export interface EditionMode { complete: boolean; apparatus: boolean }
export const EditionContext = createContext<EditionMode>({ complete: false, apparatus: false });

/** An edition outlives the reading's bounded card cache. Keep its exact
 * promises until the offscreen book is disposed, including on Suspense retries. */
export function editionReading(engine: ReadEngine): ReadEngine {
  const answers = new Map<string, unknown>();
  const methods = new Map<PropertyKey, (...args: unknown[]) => unknown>();
  return new Proxy(engine, {
    get(target, property) {
      const method: unknown = Reflect.get(target, property);
      if (typeof method !== "function") return method;
      let kept = methods.get(property);
      if (!kept) {
        kept = (...args) => {
          const key = `${String(property)}:${JSON.stringify(args)}`;
          if (!answers.has(key)) answers.set(key, Reflect.apply(method, target, args));
          return answers.get(key);
        };
        methods.set(property, kept);
      }
      return kept;
    },
  });
}

const CHROME = "[data-edition-omit], .book-sheet-head, .book-running-head, .book-copy, .book-close, .filters, .takeout, .edition-progress";
const BLOCK = "p, ul, ol, dl, table, details, article, section, div, h1, h2, h3, h4, h5, h6";
const FONT_FACES = [
  ["regular", "normal", 400], ["italic", "italic", 400],
  ["bold", "normal", 700], ["bolditalic", "italic", 700],
] as const;
let fonts: Promise<string> | undefined;

function embeddedFonts(): Promise<string> {
  return fonts ??= Promise.all(FONT_FACES.map(async ([face, style, weight]) => {
    const url = new URL(`${import.meta.env.BASE_URL}fonts/umran-latin-${face}.woff2`, window.location.href);
    const response = await fetch(url);
    if (!response.ok) throw new Error(`The edition’s ${face} font could not be read.`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    let binary = "";
    for (let at = 0; at < bytes.length; at += 32768) binary += String.fromCharCode(...bytes.subarray(at, at + 32768));
    return `@font-face{font-family:"Umran Latin";font-style:${style};font-weight:${weight};font-display:swap;src:url("data:font/woff2;base64,${btoa(binary)}") format("woff2")}`;
  })).then((rules) => rules.join("\n")).catch((error: unknown) => { fonts = undefined; throw error; });
}

/** Keep the book’s typography and chart rules, not the surrounding workbench. */
function usedStyles(root: HTMLElement, edition: HTMLElement): string {
  const matches = (selector: string) => {
    // Pseudo-elements and interaction states have no DOM node of their own.
    const plain = selector.replace(/::[\w-]+(?:\([^)]*\))?/g, "").replace(/:(?:hover|focus-visible|focus-within|focus|active)\b/g, "");
    try {
      return root.matches(plain) || root.querySelector(plain) !== null || edition.matches(plain) || edition.querySelector(plain) !== null
        || document.documentElement.matches(plain) || document.body.matches(plain);
    } catch { return false; }
  };
  const select = (rules: CSSRuleList): string => Array.from(rules).flatMap((rule): string[] => {
    if (rule instanceof CSSStyleRule) return matches(rule.selectorText) ? [rule.cssText] : [];
    if (rule instanceof CSSMediaRule || rule instanceof CSSSupportsRule) {
      const children = select(rule.cssRules);
      return children ? [`${rule instanceof CSSMediaRule ? "@media" : "@supports"} ${rule.conditionText}{${children}}`] : [];
    }
    if (rule instanceof CSSPageRule) return [rule.cssText];
    return [];
  }).join("\n");
  const rules = new Set<string>();
  for (const sheet of Array.from(document.styleSheets)) {
    try { rules.add(select(sheet.cssRules)); } catch { /* An unrelated cross-origin sheet is not part of the book. */ }
  }
  const book = new CSSStyleSheet();
  book.replaceSync(`${bookCss}\n${chapterCss}`);
  rules.add(select(book.cssRules));
  const computed = getComputedStyle(root);
  const variables = Array.from(computed).filter((name) => name.startsWith("--"))
    .map((name) => `${name}:${computed.getPropertyValue(name)};`).join("");
  const print = new CSSStyleSheet();
  print.replaceSync(printCss);
  return [...rules, `:root{${variables}--serif:"Umran Latin",serif;--sans:"Umran Latin",serif;--mono:"Umran Latin",serif;--display:"Umran Latin",serif;--caps:"Umran Latin",serif;--initial:"Umran Latin",serif;}`, select(print.cssRules)].join("\n");
}

function editionDom(root: HTMLElement): HTMLElement {
  const book = root.cloneNode(true) as HTMLElement;
  book.classList.add("book-edition");
  book.removeAttribute("role");
  book.removeAttribute("aria-modal");
  book.querySelectorAll(CHROME).forEach((element) => element.remove());
  book.querySelectorAll("details").forEach((element) => { element.open = true; });
  const ids = new Set([book.id, ...Array.from(book.querySelectorAll("[id]"), (element) => element.id)]);
  for (const control of book.querySelectorAll<HTMLElement>("a, button, [data-book-target]")) {
    let target = control.dataset.bookTarget;
    const href = control.getAttribute("href");
    if (!target && href?.startsWith("#")) {
      const route = parse(href);
      target = route?.kind === "world" && route.page === "book"
        ? route.chapter ? `book-${route.chapter}` : "book-contents"
        : href.slice(1);
    }
    const replacement = document.createElement(target && ids.has(target) ? "a" : "span");
    for (const attribute of Array.from(control.attributes)) {
      if (["class", "id", "title"].includes(attribute.name)) replacement.setAttribute(attribute.name, attribute.value);
    }
    if (replacement instanceof HTMLAnchorElement) replacement.setAttribute("href", `#${target}`);
    replacement.append(...Array.from(control.childNodes));
    control.replaceWith(replacement);
  }
  book.querySelectorAll("input, select, textarea, script, link, style").forEach((element) => element.remove());
  for (const element of [book, ...book.querySelectorAll("*")]) {
    for (const attribute of Array.from(element.attributes)) {
      if (attribute.name.startsWith("on") || ["tabindex", "aria-selected", "aria-controls", "aria-expanded"].includes(attribute.name)) element.removeAttribute(attribute.name);
    }
  }
  return book;
}

function escapeMarkdown(text: string): string {
  return text.replace(/\\/g, "\\\\").replace(/([*_`\[\]])/g, "\\$1");
}
function inline(node: Node, omitBlocks = false): string {
  if (node.nodeType === Node.TEXT_NODE) return escapeMarkdown((node.textContent ?? "").replace(/\s+/g, " "));
  if (!(node instanceof Element) || node.tagName === "SVG" || node.localName === "svg") return "";
  if (omitBlocks && node.matches(BLOCK)) return "";
  if (node.tagName === "BR") return "\n";
  const text = Array.from(node.childNodes, (child) => inline(child, omitBlocks)).join("");
  if (node.matches("strong, b")) return `**${text}**`;
  if (node.matches("em, i, .word")) return text.trim() ? `*${text.trim()}*` : text;
  if (node.tagName === "CODE") return `\`${text}\``;
  return node.classList.contains("gen") ? `${text} ` : text;
}
function tableLine(cells: string[]): string {
  return `| ${cells.map((cell) => cell.trim().replace(/\|/g, "\\|").replace(/\n/g, "<br>")).join(" | ")} |`;
}
function table(headers: string[], rows: string[][]): string {
  return [tableLine(headers), tableLine(headers.map(() => "---")), ...rows.map(tableLine)].join("\n") + "\n\n";
}
function walk(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) return inline(node);
  if (!(node instanceof Element) || node.localName === "svg") return "";
  const children = () => Array.from(node.childNodes, walk).join("");
  if (/^H[1-6]$/.test(node.tagName)) return `${"#".repeat(Number(node.tagName[1]))} ${inline(node).trim()}\n\n`;
  if (node.matches("p, figcaption, summary")) return `${inline(node).trim()}\n\n`;
  if (node.tagName === "PRE") return `\`\`\`\n${node.textContent ?? ""}\n\`\`\`\n\n`;
  if (node.tagName === "TABLE") {
    const rows = Array.from((node as HTMLTableElement).rows);
    if (!rows.length) return "";
    const width = Math.max(...rows.map((row) => row.cells.length));
    const cells = (row: HTMLTableRowElement) => Array.from({ length: width }, (_, index) => row.cells[index] ? inline(row.cells[index], true).trim() : "");
    const hasHeader = Array.from(rows[0].cells).some((cell) => cell.tagName === "TH");
    const headers = hasHeader ? cells(rows[0]) : Array.from({ length: width }, () => "");
    const body = (hasHeader ? rows.slice(1) : rows).filter((row) => !row.classList.contains("dictionary-evidence"));
    // Lexical evidence lives inside a cell. Keep its paragraphs, histories,
    // and nested tables after the ledger, rather than flattening or losing it.
    const evidence = rows.flatMap((row) => Array.from(row.cells).flatMap((cell) => Array.from(cell.children).filter((child) => child.matches(BLOCK))));
    return table(headers, body.map(cells)) + evidence.map(walk).join("");
  }
  if (node.tagName === "DL") {
    const rows = Array.from(node.children).filter((child) => child.tagName === "DIV");
    if (rows.length && rows.every((row) => row.querySelector("dt") && row.querySelector("dd"))) {
      const specimen = node.classList.contains("specimen");
      return table(specimen ? ["Meaning", "Form", "IPA"] : ["Subject", "Description"], rows.map((row) => {
        const term = row.querySelector("dt")!;
        const meaning = specimen ? escapeMarkdown(term.getAttribute("title") ?? term.textContent ?? "") : inline(term);
        const description = inline(row.querySelector("dd")!);
        const pronunciation = row.querySelector<HTMLElement>(".word[title]")?.title ?? "";
        return specimen ? [meaning, description, escapeMarkdown(pronunciation)] : [meaning, description];
      }));
    }
    return children();
  }
  if (node.matches("ul, ol")) {
    const numbered = node.tagName === "OL" && !node.matches(".history, .book-chronicle, .book-quiet, .law-ledger, .book-name-ledger");
    return Array.from(node.children).filter((child) => child.tagName === "LI").map((child, index) => {
      const content = walk(child).trim();
      const marker = numbered ? `${index + 1}. ` : "- ";
      return marker + content.replace(/\n/g, `\n${" ".repeat(marker.length)}`) + "\n";
    }).join("") + "\n";
  }
  if (node.tagName === "BR") return "\n";
  if (node.matches("article, section, div, details, figure, nav, li")) return children();
  return inline(node);
}

/** A snapshot of one fully opened Book; HTML and Markdown share that DOM. */
export function makeEdition(root: HTMLElement): { html(): Promise<string>; markdown(): string } {
  const book = editionDom(root);
  const title = root.querySelector("h1")?.textContent ?? "The book of a world";
  const language = document.documentElement.lang || "en";
  const theme = getComputedStyle(root).colorScheme === "dark" ? "dark" : "light";
  const htmlEscape = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  return {
    async html() {
      const css = usedStyles(root, book);
      const fontCss = await embeddedFonts();
      return `<!doctype html>\n<html lang="${htmlEscape(language)}" data-theme="${theme}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${htmlEscape(title)}</title><style>${fontCss}\n${css.replace(/<\/style/gi, "<\\/style")}</style></head><body>${book.outerHTML}</body></html>\n`;
    },
    markdown() { return walk(book).trim().replace(/\n{3,}/g, "\n\n") + "\n"; },
  };
}
