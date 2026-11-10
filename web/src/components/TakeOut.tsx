// A line offering files the engine writes from one reading: a lexicon, a
// law ledger, a family tree. The page names what to take and saves what
// comes back; every file is serialized in the facade's `takeout.rs`.

import { Download } from "lucide-react";
import { use, useState, type ReactNode } from "react";
import { EditionContext } from "../edition";
import { message } from "../engine";
import type { ReadEngine, Take } from "../model";
import { saveTaken } from "../takeout";

export function TakeOut({ engine, generation, choices, title, children }: {
  engine: ReadEngine;
  generation: number;
  /// Each file offered: what its link says, and what it asks the engine for.
  choices: [label: string, what: Take][];
  /// Leads the file's name, for a file that is not one language's.
  title?: string;
  /// What the files hold, said beside the links.
  children?: ReactNode;
}) {
  const { complete } = use(EditionContext);
  const [working, setWorking] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  // An edition is the book itself; it offers no downloads.
  if (complete) return null;
  const take = (what: Take) => {
    setWorking(true);
    setFailed(null);
    engine.take(generation, what)
      .then((taken) => saveTaken(taken, title), (error: unknown) => setFailed(message(error)))
      .finally(() => setWorking(false));
  };
  return <p className="compared-take" data-edition-omit>
    {choices.map(([label, what]) => <button key={label} type="button" className="link" disabled={working} onClick={() => take(what)}>
      <Download size={13} aria-hidden="true" /> {label}</button>)}
    {failed !== null ? <span className="small" role="alert">{failed}</span> : children ? <span className="muted small">{children}</span> : null}
  </p>;
}
