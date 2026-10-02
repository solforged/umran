import { useLayoutEffect, useRef, useState } from "react";

export const reducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

export function motionDuration(token: "settle" | "lift" | "turn"): number {
  if (reducedMotion()) return 0;
  return Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue(`--${token}`));
}

// Keep a closing sheet's last account alive, but never delay its next opening.
export function useLiftedValue<T>(value: T | null, onLifted?: () => void) {
  const [held, setHeld] = useState(value);
  const callback = useRef(onLifted);
  callback.current = onLifted;
  useLayoutEffect(() => {
    if (value !== null) { setHeld(value); return; }
    if (held === null) return;
    const finish = () => { setHeld(null); callback.current?.(); };
    const duration = motionDuration("lift");
    if (!duration) { finish(); return; }
    const timer = window.setTimeout(finish, duration);
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    const instant = () => { if (media.matches) { window.clearTimeout(timer); finish(); } };
    media.addEventListener("change", instant);
    return () => { window.clearTimeout(timer); media.removeEventListener("change", instant); };
  }, [value, held]);
  return {
    value: value ?? (reducedMotion() ? null : held),
    closing: value === null && held !== null,
    finishLift: () => { if (value === null && held !== null) { setHeld(null); callback.current?.(); } },
  };
}

// A fresh sheet replaces an interrupted dialog ghost, never stacks on it.
export function closeClosingDialogs() {
  document.querySelectorAll<HTMLDialogElement>("dialog[data-closing][open]").forEach((dialog) => dialog.close());
}

export function emphasizeInk(node: HTMLElement | null) {
  if (!node) return;
  node.getAnimations().forEach((animation) => animation.cancel());
  node.classList.remove("ink-emphasis");
  void node.offsetWidth;
  node.classList.add("ink-emphasis");
  window.setTimeout(() => node.classList.remove("ink-emphasis"), 600);
}
