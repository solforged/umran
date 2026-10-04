import type { ClimateView, LakeNamesView, NameView, Overview, PlaceExonym, PlaceName, RiverNamesView, Subject, WorldMap } from "./model";
import { YEARS } from "./model";
import { TERRAIN_NAME, weatherDeparture } from "./lore";

export type MapFeature =
  | { kind: "land"; region: number }
  | { kind: "city"; city: number; region: number }
  | { kind: "river"; river: number; region: number }
  | { kind: "lake"; lake: number; region: number }
  | { kind: "continent"; landmass: number; region: number }
  | { kind: "shrine"; religion: number; region: number }
  | { kind: "people"; community: number; region: number }
  | { kind: "state"; state: number; region: number };

/// One language's attested word for the feature.
export interface Saying {
  variety: number;
  language: string;
  spelled: string;
  ipa: string;
  meaning?: string;
  relation: "holders" | "selected" | "exonym" | "former" | "city";
  since?: number;
}

export interface Gazette {
  feature: MapFeature;
  title: string;
  subtitle?: string;
  sayings: Saying[];
  facts: { label: string; value: string }[];
  open: { label: string; subject: Subject } | null;
}

export interface GazetteOptions {
  selectedVariety?: number;
  riverNames?: RiverNamesView[];
  lakeNames?: LakeNamesView[];
  climate?: ClimateView;
  /// The same known-world lens as the map; unknown land reveals no names.
  known?: ReadonlySet<number> | null;
}

export function featureKey(feature: MapFeature): string {
  switch (feature.kind) {
    case "land": return `land:${feature.region}`;
    case "city": return `city:${feature.city}`;
    case "river": return `river:${feature.river}`;
    case "lake": return `lake:${feature.lake}`;
    case "continent": return `continent:${feature.landmass}`;
    case "shrine": return `shrine:${feature.religion}:${feature.region}`;
    case "people": return `people:${feature.community}`;
    case "state": return `state:${feature.state}`;
  }
}

const count = (value: number) => Math.round(value).toLocaleString();
const year = (generation: number) => `year ${generation * YEARS}`;

function sayingsFor(names: PlaceName[], exonyms: PlaceExonym[], overview: Overview, generation: number, selected?: number): Saying[] {
  const held = names.filter((name) => name.since <= generation);
  const current = held.at(-1);
  const living = exonyms.filter((name) => name.heard <= generation && overview.varieties.find((v) => v.id === name.variety)?.spoken);
  const selectedName = selected === undefined ? undefined
    : living.find((name) => name.variety === selected) ?? held.findLast((name) => name.variety === selected);
  const out: Saying[] = [];
  const same = (a: PlaceName | PlaceExonym, b: PlaceName | PlaceExonym | undefined) =>
    b !== undefined && a.variety === b.variety && a.spelled === b.spelled && a.ipa === b.ipa;
  const add = (name: PlaceName | PlaceExonym, relation: Saying["relation"]) => out.push({
    variety: name.variety, language: name.language, spelled: name.spelled, ipa: name.ipa,
    ...("meaning" in name ? { meaning: name.meaning } : {}),
    since: "since" in name ? name.since : name.heard, relation,
  });
  if (selectedName) add(selectedName, "selected");
  if (current && !same(current, selectedName)) add(current, "holders");
  for (const name of [...living].sort((a, b) => a.language.localeCompare(b.language) || a.variety - b.variety)) {
    if (!same(name, selectedName) && !same(name, current)) add(name, "exonym");
  }
  for (const name of held.slice(0, -1).reverse().filter((name) => !same(name, selectedName)).slice(0, 3)) add(name, "former");
  return out;
}

