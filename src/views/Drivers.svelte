<script lang="ts">
  import { AudioWaveform, BatteryCharging, Cpu, Fan, Fingerprint, Gamepad2, Laptop, Network, PenTool, Printer, ScanLine, Server, Tag, TabletSmartphone, Video, Wifi } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import CategoryCard from "../components/CategoryCard.svelte";
  import DriversGraphics from "./DriversGraphics.svelte";
  import DriversPrinters from "./DriversPrinters.svelte";
  import DriversPage from "./DriversPage.svelte";
  import { PAGES } from "../lib/driverPages";

  // The Drivers page is a set of sections. It opens on the cards of the
  // categories and each one that is ready opens its own page; the others
  // say they are coming. The brands are names, so they are not translated.
  const CATEGORIES = [
    { id: "graphics", icon: Cpu, color: "118 185 0", brands: "NVIDIA · AMD · Intel", ready: true },
    { id: "printers", icon: Printer, color: "0 150 214", brands: "HP · Brother · Canon · Epson · Xerox · Ricoh", ready: true },
    { id: "scanners", icon: ScanLine, color: "0 168 140", brands: "Epson · Canon · Fujitsu · Brother", ready: true },
    { id: "tablets", icon: PenTool, color: "230 70 120", brands: "Wacom · XP-Pen · Huion · Gaomon · Veikk", ready: true },
    { id: "convertibles", icon: TabletSmartphone, color: "100 120 240", brands: "iio-sensor-proxy · Onboard · Maliit", ready: true },
    { id: "labels", icon: Tag, color: "250 170 0", brands: "Zebra · Dymo · Brother QL · TSC", ready: true },
    { id: "capture", icon: Video, color: "150 90 230", brands: "Elgato · AVerMedia · Blackmagic", ready: true },
    { id: "gaming", icon: Gamepad2, color: "237 60 60", brands: "Logitech · Razer · SteelSeries · Corsair", ready: true },
    { id: "wireless", icon: Wifi, color: "40 130 240", brands: "Intel · Realtek · Broadcom · MediaTek", ready: true },
    { id: "laptops", icon: Laptop, color: "120 140 160", brands: "Lenovo · ASUS · Dell · HP · Framework", ready: true },
    { id: "audio", icon: AudioWaveform, color: "255 125 46", brands: "Focusrite · Behringer · PreSonus", ready: true },
    { id: "network", icon: Network, color: "0 190 200", brands: "Realtek · Intel · Broadcom", ready: true },
    { id: "security", icon: Fingerprint, color: "60 180 110", brands: "Goodix · Synaptics · Yubico", ready: true },
    { id: "ups", icon: BatteryCharging, color: "220 190 20", brands: "APC · Eaton · CyberPower", ready: true },
    { id: "rgb", icon: Fan, color: "200 60 220", brands: "OpenRGB · Corsair · NZXT", ready: true },
    { id: "virtual", icon: Server, color: "229 112 0", brands: "VMware · QEMU · Proxmox", ready: true },
  ];

  let section: string | null = null;
</script>

{#if section === "graphics"}
  <DriversGraphics />
{:else if section === "printers"}
  <DriversPrinters />
{:else if section && PAGES[section]}
  {#key section}
    <DriversPage page={PAGES[section]} />
  {/key}
{:else}
  <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(15rem,1fr))]">
    {#each CATEGORIES as cat (cat.id)}
      <CategoryCard
        icon={cat.icon}
        color={cat.color}
        title={$_(`drivers.cat.${cat.id}`)}
        hint={cat.brands}
        badge={cat.ready ? "" : $_("drivers.soon")}
        disabled={!cat.ready}
        on:click={() => (section = cat.id)}
      />
    {/each}
  </div>
{/if}
