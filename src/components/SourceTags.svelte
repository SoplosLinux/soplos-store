<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { SourceKind } from "../lib/api";

  // The sources come in the order of the catalog: APT first, as the
  // system's own source.
  export let sources: SourceKind[];
</script>

<div class="flex gap-1 flex-wrap">
  {#each sources as source}
    <span class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted">{$_(`sources.${source}`)}</span>
  {/each}
</div>
