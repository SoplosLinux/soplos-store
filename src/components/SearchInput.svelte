<script lang="ts">
  import { Search, X } from "lucide-svelte";
  import { _ } from "svelte-i18n";

  export let value = "";
  export let placeholder: string;
  export let onSubmit: ((value: string) => void) | null = null;
  export let onClear: (() => void) | null = null;
</script>

<div class="relative max-w-[460px]">
  <Search size={15} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-muted" />
  <!-- data-search: Ctrl+F focuses the search box of the current view. -->
  <input
    type="text"
    data-search
    bind:value
    {placeholder}
    on:keydown={(e) => e.key === "Enter" && onSubmit?.(value.trim())}
    class="w-full pl-8 pr-7 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.82rem] placeholder:text-text-muted focus:outline-none focus:border-primary"
  />
  {#if value.length > 0}
    <button
      class="absolute right-2 top-1/2 -translate-y-1/2 text-text-muted hover:text-text-bright"
      title={$_("common.clear")}
      on:click={() => {
        value = "";
        onClear?.();
      }}
    >
      <X size={14} />
    </button>
  {/if}
</div>
