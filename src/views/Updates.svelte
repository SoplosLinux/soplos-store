<script lang="ts">
  import { onMount } from "svelte";
  import { CircleCheck, Search } from "lucide-svelte";
  import { _, locale } from "svelte-i18n";
  import { api, type PlanChange, type SourceKind, type UpdateItem } from "../lib/api";
  import { jobs, runJob, refreshLists } from "../lib/jobs";
  import { availableSources } from "../lib/sources";
  import { rowProgress, type RowProgress } from "../lib/updateProgress";
  import AppIcon from "../components/AppIcon.svelte";
  import CheckNowButton from "../components/CheckNowButton.svelte";
  import StateIcon from "../components/StateIcon.svelte";
  import UpdatesMeta from "../components/UpdatesMeta.svelte";

  let items: UpdateItem[] = [];
  // What comes in and goes out with the updates; only told, never selected.
  let added: PlanChange[] = [];
  let removed: PlanChange[] = [];
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
      added = view.added ?? [];
      removed = view.removed ?? [];
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

  // Apps whose job has not started (it waits for the one before) and apps
  // whose job has finished while the list is read again. Without the second
  // the button would go back to "Update" for a moment before the page says
  // everything is up to date.
  let queued = new Set<string>();
  let settled = new Set<string>();

  async function updateOne(item: UpdateItem) {
    const title = $_("jobs.updating", { values: { name: item.name } });
    if (await runJob(title, { action: "update", source: item.source, ids: [item.id] }, keyOf(item))) {
      settled = new Set(settled).add(keyOf(item));
      await load(true);
      settled = new Set();
    }
  }

  // One job per source, with every selected package of that source.
  async function updateSelected() {
    const selected = items.filter((item) => !unselected.has(keyOf(item)));
    const sources: SourceKind[] = ["apt", "flatpak", "snap", "appimage", "firmware"];
    queued = new Set(selected.map(keyOf));
    for (const source of sources) {
      const group = selected.filter((item) => item.source === source);
      const ids = group.map((item) => item.id);
      if (!ids.length) continue;
      const title = $_("jobs.updatingAll", { values: { count: ids.length } });
      const ok = await runJob(title, { action: "update", source, ids }, `update-all:${source}`);
      const keys = group.map(keyOf);
      queued = new Set([...queued].filter((key) => !keys.includes(key)));
      if (ok) settled = new Set([...settled, ...keys]);
    }
    await load(true);
    settled = new Set();
    queued = new Set();
  }

  // Where an app is while it updates: by the job of its own button, or by the
  // job of "Update all" for its source when it is one of the selected apps.
  // Null when nothing is updating it.
  function progressOf(item: UpdateItem, list: typeof $jobs, skipped: Set<string>, waiting: Set<string>, finished: Set<string>): RowProgress | null {
    const key = keyOf(item);
    const own = list.find((j) => j.status === "running" && j.key === key);
    if (own) return rowProgress(own.progress, item.source, item.id, 1);
    if (skipped.has(key)) return null;
    const group = list.find((j) => j.status === "running" && j.key === `update-all:${item.source}`);
    if (group) {
      const size = items.filter((i) => i.source === item.source && !skipped.has(keyOf(i))).length;
      return rowProgress(group.progress, item.source, item.id, size);
    }
    if (finished.has(key)) return { phase: "done", fraction: 1 };
    if (waiting.has(key)) return { phase: "queued", fraction: 0 };
    return null;
  }

  function ago(seconds: number, lang: string | null | undefined) {
    const rtf = new Intl.RelativeTimeFormat(lang ?? undefined, { numeric: "auto" });
    const minutes = Math.round(seconds / 60);
    if (minutes < 60) return rtf.format(-minutes, "minute");
    const hours = Math.round(minutes / 60);
    if (hours < 48) return rtf.format(-hours, "hour");
    return rtf.format(-Math.round(hours / 24), "day");
  }

  $: anyRunning =
    queued.size > 0 ||
    settled.size > 0 ||
    $jobs.some((j) => j.status === "running" && (j.key === "refresh-lists" || j.key?.startsWith("update") || items.some((i) => keyOf(i) === j.key)));
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
  <!-- No overflow clipping here: it would stop the bar below from sticking to the top while the list scrolls -->
  <div class="bg-surface border border-border rounded">
    <!-- The one bar: selection and count on the left, the two actions on the right; it stays in view while the list scrolls -->
    <div class="sticky top-[-1.75rem] z-10 rounded-t flex items-center justify-between gap-3 flex-wrap px-4 py-2.5 bg-surface-2 border-b border-border text-[0.78rem] text-text-muted">
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

    <!-- What the updates take out and bring in, as GNOME Software tells it: listed, not selectable -->
    {#if removed.length || added.length}
      {#if removed.length}
        <div class="px-4 pt-3 pb-1.5 text-[0.75rem] font-semibold text-text-bright">{$_("updates.sectionRemoved")}</div>
        {#each removed as change (change.name)}
          <div class="flex items-center justify-between gap-3 px-4 py-2 border-t border-border text-[0.8rem]">
            <span class="text-text truncate">{change.name}</span>
            <span class="text-text-muted shrink-0">{change.version}</span>
          </div>
        {/each}
      {/if}
      {#if added.length}
        <div class="px-4 pt-3 pb-1.5 text-[0.75rem] font-semibold text-text-bright">{$_("updates.sectionAdded")}</div>
        {#each added as change (change.name)}
          <div class="flex items-center justify-between gap-3 px-4 py-2 border-t border-border text-[0.8rem]">
            <span class="text-text truncate">{change.name}</span>
            <span class="text-text-muted shrink-0">{change.version}</span>
          </div>
        {/each}
      {/if}
      <div class="px-4 pt-3 pb-1.5 text-[0.75rem] font-semibold text-text-bright">{$_("updates.sectionUpdates")}</div>
    {/if}

    {#each items as item (keyOf(item))}
      {@const progress = progressOf(item, $jobs, unselected, queued, settled)}
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
        <!-- The line along the lower edge of the button is how far this app is (see updateProgress.ts) -->
        <div class="relative shrink-0 rounded overflow-hidden">
          <button
            class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.75rem] font-medium disabled:opacity-60 disabled:cursor-not-allowed transition-colors"
            disabled={anyRunning || !!progress}
            on:click={() => updateOne(item)}
          >
            {progress ? $_(`updates.phase.${progress.phase}`) : $_("common.update")}
          </button>
          {#if progress && progress.fraction !== null}
            <div class="absolute left-0 right-0 bottom-0 h-[3px]" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(progress.fraction * 100)}>
              <div class="h-full bg-primary transition-[width] duration-300" style="width: {Math.round(progress.fraction * 100)}%"></div>
            </div>
          {/if}
        </div>
      </div>
    {/each}

    <!-- Same information as the empty state, once, at the foot of the list -->
    <div class="px-4 py-2.5 border-t border-border bg-surface-2/30 rounded-b">
      <UpdatesMeta checked={checkedLabel} lists={listsLabel} {listsStale} sources={sourcesLabel} />
    </div>
  </div>
{/if}
