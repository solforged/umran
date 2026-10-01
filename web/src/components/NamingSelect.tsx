import type { Catalog, Naming } from "../model";

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
  const encode = (n: Naming | null) => (n ? JSON.stringify(n) : "");
  const of = daughter && parent ? parent : "people";
  return (
    <select value={encode(value)} onChange={(e) => onChange(e.target.value ? (JSON.parse(e.target.value) as Naming) : null)}>
      {daughter ? <option value="">Let them choose</option> : null}
      {daughter ? null : (
        <>
          <option value={encode({ kind: "people" })}>“the people”</option>
          <option value={encode({ kind: "speakers" })}>“those who speak”</option>
        </>
      )}
      <optgroup label="From their land">
        {catalog.namePlaces.map((place) => (
          <option key={place} value={encode({ kind: "place", place })}>
            “the people of the {place}”
          </option>
        ))}
      </optgroup>
      <optgroup label="With an epithet">
        {catalog.nameEpithets.map((epithet) => (
          <option key={epithet} value={encode({ kind: "epithet", epithet })}>
            “the {epithet} {of}”
          </option>
        ))}
      </optgroup>
    </select>
  );
}
