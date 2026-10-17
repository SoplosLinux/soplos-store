<script lang="ts">
  export let checked: boolean;
  export let label: string;
</script>

<button
  class="relative w-[32px] h-[18px] rounded-full transition-colors shrink-0 {checked ? 'bg-primary' : 'bg-surface-2'}"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  on:click={() => (checked = !checked)}
>
  <span class="absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-white transition-transform {checked ? 'translate-x-[14px]' : ''}"></span>
</button>
