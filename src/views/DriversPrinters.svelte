<script lang="ts">
  import { onMount } from "svelte";
  import { FileText, Palette, Printer, Search } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import { api, type PackageSet, type PrinterInfo } from "../lib/api";
  import { jobs, runJob } from "../lib/jobs";
  import DriverCard from "../components/DriverCard.svelte";
  import DriverHero, { type DriverSlide } from "../components/DriverHero.svelte";
  import StateIcon from "../components/StateIcon.svelte";
  // The official logos of the makers.
  import hpLogo from "../assets/drivers/hp.svg";
  import brotherLogo from "../assets/drivers/brother.svg";
  import epsonLogo from "../assets/drivers/epson.svg";
  import canonLogo from "../assets/drivers/canon.svg";
  import xeroxLogo from "../assets/drivers/xerox.svg";
  import ricohLogo from "../assets/drivers/ricoh.svg";
  import kyoceraLogo from "../assets/drivers/kyocera.svg";
  import lexmarkLogo from "../assets/drivers/lexmark.svg";
  import okiLogo from "../assets/drivers/oki.svg";
  import konicaLogo from "../assets/drivers/konicaminolta.svg";
  import samsungLogo from "../assets/drivers/samsung.svg";
  import pantumLogo from "../assets/drivers/pantum.png";

  // The makers of the page, in the order the hero shows them. `set` is the id
  // of its Debian packages when it has some; `url` is where the maker
  // publishes its own drivers, which are per model and so cannot be installed
  // from here. Brand names are names, so they are not translated.
  type Brand = { name: string; logo: string; color: string; url: string; big: string; set?: string; removable?: boolean };
  const BRANDS: Record<string, Brand> = {
    hp: { name: "HP", logo: hpLogo, color: "0 150 214", url: "https://support.hp.com/", big: "HPLIP", set: "print-hp", removable: true },
    brother: { name: "Brother", logo: brotherLogo, color: "0 87 184", url: "https://support.brother.com/", big: "BRL", set: "print-brother", removable: true },
    epson: { name: "Epson", logo: epsonLogo, color: "0 51 153", url: "https://epson.com/Support", big: "ESCPR", set: "print-epson", removable: true },
    canon: { name: "Canon", logo: canonLogo, color: "204 0 0", url: "https://www.canon-europe.com/support/", big: "WEB" },
    xerox: { name: "Xerox", logo: xeroxLogo, color: "215 25 33", url: "https://www.support.xerox.com/", big: "DEB", set: "print-xerox", removable: true },
    ricoh: { name: "Ricoh", logo: ricohLogo, color: "200 16 46", url: "https://support.ricoh.com/", big: "DEB", set: "print-ricoh", removable: true },
    kyocera: { name: "Kyocera", logo: kyoceraLogo, color: "221 0 0", url: "https://www.kyoceradocumentsolutions.com/", big: "WEB" },
    lexmark: { name: "Lexmark", logo: lexmarkLogo, color: "28 156 59", url: "https://support.lexmark.com/", big: "WEB" },
    oki: { name: "OKI", logo: okiLogo, color: "230 0 18", url: "https://www.oki.com/", big: "DEB", set: "print-oki", removable: true },
    konicaminolta: { name: "Konica Minolta", logo: konicaLogo, color: "0 114 206", url: "https://www.konicaminolta.com/", big: "WEB" },
    samsung: { name: "Samsung", logo: samsungLogo, color: "20 40 160", url: "https://support.hp.com/", big: "DEB", set: "print-samsung", removable: true },
    pantum: { name: "Pantum", logo: pantumLogo, color: "0 159 227", url: "https://global.pantum.com/", big: "WEB" },
  };
  const ORDER = Object.keys(BRANDS);
  // The makers that have a section of their own, and those that only link.
  const WITH_PACKAGES = ORDER.filter((id) => BRANDS[id].set);
  const LINK_ONLY = ORDER.filter((id) => !BRANDS[id].set);

  // What every printer needs, whatever its maker.
  const ORANGE = "255 125 46";
  const BASE = [
    { id: "print-cups", key: "cups", icon: Printer, big: "CUPS", removable: false },
    { id: "print-pdf", key: "pdf", icon: FileText, big: "PDF", removable: true },
    { id: "print-gutenprint", key: "gutenprint", icon: Palette, big: "ALL", removable: true },
  ];

  let printers: PrinterInfo[] = [];
  let sets: PackageSet[] = [];
  let loaded = false;
  let current = "";

  async function load() {
    [printers, sets] = await Promise.all([api.printers().catch(() => []), api.printerPackages().catch(() => [])]);
    loaded = true;
  }

  onMount(load);

  $: working = $jobs.some((j) => j.status === "running" && (j.key?.startsWith("driver:") ?? false));
  $: setOf = (id: string) => sets.find((s) => s.id === id);
  const isBusy = (key: string, list: typeof $jobs) => list.some((j) => j.status === "running" && j.key === key);

  // The printers of this machine first, then every maker. Nothing is built
  // before the detection ends, so the hero starts on the first slide.
  $: slides = loaded ? buildSlides(printers, sets) : [];
  $: if (slides.length && !slides.some((s) => s.key === current)) current = slides[0].key;

  function buildSlides(found: PrinterInfo[], packageSets: PackageSet[]): DriverSlide[] {
    const out: DriverSlide[] = [];
    const taken = new Set<string>();
    for (const printer of found) {
      const brand = BRANDS[printer.brand];
      if (!brand || taken.has(printer.brand)) continue;
      taken.add(printer.brand);
      out.push(slideOf(printer.brand, packageSets, printer));
    }
    for (const id of ORDER) {
      if (!taken.has(id)) out.push(slideOf(id, packageSets));
    }
    return out;
  }

  function slideOf(id: string, packageSets: PackageSet[], printer?: PrinterInfo): DriverSlide {
    const brand = BRANDS[id];
    const set = packageSets.find((s) => s.id === brand.set);
    return {
      key: id,
      color: brand.color,
      logo: brand.logo,
      kicker: printer ? `${$_("drivers.printers.yours")} · ${brand.name}` : $_("drivers.vendors.kicker"),
      title: printer ? printer.model || brand.name : brand.name,
      kind: "",
      status: set ? (set.installed ? "installed" : "missing") : "",
      note: brand.set ? $_("drivers.printers.noteDebian") : $_("drivers.printers.noteOfficial"),
      yours: !!printer,
    };
  }

  async function installSet(id: string, title: string) {
    const set = setOf(id);
    if (!set) return;
    await runJob($_("drivers.installing", { values: { name: title } }), { action: "install", source: "apt", ids: set.packages }, `driver:${id}`);
    await load();
  }

  async function removeSet(id: string, title: string) {
    const set = setOf(id);
    if (!set) return;
    await runJob($_("drivers.removing", { values: { name: title } }), { action: "remove", source: "apt", ids: set.packages }, `driver:${id}`);
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
        {#if BRANDS[s.key].set && setOf(BRANDS[s.key].set ?? "") && !setOf(BRANDS[s.key].set ?? "")?.installed}
          <button
            class="px-4 py-2 rounded bg-primary hover:bg-primary-hover text-white text-[0.85rem] font-semibold disabled:opacity-60"
            disabled={working}
            on:click={() => installSet(BRANDS[s.key].set ?? "", $_(`drivers.printers.cards.${s.key}.title`))}
          >
            {$_("drivers.installMissing")}
          </button>
        {:else}
          <button class="px-4 py-2 rounded bg-primary hover:bg-primary-hover text-white text-[0.85rem] font-semibold" on:click={() => open(BRANDS[s.key].url)}>
            {$_("drivers.printers.open")}
          </button>
        {/if}
      </svelte:fragment>
    </DriverHero>

    <section class="flex flex-col gap-4">
      <h2 class="flex items-center gap-2.5 text-[1.15rem] font-semibold text-text-bright">
        <span class="w-1 h-6 rounded-full bg-primary"></span>
        {$_("drivers.printers.sections.base")}
      </h2>
      <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(15rem,1fr))]">
        {#each BASE as card (card.id)}
          <DriverCard
            color={ORANGE}
            icon={card.icon}
            big={card.big}
            title={$_(`drivers.printers.cards.${card.key}.title`)}
            hint={$_(`drivers.printers.cards.${card.key}.hint`)}
            installed={!!setOf(card.id)?.installed}
            canRemove={card.removable}
            busy={isBusy(`driver:${card.id}`, $jobs)}
            disabled={working}
            on:install={() => installSet(card.id, $_(`drivers.printers.cards.${card.key}.title`))}
            on:remove={() => removeSet(card.id, $_(`drivers.printers.cards.${card.key}.title`))}
          />
        {/each}
      </div>
    </section>

    {#each WITH_PACKAGES as id (id)}
      {@const brand = BRANDS[id]}
      <section class="flex flex-col gap-4">
        <h2 class="flex items-center gap-2.5 text-[1.15rem] font-semibold text-text-bright">
          <span class="w-1 h-6 rounded-full" style="background: rgb({brand.color})"></span>
          {brand.name}
        </h2>
        <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(15rem,1fr))]">
          <DriverCard
            color={brand.color}
            logo={brand.logo}
            big={brand.big}
            title={$_(`drivers.printers.cards.${id}.title`)}
            hint={$_(`drivers.printers.cards.${id}.hint`)}
            installed={!!setOf(brand.set ?? "")?.installed}
            canRemove={!!brand.removable}
            busy={isBusy(`driver:${brand.set}`, $jobs)}
            disabled={working}
            on:install={() => installSet(brand.set ?? "", $_(`drivers.printers.cards.${id}.title`))}
            on:remove={() => removeSet(brand.set ?? "", $_(`drivers.printers.cards.${id}.title`))}
          />
          <DriverCard
            color={brand.color}
            logo={brand.logo}
            big="WEB"
            title={$_("drivers.printers.official", { values: { name: brand.name } })}
            hint={$_("drivers.printers.officialHint")}
            linkText={$_("drivers.printers.open")}
            on:open={() => open(brand.url)}
          />
        </div>
      </section>
    {/each}

    <section class="flex flex-col gap-4">
      <h2 class="flex items-center gap-2.5 text-[1.15rem] font-semibold text-text-bright">
        <span class="w-1 h-6 rounded-full bg-text-muted"></span>
        {$_("drivers.printers.sections.others")}
      </h2>
      <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(15rem,1fr))]">
        {#each LINK_ONLY as id (id)}
          {@const brand = BRANDS[id]}
          <DriverCard
            color={brand.color}
            logo={brand.logo}
            big="WEB"
            title={$_("drivers.printers.official", { values: { name: brand.name } })}
            hint={$_("drivers.printers.officialHint")}
            linkText={$_("drivers.printers.open")}
            on:open={() => open(brand.url)}
          />
        {/each}
      </div>
    </section>
  </div>
{/if}
