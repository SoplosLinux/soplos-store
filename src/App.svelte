<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Compass, LayoutGrid, Package, RefreshCw, SlidersHorizontal } from "lucide-svelte";
  import { api } from "./lib/api";
  import { busy } from "./lib/busy";
  import { searchRequest } from "./lib/search";
  import { languageChanged } from "./i18n";
  import Sidebar from "./components/Sidebar.svelte";
  import AboutDialog from "./components/AboutDialog.svelte";
  import JobBar from "./components/JobBar.svelte";
  import ScopeDialog from "./components/ScopeDialog.svelte";
  import Toast from "./components/Toast.svelte";
  import { initJobs } from "./lib/jobs";
  import Explore, { clearExploreCache } from "./views/Explore.svelte";
  import Categories from "./views/Categories.svelte";
  import CategoryPage from "./views/CategoryPage.svelte";
  import AppDetail from "./views/AppDetail.svelte";
  import PackageFile from "./views/PackageFile.svelte";
  import Installed from "./views/Installed.svelte";
  import Updates from "./views/Updates.svelte";
  import Settings from "./views/Settings.svelte";

  // The footer never shows the package revision, only the base version.
  const FOOTER_VERSION = "3.0.0-beta";

  const tabs = [
    { id: "explore", icon: Compass },
    { id: "categories", icon: LayoutGrid },
    { id: "installed", icon: Package },
    { id: "updates", icon: RefreshCw },
    { id: "settings", icon: SlidersHorizontal },
  ];

  // Pages opened on top of the active tab (a category, an app, a package
  // file); Back pops the last one, choosing a tab clears them all.
  type Page = { kind: "category"; id: string } | { kind: "app"; id: string } | { kind: "package"; path: string };

  let active = "explore";
  let stack: Page[] = [];
  let envLabel = "";
  let aboutOpen = false;
  let refreshKey = 0;
  let scroller: HTMLDivElement;
  let unlisten: UnlistenFn | null = null;
  let unlistenUpdates: UnlistenFn | null = null;

  $: page = stack[stack.length - 1];
  $: footerLabel = [envLabel, FOOTER_VERSION].filter(Boolean).join(" · ");

  function selectTab(id: string) {
    active = id;
    stack = [];
    scrollTop();
  }

  function push(next: Page) {
    stack = [...stack, next];
    scrollTop();
  }

  function back() {
    if (stack.length === 0) return;
    stack = stack.slice(0, -1);
    scrollTop();
  }

  function scrollTop() {
    if (scroller) scroller.scrollTop = 0;
  }

  // Reloads whatever is on screen, ignoring the cached Explore sections.
  function refresh() {
    clearExploreCache();
    refreshKey++;
  }

  // Names and descriptions come from Debian in the app's language, so a
  // language change loads the page again.
  let seenLanguage = $languageChanged;
  $: if ($languageChanged !== seenLanguage) {
    seenLanguage = $languageChanged;
    refresh();
  }

  // Ctrl+F: goes to a page that has a search (Explore or Installed) and
  // unfolds its search box.
  async function focusSearch() {
    if (stack.length) stack = [];
    if (active !== "explore" && active !== "installed") active = "explore";
    await tick();
    searchRequest.update((n) => n + 1);
  }

  const openApp = (id: string) => push({ kind: "app", id });
  const openCategory = (id: string) => push({ kind: "category", id });
  const openPackage = (path: string) => push({ kind: "package", path });

  // F1 -> About, Ctrl+Q -> quit, Ctrl+Tab / Ctrl+Shift+Tab -> cycle sections,
  // same as the other Soplos 3.0 apps. Also F5 / Ctrl+R -> reload,
  // Ctrl+F -> search, Escape / Alt+Left -> back.
  function handleKeydown(e: KeyboardEvent) {
    if (aboutOpen) {
      if (e.key === "Escape") aboutOpen = false;
      return;
    }
    const key = e.key.toLowerCase();
    if (e.key === "F1") {
      e.preventDefault();
      aboutOpen = true;
    } else if (e.key === "F5" || (e.ctrlKey && key === "r")) {
      e.preventDefault();
      refresh();
    } else if (e.altKey && e.key === "ArrowLeft") {
      e.preventDefault();
      back();
    } else if (e.key === "Escape" && !(e.target instanceof HTMLInputElement)) {
      back();
    } else if (e.ctrlKey && key === "q") {
      e.preventDefault();
      getCurrentWindow().close();
    } else if (e.ctrlKey && key === "f") {
      e.preventDefault();
      focusSearch();
    } else if (e.ctrlKey && e.code === "Tab") {
      e.preventDefault();
      const direction = e.shiftKey ? -1 : 1;
      const index = tabs.findIndex((t) => t.id === active);
      selectTab(tabs[(index + direction + tabs.length) % tabs.length].id);
    }
  }

  onMount(async () => {
    await initJobs();
    unlisten = await listen<string>("open-package", (event) => openPackage(event.payload));
    // The tray icon and its notification open the app on Updates.
    unlistenUpdates = await listen("open-updates", () => selectTab("updates"));
    if (await api.takeStartupUpdates().catch(() => false)) selectTab("updates");
    const startupFile = await api.takeStartupFile().catch(() => null);
    if (startupFile) openPackage(startupFile);
    try {
      const env = await api.environment();
      envLabel = [env.desktop, env.session].filter(Boolean).join(" · ");
    } catch (error) {
      console.error("Failed to read environment info", error);
    }
  });

  onDestroy(() => {
    unlisten?.();
    unlistenUpdates?.();
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="flex h-screen overflow-hidden">
  <Sidebar {tabs} {active} onSelect={selectTab} {footerLabel} onAbout={() => (aboutOpen = true)} />

  <div class="flex-1 flex flex-col overflow-hidden min-w-0">
    <div class="content-scroll flex-1 overflow-y-auto p-7" bind:this={scroller}>
      {#key refreshKey}
        {#if page?.kind === "app"}
          {#key page}
            <AppDetail id={page.id} onBack={back} />
          {/key}
        {:else if page?.kind === "category"}
          {#key page}
            <CategoryPage category={page.id} onBack={back} onOpenApp={openApp} />
          {/key}
        {:else if page?.kind === "package"}
          {#key page}
            <PackageFile path={page.path} onBack={back} />
          {/key}
        {:else if active === "explore"}
          <Explore onOpenApp={openApp} onOpenCategory={openCategory} />
        {:else if active === "categories"}
          <Categories onOpenCategory={openCategory} />
        {:else if active === "installed"}
          <Installed onOpenApp={openApp} />
        {:else if active === "updates"}
          <Updates />
        {:else}
          <Settings />
        {/if}
      {/key}
    </div>

    <JobBar />
    <div class="h-1.5 bg-surface-2 shrink-0 overflow-hidden">
      {#if $busy > 0}
        <div class="h-full w-1/3 bg-primary progress-slide"></div>
      {/if}
    </div>
  </div>
</div>

{#if aboutOpen}
  <AboutDialog onClose={() => (aboutOpen = false)} />
{/if}
<ScopeDialog />
<Toast />
