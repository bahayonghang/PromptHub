import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyTheme, DEFAULT_THEME } from "./theme";
import { initializeAppearance } from "./appearance";
import { initI18n } from "./runtime/i18n";
import "./styles/globals.css";

// Apply the default theme before the first paint so the UI is never unthemed
// (Requirement 22.5). The persisted selection is loaded and applied on mount.
applyTheme(DEFAULT_THEME);

// Resolve and apply the persisted appearance (flavor base, accent, fonts, scale,
// density) before first paint, falling back to documented defaults; the flavor
// base supersedes the legacy theme toggle for appearance purposes
// (Req 2.6-2.9, 3.5, 4.5, 5.4, 6.4).
void initializeAppearance();

// Resolve and apply the active locale at startup (Req 21.5). i18next is already
// initialized eagerly on import, so this only swaps in the resolved locale's
// bundle when it differs from English; rendering need not wait on it.
void initI18n();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
