<script lang="ts">
  // The big round icon of a full-page state (searching, all up to date). It
  // is about 200 px on a normal window and shrinks with a small one: the
  // size follows the smaller side of the window, between 144 and 200 px.
  export let tone: "primary" | "success" = "primary";
  export let pulse = false;
</script>

<div
  class="w-[clamp(9rem,28vmin,12.5rem)] h-[clamp(9rem,28vmin,12.5rem)] rounded-full grid place-items-center ring-[12px] shrink-0 {tone === 'success'
    ? 'bg-success/15 text-success ring-success/5'
    : 'bg-primary/15 text-primary ring-primary/5'} {pulse ? 'animate-pulse' : ''}"
>
  <slot />
</div>
