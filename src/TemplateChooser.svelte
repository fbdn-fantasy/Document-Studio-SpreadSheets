<script lang="ts">
  import { categories, templateCategory } from "./templateCategories";
  let { templates, choose }: {templates:string[]; choose:(name:string)=>void} = $props();
  let category = $state("All types");
  let search = $state("");
  let filtered = $derived(templates.filter(name => {
    if (name === "Blank workbook" || name === "Custom Template") return false;
    const group = name.startsWith("Saved: ") ? "My templates" : templateCategory(name);
    return (category === "All types" || group === category) && name.toLowerCase().includes(search.trim().toLowerCase());
  }));
</script>
<div class="panel-tools">
  <button onclick={() => choose("Blank workbook")}>Blank workbook</button>
  <button class="primary" onclick={() => choose("Custom Template")}>Build a custom workbook</button>
</div>
<div class="form-grid">
  <label>Workbook type<select bind:value={category}><option>All types</option>{#each categories as group}<option>{group}</option>{/each}{#if templates.some(n=>n.startsWith("Saved: "))}<option>My templates</option>{/if}</select></label>
  <label>Search templates<input bind:value={search} placeholder="Budget, gradebook, project…" /></label>
</div>
{#each [...categories,"My templates"] as group}
  {@const items = filtered.filter(n => n.startsWith("Saved: ") ? group === "My templates" : templateCategory(n) === group)}
  {#if items.length}<h3>{group}</h3><div class="chooser-grid">
    {#each items as name}<button onclick={() => choose(name)}>{name.replace(/^Saved: /,"")}</button>{/each}
  </div>{/if}
{/each}
{#if !filtered.length}<p>No matching templates.</p>{/if}
