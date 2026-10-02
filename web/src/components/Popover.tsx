import { useCallback, useId, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";

export type PopoverSide = "bottom" | "top";
export type PopoverAlign = "start" | "end";

/** Spread these props onto the trigger button. */
export interface TriggerProps {
  ref: RefObject<HTMLButtonElement | null>;
  onClick: () => void;
  "aria-expanded": boolean;
  "aria-haspopup": "menu" | "dialog";
  "aria-controls": string;
}

let activeClose: (() => void) | null = null;

function focusables(panel: HTMLElement): HTMLElement[] {
  return Array.from(panel.querySelectorAll<HTMLElement>("button, input, select, textarea, a[href], [tabindex]"))
    .filter((element) => element.tabIndex >= 0 && !element.matches(":disabled")
      && !element.closest("[inert]") && element.getClientRects().length > 0
      && getComputedStyle(element).visibility !== "hidden");
}

export function Popover({
  label,
  role = "menu",
  side = "bottom",
  align = "start",
  trigger,
  children,
  onOpenChange,
}: {
  label: string;
  role?: "menu" | "dialog";
  side?: PopoverSide;
  align?: PopoverAlign;
  trigger: (props: TriggerProps, open: boolean) => ReactNode;
  children: (close: () => void) => ReactNode;
  onOpenChange?: (open: boolean) => void;
}): ReactNode {
  const id = useId();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const openRef = useRef(false);
  const onOpenChangeRef = useRef(onOpenChange);
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0, side });

  useLayoutEffect(() => {
    onOpenChangeRef.current = onOpenChange;
  }, [onOpenChange]);

  const close = useCallback((): void => {
    if (!openRef.current) return;
    openRef.current = false;
    if (activeClose === close) activeClose = null;
    if (panelRef.current?.contains(document.activeElement)) {
      triggerRef.current?.focus({ preventScroll: true });
    }
    setOpen(false);
    onOpenChangeRef.current?.(false);
  }, []);

  const toggle = useCallback(() => {
    if (openRef.current) {
      close();
      return;
    }
    activeClose?.();
    activeClose = close;
    openRef.current = true;
    setOpen(true);
    onOpenChangeRef.current?.(true);
  }, [close]);

  useLayoutEffect(() => () => {
    if (activeClose === close) activeClose = null;
    openRef.current = false;
  }, [close]);

  useLayoutEffect(() => {
    if (!open) return;
    const measure = () => {
      const button = triggerRef.current;
      const panel = panelRef.current;
      if (!button || !panel) return;
      const anchor = button.getBoundingClientRect();
      const { width, height } = panel.getBoundingClientRect();
      const preferredTop = side === "bottom" ? anchor.bottom + 4 : anchor.top - height - 4;
      const overflows = side === "bottom" ? preferredTop + height > window.innerHeight : preferredTop < 0;
      const actualSide = overflows ? (side === "bottom" ? "top" : "bottom") : side;
      const top = actualSide === "bottom" ? anchor.bottom + 4 : anchor.top - height - 4;
      const alignedLeft = align === "start" ? anchor.left : anchor.right - width;
      const left = Math.max(8, Math.min(alignedLeft, window.innerWidth - width - 8));
      setPosition((current) => current.top === top && current.left === left && current.side === actualSide
        ? current : { top, left, side: actualSide });
    };
    measure();
    window.addEventListener("resize", measure);
    window.addEventListener("scroll", measure, true);
    return () => {
      window.removeEventListener("resize", measure);
      window.removeEventListener("scroll", measure, true);
    };
  }, [open, side, align]);

  useLayoutEffect(() => {
    if (!open) return;
    const panel = panelRef.current;
    if (panel) focusables(panel)[0]?.focus({ preventScroll: true });

    const outsidePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (target instanceof Node && !panelRef.current?.contains(target) && !triggerRef.current?.contains(target)) {
        close();
      }
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      close();
      triggerRef.current?.focus({ preventScroll: true });
    };
    document.addEventListener("pointerdown", outsidePointerDown, true);
    document.addEventListener("keydown", escape, true);
    return () => {
      document.removeEventListener("pointerdown", outsidePointerDown, true);
      document.removeEventListener("keydown", escape, true);
    };
  }, [open, close]);

  return (
    <>
      {trigger({
        ref: triggerRef,
        onClick: toggle,
        "aria-expanded": open,
        "aria-haspopup": role,
        "aria-controls": id,
      }, open)}
      {open && createPortal(
        <div
          ref={panelRef}
          className="popover-panel"
          role={role}
          aria-label={label}
          id={id}
          data-side={position.side}
          data-align={align}
          style={{ top: position.top, left: position.left }}
          onKeyDown={(event) => {
            if (role !== "menu" || !["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
            const items = focusables(event.currentTarget);
            if (!items.length) return;
            event.preventDefault();
            const current = items.indexOf(document.activeElement as HTMLElement);
            const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1
              : event.key === "ArrowDown" ? (current + 1) % items.length
                : (current - 1 + items.length) % items.length;
            items[next]?.focus({ preventScroll: true });
          }}
        >
          {children(close)}
        </div>,
        document.body,
      )}
    </>
  );
}
