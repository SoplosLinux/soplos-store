<script lang="ts">
  import { onMount } from "svelte";
  import { CircleCheck, Search } from "lucide-svelte";
  import { _, locale } from "svelte-i18n";
  import { api, type SourceKind, type UpdateItem } from "../lib/api";
  import { jobs, runJob, refreshLists } from "../lib/jobs";
  import { availableSources } from "../lib/sources";
  import AppIcon from "../components/AppIcon.svelte";
  import CheckNowButton from "../components/CheckNowButton.svelte";
  import StateIcon from "../components/StateIcon.svelte";
  import UpdatesMeta from "../components/UpdatesMeta.svelte";

  let items: UpdateItem[] = [];
  let loaded = false;
  let checkedAt: Date | null = null;
  let listsAge: number | null = null;
  let unselected = new Set<string>();

  $: sourcesLabel = $availableSources.map((s) => $_(`sources.${s}`)).join(" • ");

  const keyOf = (item: UpdateItem) => `${item.source}:${item.id}`;

  // Shows the result of the last check, the same one the tray notification
  // counted, so opening the page does not look anywhere again. With
  // `again` every source is looked in once more (no "apt update").
  async function load(again = false) {
    const [status, view] = await Promise.all([
      api.aptStatus().catch(() => null),
      (again ? api.rescanUpdates() : api.updates()).catch(() => null),
    ]);
    listsAge = status?.listsAgeSecs ?? null;
    if (view) {
      items = view.items;
      checkedAt = view.checkedAt ? new Date(view.checkedAt * 1000) : null;
    }
    loaded = true;
  }

  // Opening the page always looks for real: the package lists are refreshed
  // and every source is asked, whatever the stored result or the age of the
  // lists. Anything done meanwhile in a terminal, an update or a refresh, is
  // seen. Until the answer comes the page says it is searching, because the
  // stored list could show updates that no longer exist or miss new ones.
  let autoChecking = true;

  onMount(async () => {
    try {
      await load();
      await checkNow();
    } finally {
      autoChecking = false;
    }
  });

  // "Check now" is the only thing that does work on its own: one job that
  // refreshes the package lists and looks in every source, and stores the
  // result, which the page then reads.
  async function checkNow() {
    await refreshLists();
    await load();
  }

  function toggle(item: UpdateItem) {
    const key = keyOf(item);
    unselected.has(key) ? unselected.delete(key) : unselected.add(key);
    unselected = unselected;
  }

  async function updateOne(item: UpdateItem) {
    const title = $_("jobs.updating", { values: { name: item.name } });
    if (await runJob(title, { action: "update", source: item.source, ids: [item.id] }, keyOf(item))) await load(true);
  }

  // One job per source, with every selected package of that source.
  async function updateSelected() {
    const selected = items.filter((item) => !unselected.has(keyOf(item)));
    const sources: SourceKind[] = ["apt", "flatpak", "snap", "appimage", "firmware"];
    for (const source of sources) {
      const ids = selected.filter((item) => item.source === source).map((item) => item.id);
      if (!ids.length) continue;
      const title = $_("jobs.updatingAll", { values: { count: ids.length } });
      await runJob(title, { action: "update", source, ids }, `update-all:${source}`);
    }
    await load(true);
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
  $: checkedLabel = checkedAt
    ? $_("updates.lastCheck", {
        values: { time: checkedAt.toLocaleTimeString($locale ?? undefined, { hour: "2-digit", minute: "2-digit" }) },
      })
    : "";
  $: listsLabel =
    listsAge === null ? $_("updates.listsNever") : $_("updates.listsRefreshed", { values: { when: ago(listsAge, $locale) } });
  $: listsStale = listsAge === null || listsAge > 2 * 86400;

  function toggleAll() {
    if (selectedCount === items.length) {
      items.forEach((item) => unselected.add(keyOf(item)));
    } else {
      unselected.clear();
    }
    unselected = unselected;
  }

  // The page is one block, never a header plus a body saying the same
  // thing: the spinner while the first result is read (or while a check
  // runs and there is nothing to show yet), the "all up to date" message,
  // or the list. The menu on the left already names the page.
  $: state = !loaded || autoChecking || (anyRunning && items.length === 0) ? "searching" : items.length === 0 ? "none" : "list";
</script>

{#if state === "searching"}
  <div class="h-full min-h-[20rem] flex flex-col items-center justify-center text-center px-4 gap-6">
    <StateIcon tone="primary" pulse>
      <Search size="56%" strokeWidth={1.25} />
    </StateIcon>
    <h3 class="text-[1.4rem] font-semibold text-text-bright">{$_("updates.refreshing")}...</h3>
  </div>
{:else if state === "none"}
  <div class="h-full min-h-[20rem] flex flex-col items-center justify-center text-center px-6 gap-5">
    <StateIcon tone="success">
      <CircleCheck size="56%" strokeWidth={1.25} />
    </StateIcon>
    <div>
      <h3 class="text-[1.4rem] font-semibold text-text-bright">{$_("updates.none")}</h3>
      <p class="text-[0.88rem] text-text-muted mt-1.5 max-w-md">{$_("updates.noneSub")}</p>
    </div>
    <UpdatesMeta checked={checkedLabel} lists={listsLabel} {listsStale} sources={sourcesLabel} />
    <CheckNowButton busy={anyRunning} disabled={!loaded} on:click={checkNow} />
  </div>
{:else}
  <div class="bg-surface border border-border rounded overflow-hidden">
    <!-- The one bar: selection and count on the left, the two actions on the right -->
    <div class="flex items-center justify-between gap-3 flex-wrap px-4 py-2.5 bg-surface-2/60 border-b border-border text-[0.78rem] text-text-muted">
      <label class="flex items-center gap-2.5 cursor-pointer select-none">
        <input
          type="checkbox"
          checked={selectedCount === items.length}
          indeterminate={selectedCount > 0 && selectedCount < items.length}
          class="accent-primary w-4 h-4 rounded cursor-pointer"
          on:change={toggleAll}
        />
        <span class="text-text">{$_("updates.selectedCount", { values: { selected: selectedCount, total: items.length } })}</span>
      </label>
      <div class="flex items-center gap-2">
        <CheckNowButton busy={anyRunning} on:click={checkNow} />
        <button
          class="px-3.5 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.8rem] font-semibold transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
          disabled={anyRunning || selectedCount === 0}
          on:click={updateSelected}
        >
          {anyRunning ? $_("common.updating") : $_("updates.updateAll")}
        </button>
      </div>
    </div>

    {#each items as item (keyOf(item))}
      {@const running = $jobs.some((j) => j.key === keyOf(item) && j.status === "running")}
      <div class="flex items-center gap-3.5 px-4 py-3 border-t border-border hover:bg-surface-2/40 transition-colors">
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
            <span class="text-[0.88rem] font-semibold text-text-bright truncate">{item.name}</span>
            <span class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted shrink-0 font-medium">{$_(`sources.${item.source}`)}</span>
          </div>
          <div class="text-[0.75rem] text-text-muted truncate mt-0.5">{item.from ? `${item.from} → ${item.to}` : item.to}</div>
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

    <!-- Same information as the empty state, once, at the foot of the list -->
    <div class="px-4 py-2.5 border-t border-border bg-surface-2/30">
      <UpdatesMeta checked={checkedLabel} lists={listsLabel} {listsStale} sources={sourcesLabel} />
    </div>
  </div>
{/if}
