import { invoke } from "@tauri-apps/api/core";

// Debian's icons and screenshots are JPEG XL, which the webview cannot
// show; the backend converts them to PNG. Other images load directly.
const pending = new Map<string, Promise<string | null>>();

export function resolveImage(url: string): Promise<string | null> {
  if (!url.endsWith(".jxl")) return Promise.resolve(url);
  let request = pending.get(url);
  if (!request) {
    request = invoke<string | null>("fetch_image", { url }).catch(() => null);
    pending.set(url, request);
  }
  return request;
}
