<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { api, type InstalledApp, type SourceKind } from "../lib/api";
  import { availableSources } from "../lib/sources";
  import { jobs, overrides, runJob } from "../lib/jobs";
  import AppIcon from "../components/AppIcon.svelte";
  import CollapsibleSearch from "../components/CollapsibleSearch.svelte";

  export let onOpenApp: (id: string) => void;

  let apps: InstalledApp[] = [];
  let loaded = false;
  let filter: "all" | SourceKind = "all";
  $: FILTERS = ["all", ...$availableSources] as ("all" | SourceKind)[];
  let query = "";

  onMount(load);

  async function load() {
    apps = await api.installed().catch(() => []);
    loaded = true;
  }

  async function remove(app: InstalledApp) {
    const title = $_("jobs.removing", { values: { name: app.name } });
    if (await runJob(title, { action: "remove", source: app.source, ids: [app.package] }, app.id)) {
      overrides.update((o) => ({ ...o, [`${app.id}:${app.source}`]: false }));
      await load();
    }
  }

  $: shown = apps.filter(
    (app) =>
      (filter === "all" || app.source === filter) &&
      (!query || app.name.toLowerCase().includes(query.toLowerCase())),
  );
</script>

<CollapsibleSearch bind:value={query} placeholder={$_("installed.search")} />

<div class="flex items-center justify-between gap-3 flex-wrap mb-3.5 pr-12">
  <h2 class="text-[1rem] font-semibold text-text-bright">{$_("installed.count", { values: { count: apps.length } })}</h2>
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
</div>

{#if !loaded}
  <p class="text-[0.85rem] text-text-muted">{$_("common.loading")}</p>
{:else if shown.length === 0}
  <p class="text-[0.85rem] text-text-muted">{$_("installed.empty")}</p>
{:else}
  <div class="bg-surface border border-border rounded overflow-hidden">
    {#each shown as app (app.source + app.id)}
      <div
        class="flex items-center gap-3.5 px-4 py-3 border-t border-border first:border-t-0 hover:bg-surface-2 cursor-pointer"
        role="button"
        tabindex="0"
        on:click={() => onOpenApp(app.id)}
        on:keydown={(e) => e.key === "Enter" && onOpenApp(app.id)}
      >
        <AppIcon src={app.icon} size={44} />
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <span class="text-[0.88rem] font-semibold text-text-bright truncate">{app.name}</span>
            <span class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted shrink-0">{$_(`sources.${app.source}`)}</span>
          </div>
          <div class="text-[0.75rem] text-text-muted truncate">{app.summary || app.version}</div>
        </div>
        <div class="w-[150px] text-right text-[0.75rem] text-text-muted shrink-0">
          <div class="truncate">{app.version}</div>
          <div>{app.size}</div>
        </div>
        <button
          class="px-2.5 py-1.5 rounded text-[0.75rem] font-semibold border border-danger text-danger hover:bg-danger/10 shrink-0 disabled:opacity-60 disabled:cursor-default"
          disabled={$jobs.some((j) => j.key === app.id && j.status === "running")}
          on:click|stopPropagation={() => remove(app)}
        >
          {$jobs.some((j) => j.key === app.id && j.status === "running") ? $_("common.removing") : $_("common.uninstall")}
        </button>
      </div>
    {/each}
  </div>
{/if}
