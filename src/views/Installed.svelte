<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Check, Trash2 } from "lucide-svelte";
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

  let multiSelect = false;
  let selected = new Set<string>();

  const keyOf = (app: InstalledApp) => `${app.source}:${app.package}`;

  onMount(load);

  async function load() {
    apps = await api.installed().catch(() => []);
    loaded = true;
    selected = new Set();
  }

  function toggle(app: InstalledApp) {
    const key = keyOf(app);
    if (selected.has(key)) {
      selected.delete(key);
    } else {
      selected.add(key);
    }
    selected = selected;
  }

  function exitMultiSelect() {
    multiSelect = false;
    selected.clear();
    selected = selected;
  }

  function handleCardClick(app: InstalledApp) {
    if (multiSelect) {
      toggle(app);
    } else {
      onOpenApp(app.id);
    }
  }

  async function remove(app: InstalledApp) {
    const title = $_("jobs.removing", { values: { name: app.name } });
    if (await runJob(title, { action: "remove", source: app.source, ids: [app.package] }, app.id)) {
      overrides.update((o) => ({ ...o, [`${app.id}:${app.source}`]: false }));
      await load();
    }
  }

  async function removeSelected() {
    const toRemove = apps.filter((app) => selected.has(keyOf(app)));
    if (toRemove.length === 0) return;

    const sources: SourceKind[] = ["apt", "flatpak", "snap", "appimage"];
    for (const source of sources) {
      const items = toRemove.filter((app) => app.source === source);
      if (items.length === 0) continue;

      const ids = items.map((a) => a.package);
      const title = $_("jobs.removing", { values: { name: items.length === 1 ? items[0].name : `${items.length} ${$_(`sources.${source}`)}` } });
      await runJob(title, { action: "remove", source, ids }, `remove-bulk:${source}`);
      for (const app of items) {
        overrides.update((o) => ({ ...o, [`${app.id}:${app.source}`]: false }));
      }
    }
    multiSelect = false;
    await load();
  }

  $: shown = apps.filter(
    (app) =>
      (filter === "all" || app.source === filter) &&
      (!query || app.name.toLowerCase().includes(query.toLowerCase())),
  );

  $: isAnyRunning = $jobs.some((j) => j.status === "running");
</script>

<CollapsibleSearch bind:value={query} placeholder={$_("installed.search")} />

<div class="flex items-center justify-between gap-3 mb-3.5 pr-12">
  <h2 class="text-[1rem] font-semibold text-text-bright shrink-0">
    {$_("installed.count", { values: { count: apps.length } })}
  </h2>

  <div class="flex items-center gap-2 shrink-0">
    {#if multiSelect}
      <button
        class="flex items-center gap-1.5 px-3 py-1.5 rounded text-[0.78rem] font-semibold transition-colors {selected.size > 0
          ? 'bg-danger text-white hover:bg-danger/90'
          : 'border border-border text-text-muted opacity-50 cursor-not-allowed'}"
        disabled={isAnyRunning || selected.size === 0}
        on:click={removeSelected}
      >
        <Trash2 size={13} />
        <span>{selected.size > 0 ? $_("installed.uninstallSelected", { values: { count: selected.size } }) : $_("common.uninstall")}</span>
      </button>
      <button
        class="px-3 py-1.5 rounded text-[0.78rem] font-medium border border-border text-text hover:bg-surface hover:text-text-bright transition-colors"
        on:click={exitMultiSelect}
      >
        {$_("installed.cancelSelect")}
      </button>
    {:else if apps.length > 0}
      <button
        class="px-3 py-1.5 rounded text-[0.78rem] font-medium border border-border text-text hover:bg-surface hover:text-text-bright transition-colors flex items-center gap-1.5"
        on:click={() => (multiSelect = true)}
      >
        <span>{$_("installed.multiSelect")}</span>
      </button>
    {/if}

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
</div>

{#if !loaded}
  <p class="text-[0.85rem] text-text-muted">{$_("common.loading")}</p>
{:else if shown.length === 0}
  <p class="text-[0.85rem] text-text-muted">{$_("installed.empty")}</p>
{:else}
  <div class="bg-surface border border-border rounded overflow-hidden">
    {#each shown as app (app.source + app.id)}
      {@const isSelected = selected.has(keyOf(app))}
      <div
        class="flex items-center gap-3.5 px-4 py-3 border-t border-border first:border-t-0 cursor-pointer transition-colors {multiSelect && isSelected ? 'bg-danger/10 hover:bg-danger/15' : 'hover:bg-surface-2'}"
        role="button"
        tabindex="0"
        on:click={() => handleCardClick(app)}
        on:keydown={(e) => e.key === "Enter" && handleCardClick(app)}
      >
        <!-- Selection Checkbox (only in multi-select mode) -->
        {#if multiSelect}
          <div
            class="w-4 h-4 rounded border flex items-center justify-center shrink-0 transition-colors {isSelected
              ? 'bg-danger border-danger text-white'
              : 'border-border'}"
          >
            {#if isSelected}
              <Check size={11} strokeWidth={3} />
            {/if}
          </div>
        {/if}

        <AppIcon src={app.icon} size={44} />
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <span class="text-[0.88rem] font-semibold text-text-bright truncate">{app.name}</span>
            <span class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted shrink-0 font-medium">{$_(`sources.${app.source}`)}</span>
          </div>
          <div class="text-[0.75rem] text-text-muted truncate">{app.summary || app.version}</div>
        </div>
        <div class="w-[150px] text-right text-[0.75rem] text-text-muted shrink-0">
          <div class="truncate">{app.version}</div>
          <div>{app.size}</div>
        </div>
        {#if !multiSelect}
          <button
            class="px-2.5 py-1.5 rounded text-[0.75rem] font-semibold border border-danger text-danger hover:bg-danger/10 shrink-0 disabled:opacity-60 disabled:cursor-default"
            disabled={$jobs.some((j) => j.key === app.id && j.status === "running")}
            on:click|stopPropagation={() => remove(app)}
          >
            {$jobs.some((j) => j.key === app.id && j.status === "running") ? $_("common.removing") : $_("common.uninstall")}
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}
