<script lang="ts">
  import { templateOptions } from "./templateOptions";
  let {value=$bindable(),catalog=[],locked=[]}: {value:any;catalog:any[];locked?:string[]} = $props();
  import { categories, templateCategory } from "./templateCategories";
  let category = $state("All types");
  let search = $state("");

  function toggle(name: string, checked: boolean) {
    value.sheets[name] ??= {enabled:false, columns:{}};
    value.sheets[name].enabled = checked;
    const base = catalog.find(sheet => sheet.name === name)?.requires;
    if (checked && base) {
      value.sheets[base] ??= {enabled:false, columns:{}};
      value.sheets[base].enabled = true;
    }
  }

  function setColumns(sheet: string, columns: string[], checked: boolean) {
    const selection = value.sheets[sheet].columns ??= {};
    for (const column of columns) selection[column] = checked;
  }
</script>
<label>Template name<input required bind:value={value.name} placeholder="My template" /></label>
<label>Show worksheets for<select bind:value={category}><option>All types</option>{#each categories as group}<option>{group}</option>{/each}</select></label>
<label>Find a worksheet<input bind:value={search} placeholder="Fuel, payroll, budget…" /></label>
<p>Choose a worksheet, then choose what to include in it.</p>
<div class="template-options">
  {#each catalog.filter(sheet => (category === "All types" || templateCategory(sheet.name) === category) && sheet.name.toLowerCase().includes(search.trim().toLowerCase())) as sheet}
    <details>
      <summary><label class="switch-label">
        <input type="checkbox" role="switch" disabled={locked.includes(sheet.name)}
          checked={!!value.sheets[sheet.name]?.enabled}
          onchange={e => {
            toggle(sheet.name, e.currentTarget.checked);
            const details = e.currentTarget.closest("details");
            if (details) details.open = e.currentTarget.checked;
          }} />
        {sheet.name}<small>{locked.includes(sheet.name) ? "Already included" : sheet.live ? "Linked" : "Spreadsheet"}</small>
      </label></summary>
      {#if value.sheets[sheet.name]?.enabled}
        <div class="column-options">
          {#each templateOptions(sheet.name, sheet.columns) as option}
            {@const selected = option.columns.filter(column => value.sheets[sheet.name].columns?.[column] !== false).length}
            <label class="switch-label">
              <input type="checkbox" disabled={locked.includes(sheet.name) && !(sheet.live && option.name === "Description")}
                checked={selected === option.columns.length}
                indeterminate={selected > 0 && selected < option.columns.length}
                onchange={e => setColumns(sheet.name, option.columns, e.currentTarget.checked)} />
              {option.name}
              {#if selected > 0 && selected < option.columns.length}<small>Partly selected</small>{/if}
            </label>
          {/each}
        </div>
      {/if}
    </details>
  {/each}
</div>
