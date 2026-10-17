<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { SourceKind } from "../lib/api";
  import { settings } from "../lib/settings";

  export let sources: SourceKind[];

  // The preferred source from Settings goes first.
  $: ordered = [...sources].sort((a, b) => Number(b === $settings.preferredSource) - Number(a === $settings.preferredSource));
</script>

<div class="flex gap-1 flex-wrap">
  {#each ordered as source}
    <span class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted">{$_(`sources.${source}`)}</span>
  {/each}
</div>
