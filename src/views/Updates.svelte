<script lang="ts">
  import { onMount } from "svelte";
  import { CircleCheck } from "lucide-svelte";
  import { _, locale } from "svelte-i18n";
  import { api, type SourceKind, type UpdateItem } from "../lib/api";
  import { jobs, runJob, refreshLists } from "../lib/jobs";
  import AppIcon from "../components/AppIcon.svelte";

  let items: UpdateItem[] = [];
  let loaded = false;
  let checkedAt: Date | null = null;
  let listsAge: number | null = null;
  let unselected = new Set<string>();

  onMount(check);

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
    if (await runJob(title, { action: "update", source: item.source, ids: [item.id] }, keyOf(item))) await check();
  }

  // One job per source, with every selected package of that source.
  async function updateSelected() {
    const selected = items.filter((item) => !unselected.has(keyOf(item)));
    const sources: SourceKind[] = ["apt", "flatpak", "snap", "firmware"];
    for (const source of sources) {
      const ids = selected.filter((item) => item.source === source).map((item) => item.id);
      if (!ids.length) continue;
      const title = $_("jobs.updatingAll", { values: { count: ids.length } });
      await runJob(title, { action: "update", source, ids }, `update-all:${source}`);
    }
    await check();
  }

  function ago(seconds: number, lang: string | null | undefined) {
    const rtf = new Intl.RelativeTimeFormat(lang ?? undefined, { numeric: "auto" });
    const minutes = Math.round(seconds / 60);
    if (minutes < 60) return rtf.format(-minutes, "minute");
    const hours = Math.round(minutes / 60);
    if (hours < 48) return rtf.format(-hours, "hour");
    return rtf.format(-Math.round(hours / 24), "day");
  }

  $: anyRunning = $jobs.some(
    (j) => j.status === "running" && (j.key === "refresh-lists" || j.key?.startsWith("update") || items.some((i) => keyOf(i) === j.key)),
  );
  $: selectedCount = items.filter((item) => !unselected.has(keyOf(item))).length;
  $: checkedLabel = checkedAt ? checkedAt.toLocaleTimeString($locale ?? undefined, { hour: "2-digit", minute: "2-digit" }) : "";
  $: listsLabel =
    listsAge === null
      ? $_("updates.listsNever")
      : $_("updates.listsRefreshed", { values: { when: ago(listsAge, $locale) } });
  $: upToDate = loaded && items.length === 0;
</script>

<!-- With nothing to update the page is one centered message (below) instead. -->
{#if !upToDate}
<div class="flex items-center justify-between gap-3 flex-wrap mb-3.5">
  <div>
    <h2 class="text-[1rem] font-semibold text-text-bright">
      {loaded && items.length === 0 ? $_("updates.none") : $_("updates.count", { values: { count: items.length } })}
    </h2>
    {#if checkedLabel}
      <div class="text-[0.75rem] text-text-muted mt-0.5">{$_("updates.lastCheck", { values: { time: checkedLabel } })}</div>
    {/if}
    {#if loaded}
      <div class="text-[0.75rem] mt-0.5 {listsAge === null || listsAge > 2 * 86400 ? 'text-warning' : 'text-text-muted'}">{listsLabel}</div>
    {/if}
  </div>
  <div class="flex gap-2">
    <button
      class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.78rem] disabled:opacity-60"
      disabled={anyRunning}
      on:click={checkNow}
    >
      {$_("updates.checkNow")}
    </button>
    {#if items.length}
      <button
        class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.78rem] font-semibold disabled:opacity-60 disabled:cursor-default"
        disabled={anyRunning || selectedCount === 0}
        on:click={updateSelected}
      >
        {anyRunning ? $_("common.updating") : $_("updates.updateAll")}
      </button>
    {/if}
  </div>
</div>
{/if}

{#if !loaded}
  <p class="text-[0.85rem] text-text-muted">{$_("common.loading")}</p>
{:else if upToDate}
  <div class="flex flex-col items-center justify-center text-center min-h-[62vh] px-6">
    <div class="w-[176px] h-[176px] rounded-full bg-success/15 text-success grid place-items-center">
      <CircleCheck size={104} strokeWidth={1.5} />
    </div>
    <h2 class="text-[2rem] font-semibold text-text-bright mt-9">{$_("updates.none")}</h2>
    {#if checkedLabel}
      <p class="text-[1rem] text-text-muted mt-3">{$_("updates.lastCheck", { values: { time: checkedLabel } })}</p>
    {/if}
    <p class="text-[1rem] mt-1 {listsAge === null || listsAge > 2 * 86400 ? 'text-warning' : 'text-text-muted'}">{listsLabel}</p>
    <button
      class="mt-9 px-8 py-3 rounded bg-primary hover:bg-primary-hover text-white text-[1rem] font-semibold disabled:opacity-60 disabled:cursor-default"
      disabled={anyRunning}
      on:click={checkNow}
    >
      {$_("updates.checkNow")}
    </button>
  </div>
{:else if items.length}
  <div class="bg-surface border border-border rounded overflow-hidden">
    {#each items as item (keyOf(item))}
      {@const running = $jobs.some((j) => j.key === keyOf(item) && j.status === "running")}
      <div class="flex items-center gap-3.5 px-4 py-3 border-t border-border first:border-t-0">
        <input
          type="checkbox"
          checked={!unselected.has(keyOf(item))}
          class="accent-primary w-4 h-4 shrink-0"
          aria-label={item.name}
          on:change={() => toggle(item)}
        />
        <AppIcon src={item.icon} size={40} />
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <span class="text-[0.88rem] font-semibold text-text-bright truncate">{item.name}</span>
            <span class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted shrink-0">{$_(`sources.${item.source}`)}</span>
          </div>
          <div class="text-[0.75rem] text-text-muted truncate">{item.from ? `${item.from} → ${item.to}` : item.to}</div>
        </div>
        <button
          class="px-2.5 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.75rem] shrink-0 disabled:opacity-60 disabled:cursor-default"
          disabled={anyRunning}
          on:click={() => updateOne(item)}
        >
          {running ? $_("common.updating") : $_("common.update")}
        </button>
      </div>
    {/each}
  </div>
{/if}
