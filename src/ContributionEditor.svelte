<script lang="ts">
  let { value = $bindable(), base = 0, currency = "ZAR", company = false, remove }: {
    value: any; base?: number; currency?: string; company?: boolean; remove: () => void;
  } = $props();
  const n = (v: any) => Number(v || 0);
  const money = (v: number) => new Intl.NumberFormat(undefined, { style: "currency", currency }).format(v);
  let cappedBase = $derived(value.cap == null || value.cap === "" ? base : Math.min(base, n(value.cap)));
  const cents = (v: number) => Math.round((v + Number.EPSILON) * 100) / 100;
  let pool = $derived(value.split_pool ? cents(cappedBase * n(value.total_percent) / 100 + n(value.total_fixed)) : cappedBase);
  let employeeShare = $derived(cents(pool * n(value.employee_percent) / 100));
  let employeeAmount = $derived(cents(employeeShare + n(value.employee_fixed)));
  let employerAmount = $derived(value.split_pool ? cents(pool - employeeShare + n(value.employer_fixed)) : cents(pool * n(value.employer_percent) / 100 + n(value.employer_fixed)));
  function convertLegacy() {
    const total = n(value.employee_percent) + n(value.employer_percent);
    value.total_percent = String(total); value.total_fixed = "0";
    const employeeMillionths = total ? Math.round(n(value.employee_percent) / total * 100000000) : 0;
    value.employee_percent = String(employeeMillionths / 1000000);
    value.employer_percent = String((100000000 - employeeMillionths) / 1000000);
    value.split_pool = true;
  }
</script>
<article class="contribution-editor">
  <div class="panel-tools"><strong>{value.name || "New contribution"}</strong><button type="button" class="danger-text" onclick={remove}>Remove contribution</button></div>
  <label>Contribution name<input required bind:value={value.name} /></label>
  {#if company}<label>Contribution base ({currency})<input inputmode="decimal" bind:value={value.base} /></label><small>Assign employee shares through employee settings so payroll deducts them.</small>{/if}
  {#if !value.split_pool}<div class="notice">Existing contribution: percentages apply directly to gross pay. <button type="button" onclick={convertLegacy}>Convert to total + split</button></div>
  {:else}<div class="form-grid"><label>Total fixed<input inputmode="decimal" bind:value={value.total_fixed} /></label><label>Total percent<input inputmode="decimal" bind:value={value.total_percent} /></label></div><p class="calculation-line">Pool = {money(cappedBase)} × {value.total_percent || "0"}% + {money(n(value.total_fixed))} = <strong>{money(pool)}</strong></p>{/if}
  <div class="form-grid"><label>Employee percent<input inputmode="decimal" bind:value={value.employee_percent} disabled={company} /></label><label>Employer percent<input inputmode="decimal" bind:value={value.employer_percent} disabled={company} /></label><label>Employee fixed<input inputmode="decimal" bind:value={value.employee_fixed} disabled={company} /></label><label>Employer fixed<input inputmode="decimal" bind:value={value.employer_fixed} /></label></div>
  {#if value.cap != null}<label>Maximum pay base<input inputmode="decimal" bind:value={value.cap} /></label><button type="button" onclick={() => value.cap = null}>Remove base cap</button>{/if}
  <div class="contribution-preview"><span>Employee <strong>{money(employeeAmount)}</strong></span><span>Employer <strong>{money(employerAmount)}</strong></span><span>Total contribution <strong>{money(employeeAmount + employerAmount)}</strong></span></div>
  {#if value.split_pool && pool > 0 && Math.abs(n(value.employee_percent) + n(value.employer_percent) - 100) > 0.000001}<p class="form-error">Employee and employer percentages must add up to 100% of the pool.</p>{/if}
</article>
