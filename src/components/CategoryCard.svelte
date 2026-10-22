<script lang="ts">
  import type { ComponentType } from "svelte";

  // A category on a page of categories: a dark showcase with the color of the
  // category glowing behind its icon, then its name and a line of what is in
  // it. The Categories page and the Drivers page both use it. A card that is
  // not ready can carry a badge and does nothing when pressed.
  export let icon: ComponentType;
  /** Color, as the three channels "r g b". */
  export let color: string;
  export let title: string;
  export let hint = "";
  export let badge = "";
  export let disabled = false;
</script>

<button
  class="group text-left bg-surface border border-border rounded overflow-hidden flex flex-col transition-all duration-200 {disabled
    ? 'opacity-60 cursor-default'
    : 'hover:-translate-y-0.5 hover:shadow-xl hover:border-primary/50'}"
  {disabled}
  on:click
>
  <div
    class="relative h-32 overflow-hidden flex items-center justify-center"
    style="background: radial-gradient(16rem 9rem at 85% 0%, rgb({color} / 0.42), transparent 65%), radial-gradient(12rem 8rem at 0% 100%, rgb({color} / 0.2), transparent 70%), linear-gradient(135deg, rgb(24 24 27), rgb(16 16 18))"
  >
    <svelte:component this={icon} size={64} strokeWidth={1.25} color="rgb({color})" />
    {#if badge}
      <span class="absolute top-2.5 right-3 text-[0.64rem] font-bold uppercase tracking-wider px-2 py-0.5 rounded-full bg-white/10 text-white/80">{badge}</span>
    {/if}
  </div>
  <div class="p-4">
    <h3 class="text-[1rem] font-semibold text-text-bright leading-snug">{title}</h3>
    {#if hint}
      <p class="text-[0.76rem] text-text-muted mt-1">{hint}</p>
    {/if}
  </div>
</button>
