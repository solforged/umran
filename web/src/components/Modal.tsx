import { useEffect, useId, useRef, type ReactNode } from "react";

export function Modal({
  open,
  title,
  onClose,
  children,
  wide = false,
  footer,
  initialFocus,
}: {
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
  const titleId = useId();

  useEffect(() => {
    const node = dialogRef.current;
    if (!node) return;
    if (open) {
      if (!node.open) {
        if (document.activeElement instanceof HTMLElement) returnFocus.current = document.activeElement;
        node.showModal();
      }
      if (initialFocus) node.querySelector<HTMLElement>(initialFocus)?.focus();
    } else if (node.open) {
      node.close();
    }
  }, [open, initialFocus]);

  // Conditional dialogs can unmount without receiving open=false. Restore
  // their caller after the modal has left the document in that case too.
  useEffect(() => () => {
    const target = returnFocus.current;
    queueMicrotask(() => {
      if (target?.isConnected && !document.querySelector("dialog[open]")) target.focus({ preventScroll: true });
    });
  }, []);

  return (
    <dialog
      ref={dialogRef}
      className={wide ? "modal modal-wide" : "modal"}
      aria-labelledby={titleId}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onClose={() => {
        if (!document.querySelector("dialog[open]")) returnFocus.current?.focus();
      }}
    >
      <div className="modal-head">
        <h2 id={titleId}>{title}</h2>
        <button type="button" className="icon-btn" aria-label="Close" onClick={onClose}>
          ×
        </button>
      </div>
      <div className="modal-body">{children}</div>
      {footer ? <div className="modal-foot">{footer}</div> : null}
    </dialog>
  );
}
