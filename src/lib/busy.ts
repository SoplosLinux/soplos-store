import { writable } from "svelte/store";

// Number of backend calls in flight; the progress strip at the bottom of
// the window moves while it is above zero.
export const busy = writable(0);

export async function track<T>(promise: Promise<T>): Promise<T> {
  busy.update((n) => n + 1);
  try {
    return await promise;
  } finally {
    busy.update((n) => n - 1);
  }
}
