<script lang="ts">
  import { onMount } from "svelte";
  import { CircleCheck } from "lucide-svelte";
  import { _, locale } from "svelte-i18n";
  import { api, type SourceKind, type UpdateItem } from "../lib/api";
  import { jobs, runJob, refreshLists } from "../lib/jobs";
  import { availableSources } from "../lib/sources";
  import AppIcon from "../components/AppIcon.svelte";

  let items: UpdateItem[] = [];
  let loaded = false;
  let checkedAt: Date | null = null;
  let listsAge: number | null = null;
  let unselected = new Set<string>();

  $: sourcesLabel = $availableSources
    .map((s) => $_(`sources.${s}`))
    .join(" • ");

  onMount(async () => {
    await checkNow();
  });

  const keyOf = (item: UpdateItem) => `${item.source}:${item.id}`;

  async function check() {
    loaded = false;
    const status = await api.aptStatus().catch(() => null);
    listsAge = status?.listsAgeSecs ?? null;
    items = await api.updates().catch(() => []);
    checkedAt = new Date();
    loaded = true;
  }

  // Refreshes the package lists first: reading the local lists alone
  // cannot show anything newer than the last "apt update".
  async function checkNow() {
    await refreshLists();
    await check();
  }

  function toggle(item: UpdateItem) {
    const key = keyOf(item);
    unselected.has(key) ? unselected.delete(key) : unselected.add(key);
    unselected = unselected;
  }

  async function updateOne(item: UpdateItem) {
    const title = $_("jobs.updating", { values: { name: item.name } });
    if (
      await runJob(
        title,
        { action: "update", source: item.source, ids: [item.id] },
        keyOf(item),
      )
    )
      await check();
  }

  // One job per source, with every selected package of that source.
  async function updateSelected() {
    const selected = items.filter((item) => !unselected.has(keyOf(item)));
    const sources: SourceKind[] = ["apt", "flatpak", "snap", "firmware"];
    for (const source of sources) {
      const ids = selected
        .filter((item) => item.source === source)
        .map((item) => item.id);
      if (!ids.length) continue;
      const title = $_("jobs.updatingAll", { values: { count: ids.length } });
      await runJob(
        title,
        { action: "update", source, ids },
        `update-all:${source}`,
      );
    }
    await check();
  }

  function ago(seconds: number, lang: string | null | undefined) {
    const rtf = new Intl.RelativeTimeFormat(lang ?? undefined, {
      numeric: "auto",
    });
    const minutes = Math.round(seconds / 60);
    if (minutes < 60) return rtf.format(-minutes, "minute");
    const hours = Math.round(minutes / 60);
    if (hours < 48) return rtf.format(-hours, "hour");
    return rtf.format(-Math.round(hours / 24), "day");
  }

  $: anyRunning = $jobs.some(
    (j) =>
      j.status === "running" &&
      (j.key === "refresh-lists" ||
        j.key?.startsWith("update") ||
        items.some((i) => keyOf(i) === j.key)),
  );
  $: selectedCount = items.filter(
    (item) => !unselected.has(keyOf(item)),
  ).length;
  $: checkedLabel = checkedAt
    ? checkedAt.toLocaleTimeString($locale ?? undefined, {
        hour: "2-digit",
        minute: "2-digit",
      })
    : "";
  $: listsLabel =
    listsAge === null
      ? $_("updates.listsNever")
      : $_("updates.listsRefreshed", {
          values: { when: ago(listsAge, $locale) },
        });
  function toggleAll() {
    if (selectedCount === items.length) {
      items.forEach((item) => unselected.add(keyOf(item)));
    } else {
      unselected.clear();
    }
    unselected = unselected;
  }
