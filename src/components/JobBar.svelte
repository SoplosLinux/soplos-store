<script lang="ts">
  import { _ } from "svelte-i18n";
  import { X } from "lucide-svelte";
  import { jobs } from "../lib/jobs";

  // Strip above the progress bar with the current (or last) job, its
  // latest output line and the full log on demand.
  let expanded = false;
  let dismissedId: number | null = null;

  $: job = [...$jobs].reverse().find((j) => j.status === "running") ?? $jobs[$jobs.length - 1];
  $: visible = job && job.id !== dismissedId;
  $: lastLine = job?.lines[job.lines.length - 1] ?? "";

  // Keeps the log scrolled to the newest line. Done as an action on the
  // element itself: assigning to a component variable's property
  // (log.scrollTop = ...) would invalidate it and re-run the reactive
  // code in an endless loop, freezing the window.
  function stickToBottom(node: HTMLElement, _lineCount: number) {
    const scroll = () => (node.scrollTop = node.scrollHeight);
    scroll();
    return { update: scroll };
  }

  const STATUS_CLASS = {
    running: "text-primary",
    done: "text-success",
    failed: "text-danger",
    cancelled: "text-text-muted",
  };
</script>

{#if visible && job}
  <div class="border-t border-border bg-panel shrink-0">
    <div class="flex items-center gap-3 px-4 py-2 text-[0.78rem]">
      <span class="font-semibold text-text-bright shrink-0">{job.title}</span>
      <span class="shrink-0 {STATUS_CLASS[job.status]}">
        {job.status === "running" ? "" : $_(`jobs.${job.status}`)}
      </span>
      <span class="flex-1 min-w-0 truncate text-text-muted font-mono text-[0.7rem]">{lastLine}</span>
      <button class="text-primary hover:underline shrink-0" on:click={() => (expanded = !expanded)}>
        {expanded ? $_("jobs.hideLog") : $_("jobs.showLog")}
      </button>
      {#if job.status !== "running"}
        <button
          class="p-1 rounded text-text-muted hover:bg-surface hover:text-text-bright shrink-0"
          title={$_("jobs.close")}
          on:click={() => {
            dismissedId = job.id;
            expanded = false;
          }}
        >
          <X size={14} />
        </button>
      {/if}
    </div>
    {#if expanded}
      <pre
        use:stickToBottom={job.lines.length}
        class="content-scroll max-h-[180px] overflow-auto mx-4 mb-2 p-2.5 rounded bg-dark border border-border text-[0.7rem] text-text font-mono whitespace-pre-wrap">{job.lines.join("\n")}</pre>
    {/if}
  </div>
{/if}
