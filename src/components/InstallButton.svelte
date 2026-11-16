<script lang="ts">
  import { _ } from "svelte-i18n";
  import { api, type SourceKind, type SourceOption } from "../lib/api";
  import { jobs, overrides, runJob, chooseScope } from "../lib/jobs";
  import { showToast } from "../lib/toast";

  // Installs or removes one app. The app page passes the source the user
  // chose; without one it works on the source the app is installed from,
  // or on the first source of the list (APT).
  export let appId: string;
  export let name: string;
  export let refs: SourceOption[];
  export let selected: SourceKind | null = null;
  // Bigger button, for the app page.
  export let large = false;

  $: isInstalled = (ref: SourceOption) => $overrides[`${appId}:${ref.kind}`] ?? ref.installed;
  $: target = selected
    ? refs.find((r) => r.kind === selected)
    : (refs.find((r) => isInstalled(r)) ?? refs[0]);
  $: installed = target ? isInstalled(target) : false;
  $: running = $jobs.some((j) => j.key === appId && j.status === "running");

  // An installed app gets an Open button, when it has something to open: a
  // library or a tool of the terminal has none. It is asked again after an install.
  let canOpen = false;
  $: checkOpen(target, installed, running);
  async function checkOpen(ref: SourceOption | undefined, isIn: boolean, busy: boolean) {
    canOpen = false;
    if (!ref || !isIn || busy) return;
    try {
      canOpen = await api.canOpenApp(appId, ref.kind, ref.id);
    } catch {
      canOpen = false;
    }
  }

  async function open() {
    if (!target) return;
    try {
      await api.openApp(appId, target.kind, target.id);
    } catch {
      showToast($_("common.openFailed", { values: { name } }), 3000);
    }
  }

  async function act() {
    if (!target || running) return;
    const ref = target;
    const action = installed ? "remove" : "install";
    let scope: "user" | "system" | undefined;
    if (ref.kind === "flatpak" && action === "install") {
      const chosen = await chooseScope(name);
      if (!chosen) return;
      scope = chosen;
    }
    const title = $_(action === "install" ? "jobs.installing" : "jobs.removing", { values: { name } });
    const ok = await runJob(title, { action, source: ref.kind, ids: [ref.id], scope }, appId);
    if (ok) overrides.update((o) => ({ ...o, [`${appId}:${ref.kind}`]: action === "install" }));
  }
</script>

{#if target && installed && canOpen && !running}
  <button
    class="rounded font-semibold shrink-0 bg-primary hover:bg-primary-hover text-white {large ? 'px-5 py-2 text-[0.88rem]' : 'px-2.5 py-1.5 text-[0.75rem]'}"
    on:click|stopPropagation={open}
  >
    {$_("common.open")}
  </button>
{/if}
{#if target}
  <button
    class="rounded font-semibold shrink-0 disabled:opacity-60 disabled:cursor-default {large ? 'px-5 py-2 text-[0.88rem]' : 'px-2.5 py-1.5 text-[0.75rem]'} {installed
      ? 'border border-danger text-danger hover:bg-danger/10'
      : 'bg-primary hover:bg-primary-hover text-white'}"
    disabled={running}
    on:click|stopPropagation={act}
  >
    {#if running}
      {installed ? $_("common.removing") : $_("common.installing")}
    {:else}
      {installed ? $_("common.uninstall") : $_("common.install")}
    {/if}
  </button>
{/if}
