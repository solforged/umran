import type { Catalog, Naming } from "../model";
import type { Choice } from "./Phrase";

export function encodeNaming(n: Naming | null): string {
  return n ? JSON.stringify(n) : "";
}

export function decodeNaming(key: string): Naming | null {
  return key ? JSON.parse(key) as Naming : null;
}

/// The name meanings shared by inline phrases and native selectors.
export function namingChoices(catalog: Catalog, daughter = false, parent?: string): Choice[] {
  const of = daughter && parent ? parent : "people";
  return [
    ...(daughter ? [{ key: "", text: "Let them choose" }] : [
      { key: encodeNaming({ kind: "people" }), text: "“the people”" },
      { key: encodeNaming({ kind: "speakers" }), text: "“those who speak”" },
    ]),
    ...catalog.namePlaces.map((place) => ({
      key: encodeNaming({ kind: "place", place }),
      text: `“the people of the ${place}”`,
      group: "From their land",
    })),
    ...catalog.nameEpithets.map((epithet) => ({
      key: encodeNaming({ kind: "epithet", epithet }),
      text: `“the ${epithet} ${of}”`,
      group: "With an epithet",
    })),
  ];
}

/// Choose what a people's name means; the engine builds it from their words.
/// With `daughter`, the empty choice lets the new community name itself and
/// epithets qualify the parent's name rather than "people".
export function NamingSelect({
  catalog,
  value,
  onChange,
  daughter = false,
  parent,
}: {
  catalog: Catalog;
  value: Naming | null;
  onChange: (naming: Naming | null) => void;
  daughter?: boolean;
  parent?: string;
}) {
  const choices = namingChoices(catalog, daughter, parent);
  return (
    <select value={encodeNaming(value)} onChange={(e) => onChange(decodeNaming(e.target.value))}>
      {choices.filter((choice) => choice.group === undefined).map((choice) => (
        <option key={choice.key} value={choice.key}>{choice.text}</option>
      ))}
      {["From their land", "With an epithet"].map((group) => (
        <optgroup key={group} label={group}>
          {choices.filter((choice) => choice.group === group).map((choice) => (
            <option key={choice.key} value={choice.key}>{choice.text}</option>
          ))}
        </optgroup>
      ))}
    </select>
  );
}
