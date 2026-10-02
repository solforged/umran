import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";
import "./chart.css";
import { LightSwitch } from "./components/LightSwitch";
import { PwaNotice } from "./components/PwaNotice";

// An installed window shows the app's own name, not the page's subtitle.
if (matchMedia("(display-mode: standalone)").matches) document.title = "Umran";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root");
}

createRoot(root).render(
  <StrictMode>
    <App />
    <LightSwitch />
    <PwaNotice />
  </StrictMode>,
);
