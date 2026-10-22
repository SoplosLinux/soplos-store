import type { ComponentType } from "svelte";
import {
  AudioWaveform, BatteryCharging, Bluetooth, Boxes, Cable, Camera, Cpu, Disc, Fan, Fingerprint, Gamepad2, Gauge, Keyboard, KeyRound, Laptop, Layers, Lightbulb,
  Mic, Monitor, Mouse, Network, PenTool, Radio, ScanLine, Server, ShieldCheck, Tablet, Tag, Thermometer, Usb, Video, Wifi, Zap,
} from "lucide-svelte";
import wacomLogo from "../assets/makers/wacom.svg";
import xppenLogo from "../assets/makers/xppen.svg";
import huionLogo from "../assets/makers/huion.png";
import gaomonLogo from "../assets/makers/gaomon.png";
import parbloLogo from "../assets/makers/parblo.png";
import fujitsuLogo from "../assets/makers/fujitsu.svg";
import dymoLogo from "../assets/makers/dymo.svg";
import zebraLogo from "../assets/makers/zebra.svg";
import elgatoLogo from "../assets/makers/elgato.svg";
import blackmagicLogo from "../assets/makers/blackmagic.svg";
import obsLogo from "../assets/makers/obs.svg";
import logitechLogo from "../assets/makers/logitech.svg";
import razerLogo from "../assets/makers/razer.svg";
import corsairLogo from "../assets/makers/corsair.svg";
import intelLogo from "../assets/makers/intel.svg";
import realtekLogo from "../assets/makers/realtek.svg";
import atherosLogo from "../assets/makers/atheros.svg";
import broadcomLogo from "../assets/makers/broadcom.svg";
import mediatekLogo from "../assets/makers/mediatek.svg";
import bluetoothLogo from "../assets/makers/bluetooth.svg";
import lenovoLogo from "../assets/makers/lenovo.svg";
import asusLogo from "../assets/makers/asus.svg";
import dellLogo from "../assets/makers/dell.svg";
import frameworkLogo from "../assets/makers/framework.svg";
import tuxedoLogo from "../assets/makers/tuxedo.svg";
import easyeffectsLogo from "../assets/makers/easyeffects.svg";
import ardourLogo from "../assets/makers/ardour.svg";
import focusriteLogo from "../assets/makers/focusrite.svg";
import behringerLogo from "../assets/makers/behringer.svg";
import presonusLogo from "../assets/makers/presonus.svg";
import yubicoLogo from "../assets/makers/yubico.svg";
import nutLogo from "../assets/makers/nut.svg";
import apcLogo from "../assets/makers/apc.svg";
import eatonLogo from "../assets/makers/eaton.svg";
import hpLogo from "../assets/drivers/hp.svg";
import brotherLogo from "../assets/drivers/brother.svg";
import epsonLogo from "../assets/drivers/epson.svg";
import canonLogo from "../assets/drivers/canon.svg";
import vmwareLogo from "../assets/drivers/vmware.svg";
import qemuLogo from "../assets/drivers/qemu.svg";
import virtmanagerLogo from "../assets/drivers/virt-manager.png";
import boxesLogo from "../assets/drivers/gnome-boxes.svg";

// The pages of the Drivers section that follow one pattern: a hero that goes
// through every item of the page and, below it, a few sections of cards. An
// item with packages installs them from Debian (or, with `vendor`, through a
// stack of vendor.rs); one without stands for what a maker publishes on its
// own page, which is filled in as it is found (`official: null` while it is
// not). Names of makers and tools are names, so they are not translated.

export interface PageItem {
  /** Also the id of its package set. */
  id: string;
  title: string;
  packages: string[];
  /** Id of a stack of vendor.rs that installs it instead of apt. */
  vendor?: string;
  /** Short outlined text of the card. */
  big: string;
  /** Remove the packages from the card; false for what the system relies on. */
  removable?: boolean;
  /** The logo of the maker or the tool, when there is one. */
  logo?: string;
  /** Shown instead of a logo. */
  icon: ComponentType;
  /** Where the maker publishes its drivers; null while it is not known. */
  official?: string | null;
}

export interface PageSection {
  id: string;
  /** Key of the title in the language files. */
  titleKey: string;
  items: PageItem[];
}

export interface DriverPage {
  id: string;
  /** Color of the page, as "r g b". */
  color: string;
  sections: PageSection[];
}

/** An item that installs packages. */
const tool = (id: string, title: string, packages: string[], big: string, icon: ComponentType, logo?: string, extra: Partial<PageItem> = {}): PageItem => ({
  id,
  title,
  packages,
  big,
  icon,
  logo,
  removable: true,
  ...extra,
});

/** A maker that only publishes its drivers on its own page. */
const maker = (title: string, icon: ComponentType, logo?: string): PageItem => ({
  id: `maker-${title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`,
  title,
  packages: [],
  big: "WEB",
  icon,
  logo,
  official: null,
});

const TOOLS = "drivers.printers.sections.tools";
const MAKERS = "drivers.printers.sections.makers";

