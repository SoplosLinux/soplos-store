import { writable } from "svelte/store";

// Counter raised by Ctrl+F: the search box of the page on screen opens
// when it changes.
export const searchRequest = writable(0);
