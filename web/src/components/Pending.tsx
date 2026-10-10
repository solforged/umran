import { Component, Suspense, useEffect, useReducer, useRef, type ReactNode } from "react";
import { message } from "../engine";
import type { Read } from "../model";

/// Where part of the page waits on the engine's thread: a quiet line while it
/// reads, and the engine's reason in place if the reading fails. A new
/// `reading` clears a failure, since the next reading may succeed.
export function Pending({ children, reading, label = "Reading…" }: { children: ReactNode; reading?: unknown; label?: string }) {
  return <Failure reading={reading}>
    <Suspense fallback={<p className="pending" role="status">{label}</p>}>{children}</Suspense>
  </Failure>;
}

class Failure extends Component<{ children: ReactNode; reading: unknown }, { error: unknown; reading: unknown }> {
  state = { error: null as unknown, reading: this.props.reading };
  static getDerivedStateFromError(error: unknown) { return { error }; }
  static getDerivedStateFromProps(props: { reading: unknown }, state: { error: unknown; reading: unknown }) {
    return props.reading === state.reading ? null : { error: null, reading: props.reading };
  }
  render() {
    return this.state.error === null ? this.props.children
      : <p className="notice error" role="alert">{message(this.state.error)}</p>;
  }
}

/// A read's answer without suspending: undefined until it settles, or, with
/// `keep`, the last answer this component showed. For the chart's own layers,
/// which should go on drawing last year's names rather than blank.
export function useAnswer<T>(read: Read<T> | null, keep = false): T | undefined {
  const [, settled] = useReducer((n: number) => n + 1, 0);
  const last = useRef<T | undefined>(undefined);
  useEffect(() => {
    if (!read || read.status !== "pending") return;
    let live = true;
    read.then(() => live && settled(), () => live && settled());
    return () => { live = false; };
  }, [read]);
  if (read?.status === "rejected") throw read.reason;
  if (read?.status === "fulfilled") last.current = read.value;
  else if (!keep) return undefined;
  return last.current;
}
