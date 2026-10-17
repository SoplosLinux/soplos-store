<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { _ } from "svelte-i18n";
  import { fade, scale } from "svelte/transition";
  import { api } from "../lib/api";
  import logo from "../../src-tauri/icons/128x128.png";

  export let onClose: () => void;

  let version = "";
  let closeButton: HTMLButtonElement;

  onMount(async () => {
    closeButton?.focus();
    try {
      // The full package version, revision included.
      version = await getVersion();
    } catch {
      version = "";
    }
  });
</script>

<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
  role="presentation"
  transition:fade={{ duration: 120 }}
  on:click|self={onClose}
>
  <!-- Escape is handled in App.svelte, which owns the keyboard shortcuts. -->
  <div
    class="w-full max-w-[420px] bg-surface border border-border rounded-xl p-6 flex flex-col items-center gap-2 shadow-2xl text-center"
    role="dialog"
    aria-modal="true"
    aria-label="Soplos Store"
    transition:scale={{ duration: 140, start: 0.96 }}
  >
    <img src={logo} alt="" class="w-16 h-16 object-contain" />
    <h2 class="text-text-bright font-semibold text-[1.05rem] mt-1">Soplos Store</h2>
    {#if version}
      <p class="text-[0.78rem] text-text-muted">{$_("about.version", { values: { version } })}</p>
    {/if}
    <p class="text-[0.82rem] text-text mt-1">{$_("about.tagline")}</p>

    <div class="w-full border-t border-border my-2"></div>

    <p class="text-[0.75rem] text-text-muted">{$_("about.copyright")}</p>
    <p class="text-[0.75rem] text-text-muted">{$_("about.license")}</p>
    <button class="text-[0.8rem] text-primary hover:underline mt-1" on:click={() => api.openUrl("https://soplos.org")}>soplos.org</button>

    <button
      bind:this={closeButton}
      class="mt-4 px-6 py-2 rounded bg-primary hover:bg-primary-hover text-white text-[0.82rem] font-semibold"
      on:click={onClose}
    >
      {$_("about.close")}
    </button>
  </div>
</div>