/// Arrange facade evidence, never derive a word or its pronunciation here.
export function gazette(feature: MapFeature, map: WorldMap, overview: Overview, generation: number, options: GazetteOptions): Gazette {
  const region = map.regions.find((r) => r.id === feature.region);
  const fallback = region?.terrain === "sea" ? "Sea" : region ? `Unnamed ${TERRAIN_NAME[region.terrain].toLowerCase()}` : "Unknown land";
  const result: Gazette = { feature, title: fallback, sayings: [], facts: [], open: null };
  if (region?.terrain !== "sea" && options.known && !options.known.has(feature.region)) {
    result.title = "Unknown land";
    return result;
  }
  const place = overview.places.find((p) => p.region === feature.region);
  const landSayings = sayingsFor(place?.names ?? [], place?.exonyms ?? [], overview, generation, options.selectedVariety);
  const landName = landSayings[0]?.spelled ?? fallback;
  const named = (name: NameView, variety: number, since?: number): Saying => ({
    variety, language: overview.varieties.find((v) => v.id === variety)?.name ?? "Unrecorded language",
    spelled: name.name, ipa: name.ipa, meaning: name.meaning, relation: "holders", since,
  });
  const open = (subject: Subject, label: string) => { result.open = { subject, label }; };
  const fact = (label: string, value: string) => result.facts.push({ label, value });
  switch (feature.kind) {
    case "land": {
      result.sayings = landSayings;
      if (region) {
        fact("Terrain", TERRAIN_NAME[region.terrain]);
        fact("Area", `${count(region.areaKm2)} km²`);
        const peoples = overview.communities.filter((c) => c.ended === null && c.lands.includes(region.id));
        if (peoples.length) fact("Peoples", peoples.map((c) => c.name).join(", "));
        const zone = options.climate?.regions.find((r) => r.id === region.id)?.zone;
        const weather = zone === null || zone === undefined ? undefined : options.climate?.zones.find((z) => z.id === zone);
        if (weather) fact("Weather", weatherDeparture(weather));
        if (region.terrain !== "sea") open({ kind: "land", region: region.id }, "Open land");
      }
      break;
    }
    case "city": {
      const city = overview.cities.find((c) => c.id === feature.city);
      if (!city || city.since > generation) break;
      const state = overview.states.find((s) => s.id === city.state);
      const variety = state?.standard ?? overview.communities.find((c) => c.id === state?.rulers)?.variety;
      result.title = city.name.name;
      result.subtitle = `stands on ${landName}`;
      result.sayings = [...(variety === undefined ? [] : [{ ...named(city.name, variety, city.since), relation: "city" as const }]), ...landSayings];
      fact("Size", `${count(city.size)} souls`);
      fact("Since", year(city.since));
      if (city.makeup.length) fact("Speech", city.makeup.map((share) => `${overview.varieties.find((v) => v.id === share.variety)?.name ?? "Unrecorded language"} ${Math.round(share.share * 100)}%`).join(", "));
      if (state) open({ kind: "state", id: state.id }, "Open state");
      return result;
    }
    case "river": {
      const river = map.rivers.find((r) => r.id === feature.river);
      const names = options.riverNames?.find((r) => r.river === feature.river);
      result.sayings = sayingsFor(names?.names ?? [], names?.exonyms ?? [], overview, generation, options.selectedVariety);
      result.title = "Unnamed river";
      if (river) {
        fact("Length", `${count(river.lengthKm)} km`);
        fact("Catchment", `${river.catchment.length} lands`);
        if (options.climate?.rivers.find((r) => r.id === river.id)?.flowing === false) fact("Flow", "Has failed");
        open({ kind: "river", id: river.id }, "Open river");
      }
      break;
    }
    case "lake": {
      const lake = map.lakes.find((lake) => lake.id === feature.lake);
      const names = options.lakeNames?.find((view) => view.lake === feature.lake);
      result.sayings = sayingsFor(names?.names ?? [], names?.exonyms ?? [], overview, generation, options.selectedVariety);
      result.title = "Unnamed lake";
      if (lake) {
        fact("Shore lands", String(lake.regions.length));
        const outlet = options.riverNames?.find((view) => view.river === lake.outlet);
        fact("Outlet", lake.outlet === null ? "Closed basin"
          : sayingsFor(outlet?.names ?? [], outlet?.exonyms ?? [], overview, generation, options.selectedVariety)[0]?.spelled ?? "Unnamed river");
        open({ kind: "lake", id: lake.id }, "Open lake");
      }
      break;
    }
    case "continent": {
      const continent = overview.continents.find((c) => c.landmass === feature.landmass);
      const mass = map.landmasses.find((m) => m.id === feature.landmass);
      if (continent?.name && continent.name.since <= generation) result.sayings = [named(continent.name, continent.name.variety, continent.name.since)];
      result.title = "Unnamed continent";
      if (mass) {
        fact("Lands", String(mass.regions.length));
        fact("Area", `${count(mass.regions.reduce((sum, id) => sum + (map.regions.find((r) => r.id === id)?.areaKm2 ?? 0), 0))} km²`);
        open({ kind: "continent", landmass: mass.id }, "Open continent");
      }
      break;
    }
    case "shrine": {
      const religion = overview.religions.find((r) => r.id === feature.religion);
      const shrine = religion?.shrines.find((s) => s.region === feature.region);
      if (religion) {
        result.sayings = [named(shrine?.name ?? religion, religion.sacred, religion.founded)];
        result.subtitle = shrine ? `holy to ${religion.name}` : "founded here";
        fact("Faith", religion.name);
        open({ kind: "religion", id: religion.id }, "Open faith");
      }
      break;
    }
    case "people": {
      const people = overview.communities.find((c) => c.id === feature.community);
      if (!people) break;
      const holder = named(people, people.variety, people.coined);
      const exonyms: Saying[] = people.exonyms.flatMap((name) => {
        const speaker = overview.communities.find((c) => c.id === name.by);
        const language = overview.varieties.find((v) => v.id === speaker?.variety);
        // The facade gives no IPA for a people's exonyms; do not invent it.
        return speaker?.ended === null && language?.spoken ? [{ variety: language.id, language: language.name, spelled: name.name, ipa: "", relation: "exonym" as const }] : [];
      }).sort((a, b) => a.language.localeCompare(b.language));
      const selected = exonyms.find((n) => n.variety === options.selectedVariety) ?? (holder.variety === options.selectedVariety ? holder : undefined);
      result.sayings = [...(selected ? [{ ...selected, relation: "selected" as const }] : []), ...(selected === holder ? [] : [holder]), ...exonyms.filter((n) => n !== selected)];
      fact("Size", `${count(people.size)} souls`);
      fact("Speech", holder.language);
      open({ kind: "people", id: people.id }, "Open people");
      break;
    }
    case "state": {
      const state = overview.states.find((s) => s.id === feature.state);
      if (!state) break;
      const variety = overview.communities.find((c) => c.id === state.rulers)?.variety;
      if (variety !== undefined) result.sayings = [named(state, variety, state.rose)];
      result.title = state.name;
      fact("Capital", overview.cities.find((c) => c.state === state.id)?.name.name ?? overview.places.find((p) => p.region === state.capital)?.names.at(-1)?.spelled ?? "Unnamed land");
      fact(state.fell === null ? "Standing since" : "Rose", year(state.rose));
      if (state.fell !== null) fact("Fell", year(state.fell));
      open({ kind: "state", id: state.id }, "Open state");
      break;
    }
  }
  result.title = result.sayings[0]?.spelled ?? result.title;
  return result;
}
