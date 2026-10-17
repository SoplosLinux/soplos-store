<script lang="ts">
  import { onDestroy } from "svelte";
  import type { AppCard } from "../lib/api";
  import { resolveImage } from "../lib/images";
  import AppIcon from "./AppIcon.svelte";
  import SourceTags from "./SourceTags.svelte";
  import InstallButton from "./InstallButton.svelte";

  // Shows one highlighted app at a time, with its screenshot as the
  // background, and moves to the next one every few seconds.
  export let apps: AppCard[];
  export let kicker: string;
  export let onOpen: (id: string) => void;

  const INTERVAL_MS = 6000;

  let index = 0;
  let paused = false;
  let shots: Record<string, string | null> = {};

  // Only the slide on screen and the next one are downloaded; the large
  // screenshots of all of them at once would slow the first load down.
  $: for (const app of apps.filter((_, i) => i === index || i === (index + 1) % apps.length)) loadShot(app);

  async function loadShot(app: AppCard) {
    if (!app.screenshot || app.id in shots) return;
    shots[app.id] = null;
    shots[app.id] = await resolveImage(app.screenshot);
  }

  const timer = setInterval(() => {
    if (!paused && apps.length > 1) index = (index + 1) % apps.length;
  }, INTERVAL_MS);
  onDestroy(() => clearInterval(timer));
</script>

{#if apps.length}
  <div
    class="relative h-[340px] rounded border border-border overflow-hidden bg-panel"
    role="region"
    on:mouseenter={() => (paused = true)}
    on:mouseleave={() => (paused = false)}
  >
    {#each apps as app, i (app.id)}
      <div
        class="absolute inset-0 transition-opacity duration-500 cursor-pointer {i === index ? 'opacity-100' : 'opacity-0 pointer-events-none'}"
        role="button"
        tabindex={i === index ? 0 : -1}
        on:click={() => onOpen(app.id)}
        on:keydown={(e) => e.key === "Enter" && onOpen(app.id)}
      >
        {#if shots[app.id]}
          <img src={shots[app.id]} alt="" class="absolute right-0 top-0 h-full w-[70%] object-cover object-left-top" />
        {/if}
        <div class="absolute inset-0" style="background: linear-gradient(90deg, rgb(20 20 22) 0%, rgb(20 20 22 / 0.92) 38%, rgb(20 20 22 / 0.2) 100%)"></div>
        <div class="relative h-full flex items-center gap-8 px-10 pb-8">
          <AppIcon src={app.icon} size={136} />
          <div class="min-w-0 max-w-[560px]">
            <div class="text-[0.78rem] font-bold tracking-[0.08em] uppercase text-primary">{kicker}</div>
            <div class="text-[2rem] font-bold text-white mt-0.5 truncate">{app.name}</div>
            <p class="text-[1rem] text-white/85 mt-1.5 line-clamp-3">{app.summary}</p>
            <div class="mt-2.5"><SourceTags sources={app.sources} /></div>
            <div class="mt-3"><InstallButton appId={app.id} name={app.name} refs={app.refs} /></div>
          </div>
        </div>
      </div>
    {/each}

    {#if apps.length > 1}
      <div class="absolute left-10 bottom-4 flex gap-1.5">
        {#each apps as app, i (app.id)}
          <button
            class="h-2 rounded-full transition-all {i === index ? 'w-5 bg-primary' : 'w-2 bg-white/35'}"
            aria-label={app.name}
            on:click={() => (index = i)}
          ></button>
        {/each}
      </div>
    {/if}
  </div>
{/if}
