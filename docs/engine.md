# Engine model

How `crates/langgen-sim` models language change. The rules that must
always hold are summarized in `AGENTS.md`; this is the fuller picture.

- `World` (`world.rs`) steps communities, varieties, and contacts through
  25-year generations. `Variety` holds a `SoundProfile` and a `Lexicon` of
  `Slot`s, where words compete for each concept with usage weights. Words
  keep a log of every sound law, borrowing, extension, and loss.
- Languages are founded from a `LanguageDesign` (`design.rs`): the exact
  sounds, each used or favoured, plus knobs (word length, final consonants,
  inner clusters, repetition, long vowels, affixes or root-and-pattern,
  suffixing, derivation) and spelling. It resolves to an internal
  `SoundProfile`: chosen sounds exactly, favoured ones preferred, absent
  ones discouraged but reachable by sound change. There are no named
  culture packs; `SoundProfile::base()` weights sounds by world frequency
  for "draw by world frequency", and presets (the base plus a `Flavor` from
  `flavor.rs` or `palettes.rs`) only produce starting designs to edit.
- One preference score, taste plus cross-linguistic frequency from PHOIBLE
  (`typology.rs`), drives inventories, how often sounds are used, sound-law
  odds, and acceptance of foreign sounds. Sampled inventories favour
  feature economy (series like b d g), and a forbidden sound beats the
  "voiced implies voiceless" repair (Arabic has b without p).
- The catalog (`phoneme.rs`) describes consonants by place, manner,
  voicing, and a `Secondary` articulation (aspirated, breathy,
  labialized); new segments are appended, never inserted. Vowel length is
  a per-segment flag that profiles can make contrastive.
- Concepts (`concepts.rs`): the Leipzig–Jakarta 100 in rank order plus
  cultural concepts with a `Tier`. Word length follows `length_bias`
  (Zipf's law of abbreviation: basic meanings short, specialist long, one
  to three syllables). Parent words usually follow the nursery pattern
  (mama, papa). Only `expressive` meanings (small things, insects and
  birds, cries and sounds, baby talk) reduplicate or repeat consonants;
  others avoid it. Roots are unique within a semantic field, with a weak
  sound-symbolic bias. Minting skips spellings that read as English
  vulgarities, a courtesy rather than a linguistic claim.
- Word families (`FAMILIES`, `morphology.rs`): each language mints its own
  affixes, or vowel patterns over consonant roots for root-and-pattern
  (`MorphologyKind::RootPattern`), and builds some family members from
  their bases (`Origin::Derived`). Junctions get a link vowel or a glide as
  the language needs. Root-and-pattern derivations use each word's true
  root skeleton, never a surface reading that includes a pattern's prefix.
  New words come from curated semantic shifts (`RELATED`) or fresh roots.
- Sound laws (`laws.rs`) apply simultaneously and regularly to every living
  word, never to obsolete ones, and never delete a word's last vowel.
  "No change" competes with them, so a culture is never forced into a law.
- Each language has a minimal word (`prosody.rs`), drawn at founding: any
  syllable, a heavy one (two moras), or two syllables, likelier the more
  disyllabic its roots. A rule that would wear a word below it passes that
  word by, as apocope spares short words. Words below the minimum, or
  sounding like another word in use, draw competitors more often and lose
  usage; about half of those competitors are the old word renewed, with
  the language's renewing affix (Latin auris > auricula) or compounded
  with a related concept's word (Mandarin ěr > ěrduo). Renewed words keep
  their root, so they count as retained. `examples/length.rs` measures
  word length, homophony, and renewal over time.
- Borrowability is set per concept (Leipzig–Jakarta rank or `Tier`), never
  per semantic field, so field patterns must emerge. `wold.rs` holds WOLD
  figures for validation only; the model never reads them. Loans adapt to
  the recipient's established sounds and then undergo only later laws.
- Names (`names.rs`) are words: a people's name is coined from its own
  lexicon by a `Naming` (the people, those who speak, people of a place,
  or an epithet on an older name), and its language is named after it
  with the language's belonging affix or a compound with "tongue" or
  "word" (`NameRules` in `morphology.rs`, drawn after all other founding
  draws). Names are clipped to three syllables for a people and four for
  a language, as names said every day are, and epithets go only on short
  names, so they do not stack over many splits. Names then undergo the
  same sound laws as their variety's words. Split-off peoples name themselves; a people that shifts keeps
  its name and names its new speech after itself. Exonyms are the name
  adapted to a contact's sounds, computed from current forms.
- Varieties fork on splits and shifts and keep their lineage (`Fork`);
  `World::cognate` and `root_of` give true descent. The comparative method
  (`compare.rs`) must never read lineage; it is only graded against it.
- Communities grow within their founders' territory, split when large, and
  take prestige from authored `power` plus relative size. A community shifts
  language only to another family's, keeping its own sound preferences and
  some old words as a substrate. Unspoken varieties are extinct and frozen.
- Social identity, territory, ancestry, and language are independent.
- Every random draw comes from a ChaCha8 stream keyed by purpose
  (`rng.rs`), so adding a process never shifts existing draws.

## Studying one mechanism

Each example prints a readable report; profiles are preset ids
(a flavor such as `familiar`, `germanic`, `semitic`, `polynesian`).

```sh
cargo run --release -p langgen-sim --example found -- <seed> <profile>
cargo run --release -p langgen-sim --example drift -- <seed> <profile> <generations> [flavor...]
cargo run --release -p langgen-sim --example contact -- <seed> <donor> <recipient> <kind> <generations> <seeds>
cargo run --release -p langgen-sim --example family -- <seed> <proto> <outsider> <generations>
cargo run --release -p langgen-sim --example history -- <seed> <generations>
cargo run --release -p langgen-sim --example calibrate -- <seeds> <generations> [profile]
cargo run --release -p langgen-sim --example length -- [seeds] [generations]
cargo run --release -p langgen-sim --example audit -- [seeds] [generations]
```

## Not yet modelled

Places and migration (territories stand in for a map), compounding and
derivation after founding beyond renewal, inflection, stress, tone, vowel harmony, consonant length, prenasalized
stops, syntax and alignment, dialect levelling, personal and place names.
