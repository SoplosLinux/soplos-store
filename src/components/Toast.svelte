<script lang="ts">
  import { fly } from "svelte/transition";
  import { Check } from "lucide-svelte";
  import { toast } from "../lib/toast";
</script>

{#if $toast}
  <div
    class="fixed bottom-14 left-1/2 -translate-x-1/2 z-40 flex items-center gap-2 px-4 py-2 rounded-full bg-surface border border-border shadow-xl text-[0.82rem] text-text-bright"
    role="status"
    transition:fly={{ y: 16, duration: 160 }}
  >
    <Check size={15} class="text-success" />
    {$toast}
  </div>
{/if}
