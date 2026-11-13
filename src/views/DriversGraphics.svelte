<script lang="ts">
  import { onMount } from "svelte";
  import { Search } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import { api, type GpuInfo, type NvidiaBranch, type PackageSet } from "../lib/api";
  import { jobs, runJob, type JobRequest } from "../lib/jobs";
  import DriverCard from "../components/DriverCard.svelte";
  import DriverHero, { type DriverSlide } from "../components/DriverHero.svelte";
  import StateIcon from "../components/StateIcon.svelte";
  // The official logos of the brands, in their own colors.
  import nvidiaLogo from "../assets/drivers/nvidia.svg";
  import amdLogo from "../assets/drivers/amd.svg";
  import intelLogo from "../assets/drivers/intel.svg";
  import vmwareLogo from "../assets/drivers/vmware.svg";
  import virtualboxLogo from "../assets/drivers/virtualbox.svg";
  import qemuLogo from "../assets/drivers/qemu.svg";
  import virtmanagerLogo from "../assets/drivers/virt-manager.png";
  import boxesLogo from "../assets/drivers/gnome-boxes.svg";
  import proxmoxLogo from "../assets/drivers/proxmox.svg";

  // The NVIDIA branches of the Store, newest first. The 620 only works on
  // Turing cards or newer: on an older one the module would not
  // build and, since the NVIDIA package blacklists nouveau, the machine
  // would be left without a graphics driver.
  const BRANCHES = [
    { id: "620", big: "620", needsTuring: true },
    { id: "580", big: "580", needsTuring: false },
    { id: "nvidia-driver", big: "550", needsTuring: false },
    { id: "nvidia-tesla-470-driver", big: "470", needsTuring: false },
    { id: "nvidia-legacy-390xx-driver", big: "390", needsTuring: false },
    { id: "nvidia-legacy-340xx-driver", big: "340", needsTuring: false },
  ];

  // Every brand of the page, in the order the hero shows them. `big` is the
  // outlined text of its card; `set` is the id of its package set, if it has
  // one; `removable` brands are programs, not drivers, so their card can
  // remove them.
  const BRANDS: Record<string, { logo: string; color: string; big: string; set?: string; removable?: boolean; vendor?: boolean; whole?: boolean }> = {
    nvidia: { logo: nvidiaLogo, color: "118 185 0", big: "" },
    amd: { logo: amdLogo, color: "237 28 36", big: "GPU", set: "amd" },
    intel: { logo: intelLogo, color: "0 113 197", big: "GPU", set: "intel" },
    vmware: { logo: vmwareLogo, color: "134 160 172", big: "VM", set: "vmware" },
    virtualbox: { logo: virtualboxLogo, color: "47 97 180", big: "GA" },
    qemu: { logo: qemuLogo, color: "255 102 0", big: "VM", set: "qemu" },
    virtmanager: { logo: virtmanagerLogo, color: "170 30 70", big: "VM", set: "virtmanager", removable: true },
    boxes: { logo: boxesLogo, color: "74 134 207", big: "VM", set: "boxes", removable: true },
    proxmox: { logo: proxmoxLogo, color: "229 112 0", big: "PVE", set: "proxmox" },
  };
  const ORDER = ["nvidia", "amd", "intel", "vmware", "virtualbox", "qemu", "virtmanager", "boxes", "proxmox"];

  // The cards of the page: the brands and, under AMD, Intel and NVIDIA, what
  // goes with them. `vendor` ones come from the repository of their maker
  // through the root helper; the others are Debian packages. Removing a
  // card removes every package of its set, except for the programs above.
  const extra = (brand: string, set: string, big: string, vendor = false) => ({
    logo: BRANDS[brand].logo,
    color: BRANDS[brand].color,
    big,
    set,
    removable: true,
    vendor,
    whole: true,
  });
  const CARDS: typeof BRANDS = {
    ...BRANDS,
    "amd-rocm": extra("amd", "amd-rocm", "ROCm"),
    "amd-hip": extra("amd", "amd-hip", "HIP"),
    "amd-math": extra("amd", "amd-math", "MATH"),
    "amd-tools": extra("amd", "amd-tools", "TOOLS"),
    "intel-compute": extra("intel", "intel-compute", "OCL"),
    "intel-media": extra("intel", "intel-media", "VPL"),
    "intel-tools": extra("intel", "intel-tools", "TOOLS"),
    "oneapi-dpcpp": extra("intel", "oneapi-dpcpp", "DPC++", true),
    "oneapi-mkl": extra("intel", "oneapi-mkl", "MKL", true),
    "oneapi-vtune": extra("intel", "oneapi-vtune", "VTUNE", true),
    "oneapi-base": extra("intel", "oneapi-base", "ONEAPI", true),
    "cuda-toolkit": extra("nvidia", "cuda-toolkit", "CUDA", true),
  };
  // After NVIDIA, which has its own branches, every brand gets a section of
  // its own: none is made to look like the main one.
  const GROUPS = [
    { title: "drivers.vendors.amd.name", color: BRANDS.amd.color, ids: ["amd", "amd-rocm", "amd-hip", "amd-math", "amd-tools"] },
    { title: "drivers.vendors.intel.name", color: BRANDS.intel.color, ids: ["intel", "intel-compute", "intel-media", "intel-tools", "oneapi-dpcpp", "oneapi-mkl", "oneapi-vtune", "oneapi-base"] },
    { title: "drivers.cat.virtual", color: BRANDS.proxmox.color, ids: ["vmware", "virtualbox", "qemu", "proxmox", "virtmanager", "boxes"] },
  ];

  // The brand of a graphics card. A virtual adapter belongs to its
  // hypervisor, read from the model the detection gives it.
  function brandIdOf(gpu: GpuInfo): string {
    if (gpu.vendor === "NVIDIA") return "nvidia";
    if (gpu.vendor === "AMD") return "amd";
    if (gpu.vendor === "Intel") return "intel";
    const model = gpu.model.toLowerCase();
    if (model.includes("vmware")) return "vmware";
    if (model.includes("virtualbox")) return "virtualbox";
    if (model.includes("virtio") || model.includes("qxl") || model.includes("qemu")) return "qemu";
    return "";
  }

  const KINDS: Record<string, string> = {
    Dedicated: "dedicated",
    Integrated: "integrated",
    "Dedicated/Integrated": "both",
    Virtual: "virtual",
  };

  let gpus: GpuInfo[] = [];
  let branches: NvidiaBranch[] = [];
  let sets: PackageSet[] = [];
  let loaded = false;
  let current = "";
  // What is waiting for a confirmation: a branch id, "remove" or "hero".
  let asking: string | null = null;

  async function load() {
    [gpus, branches, sets] = await Promise.all([
      api.gpus().catch(() => []),
      api.nvidiaBranches().catch(() => []),
      api.driverPackages().catch(() => []),
    ]);
    loaded = true;
  }

  onMount(load);

  // "nvidia-driver-620" is the branch "620"; the others are named as they are.
  const branchOf = (driver: string | null) => (driver ? (driver.match(/^nvidia-driver-(\d+)$/)?.[1] ?? driver) : null);

  $: nvidiaGpu = gpus.find((g) => g.vendor === "NVIDIA") ?? null;
  $: installedIds = new Set(branches.filter((b) => b.installed).map((b) => b.id));
  $: anyNvidiaInstalled = installedIds.size > 0;
  $: recommendedId = nvidiaGpu ? branchOf(nvidiaGpu.recommendedDriver) : null;
  $: turing = nvidiaGpu?.recommendedDriver === "nvidia-driver-620";
  $: working = $jobs.some((j) => j.status === "running" && (j.key?.startsWith("driver:") ?? false));
  $: setOf = (id: string) => sets.find((s) => s.id === id);
  const isBusy = (key: string, list: typeof $jobs) => list.some((j) => j.status === "running" && j.key === key);

  function heroNote(gpu: GpuInfo): string {
    if (gpu.vendor === "Virtual") return $_("drivers.hero.virtual");
    if (gpu.vendor === "NVIDIA") {
      return gpu.recommendedDriver === "nouveau"
        ? $_("drivers.hero.nouveau")
        : $_("drivers.hero.recommended", { values: { name: $_(`drivers.branch.${branchOf(gpu.recommendedDriver)}.label`) } });
    }
    return gpu.driverStatus === "installed" ? $_("drivers.hero.packagesReady") : $_("drivers.hero.packagesMissing");
  }

  // Your own cards first, then every other brand.
  // Nothing is built before the detection ends: slides made without the cards
  // of this machine would leave the hero on a brand that is not the first.
  $: slides = loaded ? buildSlides(gpus, sets, anyNvidiaInstalled) : [];
  $: if (slides.length && !slides.some((s) => s.key === current)) current = slides[0].key;
  $: slide = slides.find((s) => s.key === current);

  function buildSlides(list: GpuInfo[], packageSets: PackageSet[], nvidiaInstalled: boolean): DriverSlide[] {
    const out: DriverSlide[] = [];
    const taken = new Set<string>();
    for (const gpu of list) {
      const id = brandIdOf(gpu);
      if (!id || taken.has(id)) continue;
      taken.add(id);
      const virtual = gpu.vendor === "Virtual";
      out.push({
        key: id,
        color: BRANDS[id].color,
        logo: BRANDS[id].logo,
        kicker: `${$_("drivers.yourGpu")} · ${virtual ? $_("drivers.type.virtual") : gpu.vendor}`,
        title: gpu.model,
        kind: virtual ? "" : $_(`drivers.type.${KINDS[gpu.gpuType] ?? "virtual"}`),
        status: virtual ? "" : gpu.driverStatus,
        note: heroNote(gpu),
        yours: true,
      });
    }
    for (const id of ORDER) {
      if (taken.has(id)) continue;
      const set = packageSets.find((s) => s.id === BRANDS[id].set);
      out.push({
        key: id,
        color: BRANDS[id].color,
        logo: BRANDS[id].logo,
        kicker: $_("drivers.vendors.kicker"),
        title: $_(`drivers.vendors.${id}.name`),
        kind: "",
        status: id === "nvidia" ? (nvidiaInstalled ? "installed" : "") : set ? (set.installed ? "installed" : "missing") : "",
        note: $_(`drivers.vendors.${id}.note`),
        yours: false,
      });
    }
    return out;
  }

  async function install(id: string) {
    asking = null;
    const name = $_(`drivers.branch.${id}.label`);
    await runJob($_("drivers.installing", { values: { name } }), { action: "install", source: "driver", ids: [id] }, `driver:${id}`);
    await load();
  }

  async function removeNvidia() {
    asking = null;
    await runJob($_("drivers.removingNvidia"), { action: "remove", source: "driver" }, "driver:remove");
    await load();
  }

  // Debian packages go through apt; the stacks of the makers through the helper.
  async function installSet(id: string) {
    asking = null;
    const card = CARDS[id];
    const set = setOf(card.set ?? "");
    if (!set) return;
    const name = $_(`drivers.pack.${id}.title`);
    const request: JobRequest = card.vendor ? { action: "install", source: "driver", ids: [id] } : { action: "install", source: "apt", ids: set.packages };
    await runJob($_("drivers.installing", { values: { name } }), request, `driver:${id}`);
    await load();
  }

  async function removeSet(id: string) {
    const card = CARDS[id];
    const set = setOf(card.set ?? "");
    if (!set) return;
    const ids = card.whole ? set.packages : [set.packages[0]];
    await runJob($_("drivers.removing", { values: { name: $_(`drivers.pack.${id}.title`) } }), { action: "remove", source: "apt", ids }, `driver:${id}`);
    await load();
  }

  function blockedReason(branch: { needsTuring: boolean }, hasCard: boolean, isTuring: boolean): string {
    if (!hasCard) return $_("drivers.nvidia.noCard");
    if (branch.needsTuring && !isTuring) return $_("drivers.nvidia.needsTuring");
    return "";
  }
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
    <DriverHero {slides} {current} paused={working || asking !== null} on:change={(e) => ((current = e.detail), (asking = null))}>
      <svelte:fragment slot="actions" let:slide={s}>
        {#if s.key === "nvidia" && s.yours && nvidiaGpu && nvidiaGpu.driverStatus !== "installed" && recommendedId}
          {#if asking === "hero"}
            <p class="text-[0.8rem] text-warning max-w-[30rem]">{recommendedId === "nouveau" ? $_("drivers.nvidia.confirmRemove") : $_("drivers.nvidia.confirmInstall")}</p>
            <div class="flex gap-2">
              <button class="px-3.5 py-1.5 rounded border border-white/25 text-white hover:bg-white/10 text-[0.8rem]" on:click={() => (asking = null)}>{$_("drivers.cancel")}</button>
              <button
                class="px-3.5 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.8rem] font-semibold"
                on:click={() => (recommendedId === "nouveau" ? removeNvidia() : install(recommendedId))}
              >
                {$_("drivers.confirm")}
              </button>
            </div>
          {:else if recommendedId === "nouveau"}
            {#if anyNvidiaInstalled}
              <button class="px-4 py-2 rounded bg-primary hover:bg-primary-hover text-white text-[0.85rem] font-semibold disabled:opacity-60" disabled={working} on:click={() => (asking = "hero")}>
                {$_("drivers.nvidia.removeAll")}
              </button>
            {/if}
          {:else}
            <button class="px-4 py-2 rounded bg-primary hover:bg-primary-hover text-white text-[0.85rem] font-semibold disabled:opacity-60" disabled={working} on:click={() => (asking = "hero")}>
              {$_("drivers.install")}
            </button>
          {/if}
        {:else if BRANDS[s.key].set && !BRANDS[s.key].removable && setOf(BRANDS[s.key].set ?? "") && !setOf(BRANDS[s.key].set ?? "")?.installed}
          <button class="px-4 py-2 rounded bg-primary hover:bg-primary-hover text-white text-[0.85rem] font-semibold disabled:opacity-60" disabled={working} on:click={() => installSet(s.key)}>
            {$_("drivers.installMissing")}
          </button>
        {/if}
      </svelte:fragment>
    </DriverHero>

    <!-- The cards do not follow the hero: they are always all there. -->
    <section class="flex flex-col gap-4">
        <div class="flex items-end justify-between gap-3 flex-wrap">
          <div>
            <h2 class="flex items-center gap-2.5 text-[1.15rem] font-semibold text-text-bright">
              <span class="w-1 h-6 rounded-full bg-[#76b900]"></span>
              {$_("drivers.nvidia.title")}
            </h2>
            <p class="text-[0.78rem] text-text-muted mt-0.5 max-w-2xl">{$_("drivers.nvidia.subtitle")}</p>
          </div>
          {#if anyNvidiaInstalled}
            {#if asking === "remove"}
              <div class="flex items-center gap-2 flex-wrap">
                <span class="text-[0.75rem] text-warning max-w-[20rem]">{$_("drivers.nvidia.confirmRemove")}</span>
                <button class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.78rem]" on:click={() => (asking = null)}>{$_("drivers.cancel")}</button>
                <button class="px-3 py-1.5 rounded bg-danger text-white text-[0.78rem] font-semibold" on:click={removeNvidia}>{$_("drivers.confirm")}</button>
              </div>
            {:else}
              <button
                class="px-3.5 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.8rem] font-semibold disabled:opacity-60"
                disabled={working}
                on:click={() => (asking = "remove")}
              >
                {$_("drivers.nvidia.removeAll")}
              </button>
            {/if}
          {/if}
        </div>

        <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(15rem,1fr))]">
          {#each BRANCHES as branch (branch.id)}
            <DriverCard
              color={BRANDS.nvidia.color}
              logo={nvidiaLogo}
              big={branch.big}
              title={$_(`drivers.branch.${branch.id}.label`)}
              hint={$_(`drivers.branch.${branch.id}.hint`)}
              installed={installedIds.has(branch.id)}
              recommended={recommendedId === branch.id}
              blocked={blockedReason(branch, !!nvidiaGpu, turing)}
              confirmText={asking === branch.id ? $_("drivers.nvidia.confirmInstall") : ""}
              canRemove={false}
              busy={isBusy(`driver:${branch.id}`, $jobs)}
              disabled={working}
              on:install={() => (asking = branch.id)}
              on:cancel={() => (asking = null)}
              on:confirm={() => install(branch.id)}
            />
          {/each}
          <DriverCard
            color={CARDS["cuda-toolkit"].color}
            logo={CARDS["cuda-toolkit"].logo}
            big={CARDS["cuda-toolkit"].big}
            title={$_("drivers.pack.cuda-toolkit.title")}
            hint={$_("drivers.pack.cuda-toolkit.hint")}
            installed={!!setOf("cuda-toolkit")?.installed}
            confirmText={asking === "cuda-toolkit" ? $_("drivers.pack.vendorConfirm") : ""}
            canRemove={true}
            busy={isBusy("driver:cuda-toolkit", $jobs)}
            disabled={working}
            on:install={() => (asking = "cuda-toolkit")}
            on:cancel={() => (asking = null)}
            on:confirm={() => installSet("cuda-toolkit")}
            on:remove={() => removeSet("cuda-toolkit")}
          />
        </div>
    </section>

    {#each GROUPS as group (group.title)}
      <section class="flex flex-col gap-4">
        <h2 class="flex items-center gap-2.5 text-[1.15rem] font-semibold text-text-bright">
          <span class="w-1 h-6 rounded-full" style="background: rgb({group.color})"></span>
          {$_(group.title)}
        </h2>
        <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(15rem,1fr))]">
          {#each group.ids as id (id)}
            <DriverCard
              color={CARDS[id].color}
              logo={CARDS[id].logo}
              big={CARDS[id].big}
              title={$_(`drivers.pack.${id}.title`)}
              hint={$_(`drivers.pack.${id}.hint`)}
              installed={!!setOf(CARDS[id].set ?? "")?.installed}
              blocked={CARDS[id].set ? "" : $_(`drivers.pack.${id}.blocked`)}
              confirmText={asking === id ? $_("drivers.pack.vendorConfirm") : ""}
              canRemove={!!CARDS[id].removable}
              busy={isBusy(`driver:${id}`, $jobs)}
              disabled={working}
              on:install={() => (CARDS[id].vendor ? (asking = id) : installSet(id))}
              on:cancel={() => (asking = null)}
              on:confirm={() => installSet(id)}
              on:remove={() => removeSet(id)}
            />
          {/each}
        </div>
      </section>
    {/each}
  </div>
{/if}
