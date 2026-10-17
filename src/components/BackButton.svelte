<script lang="ts">
  import { ChevronLeft } from "lucide-svelte";
  import { _ } from "svelte-i18n";

  export let onBack: () => void;
</script>

<button
  class="inline-flex items-center gap-1 px-3 py-1.5 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.82rem] transition-colors mb-4"
  on:click={onBack}
>
  <ChevronLeft size={14} />
  {$_("common.back")}
</button>
