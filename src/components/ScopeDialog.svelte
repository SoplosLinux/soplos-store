<script lang="ts">
  import { _ } from "svelte-i18n";
  import { fade, scale } from "svelte/transition";
  import { scopeQuestion } from "../lib/jobs";

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && $scopeQuestion) $scopeQuestion.answer(null);
  }
</script>

<svelte:window on:keydown={onKeydown} />

{#if $scopeQuestion}
  {@const question = $scopeQuestion}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
    role="presentation"
    transition:fade={{ duration: 120 }}
    on:click|self={() => question.answer(null)}
  >
    <div
      class="w-full max-w-[420px] bg-surface border border-border rounded-xl p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label={$_("scope.title", { values: { name: question.name } })}
      transition:scale={{ duration: 140, start: 0.96 }}
    >
      <h2 class="text-text-bright font-semibold text-[1.05rem]">{$_("scope.title", { values: { name: question.name } })}</h2>
      <p class="text-[0.82rem] text-text mt-2">{$_("scope.question")}</p>
      <div class="flex justify-end gap-2 mt-5 flex-wrap">
        <button class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.78rem]" on:click={() => question.answer(null)}>
          {$_("scope.cancel")}
        </button>
        <button class="px-3 py-1.5 rounded border border-border text-text-bright hover:bg-surface-2 text-[0.78rem]" on:click={() => question.answer("system")}>
          {$_("scope.system")}
        </button>
        <button class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-white text-[0.78rem] font-semibold" on:click={() => question.answer("user")}>
          {$_("scope.user")}
        </button>
      </div>
    </div>
  </div>
{/if}
