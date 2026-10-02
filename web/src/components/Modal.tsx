import { useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { closeClosingDialogs, motionDuration } from "../motion";

export function Modal({ open, title, onClose, children, wide = false, footer, initialFocus }: {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  wide?: boolean;
  footer?: ReactNode;
  initialFocus?: string;
}) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const [closing, setClosing] = useState(false);
  const lifting = useRef(false);
  const titleId = useId();

  useLayoutEffect(() => {
    const node = dialogRef.current;
    if (!node) return;
    if (open) {
      if (lifting.current) node.close();
      lifting.current = false;
      closeClosingDialogs();
      setClosing(false);
      node.removeAttribute("data-closing");
      node.removeAttribute("aria-hidden");
      node.inert = false;
      if (!node.open) {
        if (!returnFocus.current && document.activeElement instanceof HTMLElement) returnFocus.current = document.activeElement;
        node.showModal();
        if (initialFocus) node.querySelector<HTMLElement>(initialFocus)?.focus();
      }
      return;
    }
    if (!node.open) return;
    const duration = motionDuration("lift");
    // A modal makes its caller inert. Release the top layer before returning
    // focus, then keep a non-modal, inaccessible paper ghost for the lift.
    lifting.current = true;
    node.close();
    if (duration) {
      node.dataset.closing = "true";
      node.setAttribute("aria-hidden", "true");
      node.inert = true;
      node.show();
      setClosing(true);
    }
    returnFocus.current?.focus({ preventScroll: true });
    const finish = () => { node.close(); lifting.current = false; setClosing(false); returnFocus.current = null; };
    if (!duration) { finish(); return; }
    const timer = window.setTimeout(finish, duration);
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    const instant = () => { if (media.matches) { window.clearTimeout(timer); finish(); } };
    media.addEventListener("change", instant);
    return () => { window.clearTimeout(timer); media.removeEventListener("change", instant); };
  }, [open, initialFocus]);

  useLayoutEffect(() => () => {
    const target = returnFocus.current;
    dialogRef.current?.close();
    queueMicrotask(() => {
      if (target?.isConnected && !document.querySelector("dialog:modal") && document.activeElement === document.body) target.focus({ preventScroll: true });
    });
  }, []);

  return <>
    {closing ? createPortal(<div className="modal-lift-backdrop" aria-hidden="true" />, document.body) : null}
    <dialog ref={dialogRef} className={wide ? "modal modal-wide" : "modal"} aria-labelledby={titleId}
      onCancel={(event) => { event.preventDefault(); onClose(); }}>
      <div className="modal-head">
        <h2 id={titleId}>{title}</h2>
        <button type="button" className="icon-btn" aria-label="Close" onClick={onClose}>×</button>
      </div>
      <div className="modal-body">{children}</div>
      {footer ? <div className="modal-foot">{footer}</div> : null}
    </dialog>
  </>;
}
