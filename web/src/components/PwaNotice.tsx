import { useEffect, useRef, useState } from "react";
import { useRegisterSW } from "virtual:pwa-register/react";
import "./pwa.css";

interface InstallPrompt extends Event {
  prompt(): Promise<void>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed" }>;
}

export function PwaNotice() {
  const [install, setInstall] = useState<InstallPrompt | null>(null);
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
    offlineReady: [offlineReady, setOfflineReady],
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

  useEffect(() => {
    const available = (event: Event) => {
      event.preventDefault();
      setInstall(event as InstallPrompt);
    };
    const installed = () => setInstall(null);
    window.addEventListener("beforeinstallprompt", available);
    window.addEventListener("appinstalled", installed);
    return () => {
      window.removeEventListener("beforeinstallprompt", available);
      window.removeEventListener("appinstalled", installed);
    };
  }, []);

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

  const installApp = async () => {
    if (!install) return;
    const prompt = install;
    setInstall(null);
    try {
      await prompt.prompt();
      await prompt.userChoice;
    } catch {
      setError("Use your browser’s install menu to add Umran.");
    }
  };

  if (!needRefresh && !offlineReady && !install && !error) return null;

  return (
    <aside className="pwa-notice" aria-label="Umran app">
      <p role="status">
        {error ?? (needRefresh
          ? "A new edition of Umran is ready. Save and reload when you are ready."
          : offlineReady
            ? "Umran is ready offline. Export worlds to keep a backup."
            : "Keep Umran as an app, with your worlds available offline.")}
      </p>
      <div className="pwa-actions">
        {needRefresh && <button type="button" onClick={() => void update()} disabled={updating}>{updating ? "Updating…" : "Save and reload"}</button>}
        {install && !needRefresh && <button type="button" onClick={() => void installApp()}>Install Umran</button>}
        <button type="button" disabled={updating} onClick={() => {
          setNeedRefresh(false);
          setOfflineReady(false);
          setInstall(null);
          setError(null);
        }}>{needRefresh ? "Later" : "Dismiss"}</button>
      </div>
    </aside>
  );
}
