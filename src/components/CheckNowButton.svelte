<script lang="ts">
  import { _ } from "svelte-i18n";

  // "Check now". While a check or an update runs it shows a spinner and
  // cannot be pressed.
  export let busy = false;
  export let disabled = false;
</script>

<button
  class="px-3.5 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.8rem] font-medium transition-colors disabled:opacity-60 disabled:cursor-not-allowed flex items-center gap-1.5"
  disabled={busy || disabled}
  on:click
>
  {#if busy}
    <span class="inline-block w-3 h-3 border-2 border-text-muted border-t-transparent rounded-full animate-spin"></span>
  {/if}
  {$_("updates.checkNow")}
</button>