export const PAGES: Record<string, DriverPage> = {
  scanners: {
    id: "scanners",
    color: "0 168 140",
    sections: [
      { id: "tools", titleKey: TOOLS, items: [tool("scan-sane", "SANE", ["sane-utils", "sane-airscan", "simple-scan", "xsane"], "SANE", ScanLine, undefined, { removable: false })] },
      { id: "makers", titleKey: MAKERS, items: [maker("Epson", ScanLine, epsonLogo), maker("Canon", ScanLine, canonLogo), maker("Fujitsu", ScanLine, fujitsuLogo), maker("Brother", ScanLine, brotherLogo)] },
    ],
  },
  tablets: {
    id: "tablets",
    color: "230 70 120",
    sections: [
      {
        id: "tools",
        titleKey: TOOLS,
        items: [
          tool("tab-wacom", "Wacom", ["xserver-xorg-input-wacom", "libwacom-bin"], "WACOM", PenTool, wacomLogo),
          tool("tab-digimend", "DIGImend", ["digimend-dkms"], "DKMS", Tablet),
        ],
      },
      {
        id: "makers",
        titleKey: MAKERS,
        items: [
          maker("Wacom", PenTool, wacomLogo),
          maker("XP-Pen", PenTool, xppenLogo),
          maker("Huion", PenTool, huionLogo),
          maker("Gaomon", PenTool, gaomonLogo),
          maker("Veikk", PenTool),
          maker("Ugee", PenTool),
          maker("Parblo", PenTool, parbloLogo),
        ],
      },
    ],
  },
  labels: {
    id: "labels",
    color: "250 170 0",
    sections: [
      {
        id: "tools",
        titleKey: TOOLS,
        items: [
          tool("lab-brother-ql", "Brother QL", ["brother-ql"], "QL", Tag, brotherLogo),
          tool("lab-ptouch", "P-touch", ["printer-driver-ptouch"], "PT", Tag, brotherLogo),
          tool("lab-dymo", "Dymo", ["printer-driver-dymo"], "DYMO", Tag, dymoLogo),
        ],
      },
      { id: "makers", titleKey: MAKERS, items: [maker("Zebra", Tag, zebraLogo), maker("TSC", Tag)] },
    ],
  },
  capture: {
    id: "capture",
    color: "150 90 230",
    sections: [
      {
        id: "tools",
        titleKey: TOOLS,
        items: [
          tool("cap-v4l", "Video4Linux", ["v4l-utils", "guvcview"], "V4L", Video, undefined, { removable: false }),
          tool("cap-obs", "OBS Studio", ["obs-studio"], "OBS", Camera, obsLogo),
          tool("cap-loopback", "v4l2loopback", ["v4l2loopback-dkms", "v4l2loopback-utils"], "DKMS", Layers),
        ],
      },
      { id: "makers", titleKey: MAKERS, items: [maker("Elgato", Video, elgatoLogo), maker("AVerMedia", Video), maker("Blackmagic", Video, blackmagicLogo)] },
    ],
  },
  gaming: {
    id: "gaming",
    color: "237 60 60",
    sections: [
      {
        id: "tools",
        titleKey: TOOLS,
        items: [
          tool("gam-libratbag", "libratbag + Piper", ["ratbagd", "piper"], "MICE", Mouse),
          tool("gam-solaar", "Solaar (Logitech)", ["solaar"], "SOLAAR", Mouse, logitechLogo),
          tool("gam-razer", "OpenRazer (Razer)", ["openrazer-meta"], "RAZER", Keyboard, razerLogo),
          tool("gam-corsair", "ckb-next (Corsair)", ["ckb-next"], "CKB", Keyboard, corsairLogo),
          tool("gam-controllers", "Steam devices + input-remapper", ["steam-devices", "input-remapper"], "PAD", Gamepad2),
        ],
      },
    ],
  },
  wireless: {
    id: "wireless",
    color: "40 130 240",
    sections: [
      {
        id: "wireless",
        titleKey: "drivers.cat.wireless",
        items: [
          tool("wl-intel", "Intel Wi-Fi", ["firmware-iwlwifi"], "INTEL", Wifi, intelLogo),
          tool("wl-realtek", "Realtek", ["firmware-realtek"], "RTL", Wifi, realtekLogo),
          tool("wl-atheros", "Atheros", ["firmware-atheros"], "ATH", Wifi, atherosLogo),
          tool("wl-broadcom", "Broadcom", ["firmware-brcm80211"], "BRCM", Wifi, broadcomLogo),
          tool("wl-mediatek", "MediaTek", ["firmware-mediatek"], "MTK", Wifi, mediatekLogo),
          tool("wl-bluez", "BlueZ + Blueman", ["bluez", "blueman"], "BT", Bluetooth, bluetoothLogo, { removable: false }),
        ],
      },
    ],
  },
  laptops: {
    id: "laptops",
    color: "120 140 160",
    sections: [
      {
        id: "tools",
        titleKey: TOOLS,
        items: [
          tool("lap-tlp", "TLP", ["tlp", "tlp-rdw"], "TLP", BatteryCharging),
          tool("lap-thermald", "thermald", ["thermald"], "TEMP", Thermometer),
          tool("lap-fwupd", "fwupd (LVFS)", ["fwupd"], "LVFS", Cpu, undefined, { removable: false }),
        ],
      },
      {
        id: "makers",
        titleKey: MAKERS,
        items: [maker("Lenovo", Laptop, lenovoLogo), maker("ASUS", Laptop, asusLogo), maker("Dell", Laptop, dellLogo), maker("HP", Laptop, hpLogo), maker("Framework", Laptop, frameworkLogo), maker("TUXEDO", Laptop, tuxedoLogo)],
      },
    ],
  },
  audio: {
    id: "audio",
    color: "255 125 46",
    sections: [
      {
        id: "tools",
        titleKey: TOOLS,
        items: [
          tool("aud-pipewire", "PipeWire JACK", ["pipewire-jack", "qjackctl", "qpwgraph"], "JACK", AudioWaveform),
          tool("aud-mixer", "pavucontrol", ["pavucontrol"], "MIX", Mic),
          tool("aud-easyeffects", "EasyEffects", ["easyeffects"], "FX", Zap, easyeffectsLogo),
          tool("aud-ardour", "Ardour", ["ardour"], "DAW", Disc, ardourLogo),
        ],
      },
      { id: "makers", titleKey: MAKERS, items: [maker("Focusrite", AudioWaveform, focusriteLogo), maker("Behringer", AudioWaveform, behringerLogo), maker("PreSonus", AudioWaveform, presonusLogo)] },
    ],
  },
  network: {
    id: "network",
    color: "0 190 200",
    sections: [
      {
        id: "network",
        titleKey: "drivers.cat.network",
        items: [
          tool("net-r8168", "Realtek r8168", ["r8168-dkms"], "DKMS", Network, realtekLogo),
          tool("net-realtek", "Realtek firmware", ["firmware-realtek"], "RTL", Network, realtekLogo),
          tool("net-firmware", "firmware-misc-nonfree", ["firmware-misc-nonfree"], "FW", Cpu),
          tool("net-tools", "ethtool", ["ethtool"], "ETH", Cable),
        ],
      },
    ],
  },
  security: {
    id: "security",
    color: "60 180 110",
    sections: [
      {
        id: "security",
        titleKey: "drivers.cat.security",
        items: [
          tool("sec-fprintd", "fprintd", ["fprintd", "libpam-fprintd"], "FP", Fingerprint),
          tool("sec-yubikey", "YubiKey", ["yubikey-manager", "yubikey-personalization", "libpam-u2f", "pcscd"], "YUBI", KeyRound, yubicoLogo),
          tool("sec-opensc", "OpenSC", ["opensc", "pcscd"], "CARD", ShieldCheck),
        ],
      },
    ],
  },
  ups: {
    id: "ups",
    color: "220 190 20",
    sections: [
      {
        id: "tools",
        titleKey: TOOLS,
        items: [tool("ups-nut", "Network UPS Tools", ["nut"], "NUT", BatteryCharging, nutLogo), tool("ups-apcupsd", "apcupsd (APC)", ["apcupsd"], "APC", Zap, apcLogo)],
      },
      { id: "makers", titleKey: MAKERS, items: [maker("APC", BatteryCharging, apcLogo), maker("CyberPower", BatteryCharging), maker("Eaton", BatteryCharging, eatonLogo)] },
    ],
  },
  rgb: {
    id: "rgb",
    color: "200 60 220",
    sections: [
      {
        id: "rgb",
        titleKey: "drivers.cat.rgb",
        items: [
          tool("rgb-openrgb", "OpenRGB", ["openrgb"], "RGB", Lightbulb, undefined, { vendor: "openrgb" }),
          tool("rgb-ckb", "ckb-next (Corsair)", ["ckb-next"], "CKB", Keyboard, corsairLogo),
          tool("rgb-liquidctl", "liquidctl", ["liquidctl"], "COOL", Usb),
          tool("rgb-sensors", "lm-sensors + fancontrol", ["lm-sensors", "fancontrol"], "FAN", Fan),
        ],
      },
    ],
  },
  virtual: {
    id: "virtual",
    color: "229 112 0",
    sections: [
      {
        id: "virtual",
        titleKey: "drivers.cat.virtual",
        items: [
          tool("vm-vmware", "VMware", ["open-vm-tools", "open-vm-tools-desktop"], "VM", Monitor, vmwareLogo),
          tool("vm-qemu", "QEMU / KVM", ["qemu-guest-agent", "spice-vdagent"], "KVM", Server, qemuLogo),
          tool("vm-virtmanager", "Virt-Manager", ["virt-manager", "qemu-system-x86", "libvirt-daemon-system"], "VM", Boxes, virtmanagerLogo),
          tool("vm-boxes", "GNOME Boxes", ["gnome-boxes"], "VM", Boxes, boxesLogo),
        ],
      },
    ],
  },
};
