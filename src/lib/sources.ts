import { derived, readable } from "svelte/store";
import { api, type SourceKind, type SourcesStatus } from "./api";
import { settings } from "./settings";

// What is installed on the system: Flatpak and Snap when their tools are
// there, AppImage when its manager is.
const installed = readable<SourcesStatus | null>(null, (set) => {
  api
    .sources()
    .then((status) => set(status))
    .catch(() => set(null));
});

// The sources the Store works with: APT is always there; the others when
// they are installed and not turned off in Settings.
export const availableSources = derived([installed, settings], ([status, prefs]): SourceKind[] => {
  const list: SourceKind[] = ["apt"];
  if (status?.flatpak && prefs.showFlatpak) list.push("flatpak");
  if (status?.snap && prefs.showSnap) list.push("snap");
  if (status?.appimageManager && prefs.showAppimage) list.push("appimage");
  return list;
});
