<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { FileArchive } from "lucide-svelte";
  import { api, type LocalPackage } from "../lib/api";
  import BackButton from "../components/BackButton.svelte";
  import { jobs, runJob, chooseScope } from "../lib/jobs";

  // A package file opened from the file manager (.deb, .flatpak,
  // .flatpakref, .snap or .AppImage).
  export let path: string;
  export let onBack: (() => void) | null = null;

  let pkg: LocalPackage | null = null;
  let loaded = false;
  let launchError = "";

  onMount(async () => {
    pkg = await api.inspectPackage(path).catch(() => null);
    loaded = true;
    // A .deb that is already installed can be opened at once.
    await checkOpen();
  });

  let installedOk = false;
  // What a .deb installed in the menu, read from dpkg once it is installed.
  let canOpen = false;

  async function checkOpen() {
    canOpen = pkg?.kind === "deb" ? await api.canOpenPackage(pkg.name).catch(() => false) : false;
  }

  async function openPackage() {
    if (!pkg) return;
    launchError = "";
    try {
      await api.openPackage(pkg.name);
    } catch {
      launchError = $_("common.openFailed", { values: { name: pkg.name } });
    }
  }
  $: running = $jobs.some((j) => j.key === path && j.status === "running");

  async function install() {
    if (!pkg || running) return;
    const title = $_("jobs.installing", { values: { name: pkg.name } });
    if (pkg.kind === "deb") {
      installedOk = await runJob(title, { action: "install-file", source: "apt", file: pkg.path }, path);
      if (installedOk) await checkOpen();
    } else if (pkg.kind === "flatpak" || pkg.kind === "flatpakref") {
      const scope = await chooseScope(pkg.name);
      if (!scope) return;
      installedOk = await runJob(title, { action: "install-file", source: "flatpak", file: pkg.path, scope }, path);
    } else if (pkg.kind === "snap") {
      installedOk = await runJob(title, { action: "install-file", source: "snap", file: pkg.path }, path);
    } else if (pkg.kind === "appimage") {
      installedOk = await runJob(title, { action: "install-file", source: "appimage", file: pkg.path }, path);
    }
  }

  async function openManager() {
    launchError = "";
    try {
      await api.launchApp("soplos-appimage-manager");
    } catch {
      launchError = $_("settings.launchFailed", { values: { app: "Soplos AppImage Manager" } });
    }
  }
</script>

{#if onBack}
  <BackButton {onBack} />
{/if}

{#if !loaded}
  <p class="text-[0.85rem] text-text-muted">{$_("common.loading")}</p>
{:else if !pkg}
  <p class="text-[0.85rem] text-text-muted">{$_("local.unreadable")}</p>
{:else}
  <div class="bg-surface border border-border rounded p-5 flex items-center gap-5">
    <div class="w-[96px] h-[96px] rounded-xl bg-primary/15 text-primary grid place-items-center shrink-0">
      <FileArchive size={48} strokeWidth={1.5} />
    </div>
    <div class="flex-1 min-w-0">
      <div class="text-[0.68rem] font-bold tracking-[0.08em] uppercase text-primary">{$_("local.kicker")}</div>
      <h2 class="text-[1.4rem] font-bold text-text-bright truncate">{pkg.name}</h2>
      {#if pkg.summary}
        <p class="text-[0.85rem] text-text mt-1">{pkg.summary}</p>
      {/if}
      <div class="mt-3 flex items-center gap-3">
        <button
          class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.78rem] font-semibold disabled:opacity-60 disabled:cursor-default"
          disabled={running || installedOk}
          on:click={install}
        >
          {running ? $_("common.installing") : installedOk ? $_("settings.installedTag") : $_("common.install")}
        </button>
        {#if canOpen}
          <button class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.78rem]" on:click={openPackage}>
            {$_("common.open")}
          </button>
        {/if}
        {#if pkg.kind === "appimage"}
          <button class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.78rem]" on:click={openManager}>
            {$_("local.openManager")}
          </button>
        {/if}
      </div>
      {#if launchError}
        <p class="text-[0.75rem] text-danger mt-2">{launchError}</p>
      {/if}
    </div>
  </div>

  <div class="bg-surface border border-border rounded p-4 mt-3.5 text-[0.8rem]">
    {#each [
      [$_("local.type"), $_(`local.kinds.${pkg.kind}`)],
      [$_("local.version"), pkg.version],
      [$_("local.size"), pkg.size],
      [$_("local.file"), pkg.path],
    ] as [label, value]}
      {#if value}
        <div class="flex justify-between gap-3 py-2 border-t border-border first:border-t-0">
          <span class="text-text-muted shrink-0">{label}</span>
          <span class="text-text-bright text-right break-all">{value}</span>
        </div>
      {/if}
    {/each}
  </div>
{/if}
