<script context="module" lang="ts">
  import type { AppCard, Section } from "../lib/api";
  // Kept between visits so going back to Explore does not reload it.
  let cachedSections: Section[] | null = null;

  /** Forgets the cached sections so the next visit loads them again (F5). */
  export function clearExploreCache() {
    cachedSections = null;
  }
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { api } from "../lib/api";
  import { settings } from "../lib/settings";
  import Carousel from "../components/Carousel.svelte";
  import AppTile from "../components/AppTile.svelte";
  import CollapsibleSearch from "../components/CollapsibleSearch.svelte";

  export let onOpenApp: (id: string) => void;
  export let onOpenCategory: (id: string) => void;

  const FIXED_SECTIONS = ["featured", "popular", "trending"];

  let sections: Section[] = cachedSections ?? [];
  let loaded = cachedSections !== null;
  let query = "";
  let searchedFor = "";
  let results: AppCard[] = [];
  let searching = false;

  onMount(async () => {
    if (cachedSections) return;
    sections = await api.explore().catch(() => []);
    cachedSections = sections.length ? sections : null;
    loaded = true;
  });

  // The three sections of the top can be turned off in Settings.
  $: visibleSections = sections.filter((s) =>
    s.id === "featured" ? $settings.showFeatured : s.id === "popular" ? $settings.showPopular : s.id === "trending" ? $settings.showTrending : true,
  );

  // Reactive on purpose: a plain function calling $_ inside would not be
  // re-evaluated when the language changes.
  $: title = (id: string) => (FIXED_SECTIONS.includes(id) ? $_(`sections.${id}`) : $_(`categories.${id}`));

  async function runSearch(value: string) {
    if (!value) return clearSearch();
    searching = true;
    searchedFor = value;
    results = await api.search(value).catch(() => []);
    searching = false;
  }

  function clearSearch() {
    searchedFor = "";
    results = [];
  }
</script>

<CollapsibleSearch bind:value={query} placeholder={$_("explore.searchPlaceholder")} onSubmit={runSearch} onClear={clearSearch} />

{#if searchedFor}
  <h2 class="text-[1rem] font-semibold text-text-bright mb-3">{$_("explore.results", { values: { query: searchedFor } })}</h2>
  {#if searching}
    <p class="text-[0.85rem] text-text-muted">{$_("common.loading")}</p>
  {:else if results.length === 0}
    <p class="text-[0.85rem] text-text-muted">{$_("explore.noResults", { values: { query: searchedFor } })}</p>
  {:else}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3.5">
      {#each results as app (app.id)}
        <AppTile {app} onOpen={onOpenApp} />
      {/each}
    </div>
  {/if}
{:else if !loaded}
  <p class="text-[0.85rem] text-text-muted">{$_("common.loading")}</p>
{:else if sections.length === 0}
  <p class="text-[0.85rem] text-text-muted">{$_("explore.unavailable")}</p>
{:else}
  {#each visibleSections as section, i (section.id)}
    <div class="flex items-center justify-between mb-3 {i === 0 ? 'mt-1' : 'mt-8'}">
      <h2 class="text-[1rem] font-semibold text-text-bright">{title(section.id)}</h2>
      {#if !FIXED_SECTIONS.includes(section.id)}
        <button class="text-[0.8rem] text-primary hover:underline" on:click={() => onOpenCategory(section.id)}>{$_("common.seeAll")}</button>
      {/if}
    </div>
    <Carousel
      apps={section.featured}
      kicker={section.id === "featured" ? $_("sections.featuredApp") : $_("sections.topIn", { values: { section: title(section.id) } })}
      onOpen={onOpenApp}
    />
    {#if section.apps.length}
      <div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3.5 mt-3.5">
        {#each section.apps as app (app.id)}
          <AppTile {app} onOpen={onOpenApp} />
        {/each}
      </div>
    {/if}
  {/each}
{/if}
