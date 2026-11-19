<script lang="ts">
  let { value = $bindable(), base = 0, purchase = false, visible = () => true, prefix = "", vehicle = false }: {
    value: any; base?: number; purchase?: boolean; visible?: (column: string) => boolean; prefix?: string; vehicle?: boolean;
  } = $props();
  function has(label: string) {
    return visible(prefix ? prefix + " " + label.toLowerCase() : label);
  }
  let tax = $derived(Math.round((base * Number(value.tax_percent || 0) / 100 + Number(value.tax_fixed || 0)) * 100) / 100);
  let deduction = $derived(Math.round(((value.deductible_base === "Tax" ? tax : base) * Number(value.deductible_percent || 0) / 100 + Number(value.deductible_fixed || 0)) * 100) / 100);
  let showTax = $derived(has(vehicle ? "Tax included" : "Tax"));
  let showDeduction = $derived(has(vehicle && !prefix ? "Deductible amount" : "Deductible"));
</script>
{#if has("Tax percent") || has("Tax fixed")}
  <div class="form-grid">
    {#if has("Tax percent")}<label>Tax percent<input inputmode="decimal" bind:value={value.tax_percent} /></label>{/if}
    {#if has("Tax fixed")}<label>Tax fixed<input inputmode="decimal" bind:value={value.tax_fixed} /></label>{/if}
  </div>
{/if}
{#if showTax}
  {#if !purchase}<label>Transaction tax treatment<select bind:value={value.purchase_tax}><option value={true}>Included in supplier payment — information only</option><option value={false}>Tax liability payable by employer</option></select></label>{/if}
  <p class="calculation-line">Tax: {tax.toFixed(2)}{purchase || value.purchase_tax ? " (included in amount paid)" : ""}</p>
{/if}
{#if has("Deductible percent") || has("Deductible fixed")}
  <div class="form-grid">
    {#if has("Deductible percent")}<label>Deductible percent<input inputmode="decimal" bind:value={value.deductible_percent} /></label>{/if}
    {#if has("Deductible fixed")}<label>Deductible fixed<input inputmode="decimal" bind:value={value.deductible_fixed} /></label>{/if}
  </div>
{/if}
{#if has("Deductible base") || has("Deduction treatment")}
  <div class="form-grid">
    {#if has("Deductible base")}<label>Calculate deductible percent from<select bind:value={value.deductible_base}><option value="Total">Total amount</option><option value="Tax">Tax amount only</option></select></label>{/if}
    {#if has("Deduction treatment")}<label>Apply deductible to<select bind:value={value.deduction_target}><option value="Income">Taxable income (before tax)</option><option value="Tax">Tax payable (tax credit)</option></select></label>{/if}
  </div>
{/if}
{#if showDeduction}<p class="calculation-line">Deductible: {deduction.toFixed(2)}</p>{/if}
