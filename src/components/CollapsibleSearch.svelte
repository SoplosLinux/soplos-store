<script lang="ts">
  import { tick } from "svelte";
  import { get } from "svelte/store";
  import { fly } from "svelte/transition";
  import { Search, X } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import { searchRequest } from "../lib/search";

  // A magnifier at the top right of the page; pressing it (or Ctrl+F)
  // unfolds the search box, and Escape or the cross folds it again.
  export let value = "";
  export let placeholder: string;
  export let onSubmit: ((value: string) => void) | null = null;
  export let onClear: (() => void) | null = null;

  let open = value.length > 0;
  let input: HTMLInputElement;
  let seen = get(searchRequest);

  $: if ($searchRequest !== seen) {
    seen = $searchRequest;
    openBox();
  }

  async function openBox() {
    open = true;
    await tick();
    input?.focus();
    input?.select();
  }

  function close() {
    value = "";
    open = false;
    onClear?.();
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") onSubmit?.(value.trim());
    else if (event.key === "Escape") close();
  }
</script>

<!-- Takes no height of its own: it sits on the first row of the page (the
     title of the first section), at the right, and stays in view on scroll. -->
<div class="sticky top-[-28px] z-30 h-0">
  <div class="absolute right-0 top-[-5px]">
  {#if open}
    <div class="relative w-[min(460px,70vw)]" in:fly={{ x: 28, duration: 170 }}>
      <Search size={15} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-muted" />
      <input
        bind:this={input}
        type="text"
        bind:value
        {placeholder}
        on:keydown={onKeydown}
        class="w-full pl-8 pr-8 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.82rem] placeholder:text-text-muted focus:outline-none focus:border-primary"
      />
      <button class="absolute right-2 top-1/2 -translate-y-1/2 text-text-muted hover:text-text-bright" title={$_("common.clear")} on:click={close}>
        <X size={14} />
      </button>
    </div>
  {:else}
    <button
      class="p-2 rounded bg-dark/80 text-text-muted hover:bg-surface hover:text-text-bright transition-colors"
      title={placeholder}
      aria-label={placeholder}
      on:click={openBox}
    >
      <Search size={20} />
    </button>
  {/if}
  </div>
</div>
