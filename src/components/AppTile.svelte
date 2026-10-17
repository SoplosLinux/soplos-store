<script lang="ts">
  import type { AppCard } from "../lib/api";
  import AppIcon from "./AppIcon.svelte";
  import SourceTags from "./SourceTags.svelte";
  import Rating from "./Rating.svelte";
  import InstallButton from "./InstallButton.svelte";

  export let app: AppCard;
  export let onOpen: (id: string) => void;
</script>

<div
  class="bg-surface border border-border rounded p-3.5 flex flex-col gap-2.5 cursor-pointer transition-colors hover:border-primary/60 text-left min-w-0"
  role="button"
  tabindex="0"
  on:click={() => onOpen(app.id)}
  on:keydown={(e) => (e.key === "Enter" || e.key === " ") && onOpen(app.id)}
>
  <div class="flex items-center gap-3 min-w-0">
    <AppIcon src={app.icon} size={52} />
    <div class="min-w-0">
      <div class="text-[0.9rem] font-semibold text-text-bright truncate">{app.name}</div>
      <Rating rating={app.rating} reviews={app.reviews} />
    </div>
  </div>
  <p class="text-[0.78rem] text-text line-clamp-2 min-h-[2.2em]">{app.summary}</p>
  <div class="flex items-center justify-between gap-2 mt-auto">
    <SourceTags sources={app.sources} />
    <InstallButton appId={app.id} name={app.name} refs={app.refs} />
  </div>
</div>
