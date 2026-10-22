<script lang="ts">
  import { _ } from "svelte-i18n";
  import { CATEGORIES, SUBCATEGORIES } from "../lib/categories";
  import CategoryCard from "../components/CategoryCard.svelte";

  export let onOpenCategory: (id: string) => void;

  // A few of the subcategories of each one, to say what is inside.
  const hintOf = (id: string) =>
    (SUBCATEGORIES[id] ?? [])
      .slice(0, 4)
      .map((sub) => $_(`subcategories.${sub}`))
      .join(" · ");
</script>

<div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(15rem,1fr))]">
  {#each CATEGORIES as category (category.id)}
    <CategoryCard
      icon={category.icon}
      color={category.color}
      title={$_(`categories.${category.id}`)}
      hint={hintOf(category.id)}
      on:click={() => onOpenCategory(category.id)}
    />
  {/each}
</div>
