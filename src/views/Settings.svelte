<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { api, type SourcesStatus } from "../lib/api";
  import { settings } from "../lib/settings";
  import { showToast } from "../lib/toast";
  import { jobs, runJob, type JobRequest } from "../lib/jobs";
  import Toggle from "../components/Toggle.svelte";

  let status: SourcesStatus | null = null;
  let launchError = "";

  // Every change is saved at once; the short message tells the user so.
  let lastSaved = JSON.stringify($settings);
  const saved = () => showToast($_("settings.saved"));
  $: {
    const now = JSON.stringify($settings);
    if (now !== lastSaved) {
      lastSaved = now;
      saved();
    }
  }

  onMount(async () => {
    status = await api.sources().catch(() => null);
  });

  // Installs a system package from APT and reads the status again.
  async function installPackage(pkg: string, label: string) {
    const title = $_("jobs.installing", { values: { name: label } });
    if (await runJob(title, { action: "install", source: "apt", ids: [pkg] }, `settings:${pkg}`)) {
      status = await api.sources().catch(() => status);
    }
  }

  // The testing switch follows what the system reports, not what was
  // clicked: it is set again from the status once the job ends, so it goes
  // back to where it was when the change fails.
  let testing = false;
  $: if (status) testing = status.fwupdTesting;

  // Turns the LVFS testing channel on or off and reads the status again.
  async function toggleTesting() {
    if (!status) return;
    const action = status.fwupdTesting ? "testing-off" : "testing-on";
    if (await runJob($_("jobs.firmwareChannel"), { action, source: "firmware" }, "settings:firmware")) {
      status = await api.sources().catch(() => status);
    }
    testing = status?.fwupdTesting ?? false;
  }

  async function maintenance(request: JobRequest) {
    await runJob($_("jobs.cleaning"), request, `settings:${request.action}`);
  }

  $: busyWith = (key: string) => $jobs.some((j) => j.key === `settings:${key}` && j.status === "running");

  async function launch(name: "soplos-repo-selector" | "soplos-appimage-manager", label: string) {
    launchError = "";
    try {
      await api.launchApp(name);
    } catch {
      launchError = $_("settings.launchFailed", { values: { app: label } });
    }
  }

  const selectClass =
    "appearance-none bg-surface-2 border border-border rounded px-2.5 py-1.5 text-[0.78rem] text-text-bright outline-none focus:border-primary";
  const ghostClass = "px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.78rem]";
  const primaryClass = "px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.78rem] font-semibold";
  const tagOk = "text-[0.62rem] px-1.5 py-0.5 rounded bg-success/15 text-success";
  const tagOff = "text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted";
</script>

