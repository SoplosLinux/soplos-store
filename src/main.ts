import "./styles/global.css";
// Registers the dictionaries before any component uses them.
import "./i18n";
import App from "./App.svelte";

const app = new App({
  target: document.getElementById("app")!,
});

export default app;
