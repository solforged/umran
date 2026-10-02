import { Fragment, type ReactNode } from "react";
import { Popover } from "./Popover";

export interface Choice {
  /// Stable key; "" is allowed (the default/unset choice).
  key: string;
  /// The phrase as it reads in the sentence.
  text: string;
  /// Choices with the same group sit together in first-seen order.
  group?: string;
  /// Optional tooltip for the menu row.
  title?: string;
}

/// A choice set as an inline phrase; native buttons cannot wrap with prose.
export function Phrase({ label, value, choices, onChange }: {
  label: string;
  value: string;
  choices: Choice[];
  onChange: (key: string) => void;
}): ReactNode {
  const selected = choices.find((choice) => choice.key === value) ?? choices[0];
  const groups = new Map<string | undefined, Choice[]>();
  for (const choice of choices) {
    const group = groups.get(choice.group);
    if (group) group.push(choice);
    else groups.set(choice.group, [choice]);
  }

  return (
    <Popover<HTMLSpanElement>
      label={label}
      role="menu"
      side="bottom"
      align="start"
      trigger={(props) => (
        <span
          role="button"
          tabIndex={0}
          className="phrase"
          aria-label={label}
          {...props}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              props.onClick();
            }
          }}
        >
          {selected?.text}
        </span>
      )}
    >
      {(close) => Array.from(groups, ([group, entries]) => (
        <Fragment key={group === undefined ? "ungrouped" : `group:${group}`}>
          {group !== undefined ? <small className="phrase-group">{group}</small> : null}
          {entries.map((choice) => (
            <button
              key={choice.key}
              type="button"
              className="phrase-choice"
              role="menuitemradio"
              aria-checked={choice.key === selected?.key}
              title={choice.title}
              onClick={() => {
                onChange(choice.key);
                close();
              }}
            >
              {choice.text}
            </button>
          ))}
        </Fragment>
      ))}
    </Popover>
  );
}
