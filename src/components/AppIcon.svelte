<script lang="ts">
  import { Package } from "lucide-svelte";
  import { resolveImage } from "../lib/images";

  export let src: string | null | undefined;
  export let size = 48;

  let url: string | null = null;
  let failed = false;

  $: load(src);

  async function load(source: string | null | undefined) {
    failed = false;
    url = null;
    if (!source) return;
    const resolved = await resolveImage(source);
    if (source === src) url = resolved;
  }
</script>

{#if url && !failed}
  <img
    src={url}
    alt=""
    class="object-contain shrink-0"
    style="width: {size}px; height: {size}px"
    on:error={() => (failed = true)}
  />
{:else}
  <div class="rounded-lg bg-surface-2 grid place-items-center shrink-0 text-text-muted" style="width: {size}px; height: {size}px">
    <Package size={Math.round(size * 0.5)} />
  </div>
{/if}
