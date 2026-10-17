import { writable } from "svelte/store";

// A short message at the bottom of the window that goes away by itself.
export const toast = writable<string | null>(null);

let timer: ReturnType<typeof setTimeout> | undefined;

export function showToast(message: string, milliseconds = 1800) {
  toast.set(message);
  clearTimeout(timer);
  timer = setTimeout(() => toast.set(null), milliseconds);
}
