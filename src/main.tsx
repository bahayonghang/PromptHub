import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyAppearancePreferences } from "./appearance/preferences";
import { DEFAULT_BOOTSTRAP_SETTINGS, startDefaultApplication } from "./bootstrap";
import "./styles/globals.css";

// Paint one safe fallback synchronously; the same controller applies persisted
// preferences before React mounts.
applyAppearancePreferences(DEFAULT_BOOTSTRAP_SETTINGS, "en");

void startDefaultApplication(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