</script><!-- Fixed Top Header -->
<div class="flex items-center justify-between gap-3 flex-wrap mb-4">
  <div>
    <h2 class="text-[1.1rem] font-semibold text-text-bright">
      {#if !loaded || anyRunning}
        {$_("updates.refreshing")}...
      {:else if items.length === 0}
        {$_("updates.none")}
      {:else}
        {$_("updates.count", { values: { count: items.length } })}
      {/if}
    </h2>
    <div class="flex items-center gap-2 text-[0.78rem] text-text-muted mt-0.5 flex-wrap">
      {#if checkedLabel}
        <span>{$_("updates.lastCheck", { values: { time: checkedLabel } })}</span>
      {/if}
      {#if checkedLabel && listsLabel}<span>•</span>{/if}
      <span class={listsAge === null || listsAge > 2 * 86400 ? "text-warning" : "text-text-muted"}>{listsLabel}</span>
      {#if sourcesLabel}
        <span>•</span>
        <span>{sourcesLabel}</span>
      {/if}
    </div>
  </div>
  <div class="flex items-center gap-2">
    <button
      class="px-3.5 py-1.5 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.8rem] font-medium transition-colors disabled:opacity-60 disabled:cursor-not-allowed flex items-center gap-1.5"
      disabled={anyRunning}
      on:click={checkNow}
    >
      {#if anyRunning}
        <span class="inline-block w-3 h-3 border-2 border-text-muted border-t-transparent rounded-full animate-spin"></span>
      {/if}
      {$_("updates.checkNow")}
    </button>
    {#if loaded && items.length > 0}
      <button
        class="px-3.5 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.8rem] font-semibold transition-colors disabled:opacity-60 disabled:cursor-not-allowed flex items-center gap-1.5"
        disabled={anyRunning || selectedCount === 0}
        on:click={updateSelected}
      >
        {anyRunning ? $_("common.updating") : $_("updates.updateAll")}
      </button>
    {/if}
  </div>
</div>

<!-- Body Content -->
{#if !loaded}
  <div class="flex flex-col items-center justify-center min-h-[50vh] text-center px-4">
    <div class="w-10 h-10 border-3 border-primary border-t-transparent rounded-full animate-spin mb-3"></div>
    <p class="text-[0.88rem] text-text-muted">{$_("updates.refreshing")}...</p>
  </div>
{:else if items.length === 0}
  <div class="flex flex-col items-center justify-center text-center min-h-[50vh] px-6">
    <div class="w-24 h-24 rounded-full bg-success/15 text-success grid place-items-center mb-4 ring-8 ring-success/5">
      <CircleCheck size={64} strokeWidth={1.5} />
    </div>
    <h3 class="text-[1.4rem] font-semibold text-text-bright">{$_("updates.none")}</h3>
    <p class="text-[0.88rem] text-text-muted mt-1.5 max-w-md">{$_("updates.noneSub")}</p>
  </div>
{:else}

  <div class="bg-surface border border-border rounded overflow-hidden">
    <!-- List control header: Select all -->
    <div
      class="flex items-center justify-between px-4 py-2 bg-surface-2/60 border-b border-border text-[0.75rem] text-text-muted"
    >
      <label class="flex items-center gap-2 cursor-pointer select-none">
        <input
          type="checkbox"
          checked={selectedCount === items.length}
          indeterminate={selectedCount > 0 && selectedCount < items.length}
          class="accent-primary w-4 h-4 rounded cursor-pointer"
          on:change={toggleAll}
        />
        <span
          >{selectedCount === items.length
            ? $_("updates.deselectAll")
            : $_("updates.selectAll")}</span
        >
      </label>
      <span
        >{$_("updates.selectedCount", {
          values: { selected: selectedCount, total: items.length },
        })}</span
      >
    </div>

    <!-- Items list -->
    {#each items as item (keyOf(item))}
      {@const running = $jobs.some(
        (j) => j.key === keyOf(item) && j.status === "running",
      )}
      <div
        class="flex items-center gap-3.5 px-4 py-3 border-t border-border first:border-t-0 hover:bg-surface-2/40 transition-colors"
      >
        <input
          type="checkbox"
          checked={!unselected.has(keyOf(item))}
          class="accent-primary w-4 h-4 shrink-0 cursor-pointer"
          aria-label={item.name}
          on:change={() => toggle(item)}
        />
        <AppIcon src={item.icon} size={40} />
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <span class="text-[0.88rem] font-semibold text-text-bright truncate"
              >{item.name}</span
            >
            <span
              class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted shrink-0 font-medium"
              >{$_(`sources.${item.source}`)}</span
            >
          </div>
          <div class="text-[0.75rem] text-text-muted truncate mt-0.5">
            {item.from ? `${item.from} → ${item.to}` : item.to}
          </div>
        </div>
        <button
          class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.75rem] font-medium shrink-0 disabled:opacity-60 disabled:cursor-not-allowed transition-colors"
          disabled={anyRunning}
          on:click={() => updateOne(item)}
        >
          {running ? $_("common.updating") : $_("common.update")}
        </button>
      </div>
    {/each}
  </div>
{/if}
