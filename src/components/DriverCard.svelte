<script lang="ts">
  import { createEventDispatcher, type ComponentType } from "svelte";
  import { _ } from "svelte-i18n";

  // One driver on the Drivers page: a dark showcase on top, with the brand
  // color glowing behind its logo and the branch number big and outlined,
  // then what the driver is, whether it is installed and the buttons.
  // Installing or removing asks once, inside the card, before it does anything.
  /** Brand color, as the three channels "r g b". */
  export let color = "255 125 46";
  /** The official logo of the brand, shown on a white tile. */
  export let logo = "";
  /** Shown on the white tile when there is no logo. */
  export let icon: ComponentType | null = null;
  /** Text of a link under the description that opens the page of the maker, for a card that installs. */
  export let moreText = "";
  /** Hides the installed or not badge, for a card that stands for no packages. */
  export let noStatus = false;
  /** Text of the button of a card that opens a web page instead of installing. */
  export let linkText = "";
  /** Big text of the showcase, such as the branch number. */
  export let big = "";
  export let title: string;
  export let hint = "";
  export let installed = false;
  export let recommended = false;
  /** Why the driver cannot be installed here; empty when it can. */
  export let blocked = "";
  /** What the card says while it asks for confirmation; empty when it does not. */
  export let confirmText = "";
  export let canRemove = true;
  export let busy = false;
  export let disabled = false;

  const dispatch = createEventDispatcher<{ install: void; remove: void; confirm: void; cancel: void; open: void }>();

  $: rgb = color;
</script>

<div
  class="group bg-surface border rounded overflow-hidden flex flex-col transition-all duration-200 hover:-translate-y-0.5 hover:shadow-xl {recommended
    ? 'border-primary shadow-[0_0_28px_-8px_rgba(255,125,46,0.55)]'
    : 'border-border hover:border-primary/50'}"
>
  <div
    class="relative h-36 overflow-hidden flex items-center justify-between px-6"
    style="background: radial-gradient(18rem 10rem at 90% 0%, rgb({rgb} / 0.42), transparent 65%), radial-gradient(14rem 9rem at 0% 100%, rgb({rgb} / 0.2), transparent 70%), linear-gradient(135deg, rgb(24 24 27), rgb(16 16 18))"
  >
    {#if logo}
      <img src={logo} alt="" class="absolute -right-6 -bottom-10 h-44 w-44 object-contain opacity-[0.10] pointer-events-none select-none" />
      <div class="relative shrink-0 rounded-2xl bg-white p-3" style="box-shadow: 0 0 0 1px rgb(255 255 255 / 0.5), 0 0 34px -6px rgb({rgb} / 0.75), 0 10px 24px -10px rgb(0 0 0 / 0.6)">
        <img src={logo} alt="" class="h-14 w-14 shrink-0 object-contain" />
      </div>
    {:else if icon}
      <div class="relative shrink-0 rounded-2xl bg-white p-3" style="box-shadow: 0 0 0 1px rgb(255 255 255 / 0.5), 0 0 34px -6px rgb({rgb} / 0.75), 0 10px 24px -10px rgb(0 0 0 / 0.6)">
        <svelte:component this={icon} size={56} strokeWidth={1.5} color="rgb({rgb})" />
      </div>
    {:else}
      <span></span>
    {/if}
    <span
      class="relative leading-none font-black tracking-tight {big.length <= 3 ? 'text-[3.8rem]' : big.length === 4 ? 'text-[2.8rem]' : big.length === 5 ? 'text-[2.2rem]' : 'text-[1.7rem]'}"
      style="color: transparent; -webkit-text-stroke: 2px rgb({rgb}); text-shadow: 0 0 28px rgb({rgb} / 0.45)"
    >
      {big}
    </span>
    {#if recommended}
      <span class="absolute top-2.5 right-3 text-[0.64rem] font-bold uppercase tracking-wider px-2 py-0.5 rounded-full bg-primary text-white shadow">
        {$_("drivers.recommended")}
      </span>
    {/if}
  </div>

  <div class="p-5 flex flex-col gap-2.5 flex-1">
    <div class="flex items-start justify-between gap-2">
      <h3 class="text-[1rem] font-semibold text-text-bright leading-snug">{title}</h3>
      {#if !linkText && !noStatus}
        <span class="text-[0.66rem] px-2 py-0.5 rounded-full shrink-0 font-semibold {installed ? 'bg-success/15 text-success' : 'bg-surface-2 text-text-muted'}">
          {installed ? $_("drivers.status.installed") : $_("drivers.status.missing")}
        </span>
      {/if}
    </div>
    <p class="text-[0.8rem] text-text-muted flex-1">{hint}</p>
    {#if moreText && !linkText}
      <button class="self-start text-[0.76rem] text-primary hover:underline" on:click={() => dispatch("open")}>{moreText}</button>
    {/if}

    {#if confirmText}
      <p class="text-[0.76rem] text-warning bg-warning/10 border border-warning/30 rounded px-3 py-2">{confirmText}</p>
      <div class="flex gap-2 justify-end">
        <button class="px-3.5 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.8rem]" on:click={() => dispatch("cancel")}>
          {$_("drivers.cancel")}
        </button>
        <button class="px-3.5 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.8rem] font-semibold" on:click={() => dispatch("confirm")}>
          {$_("drivers.confirm")}
        </button>
      </div>
    {:else if linkText}
      <div class="flex justify-end">
        <button class="px-4 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.8rem] font-semibold" on:click={() => dispatch("open")}>
          {linkText}
        </button>
      </div>
    {:else if blocked}
      <p class="text-[0.74rem] text-text-muted italic">{blocked}</p>
    {:else}
      <div class="flex gap-2 justify-end">
        {#if installed && canRemove}
          <button
            class="px-3.5 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.8rem] font-semibold disabled:opacity-60 disabled:cursor-not-allowed"
            disabled={disabled || busy}
            on:click={() => dispatch("remove")}
          >
            {$_("drivers.remove")}
          </button>
        {/if}
        {#if !installed}
          <button
            class="px-4 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.8rem] font-semibold disabled:opacity-60 disabled:cursor-not-allowed"
            disabled={disabled || busy}
            on:click={() => dispatch("install")}
          >
            {busy ? $_("common.installing") : $_("drivers.install")}
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>
