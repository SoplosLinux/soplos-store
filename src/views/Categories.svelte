<script lang="ts">
  import { _ } from "svelte-i18n";
  import { CATEGORIES } from "../lib/categories";

  export let onOpenCategory: (id: string) => void;
</script>

<div class="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3.5">
  {#each CATEGORIES as category (category.id)}
    <button
      class="relative h-[120px] rounded border border-border p-4 overflow-hidden text-left text-white transition-transform hover:-translate-y-0.5 hover:border-white/35"
      style="background: {category.background}"
      on:click={() => onOpenCategory(category.id)}
    >
      <div class="text-[1rem] font-bold max-w-[65%] leading-tight">{$_(`categories.${category.id}`)}</div>
      <svelte:component this={category.icon} size={64} strokeWidth={1.5} class="absolute right-3 bottom-3 text-white/80" />
    </button>
  {/each}
</div>
