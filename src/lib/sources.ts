import { readable } from "svelte/store";
import { api, type SourceKind } from "./api";

// Package sources installed on the system. APT is always there; Flatpak
// and Snap are only shown when their tools are installed.
export const availableSources = readable<SourceKind[]>(["apt"], (set) => {
  api
    .sources()
    .then((status) => {
      const list: SourceKind[] = ["apt"];
      if (status?.flatpak) list.push("flatpak");
      if (status?.snap) list.push("snap");
      if (status?.appimageManager) list.push("appimage");
      set(list);
    })
    .catch(() => set(["apt"]));
});
