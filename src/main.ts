import "./styles/global.css";
// Registers the dictionaries before any component uses them.
import "./i18n";
import App from "./App.svelte";

// This is a desktop app, not a browser page: the WebKitGTK context menu
// (Back/Forward/Reload/Inspect) has no place here. Left enabled in dev so
// devtools stays reachable via right-click while working.
if (!import.meta.env.DEV) {
  document.addEventListener("contextmenu", (e) => e.preventDefault());
}

const app = new App({
  target: document.getElementById("app")!,
});

export default app;
