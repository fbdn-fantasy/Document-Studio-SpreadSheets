<script lang="ts">
 let { value = $bindable(), income = 0 }: {value:any;income?:number} = $props();
 function add(){const previous=value.bands.at(-1); if(previous && !previous.upper){previous.upper=String(Number(previous.lower||0)+100000);} value.bands.push({lower:previous?.upper||'0',upper:'',percent:'0',fixed:''});}
 function remove(i:number){value.bands.splice(i,1); for(let j=0;j<value.bands.length;j++)value.bands[j].lower=j?value.bands[j-1].upper:'0';}
 function preview(){let carry=0;for(const b of value.bands||[]){const lo=Number(b.lower||0),hi=b.upper===''?Infinity:Number(b.upper),fixed=b.fixed===''?carry:Number(b.fixed);if(income<=hi)return fixed+Math.max(0,income-lo)*Number(b.percent||0)/100;carry=fixed+(hi-lo)*Number(b.percent||0)/100;}return 0;}
</script>
<label>Tax name<input required bind:value={value.name} /></label>
<label>Calculation<select bind:value={value.automatic}><option value={true}>Automatic income ranges</option><option value={false}>Legacy manual taxable base</option></select></label>
{#if value.automatic}
 <p class="notice">Uses this workbook's current-period income less refunds and income deductions. Enter ranges for that same period. The higher rate applies only above its lower limit. Leave the final cap blank for unlimited income.</p>
 {#each value.bands as b,i}<fieldset><legend>Range {i+1}</legend><div class="form-grid"><label>Income above<input required inputmode="decimal" bind:value={b.lower} /></label><label>Income cap (blank = unlimited)<input inputmode="decimal" bind:value={b.upper} /></label><label>Percent above lower limit<input inputmode="decimal" bind:value={b.percent} /></label><label>Fixed tax (blank = calculated from lower ranges)<input inputmode="decimal" bind:value={b.fixed} /></label></div><button type="button" onclick={()=>remove(i)}>Remove range</button></fieldset>{/each}
 <button type="button" onclick={add}>+ Add income range</button>
 <p class="calculation-line">Taxable income: {income.toFixed(2)} · Estimated tax: {preview().toFixed(2)}</p><p class="notice">An entered fixed tax replaces the tax carried from lower ranges. It is not charged again for every range. Ranges are checked when saving.</p>
{:else}<label>Taxable base<input inputmode="decimal" bind:value={value.base} /></label><div class="form-grid"><label>Percent<input inputmode="decimal" bind:value={value.percent} /></label><label>Fixed<input inputmode="decimal" bind:value={value.fixed} /></label></div>{/if}
