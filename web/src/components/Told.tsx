import type { ReactNode } from "react";

/// The annalist's text, with words of the language (marked *thus*) set
/// in the language's style.
export function Told({ text }: { text: string }) {
  const parts: ReactNode[] = text.split(/\*([^*]+)\*/).map((part, i) =>
    i % 2 === 1 ? (
      <i key={i} className="word">
        {part}
      </i>
    ) : (
      part
    ),
  );
  return <>{parts}</>;
}
