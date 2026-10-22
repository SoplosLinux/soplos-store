<script lang="ts">
  // When the last check was made, how fresh apt's package lists are and
  // which sources are looked in. The Updates page shows it once.
  export let checked: string;
  export let lists: string;
  export let listsStale = false;
  export let sources: string;
</script>

<div class="flex items-center justify-center gap-2 text-[0.75rem] text-text-muted flex-wrap">
  {#if checked}
    <span>{checked}</span>
    <span>•</span>
  {/if}
  <span class={listsStale ? "text-warning" : ""}>{lists}</span>
  {#if sources}
    <span>•</span>
    <span>{sources}</span>
  {/if}
</div>
