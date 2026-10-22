<script context="module" lang="ts">
  import type { ComponentType } from "svelte";

  export interface DriverSlide {
    key: string;
    /** Brand color, as "r g b". */
    color: string;
    /** The official logo, shown on a white tile. */
    logo?: string;
    /** Shown on the tile instead, for what has no logo of its own. */
    icon?: ComponentType;
    kicker: string;
    title: string;
    /** Dedicated, integrated...; empty for none. */
    kind: string;
    /** "installed", "different_version", "partial" or "missing"; empty for no badge. */
    status: string;
    note: string;
    /** This slide is a card of this machine. */
    yours: boolean;
  }
</script>

<script lang="ts">
  import { createEventDispatcher, onDestroy } from "svelte";
  import { ChevronLeft, ChevronRight } from "lucide-svelte";
  import { _ } from "svelte-i18n";

  // The hero of the Drivers page: a dark showcase with the brand color
  // glowing behind its official logo. It starts on your own graphics card,
  // then moves on by itself through every brand the page has drivers for.
  // The page shows the drivers of the slide on screen; the slot of each slide
  // holds the action that fits it.
  export let slides: DriverSlide[];
  /** The slide on screen, by key. */
  export let current: string;
  /** Stops the timer, while the user is working with the page. */
  export let paused = false;
  // The timer also stops while the pointer or the keyboard is on the buttons
  // of the slide, so it does not change under someone about to press one. It
  // does not stop for the pointer being anywhere else in the hero: the page is
  // reached by clicking a card that sits where the hero then appears.
  let hovering = false;

  const dispatch = createEventDispatcher<{ change: string }>();
  const INTERVAL_MS = 7000;

  $: index = Math.max(0, slides.findIndex((s) => s.key === current));

  function go(i: number) {
    const next = slides[(i + slides.length) % slides.length];
    if (next) dispatch("change", next.key);
  }

  const timer = setInterval(() => {
    if (!paused && !hovering && slides.length > 1) go(index + 1);
  }, INTERVAL_MS);
  onDestroy(() => clearInterval(timer));

  $: badge = (status: string) =>
    status === "installed"
      ? { key: "drivers.status.installed", cls: "bg-success/20 text-success" }
      : status === "different_version"
        ? { key: "drivers.status.different", cls: "bg-warning/20 text-warning" }
        : status === "partial"
          ? { key: "drivers.status.partial", cls: "bg-warning/20 text-warning" }
          : { key: "drivers.status.missing", cls: "bg-white/10 text-white/70" };
</script>

<div class="relative rounded border border-border overflow-hidden bg-[rgb(18_18_20)] grid" role="region" aria-label={$_("drivers.vendors.kicker")}>
  {#each slides as slide, i (slide.key)}
    {@const rgb = slide.color}
    {@const b = badge(slide.status)}
    <div
      class="relative [grid-area:1/1] h-[22rem] overflow-hidden transition-opacity duration-500 {i === index ? 'opacity-100' : 'opacity-0 pointer-events-none'}"
      style="background: radial-gradient(60rem 28rem at 88% 8%, rgb({rgb} / 0.34), transparent 62%), radial-gradient(34rem 20rem at 0% 100%, rgb({rgb} / 0.16), transparent 70%), linear-gradient(135deg, rgb(24 24 27) 0%, rgb(16 16 18) 100%)"
      aria-hidden={i !== index}
    >
      {#if slide.logo}
        <img src={slide.logo} alt="" class="absolute -right-14 -bottom-20 h-[28rem] w-[28rem] object-contain opacity-[0.11] pointer-events-none select-none" />
      {:else if slide.icon}
        <svelte:component this={slide.icon} size={448} strokeWidth={0.75} color="rgb({rgb})" class="absolute -right-14 -bottom-20 opacity-[0.10] pointer-events-none select-none" />
      {/if}

      <div class="relative h-full flex items-center gap-9 px-14 pt-6 pb-12 flex-wrap content-center">
        <div
          class="shrink-0 rounded-[1.75rem] bg-white p-[clamp(1.2rem,2.6vmin,1.8rem)]"
          style="box-shadow: 0 0 0 1px rgb(255 255 255 / 0.5), 0 0 70px -8px rgb({rgb} / 0.7), 0 18px 40px -12px rgb(0 0 0 / 0.6)"
        >
          {#if slide.logo}
            <img src={slide.logo} alt="" class="h-[clamp(5.5rem,15vmin,9rem)] w-[clamp(5.5rem,15vmin,9rem)] object-contain" />
          {:else if slide.icon}
            <div class="h-[clamp(5.5rem,15vmin,9rem)] w-[clamp(5.5rem,15vmin,9rem)] flex items-center justify-center">
              <svelte:component this={slide.icon} size="100%" strokeWidth={1.25} color="rgb({rgb})" />
            </div>
          {/if}
        </div>

        <!-- Every part has a slot of fixed height, so the texts sit in the same place on every slide. -->
        <div class="flex-1 min-w-[16rem] min-w-0">
          <div class="h-6 flex items-center gap-2">
            <span class="text-[0.78rem] font-bold tracking-[0.1em] uppercase truncate" style="color: rgb({rgb})">{slide.kicker}</span>
            {#if slide.yours}
              <span class="shrink-0 text-[0.64rem] font-bold uppercase tracking-wider px-2 py-0.5 rounded-full bg-primary text-white">{$_("drivers.vendors.yours")}</span>
            {/if}
          </div>
          <h2 class="h-[3.1rem] flex items-center text-[clamp(1.7rem,4.4vmin,2.6rem)] font-bold text-white leading-none">
            <span class="truncate" title={slide.title}>{slide.title}</span>
          </h2>
          <div class="h-7 mt-1.5 flex items-center gap-2 text-[0.78rem]">
            {#if slide.kind}
              <span class="px-2 py-0.5 rounded bg-white/10 text-white/80 font-medium">{slide.kind}</span>
            {/if}
            {#if slide.status}
              <span class="px-2 py-0.5 rounded font-semibold {b.cls}">{$_(b.key)}</span>
            {/if}
          </div>
          <p class="h-[3rem] mt-1.5 text-[1rem] leading-6 text-white/85 max-w-xl line-clamp-2">{slide.note}</p>
          <!-- svelte-ignore a11y-no-static-element-interactions -->
          <div
            class="h-[5rem] mt-2 flex flex-col gap-2 items-start"
            on:mouseenter={() => (hovering = true)}
            on:mouseleave={() => (hovering = false)}
            on:focusin={() => (hovering = true)}
            on:focusout={() => (hovering = false)}
          >
            {#if i === index}
              <slot name="actions" {slide} />
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/each}

  {#if slides.length > 1}
    <button
      class="absolute left-2 top-1/2 -translate-y-1/2 p-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white"
      aria-label="previous"
      on:click={() => go(index - 1)}
    >
      <ChevronLeft size={20} />
    </button>
    <button
      class="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white"
      aria-label="next"
      on:click={() => go(index + 1)}
    >
      <ChevronRight size={20} />
    </button>
    <div class="absolute left-0 right-0 bottom-4 flex justify-center gap-1.5">
      {#each slides as slide, i (slide.key)}
        <button
          class="h-2 rounded-full transition-all {i === index ? 'w-5 bg-primary' : 'w-2 bg-white/35 hover:bg-white/60'}"
          aria-label={slide.title}
          on:click={() => go(i)}
        ></button>
      {/each}
    </div>
  {/if}
</div>
