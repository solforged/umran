import { Fragment, type ReactNode } from "react";
import type { Annal, Subject } from "../model";

/// The annalist's text stays one inline flow. Named spans retain their
/// recorded subjects; other words marked *thus* keep the language's style.
export function Told({ text, names = [], link, go }: {
  text: string;
  names?: Annal["names"];
  link?: (subject: Subject, label: ReactNode) => ReactNode;
  go?: (subject: Subject) => void;
}) {
  let offset = 0;
  let nextName = 0;
  const parts = text.split(/\*([^*]+)\*/).map((part, i) => {
    const word = i % 2 === 1;
    if (word) offset++;
    const start = offset;
    const end = start + part.length;
    const content: ReactNode[] = [];
    let cursor = start;
    while (nextName < names.length && names[nextName].start < end) {
      const name = names[nextName++];
      const through = name.start + name.text.length;
      if ((!link && !go) || name.start < start || through > end) continue;
      content.push(text.slice(cursor, name.start));
      content.push(<Fragment key={name.start}>{link ? link(name.subject, name.text)
        : <button type="button" className="link" onClick={() => go!(name.subject)}>{name.text}</button>}</Fragment>);
      cursor = through;
    }
    content.push(text.slice(cursor, end));
    offset = end + (word ? 1 : 0);
    return word ? <i key={i} className="word">{content}</i> : <Fragment key={i}>{content}</Fragment>;
  });
  return <>{parts}</>;
}
