# Prose and linting

The prose should explain what happens, what a reader can do, and what the
engine records. Keep the chart-and-book vocabulary in `docs/book.md`, but
use ordinary sentence structure. Literary flavour should not make an
instruction harder to follow.

## Run the checks

Install Vale 3.25.1, then run:

```sh
vale sync --no-global
bun run test:prose
bun run lint:prose
```

The repository keeps its own configuration and Voice rules. The commands
use `--no-global` so contributors get the same checks. `vale sync` installs
the pinned `ai-tells` package into an ignored directory. Run it again after
changing the package version.

`lint:prose` checks every Git-visible Markdown and MDX file, including new
files and hidden agent instructions. It fails on warnings and missing rule
files. Vale normally exits successfully when it reports only warnings,
and configured rules can remain inactive when their package is absent.

`test:prose` uses [Vale's native rule tests](https://docs.vale.sh/topics/testing).
Its configuration-level cases exercise every enabled rule, so a missing
check cannot silently pass. Coverage also requires each local Voice rule
to produce an alert. The Prose CI job runs both commands with pinned Vale.

## What the audit found

The first Markdown run reported 47 warnings. The global configuration
also enabled `ai-tells`, but that package had not been installed. After
`vale sync`, the same files produced 160 warnings:

| Rule | Warnings | Finding |
|---|---:|---|
| Dash usage | 81 | 79 were en dashes, chiefly ranges and compound names. Two were em dashes. |
| Semicolon density | 43 | Dense paragraphs joined unrelated facts or hid procedures in long lists. |
| Contrastive formulas | 27 | Many corrections could state the behaviour directly. Technical boundaries still needed to be preserved. |
| Disclaimer tails | 4 | Some rebuttals added little; others expressed a real model limitation. |
| Consequence participles | 3 | Separate sentences made the action and its effect easier to follow. |
| Dramatic closers | 2 | One match was a numerical comparison rather than rhetorical emphasis. |

The cleanup split overloaded documentation, made contributor instructions
scannable, and clarified founding guidance, language explanations, the desk,
and several Rust-generated grammar notes. For example:

| Before | After |
|---|---|
| `a cluster their shape avoids` | a consonant cluster this speech does not normally use |
| `ahead of the nouns` | Nouns still use a separate word. |
| `not a defective grammar` | This category had no overt marker at founding. |

The Rust edits change facade explanations only. They leave simulation
state, random draws, template selection, recipe formats, and the engine
revision unchanged. Reopened worlds can show the revised explanations.

## Changes to the guardrails

- **Install and verify the rules.** The local configuration pins the
  upstream package. The lint command checks that every enabled rule file
  exists before running Vale.
- **Distinguish dashes by use.** `Voice.Dashes` replaces the upstream dash
  rule. It flags em dashes and spaced en dashes between clauses, while
  permitting `0–39`, `V1–V5`, and `Leipzig–Jakarta`.
- **Check quoted interface copy.** The global quote exclusions hid
  authored captions and labels. Here quotation marks leave prose visible
  to Vale. Blockquotes remain excluded for quoted sources, and inline
  and fenced code keep their normal syntax exclusions.
- **Show the count.** The semicolon warning now reports how many it found.
  This helps distinguish a short pair of clauses from a paragraph that
  needs restructuring.
- **Test both matches and exceptions.** Fixtures cover each enabled rule,
  legitimate en dashes, quoted copy, source quotations, code, and paragraph
  boundaries.

## Review beyond the warnings

Keep necessary distinctions. Sound similarity and shared roots measure
different things. Borrowing an inflected pair and adopting a productive
pattern are separate changes. A fictional risk band needs its calibration
limits stated. Rephrase these explanations when helpful, but keep the
qualification.

Check the paragraph before editing a reported location. In this audit,
Vale 3.25.1 misplaced a semicolon-count alert after a nested table in
`docs/engine.md`. The paragraph really had two semicolons, but the line
number pointed much farther down the file.

## Coverage still to add

Automated lint covers Markdown and MDX. Browser copy, glossary strings,
Rust-produced prose, code comments, and command-line output still need
manual review. This audit sampled the browser's shared explanations and
facade grammar notes; it did not validate every generated sentence.

The next useful extension is a source-mapped collection of visible text:

- Join JSX prose across inline links and explanations, and include visible
  labels, accessible names, and shared glossary entries.
- Collect Rust facade text from representative generated views, preserving
  the event and field that supplied each passage.
- Keep identifiers, routes, IPA, derived forms, code examples, and the
  author's own writing outside the prose check.
- Treat count summaries and ledgers separately from narrative paragraphs.

Avoid linting whole source files as plain text. Programming syntax would
add noise, and quote exclusions could hide the very strings being audited.
