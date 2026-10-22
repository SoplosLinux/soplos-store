<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { _ } from "svelte-i18n";
  import { ChevronLeft, ChevronRight, ExternalLink, Star } from "lucide-svelte";
  import { api, type AppDetail } from "../lib/api";
  import { resolveImage } from "../lib/images";
  import BackButton from "../components/BackButton.svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import InstallButton from "../components/InstallButton.svelte";

  export let id: string;
  export let onBack: () => void;

  // AppStream categories that have a name in the dictionaries.
  const CATEGORY_IDS: Record<string, string> = {
    AudioVideo: "audiovideo",
    Audio: "audiovideo",
    Video: "audiovideo",
    Development: "development",
    Education: "education",
    Game: "game",
    Graphics: "graphics",
    Network: "network",
    Office: "office",
    Science: "science",
    System: "system",
    Utility: "utility",
  };

  const LINK_LABELS: Record<string, string> = {
    homepage: "detail.website",
    help: "detail.linkHelp",
    bugtracker: "detail.linkBugtracker",
    donation: "detail.linkDonation",
    source: "detail.linkSource",
  };

  let detail: AppDetail | null = null;
  let loaded = false;
  let selected = 0;
  let shots: (string | null)[] = [];
  let shotIndex = 0;
  let lightbox = false;

  onMount(async () => {
    detail = await api.detail(id).catch(() => null);
    loaded = true;
    if (detail) {
      // Start on a source the app is already installed from, otherwise on
      // the first one (APT, the system's own source); the user picks another
      // one here before installing.
      const installedIndex = detail.sources.findIndex((s) => s.installed);
      selected = Math.max(installedIndex, 0);
      shots = await Promise.all(detail.screenshots.map(resolveImage));
    }
  });

  $: source = detail?.sources[selected];
  $: categories = [...new Set((detail?.categories ?? []).map((c) => CATEGORY_IDS[c]).filter(Boolean))];
  $: maxStars = Math.max(1, ...(detail?.ratingStars ?? [0]));
  $: shotCount = shots.filter(Boolean).length;

  function go(step: number) {
    if (!shotCount) return;
    shotIndex = (shotIndex + step + shots.length) % shots.length;
  }

  // Escape closes the large view first, before the "go back" shortcut.
  function onKeydown(event: KeyboardEvent) {
    if (!lightbox) return;
    if (event.key === "Escape") {
      event.stopImmediatePropagation();
      lightbox = false;
    } else if (event.key === "ArrowLeft") go(-1);
    else if (event.key === "ArrowRight") go(1);
  }

  const rows = (d: AppDetail, s: AppDetail["sources"][number] | undefined) =>
    [
      [$_("detail.developer"), d.developer],
      [$_("detail.license"), d.license],
      [$_("detail.version"), s?.version ?? ""],
      [$_("detail.downloadSize"), s?.downloadSize ?? ""],
      [$_("detail.installedSize"), s?.installedSize ?? ""],
      [$_("detail.released"), d.release?.date ?? ""],
      [$_("detail.package"), s?.id ?? ""],
    ].filter(([, value]) => value);
</script>

<svelte:window on:keydown|capture={onKeydown} />

<BackButton {onBack} />