<div class="flex flex-col gap-3.5 max-w-[900px]">
  {#if launchError}
    <p class="text-[0.8rem] text-danger bg-danger/10 border border-danger/40 rounded px-3 py-2">{launchError}</p>
  {/if}
  <section class="bg-surface border border-border rounded px-4 py-1.5">
    <h3 class="text-[0.95rem] font-semibold text-text-bright pt-3 pb-1.5">{$_("settings.sources")}</h3>
    <div class="settings-row">
      <div>
        <div class="settings-label">
          {$_("settings.apt")}
          {#if status && !status.repoSelector}
            <span class={tagOff}>Soplos Repo Selector: {$_("settings.notInstalled")}</span>
          {/if}
        </div>
        <div class="settings-desc">{$_("settings.aptDesc")}</div>
      </div>
      {#if status && !status.repoSelector}
        <button class={primaryClass} disabled={busyWith("soplos-repo-selector")} on:click={() => installPackage("soplos-repo-selector", "Soplos Repo Selector")}>
          {busyWith("soplos-repo-selector") ? $_("common.installing") : `${$_("common.install")} Soplos Repo Selector`}
        </button>
      {:else}
        <button class={ghostClass} on:click={() => launch("soplos-repo-selector", "Soplos Repo Selector")}>{$_("settings.manageRepos")}</button>
      {/if}
    </div>
    <div class="settings-row">
      <div>
        <div class="settings-label">
          {$_("settings.flatpak")}
          {#if status}
            <span class={status.flatpak ? tagOk : tagOff}>{status.flatpak ? $_("settings.installedTag") : $_("settings.notInstalled")}</span>
          {/if}
        </div>
        <div class="settings-desc">{status && !status.flatpak ? $_("settings.flatpakMissing") : $_("settings.flatpakDesc")}</div>
      </div>
      {#if status && !status.flatpak}
        <button class={primaryClass} disabled={busyWith("flatpak")} on:click={() => installPackage("flatpak", "Flatpak")}>
          {busyWith("flatpak") ? $_("common.installing") : $_("settings.installFlatpak")}
        </button>
      {:else}
        <select class={selectClass} title={$_("settings.scope")} bind:value={$settings.flatpakScope}>
          <option value="user">{$_("settings.scopeUser")}</option>
          <option value="system">{$_("settings.scopeSystem")}</option>
          <option value="ask">{$_("settings.scopeAsk")}</option>
        </select>
      {/if}
    </div>
    <div class="settings-row">
      <div>
        <div class="settings-label">
          {$_("settings.snap")}
          {#if status}
            <span class={status.snap ? tagOk : tagOff}>{status.snap ? $_("settings.installedTag") : $_("settings.notInstalled")}</span>
          {/if}
        </div>
        <div class="settings-desc">{$_("settings.snapDesc")}</div>
      </div>
      {#if status && !status.snap}
        <button class={primaryClass} disabled={busyWith("snapd")} on:click={() => installPackage("snapd", "snapd")}>
          {busyWith("snapd") ? $_("common.installing") : $_("settings.installSnapd")}
        </button>
      {/if}
    </div>
    <div class="settings-row">
      <div>
        <div class="settings-label">
          {$_("settings.firmware")}
          {#if status}
            <span class={status.fwupd ? tagOk : tagOff}>{status.fwupd ? $_("settings.installedTag") : $_("settings.notInstalled")}</span>
          {/if}
        </div>
        <div class="settings-desc">{status && !status.fwupd ? $_("settings.firmwareMissing") : $_("settings.firmwareDesc")}</div>
      </div>
      <div class="flex items-center gap-3">
        {#if status && !status.fwupd}
          <button class={primaryClass} disabled={busyWith("fwupd")} on:click={() => installPackage("fwupd", "fwupd")}>
            {busyWith("fwupd") ? $_("common.installing") : $_("settings.installFwupd")}
          </button>
        {/if}
        <Toggle bind:checked={$settings.firmwareUpdates} label={$_("settings.firmwareUpdates")} disabled={!status?.fwupd} />
      </div>
    </div>
    <div class="settings-row">
      <div>
        <div class="settings-label">{$_("settings.firmwareTesting")}</div>
        <div class="settings-desc">{$_("settings.firmwareTestingDesc")}</div>
      </div>
      <Toggle
        bind:checked={testing}
        label={$_("settings.firmwareTesting")}
        disabled={!status?.fwupd || !$settings.firmwareUpdates || busyWith("firmware")}
        on:change={toggleTesting}
      />
    </div>
    <div class="settings-row">
      <div>
        <div class="settings-label">
          {$_("settings.appimage")}
          {#if status}
            <span class={status.appimageManager ? tagOk : tagOff}>
              {status.appimageManager ? $_("settings.installedTag") : $_("settings.notInstalled")}
            </span>
          {/if}
        </div>
        <div class="settings-desc">{$_("settings.appimageDesc")}</div>
      </div>
      {#if status && !status.appimageManager}
        <button class={primaryClass} disabled={busyWith("soplos-appimage-manager")} on:click={() => installPackage("soplos-appimage-manager", "Soplos AppImage Manager")}>
          {busyWith("soplos-appimage-manager") ? $_("common.installing") : `${$_("common.install")} Soplos AppImage Manager`}
        </button>
      {:else}
        <button class={ghostClass} on:click={() => launch("soplos-appimage-manager", "Soplos AppImage Manager")}>{$_("common.open")}</button>
      {/if}
    </div>
    <div class="settings-row">
      <div>
        <div class="settings-label">{$_("settings.allPackages")}</div>
        <div class="settings-desc">{$_("settings.allPackagesDesc")}</div>
      </div>
      <Toggle bind:checked={$settings.showAllPackages} label={$_("settings.allPackages")} />
    </div>
  </section>

  <section class="bg-surface border border-border rounded px-4 py-1.5">
    <h3 class="text-[0.95rem] font-semibold text-text-bright pt-3 pb-1.5">{$_("settings.checks")}</h3>
    <div class="settings-row">
      <div>
        <div class="settings-label">{$_("settings.auto")}</div>
        <div class="settings-desc">{$_("settings.autoDesc")}</div>
      </div>
      <Toggle bind:checked={$settings.autoCheck} label={$_("settings.auto")} />
    </div>
    <div class="settings-row">
      <div class="settings-label">{$_("settings.frequency")}</div>
      <select class={selectClass} bind:value={$settings.frequency} disabled={!$settings.autoCheck}>
        <option value="1h">{$_("settings.every1")}</option>
        <option value="6h">{$_("settings.every6")}</option>
        <option value="12h">{$_("settings.every12")}</option>
        <option value="daily">{$_("settings.daily")}</option>
        <option value="weekly">{$_("settings.weekly")}</option>
      </select>
    </div>
    <div class="settings-row">
      <div>
        <div class="settings-label">{$_("settings.notify")}</div>
        <div class="settings-desc">{$_("settings.notifyDesc")}</div>
      </div>
      <Toggle bind:checked={$settings.notify} label={$_("settings.notify")} />
    </div>
    <div class="settings-row">
      <div class="settings-label">{$_("settings.flatpakAuto")}</div>
      <Toggle bind:checked={$settings.flatpakAutoUpdate} label={$_("settings.flatpakAuto")} />
    </div>
  </section>

  <section class="bg-surface border border-border rounded px-4 py-1.5">
    <h3 class="text-[0.95rem] font-semibold text-text-bright pt-3 pb-1.5">{$_("settings.recommendations")}</h3>
    <div class="settings-row">
      <div>
        <div class="settings-label">{$_("settings.featured")}</div>
        <div class="settings-desc">{$_("settings.featuredDesc")}</div>
      </div>
      <Toggle bind:checked={$settings.featured} label={$_("settings.featured")} />
    </div>
    <div class="settings-row">
      <div class="settings-label">{$_("settings.ratings")}</div>
      <Toggle bind:checked={$settings.ratings} label={$_("settings.ratings")} />
    </div>
    <div class="settings-row">
      <div class="settings-label">{$_("settings.proprietary")}</div>
      <Toggle bind:checked={$settings.proprietary} label={$_("settings.proprietary")} />
    </div>
  </section>

  <section class="bg-surface border border-border rounded px-4 py-1.5">
    <h3 class="text-[0.95rem] font-semibold text-text-bright pt-3 pb-1.5">{$_("settings.maintenance")}</h3>
    <div class="settings-row">
      <div>
        <div class="settings-label">{$_("settings.cleanup")}</div>
        <div class="settings-desc">{$_("settings.cleanupDesc")}</div>
      </div>
      <div class="flex gap-2 flex-wrap justify-end">
        <button class={ghostClass} disabled={busyWith("autoremove")} on:click={() => maintenance({ action: "autoremove", source: "apt" })}>
          {$_("settings.orphans")}
        </button>
        <button class={ghostClass} disabled={busyWith("clean")} on:click={() => maintenance({ action: "clean", source: "apt" })}>
          {$_("settings.cache")}
        </button>
        {#if status?.flatpak}
          <button class={ghostClass} disabled={busyWith("unused")} on:click={() => maintenance({ action: "unused", source: "flatpak" })}>
            {$_("settings.runtimes")}
          </button>
        {/if}
      </div>
    </div>
  </section>
</div>
