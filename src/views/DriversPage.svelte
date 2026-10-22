<script lang="ts">
  import { onMount } from "svelte";
  import { Search } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import { api, type PackageSet } from "../lib/api";
  import { jobs, runJob, type JobRequest } from "../lib/jobs";
  import type { DriverPage, PageItem } from "../lib/driverPages";
  import DriverCard from "../components/DriverCard.svelte";
  import DriverHero, { type DriverSlide } from "../components/DriverHero.svelte";
  import StateIcon from "../components/StateIcon.svelte";

  // A page of the Drivers section made from its description (lib/driverPages):
  // the hero goes through every item of the page and the items are laid out
  // in a few sections of cards.
  export let page: DriverPage;

  const items: PageItem[] = page.sections.flatMap((s) => s.items);
  const withPackages = items.filter((i) => i.packages.length > 0);

  let sets: PackageSet[] = [];
  let loaded = false;
  let current = "";

  async function load() {
    sets = await api.packageSets(withPackages.map((i) => ({ id: i.id, packages: i.packages }))).catch(() => []);
    loaded = true;
  }

  onMount(load);

  $: working = $jobs.some((j) => j.status === "running" && (j.key?.startsWith("driver:") ?? false));
  $: setOf = (id: string) => sets.find((s) => s.id === id);
  const isBusy = (key: string, list: typeof $jobs) => list.some((j) => j.status === "running" && j.key === key);

  // Nothing is built before the status is known, so the hero starts on the
  // first item.
  $: slides = loaded ? items.map((item) => slideOf(item, sets)) : [];
  $: if (slides.length && !slides.some((s) => s.key === current)) current = slides[0].key;

  function slideOf(item: PageItem, packageSets: PackageSet[]): DriverSlide {
    const hasPackages = item.packages.length > 0;
    return {
      key: item.id,
      color: page.color,
      logo: item.logo,
      icon: item.icon,
      kicker: $_(`drivers.cat.${page.id}`),
      title: item.title,
      kind: "",
      status: hasPackages ? (packageSets.find((s) => s.id === item.id)?.installed ? "installed" : "missing") : "",
      note: hasPackages ? $_("drivers.printers.noteDebian") : $_("drivers.printers.noteOfficial"),
      yours: false,
    };
  }

  const itemOf = (id: string) => items.find((i) => i.id === id);
  const isMissing = (item: PageItem | undefined) => !!item && item.packages.length > 0 && !!setOf(item.id) && !setOf(item.id)?.installed;

  async function install(item: PageItem) {
    const set = setOf(item.id);
    if (!set) return;
    const request: JobRequest = item.vendor ? { action: "install", source: "driver", ids: [item.vendor] } : { action: "install", source: "apt", ids: set.packages };
    await runJob($_("drivers.installing", { values: { name: item.title } }), request, `driver:${item.id}`);
    await load();
  }

  async function remove(item: PageItem) {
    const set = setOf(item.id);
    if (!set) return;
    await runJob($_("drivers.removing", { values: { name: item.title } }), { action: "remove", source: "apt", ids: set.packages }, `driver:${item.id}`);
    await load();
  }

  const open = (url: string) => void api.openUrl(url).catch(() => {});
</script>

{#if !loaded}
  <div class="h-full min-h-[20rem] flex flex-col items-center justify-center text-center px-4 gap-6">
    <StateIcon tone="primary" pulse>
      <Search size="56%" strokeWidth={1.25} />
    </StateIcon>
    <h3 class="text-[1.4rem] font-semibold text-text-bright">{$_("drivers.detecting")}</h3>
  </div>
{:else}
  <div class="flex flex-col gap-8">
    <DriverHero {slides} {current} paused={working} on:change={(e) => (current = e.detail)}>
      <svelte:fragment slot="actions" let:slide={s}>
        {#if isMissing(itemOf(s.key))}
          <button
            class="px-4 py-2 rounded bg-primary hover:bg-primary-hover text-white text-[0.85rem] font-semibold disabled:opacity-60"
            disabled={working}
            on:click={() => {
              const item = itemOf(s.key);
              if (item) void install(item);
            }}
          >
            {$_("drivers.installMissing")}
          </button>
        {:else if itemOf(s.key)?.official}
          <button class="px-4 py-2 rounded bg-primary hover:bg-primary-hover text-white text-[0.85rem] font-semibold" on:click={() => open(itemOf(s.key)?.official ?? "")}>
            {$_("drivers.printers.open")}
          </button>
        {/if}
      </svelte:fragment>
    </DriverHero>

    {#each page.sections as section (section.id)}
      <section class="flex flex-col gap-4">
        <h2 class="flex items-center gap-2.5 text-[1.15rem] font-semibold text-text-bright">
          <span class="w-1 h-6 rounded-full" style="background: rgb({page.color})"></span>
          {$_(section.titleKey)}
        </h2>
        <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(15rem,1fr))]">
          {#each section.items as item (item.id)}
            {#if item.packages.length > 0}
              <DriverCard
                color={page.color}
                logo={item.logo ?? ""}
                icon={item.logo ? null : item.icon}
                big={item.big}
                title={item.title}
                hint={item.packages.join(" · ")}
                installed={!!setOf(item.id)?.installed}
                canRemove={!!item.removable}
                busy={isBusy(`driver:${item.id}`, $jobs)}
                disabled={working}
                on:install={() => install(item)}
                on:remove={() => remove(item)}
              />
            {:else}
              <DriverCard
                color={page.color}
                logo={item.logo ?? ""}
                icon={item.logo ? null : item.icon}
                big={item.big}
                title={$_("drivers.printers.official", { values: { name: item.title } })}
                hint={$_("drivers.printers.officialHint")}
                linkText={item.official ? $_("drivers.printers.open") : ""}
                blocked={item.official ? "" : $_("drivers.soon")}
                noStatus
                on:open={() => open(item.official ?? "")}
              />
            {/if}
          {/each}
        </div>
      </section>
    {/each}
  </div>
{/if}
