import { useRef, useState } from "react";
import { useRegisterSW } from "virtual:pwa-register/react";
import "./pwa.css";

export function PwaNotice() {
  const [error, setError] = useState<string | null>(null);
  const [updating, setUpdating] = useState(false);
  const requestedReload = useRef(false);
  const readyToReload = useRef(false);

  const reload = () => {
    if (window.dispatchEvent(new Event("umran:before-update", { cancelable: true }))) {
      window.location.reload();
    } else {
      requestedReload.current = false;
      setUpdating(false);
      setError("The world could not be saved. Export it before reloading.");
    }
  };

  const {
    needRefresh: [needRefresh, setNeedRefresh],
    updateServiceWorker,
  } = useRegisterSW({
    onNeedReload() {
      readyToReload.current = true;
      // Another open window may have accepted the update. This window stays
      // on its current engine until its reader also chooses to reload.
      if (requestedReload.current) reload();
      else setNeedRefresh(true);
    },
    onRegisterError() {
      setError("Offline setup failed. Reopen online to try again.");
    },
  });

  const update = async () => {
    setError(null);
    if (readyToReload.current) {
      reload();
      return;
    }
    // Do not activate an update if storage has failed. Save once more at
    // reload, since playback can advance while activation is in progress.
    if (!window.dispatchEvent(new Event("umran:before-update", { cancelable: true }))) {
      setError("The world could not be saved. Export it before reloading.");
      return;
    }
    requestedReload.current = true;
    setUpdating(true);
    try {
      await updateServiceWorker();
    } catch {
      requestedReload.current = false;
      setUpdating(false);
      setError("The update could not be applied. Try again when online.");
    }
  };

  if (!needRefresh && !error) return null;

  return (
    <aside className="pwa-notice" aria-label="Umran app">
      <p role="status">
        {error ?? "A new edition of Umran is ready. Save and reload when you are ready."}
      </p>
      <div className="pwa-actions">
        {needRefresh && <button type="button" onClick={() => void update()} disabled={updating}>{updating ? "Updating…" : "Save and reload"}</button>}
        <button type="button" disabled={updating} onClick={() => {
          setNeedRefresh(false);
          setError(null);
        }}>{needRefresh ? "Later" : "Dismiss"}</button>
      </div>
    </aside>
  );
}
