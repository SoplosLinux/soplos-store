<script lang="ts">
  import { _ } from "svelte-i18n";

  export let rating: number | null;
  export let reviews: number;
</script>

<span class="text-[0.72rem] text-text-muted">
  {#if rating !== null && reviews > 0}
    <span class="text-warning">&#9733;</span> {rating.toFixed(1)} · {$_("common.reviews", { values: { count: reviews } })}
  {:else}
    {$_("common.noRatings")}
  {/if}
</span>
