<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let checked: boolean;
  export let label: string;
  // A switch that cannot be changed right now (it keeps its value).
  export let disabled = false;

  // Sent after the user flips the switch, for the cases where the change
  // is not just a stored value (it starts a job and may fail).
  const dispatch = createEventDispatcher<{ change: boolean }>();

  function flip() {
    checked = !checked;
    dispatch("change", checked);
  }
</script>

<button
  class="relative w-[32px] h-[18px] rounded-full transition-colors shrink-0 {checked ? 'bg-primary' : 'bg-surface-2'} {disabled ? 'opacity-50 cursor-not-allowed' : ''}"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  on:click={flip}
>
  <span class="absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-white transition-transform {checked ? 'translate-x-[14px]' : ''}"></span>
</button>
