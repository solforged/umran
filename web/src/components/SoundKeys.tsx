import { useMemo, type RefObject } from "react";
import { catalogSounds, soundHints, writeHint } from "../engine";
import type { SoundHint } from "../model";

type Field = HTMLInputElement | HTMLTextAreaElement;

/// The law notation's own marks, offered beside a language's sounds.
const NOTATION = [">", "/", "_", "#", "∅", "C", "V"];

/// A language's sounds as keys that type their catalog IPA where the caret
/// is, so the author need not hunt for ʃ or ɾ; and, beneath, the sounds the
/// text seems to write as a romanization would (`sh` for ʃ), each a button
/// that writes it in IPA throughout. The engine reads the hints and writes
/// them; this component never edits a form itself.
export function SoundKeys({ sounds, field, value, sounding, onChange, onHint, notation = false, long = true }: {
  /// The language's sounds in catalog IPA, or null where none is chosen yet.
  sounds: string[] | null;
  field: RefObject<Field | null>;
  value: string;
  /// The part of the value that writes sounds, where the rest names meanings.
  sounding?: string;
  onChange: (value: string) => void;
  /// Writes a hint's IPA where the text means sounds; by default, everywhere.
  onHint?: (hint: SoundHint) => void;
  /// Offer the law notation's marks and read the text as notation.
  notation?: boolean;
  /// Offer the length mark.
  long?: boolean;
}) {
  const catalog = catalogSounds();
  const keys = useMemo(() => {
    const have = sounds ? new Set(sounds) : null;
    const known = have ? catalog.filter((sound) => have.has(sound.ipa)) : [];
    return { consonants: known.filter((sound) => !sound.vowel), vowels: known.filter((sound) => sound.vowel) };
  }, [catalog, sounds]);
  const read = sounding ?? value;
  const hints = useMemo(() => read.trim() ? soundHints(read, sounds, notation) : [], [read, sounds, notation]);
  const type = (text: string) => {
    const node = field.current;
    const start = node?.selectionStart ?? value.length;
    const end = node?.selectionEnd ?? value.length;
    onChange(value.slice(0, start) + text + value.slice(end));
    requestAnimationFrame(() => {
      if (!node) return;
      node.focus();
      node.setSelectionRange(start + text.length, start + text.length);
    });
  };
  const key = (ipa: string, title: string) => <button type="button" key={ipa} className="sound-key" title={title}
    // Keep the caret in the field while the key is pressed.
    onMouseDown={(event) => event.preventDefault()} onClick={() => type(ipa)}>{ipa}</button>;
  return <div className="sound-keys">
    {keys.consonants.length || keys.vowels.length ? <div className="sound-keys-row" role="group" aria-label="Type a sound of this language">
      {keys.consonants.map((sound) => key(sound.ipa, `${sound.ipa}, written ${sound.roman} in the catalog`))}
      <span className="sound-keys-gap" aria-hidden="true" />
      {keys.vowels.map((sound) => key(sound.ipa, `${sound.ipa}, written ${sound.roman} in the catalog`))}
      {long ? <><span className="sound-keys-gap" aria-hidden="true" />{key("ː", "long: after a vowel or consonant")}</> : null}
      {notation ? <><span className="sound-keys-gap" aria-hidden="true" />{NOTATION.map((mark) => key(mark, `notation: ${mark}`))}</> : null}
    </div> : null}
    {hints.length ? <p className="sound-keys-hints small" role="status">In the catalog’s IPA: {hints.map((hint, i) => <span key={hint.written}>{i ? ", " : ""}
      <button type="button" className="link" title={`Write ${hint.ipa} for every ${hint.written}`} onClick={() => onHint ? onHint(hint) : onChange(writeHint(value, sounds, notation, hint.written))}>
        <code>{hint.written}</code> is <code>{hint.ipa}</code></button></span>)}.</p> : null}
  </div>;
}
