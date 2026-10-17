<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { SourceKind, SourceOption } from "../lib/api";
  import { settings } from "../lib/settings";
  import { jobs, overrides, runJob, chooseScope } from "../lib/jobs";

  // Installs or removes one app. Without `selected` (tiles, carousel) it
  // removes from the source it is installed from, or installs from the
  // preferred source in Settings; the app page passes its chosen source.
  export let appId: string;
  export let name: string;
  export let refs: SourceOption[];
  export let selected: SourceKind | null = null;
  // Bigger button, for the app page.
  export let large = false;

  $: isInstalled = (ref: SourceOption) => $overrides[`${appId}:${ref.kind}`] ?? ref.installed;
  $: target = selected
    ? refs.find((r) => r.kind === selected)
    : (refs.find((r) => isInstalled(r)) ?? refs.find((r) => r.kind === $settings.preferredSource) ?? refs[0]);
  $: installed = target ? isInstalled(target) : false;
  $: running = $jobs.some((j) => j.key === appId && j.status === "running");

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
