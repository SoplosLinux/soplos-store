<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { api, type AppCard, type SourceKind } from "../lib/api";
  import { availableSources } from "../lib/sources";
  import { SUBCATEGORIES } from "../lib/categories";
  import BackButton from "../components/BackButton.svelte";
  import Carousel from "../components/Carousel.svelte";
  import AppTile from "../components/AppTile.svelte";

  export let category: string;
  export let onBack: () => void;
  export let onOpenApp: (id: string) => void;

  let featured: AppCard[] = [];
  let apps: AppCard[] = [];
  let loaded = false;
  let filter: "all" | SourceKind = "all";
  $: FILTERS = ["all", ...$availableSources] as ("all" | SourceKind)[];
  let sort: "top" | "name" = "top";
  let sub = "";

  // Subcategories that have at least one app in this page, most apps first.
  $: subs = (SUBCATEGORIES[category] ?? [])
    .map((name) => ({ name, count: apps.filter((app) => app.categories.includes(name)).length }))
    .filter((entry) => entry.count > 0)
    .sort((a, b) => b.count - a.count);

  onMount(async () => {
    const section = await api.category(category).catch(() => null);
    featured = section?.featured ?? [];
    apps = [...featured, ...(section?.apps ?? [])];
    loaded = true;
  });

  $: shown = apps
    .filter((app) => filter === "all" || app.sources.includes(filter))
    .filter((app) => !sub || app.categories.includes(sub))
    .sort((a, b) => (sort === "name" ? a.name.localeCompare(b.name) : (b.rating ?? 0) * b.reviews - (a.rating ?? 0) * a.reviews));

  $: title = $_(`categories.${category}`);
</script>

<BackButton {onBack} />
<h2 class="text-[1.15rem] font-semibold text-text-bright mb-3">{title}</h2>

{#if !loaded}
  <p class="text-[0.85rem] text-text-muted">{$_("common.loading")}</p>
{:else}
  <Carousel apps={featured} kicker={$_("sections.topIn", { values: { section: title } })} onOpen={onOpenApp} />

  <div class="flex items-center justify-between gap-3 flex-wrap mt-6 mb-3.5">
    <div class="flex gap-1.5">
      {#each FILTERS as option}
        <button
          class="px-3 py-1.5 rounded text-[0.78rem] {filter === option
            ? 'bg-primary/15 text-primary font-semibold'
            : 'border border-border text-text hover:bg-surface hover:text-text-bright'}"
          on:click={() => (filter = option)}
        >
          {option === "all" ? $_("category.all") : $_(`sources.${option}`)}
        </button>
      {/each}
    </div>
    <div class="flex gap-2">
    {#if subs.length}
      <select
        bind:value={sub}
        title={$_("subcategories.label")}
        class="appearance-none bg-surface-2 border border-border rounded px-2.5 py-1.5 text-[0.78rem] text-text-bright outline-none focus:border-primary"
      >
        <option value="">{$_("subcategories.all")}</option>
        {#each subs as entry}
          <option value={entry.name}>{$_(`subcategories.${entry.name}`)} ({entry.count})</option>
        {/each}
      </select>
    {/if}
    <select
      bind:value={sort}
      title={$_("category.sort")}
      class="appearance-none bg-surface-2 border border-border rounded px-2.5 py-1.5 text-[0.78rem] text-text-bright outline-none focus:border-primary"
    >
      <option value="top">{$_("category.sortTop")}</option>
      <option value="name">{$_("category.sortName")}</option>
    </select>
    </div>
  </div>

  {#if shown.length === 0}
    <p class="text-[0.85rem] text-text-muted">{$_("category.empty")}</p>
  {:else}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-3.5">
      {#each shown as app (app.id)}
        <AppTile {app} onOpen={onOpenApp} />
      {/each}
    </div>
  {/if}
{/if}