{#if !loaded}
  <p class="text-[0.85rem] text-text-muted">{$_("common.loading")}</p>
{:else if !detail}
  <p class="text-[0.85rem] text-text-muted">{$_("detail.notFound")}</p>
{:else}
  <!-- Header -->
  <div class="bg-surface border border-border rounded p-6 flex items-center gap-6 mb-5">
    <AppIcon src={detail.icon} size={120} />
    <div class="flex-1 min-w-0">
      <h2 class="text-[1.8rem] font-bold text-text-bright leading-tight">{detail.name}</h2>
      {#if detail.developer}
        <div class="text-[0.85rem] text-text-muted mt-0.5">{detail.developer}</div>
      {/if}
      <p class="text-[0.95rem] text-text mt-2">{detail.summary}</p>
      <div class="flex items-center gap-1.5 mt-2 text-[0.82rem] text-text-muted">
        {#if detail.rating !== null && detail.reviews > 0}
          <Star size={15} class="text-warning" fill="currentColor" />
          <span class="text-text-bright font-semibold">{detail.rating.toFixed(1)}</span>
          <span>· {$_("common.reviews", { values: { count: detail.reviews } })}</span>
        {:else}
          {$_("common.noRatings")}
        {/if}
      </div>
      <div class="flex items-center gap-2.5 mt-4 flex-wrap">
        {#if detail.sources.length}
          <select
            bind:value={selected}
            title={$_("detail.source")}
            class="appearance-none bg-surface-2 border border-border rounded px-3 py-2 text-[0.82rem] text-text-bright outline-none focus:border-primary"
          >
            {#each detail.sources as option, i}
              <option value={i}>{$_(`sources.${option.kind}`)}{option.version ? ` · ${option.version}` : ""}</option>
            {/each}
          </select>
        {/if}
        <InstallButton appId={detail.id} name={detail.name} refs={detail.sources} selected={source?.kind ?? null} large />
      </div>
    </div>
  </div>

  <!-- Screenshots: a large one with thumbnails below -->
  {#if shotCount}
    <div class="mb-5">
      <div class="relative bg-panel border border-border rounded overflow-hidden">
        {#key shotIndex}
          {#if shots[shotIndex]}
            <button class="block w-full cursor-zoom-in" on:click={() => (lightbox = true)} in:fade={{ duration: 150 }}>
              <img src={shots[shotIndex]} alt="" class="mx-auto max-h-[480px] w-auto max-w-full object-contain" />
            </button>
          {/if}
        {/key}
        {#if shotCount > 1}
          <button
            class="absolute left-3 top-1/2 -translate-y-1/2 w-9 h-9 rounded-full bg-black/55 hover:bg-black/75 text-white grid place-items-center"
            title={$_("detail.previous")}
            aria-label={$_("detail.previous")}
            on:click={() => go(-1)}
          >
            <ChevronLeft size={20} />
          </button>
          <button
            class="absolute right-3 top-1/2 -translate-y-1/2 w-9 h-9 rounded-full bg-black/55 hover:bg-black/75 text-white grid place-items-center"
            title={$_("detail.next")}
            aria-label={$_("detail.next")}
            on:click={() => go(1)}
          >
            <ChevronRight size={20} />
          </button>
        {/if}
      </div>
      {#if shotCount > 1}
        <div class="content-scroll flex gap-2.5 overflow-x-auto mt-3 pb-1.5">
          {#each shots as shot, i}
            {#if shot}
              <button
                class="shrink-0 rounded border-2 overflow-hidden {i === shotIndex ? 'border-primary' : 'border-border opacity-70 hover:opacity-100'}"
                aria-label={`${i + 1}`}
                on:click={() => (shotIndex = i)}
              >
                <img src={shot} alt="" class="h-[72px] w-auto" />
              </button>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <div class="grid grid-cols-1 xl:grid-cols-[1fr_340px] gap-5">
    <!-- Description and news -->
    <div class="flex flex-col gap-5 min-w-0">
      <div class="bg-surface border border-border rounded p-5">
        <h3 class="text-[1rem] font-semibold text-text-bright mb-3">{$_("detail.description")}</h3>
        {#if detail.description.length}
          {#each detail.description as paragraph}
            <p class="text-[0.9rem] leading-relaxed mb-3 last:mb-0">{paragraph}</p>
          {/each}
        {:else}
          <p class="text-[0.9rem] text-text-muted">{$_("detail.noDescription")}</p>
        {/if}
      </div>

      {#if detail.release?.notes.length}
        <div class="bg-surface border border-border rounded p-5">
          <h3 class="text-[1rem] font-semibold text-text-bright mb-3">{$_("detail.whatsNew", { values: { version: detail.release.version } })}</h3>
          {#each detail.release.notes as note}
            <p class="text-[0.88rem] leading-relaxed mb-2 last:mb-0">{note}</p>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Ratings and details -->
    <div class="flex flex-col gap-5 min-w-0">
      {#if detail.ratingStars.length && detail.reviews > 0}
        <div class="bg-surface border border-border rounded p-5">
          <h3 class="text-[1rem] font-semibold text-text-bright mb-3">{$_("detail.ratings")}</h3>
          <div class="flex items-center gap-4 mb-4">
            <div class="text-[2.6rem] font-bold text-text-bright leading-none">{detail.rating?.toFixed(1)}</div>
            <div>
              <div class="flex gap-0.5">
                {#each [1, 2, 3, 4, 5] as n}
                  <Star size={16} class="text-warning" fill={n <= Math.round(detail.rating ?? 0) ? "currentColor" : "none"} />
                {/each}
              </div>
              <div class="text-[0.75rem] text-text-muted mt-1">{$_("detail.basedOn", { values: { count: detail.reviews } })}</div>
            </div>
          </div>
          {#each [5, 4, 3, 2, 1] as n}
            <div class="flex items-center gap-2 text-[0.75rem] text-text-muted mb-1.5 last:mb-0">
              <span class="w-3 text-right">{n}</span>
              <Star size={11} class="text-warning" fill="currentColor" />
              <div class="flex-1 h-2 rounded-full bg-surface-2 overflow-hidden">
                <div class="h-full bg-warning rounded-full" style="width: {((detail.ratingStars[n - 1] ?? 0) / maxStars) * 100}%"></div>
              </div>
              <span class="w-10 text-right">{detail.ratingStars[n - 1] ?? 0}</span>
            </div>
          {/each}
        </div>
      {/if}

      <div class="bg-surface border border-border rounded p-5 text-[0.82rem]">
        <h3 class="text-[1rem] font-semibold text-text-bright mb-3">{$_("detail.details")}</h3>
        {#each rows(detail, source) as [label, value]}
          <div class="flex justify-between gap-3 py-2 border-t border-border first:border-t-0">
            <span class="text-text-muted shrink-0">{label}</span>
            <span class="text-text-bright text-right break-all">{value}</span>
          </div>
        {/each}
        {#if categories.length}
          <div class="py-2 border-t border-border">
            <div class="text-text-muted mb-2">{$_("detail.categories")}</div>
            <div class="flex flex-wrap gap-1.5">
              {#each categories as category}
                <span class="px-2 py-0.5 rounded bg-surface-2 text-text-bright text-[0.75rem]">{$_(`categories.${category}`)}</span>
              {/each}
            </div>
          </div>
        {/if}
        {#if detail.links.length}
          <div class="py-2 border-t border-border">
            <div class="text-text-muted mb-2">{$_("detail.links")}</div>
            <div class="flex flex-col gap-1.5">
              {#each detail.links as link}
                <button class="flex items-center gap-1.5 text-primary hover:underline text-left" on:click={() => api.openUrl(link.url)}>
                  <ExternalLink size={13} class="shrink-0" />
                  {$_(LINK_LABELS[link.kind])}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Large view of the screenshot -->
  {#if lightbox && shots[shotIndex]}
    <div
      class="fixed inset-0 z-50 bg-black/85 flex items-center justify-center p-6"
      role="presentation"
      transition:fade={{ duration: 120 }}
      on:click|self={() => (lightbox = false)}
    >
      <img src={shots[shotIndex]} alt="" class="max-w-[94vw] max-h-[92vh] object-contain rounded" />
    </div>
  {/if}
{/if}
