<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open, confirm } from "@tauri-apps/plugin-dialog";
  import TemplateBuilder from "./TemplateBuilder.svelte";
  let templateConfig=$state<any>({name:"",sheets:{}}), templateCatalog=$state<any[]>([]);
  import TaxFields from "./TaxFields.svelte";
  import TaxRanges from "./TaxRanges.svelte";
  import ContributionEditor from "./ContributionEditor.svelte";
  let { doc, changed, error, newMonth, openSheet }: {
    doc: any; changed: (d: any) => void; error: (e: unknown) => void;
    newMonth: () => void; openSheet: (sheet: number) => void;
  } = $props();
  const employerTab = "Employer taxes & contributions", employeeTab = "Employee taxes & contributions";
  const tabs = ["Overview", "Transactions", "Payroll", employerTab, employeeTab, "Inventory", "Profit & loss", "Outstanding balances", "Payment history", "Settings", "Rollover & year-end", "Worksheets", "Audit"];
  const allCategories = ["Income", "Expense", "Fuel", "Vehicle repairs", "Asset", "Refund", "Other payable"];
  const categorySheets:Record<string,string>={Income:"Income",Expense:"Expenses",Fuel:"Fuel","Vehicle repairs":"Vehicle Repairs",Asset:"Assets",Refund:"Refunds","Other payable":"Expenses"};
  let categories=$derived(allCategories.filter(c=>!doc.metadata.settings?.custom_template || doc.metadata.settings.custom_template.sheets?.[categorySheets[c]]?.enabled));
  let tab = $state("Overview"), edit = $state(""), busy = $state(false), formError = $state("");
  let view = $state<any>({ summary: {}, balances: [], payroll: [], taxes: { rows: [] }, days: 31 });
  let filter = $state(""), categoryFilter = $state("All"), expanded = $state("");
  let selected = $state<any>(null), transaction = $state<any>({}), payment = $state<any>({}), employee = $state<any>({});
  let deduction = $state<any>({}), extra = $state<any>({});
  let tax = $state<any>({}), companyContribution = $state<any>({}), contributionIndex = $state(-1);
  let day = $state<any>(null), dayEmployee = $state(""), settings = $state<any>({});
  let inventory = $state<any[]>([]), stock = $state<any>({});
  let periodFrom = $state(""), periodTo = $state(""), report = $state<any>(null), preview = $state<any>(null);
  let importPaths = $state<string[]>([]), rollover = $state(true);
  let metadata = $derived(doc.metadata), business = $derived(metadata.business), taxes = $derived(view.taxes || { rows: [] });
  let currentRecords = $derived(metadata.records.filter((r: any) => r.period === business.period));
  let shownRecords = $derived(metadata.records.filter((r: any) => {
    if (!JSON.stringify(r).toLowerCase().includes(filter.toLowerCase())) return false;
    if (tab === "Outstanding balances") return Number(balance(r)?.outstanding) > 0;
    return !r.managed_key && categories.includes(r.category) && (categoryFilter === "All" || categoryFilter === r.category);
  }));
  let taxObligations = $derived(currentRecords.filter((r: any) => tab === employeeTab
    ? r.managed_key?.includes(":employee-tax:") || r.managed_key?.includes(":employee-contributions:") || r.category === "Employee withholding" || r.category === "Payroll withholding"
    : r.category === "Employer tax" || r.category === "Employer contribution" || r.category === "Tax"));
  let draftGross = $derived(grossFor(employee));
  const labels: Record<string, string> = { revenue: "Revenue", expenses: "Expenses", profit: "Operating profit", cash_received: "Cash received", cash_paid: "Cash paid", cash_movement: "Cash movement", receivables: "Receivables", payables: "Payables" };
  const taxLabels: Record<string, string> = { income: "Income less refunds", income_deductions: "Income deductions", taxable_income: "Taxable income", employer_tax: "Employer tax before deductions", deductions: "Tax credits / claims", net_employer_tax: "Employer tax after deductions", unused_deductions: "Unused deductions", employer_contributions: "Total employer contributions", employee_tax: "Employee tax to remit", employee_contributions: "Total employee contributions", employer_due: "Employer’s own obligation", employee_due: "Payable on behalf of employees" };
  const titles: Record<string,string> = { sheets:"Manage worksheets", deduction: "Employer deduction", extra: "Additional employee tax / contribution", employee: "Employee settings", payment: "Record a payment", day: "Attendance and exceptions", tax: "Employer tax", contribution: "Company contribution", stock: "Product and stock", settings: "Business details" };
  $effect(() => { const id = doc.id; doc.metadata; void refresh(id); });
  $effect(() => { if (tab === "Inventory") { const id = doc.id; void refreshInventory(id); } });
  async function refresh(id = doc.id) {
    try { const revision = metadata; const next = await invoke<any>("financial_view", { id }); if (doc.id === id && revision === metadata) view = next; } catch (e) { error(e); }
  }
  async function refreshInventory(id = doc.id) {
    try { const next = await invoke<any>("inventory_view", { id }); if (doc.id === id) inventory = next.rows; } catch (e) { error(e); }
  }
  function clone() { return structuredClone($state.snapshot(metadata)); }
  function copy(value: any) { return structuredClone($state.snapshot(value)); }
  function today() { const d = new Date(); return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}-${String(d.getDate()).padStart(2,"0")}`; }
  function balance(r: any) { return view.balances.find((b: any) => b.id === r?.id); }
  function formatted(v: any) { return new Intl.NumberFormat(undefined, { style: "currency", currency: business.currency }).format(Number(v || 0)); }
  function total(rows: any[], field: string) { return rows.reduce((sum, row) => sum + Number(row[field] || 0), 0); }
  function percentFixed(base: any, percent: any, fixed: any) { return Number(base || 0) * Number(percent || 0) / 100 + Number(fixed || 0); }
  function grossFor(e: any) {
    if (!e.structure) return 0;
    const rate = Number(e.rate || 0), standard = Number(e.standard_hours || 8);
    const hourly = e.structure === "Hourly" ? rate : e.structure === "Daily" ? rate / standard : 0;
    let gross = e.structure === "Monthly" ? rate : 0;
    for (const d of e.days || []) {
      if (!d.date.startsWith(business.period)) continue;
      if (["Present", "Paid leave", "Public holiday", "Sick leave"].includes(d.status) && e.structure !== "Monthly") gross += Number(d.hours || 0) * hourly;
      gross += Number(d.overtime || 0) * hourly * Number(e.overtime_multiplier || 0) + Number(d.extra || 0);
    }
    return Math.round(gross * 100) / 100;
  }
  function openEdit(type: string) { formError = ""; edit = type; }
  async function run(action: () => Promise<void>, close = false) {
    if (busy) return;
    busy = true; formError = "";
    try { await action(); if (close) edit = ""; } catch (e) { if (edit) formError = String(e); else error(e); } finally { busy = false; }
  }
  async function persist(m: any) {
    m.audit.push(`${new Date().toISOString()}: ${edit || tab} updated`);
    changed(await invoke("update_business", { id: doc.id, metadata: m })); await refresh();
  }
  function unitInfo(code=transaction.vehicle?.units || "") {const country=business.country;const u=code || (country==="United States"?"US":country==="United Kingdom"?"UK":"Metric");return {code:u,volume:u==="US"?"US gallons":u==="Imperial"?"Imperial gallons":"litres",price:u==="US"?"US gallon":u==="Imperial"?"Imperial gallon":"litre",distance:u==="Metric"?"km":"miles",economy:u==="Metric"?"km/L":u==="US"?"mpg (US)":"mpg (Imperial)"};}
  let fuelDistance=$derived(Number(transaction.vehicle?.end_odometer||0)-Number(transaction.vehicle?.start_odometer||0));
  let fuelEconomy=$derived(Number(transaction.vehicle?.litres)>0?fuelDistance/Number(transaction.vehicle.litres)*(unitInfo().code==="UK"?4.54609:1):0);
  let draftBase = $derived(transaction.category === "Fuel"
    ? Number(transaction.vehicle?.price_per_litre || 0) * Number(transaction.vehicle?.litres || 0)
    : transaction.category === "Vehicle repairs"
      ? Number(transaction.vehicle?.part_amount || 0) + Number(transaction.vehicle?.labour_amount || 0)
      : Number(transaction.pricing?.gross || 0));
  let draftDiscount = $derived(["Fuel","Income"].includes(transaction.category)
    ? percentFixed(draftBase, transaction.pricing?.discount_percent, transaction.pricing?.discount_fixed) : 0);
  $effect(() => {
    if (edit === "transaction" && transaction.pricing) transaction.amount = (draftBase - draftDiscount).toFixed(2);
  });
  function tabVisible(t:string){const cfg=metadata.settings?.custom_template;if(!cfg)return true;const names:Record<string,string>={Overview:"Overview",Settings:"Settings",Payroll:"Payroll",[employerTab]:"Employer Tax & Contributions",[employeeTab]:"Employee Tax & Contributions",Inventory:"Inventory","Profit & loss":"Profit and Loss","Outstanding balances":"Outstanding Balances","Payment history":"Payment History"};if(t==="Transactions")return categories.length>0;if(t==="Payroll")return ["Payroll","Payslips","Timesheets","Employee Tax & Contributions"].some(n=>cfg.sheets?.[n]?.enabled);if(t==="Rollover & year-end")return cfg.sheets?.Income?.enabled&&cfg.sheets?.Payroll?.enabled;return !names[t]||cfg.sheets?.[names[t]]?.enabled;}
  $effect(()=>{if(!tabVisible(tab))tab=tabs.find(tabVisible) || "Worksheets";if(categoryFilter!=="All" && !categories.includes(categoryFilter))categoryFilter="All";});
  function visible(sheet: string, column: string) {
    const sheets = metadata.settings?.custom_template?.sheets;
    return !sheets || (sheets[sheet]?.enabled === true && sheets[sheet]?.columns?.[column] !== false);
  }
  let showDescription = $derived(metadata.settings?.custom_template?.sheets?.[categorySheets[transaction.category]]?.columns?.Description !== false);
  function tx(column: string) { return visible(categorySheets[transaction.category], column); }
  function payColumn(column: string, slip = column) {
    return visible("Payroll", column) || visible("Payslips", slip);
  }
  function attendance(column: string) {
    const cfg = metadata.settings?.custom_template?.sheets;
    return !cfg || (cfg.Timesheets?.enabled ? visible("Timesheets",column) : visible("Payroll","Days present"));
  }
  let overviewSheet = $derived(tab === "Profit & loss" ? "Profit and Loss" : "Overview");
  let taxSheet = $derived(tab === employerTab ? "Employer Tax & Contributions" : "Employee Tax & Contributions");
  function taxMetric(key: string) {
    if (key === "income_deductions") return visible(taxSheet, "Income deductions");
    if (key === "deductions" || key === "unused_deductions") return visible(taxSheet, "Tax credits");
    return visible(taxSheet, tab === employerTab ? "Employer amount" : "Employee amount");
  }
  function taxFields(purchase=false) { return { tax_percent:"0", tax_fixed:"0", deductible_percent:"0", deductible_fixed:"0", deductible_base:"Total", deduction_target:"Income", purchase_tax:purchase }; }
  function editDeduction(d?:any){deduction=d?copy(d):{id:crypto.randomUUID(),name:"",percent:"0",fixed:"0",target:visible(taxSheet,"Income deductions")?"Income":"Tax"};openEdit("deduction");}
  async function saveDeduction(){await run(async()=>{const m=clone();m.deductions||=[];const i=m.deductions.findIndex((d:any)=>d.id===deduction.id);if(i<0)m.deductions.push(copy(deduction));else m.deductions[i]=copy(deduction);await persist(m);},true);}
  async function removeDeduction(id:string){await run(async()=>{const m=clone();m.deductions=m.deductions.filter((d:any)=>d.id!==id);await persist(m);});}
  function editExtra(x?:any){extra=x?copy(x):{id:crypto.randomUUID(),number:"",employee:"",name:"",kind:"Contribution",base:"0",percent:"0",fixed:"0"};openEdit("extra");}
  async function saveExtra(){await run(async()=>{const m=clone();m.employee_extras||=[];const i=m.employee_extras.findIndex((x:any)=>x.id===extra.id);if(i<0)m.employee_extras.push(copy(extra));else m.employee_extras[i]=copy(extra);await persist(m);},true);}
  async function removeExtra(id:string){await run(async()=>{const m=clone();m.employee_extras=m.employee_extras.filter((x:any)=>x.id!==id);await persist(m);});}
  function addTransaction(r?: any) {
    if(categoryFilter === "All" && !categories.includes("Income"))categoryFilter=categories[0]||"Income";
    transaction = r ? copy(r) : { category: categoryFilter === "All" ? "Income" : categoryFilter, party: "", description: "", amount: "0.00", due: "", calculation: taxFields(categoryFilter !== "Income" && categoryFilter !== "All") };
    if(!transaction.pricing?.enabled)transaction.pricing={enabled:true,gross:transaction.amount||"0",discount_percent:"0",discount_fixed:"0"};
    transaction.pricing = {enabled:true,gross:transaction.amount || "0",discount_percent:"0",discount_fixed:"0",...transaction.pricing}; transaction.pricing.enabled=true;
    transaction.calculation ||= { tax_percent: "0", tax_fixed: "0", deductible_percent: "0", deductible_fixed: "0" };
    transaction.calculation.deductible_base ||= "Total"; transaction.calculation.deduction_target ||= "Tax"; transaction.invoice ||= "";
    transaction.vehicle ||= {}; transaction.vehicle = { start_odometer:"",end_odometer:"",units:"",registration:"",employee:"",price_per_litre:"0",litres:"0",part_name:"",part_number:"",part_amount:"0",labour_amount:"0",...transaction.vehicle };
    transaction.vehicle.part = {...taxFields(true),...transaction.vehicle.part}; transaction.vehicle.labour = {...taxFields(true),...transaction.vehicle.labour}; openEdit("transaction");
  }
  async function saveTransaction() {
    await run(async () => { const m = clone(), draft = copy(transaction);
      if(draft.category === "Vehicle repairs") draft.amount=(Number(draft.vehicle.part_amount||0)+Number(draft.vehicle.labour_amount||0)).toFixed(2);
      if(["Fuel","Vehicle repairs"].includes(draft.category)) draft.calculation.purchase_tax=true;
      if (draft.id) { draft.revision++; m.records[m.records.findIndex((r: any) => r.id === draft.id)] = draft; }
      else m.records.push({ ...draft, id: crypto.randomUUID(), revision: 1, period: business.period, payments: [], disputed: false, source_workbook: metadata.workbook_id, managed_key: "" });
      await persist(m);
    }, true);
  }
  async function removeTransaction(r: any) {
    if (!(await confirm(`Remove “${r.description}”? If it has payment history, its amount will be set to zero and the history retained.`, { title: "Remove transaction", kind: "warning" }))) return;
    await run(async () => { const m = clone(); if (r.payments.length) { const record = m.records.find((x: any) => x.id === r.id); m.audit.push(`Voided transaction ${r.id}; previous amount ${r.amount}`); record.amount = "0"; record.pricing={enabled:record.category==="Fuel",gross:"0",discount_percent:"0",discount_fixed:"0"}; record.vehicle.litres="0"; record.vehicle.start_odometer="";record.vehicle.end_odometer=""; record.vehicle.part_amount="0"; record.vehicle.labour_amount="0"; record.vehicle.part=taxFields(true); record.vehicle.labour=taxFields(true); record.description = `Voided: ${record.description}`; record.calculation = { tax_percent: "0", tax_fixed: "0", deductible_percent: "0", deductible_fixed: "0" }; record.revision++; } else m.records = m.records.filter((x: any) => x.id !== r.id); await persist(m); }, edit === "transaction");
  }
  async function togglePaid(r: any) {
    if (!r) return;
    await run(async () => { changed(await invoke("set_paid", { id: doc.id, recordId: r.id, paid: Number(balance(r)?.outstanding || 0) > 0 })); await refresh(); });
  }
  function pay(r: any) { selected = r; payment = { amount: balance(r)?.outstanding || "0.00", date: today(), method: "Bank transfer", reference: "", notes: "" }; openEdit("payment"); }
  async function savePayment() { await run(async () => { const m = clone(); m.records.find((r: any) => r.id === selected.id).payments.push({ ...copy(payment), id: crypto.randomUUID(), reversal_of: null }); await persist(m); }, true); }
  async function reverse(r: any, p: any) { await run(async () => { const m = clone(); m.records.find((x: any) => x.id === r.id).payments.push({ ...copy(p), id: crypto.randomUUID(), date: today(), amount: "-" + p.amount.replace(/^-/, ""), reversal_of: p.id, notes: "Payment reversal" }); await persist(m); }); }
  function addEmployee(e?: any) {
    employee = e ? copy(e) : { id: crypto.randomUUID(), name: "", number: "", site: "", role: "", structure: "Monthly", rate: "0.00", standard_hours: "8", overtime_multiplier: "1.5", manual_tax: "0.00", tax_percent: "0", tax_payer: "Employee", status: "Active", contributions: [], days: [] };
    employee.identifiers ||= []; employee.number ||= ""; employee.site ||= ""; employee.status ||= "Active"; employee.tax_payer ||= "Employee"; employee.tax_percent ||= "0"; openEdit("employee");
  }
  async function saveEmployee() { await run(async () => { const m = clone(), i = m.employees.findIndex((e: any) => e.id === employee.id); if (i < 0) m.employees.push(copy(employee)); else m.employees[i] = copy(employee); await persist(m); }, true); }
  async function removeEmployee(e: any) {
    if (!(await confirm(`Remove ${e.name} and their current payroll inputs? To retain this month and stop next month’s rollover, use Resigned or Fired instead.`, { title: "Remove employee", kind: "warning" }))) return;
    await run(async () => { const m = clone(); m.employees = m.employees.filter((x: any) => x.id !== e.id); await persist(m); }, edit === "employee");
  }
  function newContribution(company = false) { return { name: "", total_percent: "0", total_fixed: "0", employee_percent: company ? "0" : "50", employer_percent: company ? "100" : "50", employee_fixed: "0", employer_fixed: "0", base: "0", cap: null, split_pool: true }; }
  function payrollRecord(e: any) { return currentRecords.find((r: any) => r.managed_key === `${business.period}:payroll:${e.id}`); }
  function openDay(e: any, n: number) { const date = `${business.period}-${String(n).padStart(2,"0")}`; day = copy(e.days.find((d: any) => d.date === date) || { date, status: "Unmarked", hours: "0", overtime: "0", extra: "0.00", deduction: "0.00", note: "" }); dayEmployee = e.id; openEdit("day"); }
  async function saveDay() { await run(async () => { const m = clone(), e = m.employees.find((e: any) => e.id === dayEmployee); e.days = e.days.filter((d: any) => d.date !== day.date); e.days.push(copy(day)); await persist(m); }, true); }
  function addTax(t?: any) { tax = t ? copy(t) : { id: crypto.randomUUID(), name: "", base: "0", percent: "0", fixed: "0", automatic: true, bands: [{lower:"0",upper:"",percent:"0",fixed:""}] }; tax.bands ||= []; if(!tax.bands.length)tax.bands=[{lower:"0",upper:"",percent:"0",fixed:""}]; openEdit("tax"); }
  async function saveTax() { await run(async () => { const m = clone(); m.business_taxes ||= []; const i = m.business_taxes.findIndex((t: any) => t.id === tax.id); if (i < 0) m.business_taxes.push(copy(tax)); else m.business_taxes[i] = copy(tax); await persist(m); }, true); }
  async function removeTax(t: any) { await run(async () => { const m = clone(); m.business_taxes = m.business_taxes.filter((x: any) => x.id !== t.id); await persist(m); }); }
  function editCompanyContribution(c?: any, i = -1) { companyContribution = c ? copy(c) : newContribution(true); contributionIndex = i; openEdit("contribution"); }
  async function saveCompanyContribution() { await run(async () => { const m = clone(); m.business_contributions ||= []; if (contributionIndex < 0) m.business_contributions.push(copy(companyContribution)); else m.business_contributions[contributionIndex] = copy(companyContribution); await persist(m); }, true); }
  async function removeCompanyContribution(i: number) { await run(async () => { const m = clone(); m.business_contributions.splice(i,1); await persist(m); }, edit === "contribution"); }
  function editStock(s?: any) { stock = s ? copy(s) : { row: 0, product: "", opening: "0", received: "0", sold: "0", cost: "0" }; openEdit("stock"); }
  async function saveStock(remove = false, s = stock) { await run(async () => { changed(await invoke("save_inventory", { id: doc.id, row: s.row, product: String(s.product || (!visible("Inventory","Product") ? "Stock item" : "")), opening: String(s.opening), received: String(s.received), sold: String(s.sold), cost: String(s.cost), remove })); await refreshInventory(); }, edit === "stock"); }
  async function manageSheets(){await run(async()=>{templateCatalog=await invoke<any[]>("template_catalog");templateConfig=metadata.settings?.custom_template?copy(metadata.settings.custom_template):{name:metadata.template,sheets:Object.fromEntries(doc.sheets.map((s:any)=>[s.name,{enabled:true,columns:{}}]))};openEdit("sheets");});}
  async function saveSheets(){await run(async()=>{const m=clone();m.settings.custom_template=copy(templateConfig);await persist(m);},true);}
  function editSettings() { settings = copy(business); openEdit("settings"); }
  async function saveSettings() { await run(async () => { const m = clone(); m.business = copy(settings); await persist(m); }, true); }
  async function importFiles() { await run(async () => { const result = await open({ multiple: true, filters: [{ name: "Monthly workbooks", extensions: ["xlsx", "ods"] }] }); if (!result) return; importPaths = Array.isArray(result) ? result : [result]; preview = await invoke("import_periods", { id: doc.id, paths: importPaths, rollover, commit: false }); }); }
  async function commitImport() { await run(async () => { changed(await invoke("import_periods", { id: doc.id, paths: importPaths, rollover, commit: true })); preview = null; await refresh(); }); }
  async function reportPeriod() { await run(async () => { report = await invoke("financial_view", { id: doc.id, from: periodFrom, to: periodTo }); }); }
  function goSheet(name: string) { const i = doc.sheets.findIndex((s: any) => s.name === name); if (i >= 0) openSheet(i); }
</script>

{#snippet paymentControls(r: any, dueLabel = "Mark unpaid")}
  {#if r.category === "Fuel" && r.pricing?.enabled}<span class="settled">Paid automatically</span>{:else}
  {@const b = balance(r)}
  <div class="row-actions"><button disabled={busy || Number(r.amount) === 0} class:settled={b?.status === "Settled"} onclick={() => togglePaid(r)}>{Number(r.amount) === 0 ? "No amount due" : Number(b?.outstanding || 0) > 0 ? "Mark paid" : dueLabel}</button>{#if Number(b?.outstanding) > 0}<button disabled={busy} onclick={() => pay(r)}>Part payment</button>{/if}</div>
{/if}
{/snippet}

<section class="business-panel">
  <div class="section-heading"><div><p class="eyebrow">BUSINESS WORKSPACE · {business.currency}</p><h2>{business.name}</h2><p>{business.period} · {business.country} · Live ledger</p></div>{#if tabVisible("Rollover & year-end")}<button class="primary" disabled={busy} onclick={newMonth}>Start a new month</button>{/if}</div>
  <nav class="subnav" aria-label="Business sections">{#each tabs.filter(tabVisible) as item}<button class:active={tab === item} onclick={() => tab = item}>{item}</button>{/each}</nav>
  {#if tab === "Overview" || tab === "Profit & loss"}
    {#if tab === "Profit & loss"}<div class="panel-tools"><h3>Current period · income, costs and cash</h3><button onclick={() => goSheet("Profit and Loss")}>Open worksheet</button></div>{/if}
    <div class="metric-grid">{#each Object.entries(view.summary) as [key,value]}<button class="metric" onclick={() => tab = ["receivables", "payables"].includes(key) ? "Outstanding balances" : "Transactions"}>{#if visible(overviewSheet,"Current period measure")}<span>{labels[key] || key}</span>{/if}{#if visible(overviewSheet,"Amount")}<strong>{formatted(value)}</strong>{/if}<small>{["receivables", "payables"].includes(key) ? "Closing balance, including earlier periods" : "Current period"}</small></button>{/each}{#if tabVisible(employerTab)}<button class="metric" onclick={() => tab = employerTab}>{#if visible(overviewSheet,"Current period measure")}<span>Employer tax & contributions</span>{/if}{#if visible(overviewSheet,"Amount")}<strong>{formatted(taxes.employer_due)}</strong>{/if}<small>Own obligation, after transaction deductions</small></button>{/if}{#if tabVisible(employeeTab)}<button class="metric" onclick={() => tab = employeeTab}>{#if visible(overviewSheet,"Current period measure")}<span>Employee tax & contributions</span>{/if}{#if visible(overviewSheet,"Amount")}<strong>{formatted(taxes.employee_due)}</strong>{/if}<small>Amounts to remit on behalf of employees</small></button>{/if}</div>
    <p class="notice">Saving transactions, employee settings or attendance immediately updates payroll, tax, contribution and worksheet totals. Tax percentages and fixed amounts are entered by you. Inventory is maintained separately from expense transactions.</p>
  {:else if tab === "Transactions" || tab === "Outstanding balances"}
    <div class="panel-tools"><input aria-label="Filter transactions" placeholder="Search transactions…" bind:value={filter} />{#if tab === "Transactions"}<select aria-label="Transaction category" bind:value={categoryFilter}><option value="All">All transactions</option>{#each categories as c}<option>{c}</option>{/each}</select>{/if}{#if categories.length}<button class="primary" onclick={() => addTransaction()}>+ Add transaction</button>{/if}{#if categoryFilter === "Fuel" || categoryFilter === "Vehicle repairs"}<button onclick={()=>goSheet(categoryFilter === "Fuel" ? "Fuel" : "Vehicle Repairs")}>Open {categoryFilter} worksheet</button>{/if}</div>
    {#if tab === "Transactions"}
      {#each categories as category}
        {@const records = shownRecords.filter((r: any) => r.category === category)}
        {#if records.length}
          <h3>{category}</h3>
          {#if category === "Fuel"}
            <div class="table-scroll"><table aria-label="Fuel purchases" class="fuel-summary">
              <thead><tr><th>Station</th><th>Filled by</th><th>Vehicle registration</th><th>Fuel economy</th><th>Distance travelled</th><th>Total price</th></tr></thead>
              <tbody>{#each records as r (r.id)}
                {@const stats = view.fuel?.[r.id]}
                <tr>
                  <td>{#if r.period === business.period}<button class="record-link" aria-label={"Edit fuel purchase at " + (r.party || "station")} onclick={() => addTransaction(r)}>{r.party || "View purchase"}</button>{:else}{r.party || "—"}{/if}</td>
                  <td>{r.vehicle?.employee || "—"}</td><td>{r.vehicle?.registration || "—"}</td>
                  <td>{stats?.economy ? stats.economy + " " + stats.economyUnit : "—"}</td>
                  <td>{stats?.distance ? stats.distance + " " + stats.distanceUnit : "—"}</td>
                  <td>{formatted(r.amount)}</td>
                </tr>
              {/each}</tbody>
            </table></div>
          {:else}
            {@const table = view.tables?.[categorySheets[category]]}
            {#if table}<div class="table-scroll"><table aria-label={category + " transactions"}>
              <thead><tr>{#each table.headers as header}<th>{header}</th>{/each}<th>Actions</th></tr></thead>
              <tbody>{#each records as r (r.id)}<tr>
                {#each table.headers as header, i}<td>{table.rows[r.id]?.[i] || "—"}</td>{/each}
                <td>{@render paymentControls(r)}{#if r.period === business.period}<div class="row-actions"><button onclick={() => addTransaction(r)}>Edit</button><button class="danger-text" disabled={busy} onclick={() => removeTransaction(r)}>Delete</button></div>{/if}</td>
              </tr>{/each}</tbody>
            </table></div>{/if}
          {/if}
        {/if}
      {/each}
    {:else}
      {@const columns = ["Source period","Category","Party","Amount","Paid","Remaining","Due date","Status","Paid date","Invoice number"].filter(c => visible("Outstanding Balances",c))}
      <div class="table-scroll"><table><thead><tr>{#each columns as c}<th>{c}</th>{/each}<th>Actions</th></tr></thead><tbody>
        {#each shownRecords as r}{@const b = balance(r)}{@const fields: Record<string,string> = {"Source period":r.period,Category:r.category,Party:r.party,Amount:formatted(r.amount),Paid:formatted(b?.paid),Remaining:formatted(b?.outstanding),"Due date":r.due,Status:b?.status,"Paid date":b?.paid_date,"Invoice number":r.invoice}}
        <tr>{#each columns as c}<td>{fields[c]}</td>{/each}<td>{@render paymentControls(r)}</td></tr>{/each}
      </tbody></table></div>
    {/if}
    {#if !shownRecords.length}<div class="empty"><h3>{tab === "Transactions" ? "No transactions yet" : "No outstanding balances"}</h3><p>Income, expenses, assets and refunds belong here. Tax and payroll obligations have their own menus.</p></div>{/if}
  {:else if tab === "Payroll"}
    <div class="panel-tools"><p>Expand an employee to enter attendance. Every saved day updates the entire workbook.</p><button class="primary" onclick={() => addEmployee()}>+ Add employee</button></div><p class="notice">Only Active employees carry into a new month. Resigned and Fired employees remain in this month’s records. Unmarked days do not deduct monthly salary; enter agreed monthly overtime as additional earnings.</p>
    {#each metadata.employees as e,i}{@const p = view.payroll[i]}{@const r = payrollRecord(e)}<article class="employee"><div class="employee-heading"><button onclick={() => expanded = expanded === e.id ? "" : e.id}><strong>{expanded === e.id ? "▾" : "▸"} {e.name}</strong><small>{[payColumn("Employee number") && e.number,visible("Payroll","Work site") && e.site,e.role,visible("Payroll","Structure") && e.structure,visible("Payroll","Employment status") && (e.status || "Active")].filter(Boolean).join(" · ")}</small>{#each (payColumn("Employee identifiers") ? e.identifiers || [] : []) as field}<small class="identifier">{field.name}: {field.value}</small>{/each}</button>{#if payColumn("Gross","Gross pay")}<span>Gross<b>{formatted(p?.gross)}</b></span>{/if}{#if payColumn("Net salary","Net salary / employer cost")}<span>Net salary<b>{formatted(p?.net)}</b></span>{/if}{#if payColumn("Employer cost","Net salary / employer cost")}<span>Employer cost<b>{formatted(p?.employer_cost)}</b></span>{/if}<button onclick={() => addEmployee(e)}>Edit employee</button>{#if r && visible("Payroll","Payment status")}<div><small>{Number(r.amount) === 0 ? "No salary due" : balance(r)?.status === "Settled" ? "Paid" : "Unpaid"}{balance(r)?.paid_date ? ` · ${balance(r).paid_date}` : ""}</small>{@render paymentControls(r)}</div>{/if}</div>
      {#if visible("Payroll","Deduction shortfall") && Number(p?.deduction_shortfall) > 0}<p class="form-error">Configured tax and deductions exceed accrued gross pay by {formatted(p.deduction_shortfall)}. Net salary is 0; review these amounts as attendance accrues.</p>{/if}{#if expanded === e.id}{#if attendance("Attendance")}<div class="calendar">{#each Array.from({length:view.days},(_,n) => n+1) as n}{@const d = e.days.find((x: any) => x.date === `${business.period}-${String(n).padStart(2,"0")}`)}<button class:worked={d?.status === "Present"} onclick={() => openDay(e,n)} title={d?.status || "Unmarked"}><b>{n}</b><small>{d?.status || "Unmarked"}</small></button>{/each}</div>{/if}<div class="employee-details">{#if payColumn("Employee tax","Tax") || visible("Payroll","Employer-paid tax")}<p>{#if payColumn("Employee tax","Tax")}Employee tax {formatted(p?.tax)}{/if} {#if visible("Payroll","Employer-paid tax")}Employer-paid tax {formatted(p?.employer_tax)}{/if}</p>{/if}{#each (payColumn("Individual contributions") ? p?.contributions || [] : []) as c}<p><strong>{c.name}</strong> · Employee {formatted(c.employee)} · Employer {formatted(c.employer)} · Total {formatted(c.total)}</p>{/each}{#if payColumn("Employee contributions","Total contributions") || visible("Payroll","Employer contributions")}<p><strong>Total contributions: {formatted(Number(p?.employee_contributions || 0) + Number(p?.employer_contributions || 0))}</strong></p>{/if}</div>{/if}</article>{/each}
    {#if !metadata.employees.length}<div class="empty"><h3>Add your first employee</h3><p>Enter pay, manual tax rules and named contributions, then use the attendance calendar.</p></div>{:else}<div class="contribution-preview">{#if payColumn("Gross","Gross pay")}<span>Total gross <strong>{formatted(total(view.payroll,"gross"))}</strong></span>{/if}{#if payColumn("Net salary","Net salary / employer cost")}<span>Total net salary <strong>{formatted(total(view.payroll,"net"))}</strong></span>{/if}{#if payColumn("Employer cost","Net salary / employer cost")}<span>Total employer cost <strong>{formatted(total(view.payroll,"employer_cost"))}</strong></span>{/if}</div>{/if}
  {:else if tab === employerTab || tab === employeeTab}
    <div class="panel-tools"><div><h3>{tab === employerTab ? "What the employer owes for itself" : "What the employer remits for employees"}</h3><p>{tab === employerTab ? "Employer-borne tax and contributions, with transaction deductions shown separately." : "Employee tax and contributions withheld from gross pay. Employer shares appear in the employer menu."}</p></div><button onclick={() => goSheet(tab === employerTab ? "Employer Tax & Contributions" : "Employee Tax & Contributions")}>Open worksheet</button></div>
    <div class="metric-grid">{#each (tab === employerTab ? ["income","income_deductions","taxable_income","employer_tax","deductions","net_employer_tax","employer_contributions","employer_due","unused_deductions"] : ["employee_tax","employee_contributions","employee_due"]).filter(taxMetric) as key}<div class="metric"><span>{taxLabels[key]}</span><strong>{formatted(taxes[key])}</strong><small>{key === "unused_deductions" ? "Shown for review; not automatically carried forward" : "Current period obligation"}</small></div>{/each}</div>
    {#if tab === employerTab}<div class="panel-tools"><h3>Automatic income tax ranges</h3><button class="primary" onclick={() => addTax()}>+ Add employer tax</button></div>
      {#each metadata.business_taxes || [] as t}<article class="employee"><div class="employee-heading"><strong>{t.name}</strong><span>{t.automatic ? t.bands.length + " income ranges" : "Legacy manual base"}</span><span>Calculated tax<b>{formatted((taxes.rows||[]).find((r:any)=>r.name===t.name && r.kind==="Tax" && r.source===business.name)?.employer)}</b></span><button onclick={()=>addTax(t)}>Edit ranges</button><button disabled={busy} onclick={()=>removeTax(t)}>Delete</button></div></article>{/each}
      {#if visible(taxSheet,"Income deductions") || visible(taxSheet,"Tax credits")}
      <div class="panel-tools"><h3>Additional deductions</h3><button onclick={()=>editDeduction()}>+ Add deduction</button></div>
      <p class="notice">For deductions not already recorded in Transactions. Fixed-only is the default; an optional percentage uses current-period income less refunds.</p>
      {#each (metadata.deductions || []).filter((d:any) => visible(taxSheet,d.target === "Tax" ? "Tax credits" : "Income deductions")) as d}<article class="employee"><div class="employee-heading"><strong>{d.name}</strong><span>{d.target === "Income" ? "Reduces taxable income" : "Reduces tax payable"}</span><span>Deduction<b>{formatted(percentFixed(taxes.income,d.percent,d.fixed))}</b></span><button onclick={()=>editDeduction(d)}>Edit</button><button disabled={busy} onclick={()=>removeDeduction(d.id)}>Delete</button></div></article>{/each}
      {/if}
      <div class="panel-tools"><h3>Company contributions</h3><button class="primary" onclick={() => editCompanyContribution()}>+ Add company contribution</button></div>{#each metadata.business_contributions || [] as c,i}<article class="employee"><div class="employee-heading"><strong>{c.name}</strong><span>Total pool<b>{formatted(c.split_pool ? percentFixed(c.base,c.total_percent,c.total_fixed) : c.base)}</b></span><span>Employer total<b>{formatted(percentFixed(c.split_pool ? percentFixed(c.base,c.total_percent,c.total_fixed) : c.base,c.employer_percent,c.employer_fixed))}</b></span><button onclick={() => editCompanyContribution(c,i)}>Edit contribution</button><button class="danger-text" disabled={busy} onclick={() => removeCompanyContribution(i)}>Delete</button></div></article>{/each}<p class="notice">Transaction tax and deductions are entered on each transaction. Payroll tax paid by the employer and employer contribution shares update here when employee settings or attendance change.</p>
    {:else}<div class="panel-tools"><h3>Employee tax and contribution settings</h3><button onclick={() => addEmployee()}>+ Add employee</button></div>{#each metadata.employees as e,i}{@const p = view.payroll[i]}<article class="employee"><div class="employee-heading"><strong>{e.name}<small>{e.number || "No employee number"} · {e.site}{#each (payColumn("Employee identifiers") ? e.identifiers || [] : []) as field}<span class="identifier">{field.name}: {field.value}</span>{/each}</small></strong><span>Tax withheld<b>{formatted(p?.tax)}</b></span><span>Contributions withheld<b>{formatted(p?.employee_contributions)}</b></span><button onclick={() => addEmployee(e)}>Edit tax & contributions</button></div></article>{/each}
      <div class="panel-tools"><h3>Additional employee items</h3><button onclick={()=>editExtra()}>+ Add employee item</button></div><p class="notice">Additional remittances are separate from payroll calculations and do not change payroll net salary. Use employee settings above for deductions from salary.</p>
      {#each metadata.employee_extras || [] as x}<article class="employee"><div class="employee-heading"><strong>{x.number} · {x.employee}<small>{x.name} · {x.kind}</small></strong><span>{formatted(percentFixed(x.base,x.percent,x.fixed))}</span><button onclick={()=>editExtra(x)}>Edit</button><button disabled={busy} onclick={()=>removeExtra(x.id)}>Delete</button></div></article>{/each}{/if}
    <h3 class="business-section-title">Named tax and contribution breakdown</h3><div class="table-scroll"><table><thead><tr>{#if visible(taxSheet,"Tax or contribution name")}<th>Name</th>{/if}{#if visible(taxSheet,"Source / employee")}<th>Employee / source</th>{/if}{#if visible(taxSheet,"Type")}<th>Type</th>{/if}{#if taxMetric("amount")}<th>{tab === employerTab ? "Employer amount" : "Employee amount"}</th>{/if}{#if tab === employerTab && (visible(taxSheet,"Tax credits") || visible(taxSheet,"Income deductions"))}<th>Deductible / claim</th>{/if}</tr></thead><tbody>{#each taxes.rows || [] as line}{#if line.kind === "Contribution" || Number(tab === employerTab ? line.employer : line.employee) !== 0 || (tab === employerTab && Number(line.deductible) !== 0)}<tr>{#if visible(taxSheet,"Tax or contribution name")}<td>{line.name}</td>{/if}{#if visible(taxSheet,"Source / employee")}<td>{line.source}</td>{/if}{#if visible(taxSheet,"Type")}<td>{line.kind}</td>{/if}{#if taxMetric("amount")}<td>{formatted(tab === employerTab ? line.employer : line.employee)}</td>{/if}{#if tab === employerTab && (visible(taxSheet,"Tax credits") || visible(taxSheet,"Income deductions"))}<td>{visible(taxSheet,line.kind === "Income deduction" ? "Income deductions" : "Tax credits") ? formatted(line.deductible) : ""}</td>{/if}</tr>{/if}{/each}</tbody><tfoot><tr><td colspan="3">Total contribution</td>{#if taxMetric("amount")}<td>{formatted(tab === employerTab ? taxes.employer_contributions : taxes.employee_contributions)}</td>{/if}{#if tab === employerTab && (visible(taxSheet,"Tax credits") || visible(taxSheet,"Income deductions"))}<td></td>{/if}</tr><tr><td colspan="3">Total tax & contributions {tab === employerTab ? "after deductions" : "to remit"}</td>{#if taxMetric("amount")}<td>{formatted(tab === employerTab ? taxes.employer_due : taxes.employee_due)}</td>{/if}{#if tab === employerTab && visible(taxSheet,"Tax credits")}<td>{formatted(taxes.deductions)}</td>{/if}</tr></tfoot></table></div>
    <h3 class="business-section-title">Payment status</h3><div class="table-scroll"><table><thead><tr><th>Recipient / obligation</th><th>Amount</th><th>Paid</th><th>Remaining</th><th>Paid date</th><th>Actions</th></tr></thead><tbody>{#each taxObligations.filter((r: any) => Number(r.amount) > 0 || r.payments.length) as r}{@const b = balance(r)}<tr><td>{r.party}<small>{r.description}</small></td><td>{formatted(r.amount)}</td><td>{formatted(b?.paid)}</td><td>{formatted(b?.outstanding)}</td><td>{b?.paid_date || "—"}</td><td>{@render paymentControls(r)}</td></tr>{/each}</tbody></table></div>
  {:else if tab === "Inventory"}<div class="panel-tools"><p>Stock totals update the Inventory worksheet immediately.</p><button onclick={() => goSheet("Inventory")}>Open worksheet</button><button class="primary" onclick={() => editStock()}>+ Add product</button></div><div class="table-scroll"><table><thead><tr>{#if visible("Inventory","Product")}<th>Product</th>{/if}{#if visible("Inventory","Opening units")}<th>Opening units</th>{/if}{#if visible("Inventory","Received")}<th>Received</th>{/if}{#if visible("Inventory","Sold")}<th>Sold</th>{/if}{#if visible("Inventory","Closing units")}<th>Closing units</th>{/if}{#if visible("Inventory","Unit cost")}<th>Unit cost</th>{/if}{#if visible("Inventory","Stock value")}<th>Stock value</th>{/if}<th>Actions</th></tr></thead><tbody>{#each inventory as s}<tr>{#if visible("Inventory","Product")}<td>{s.product}</td>{/if}{#if visible("Inventory","Opening units")}<td>{s.opening}</td>{/if}{#if visible("Inventory","Received")}<td>{s.received}</td>{/if}{#if visible("Inventory","Sold")}<td>{s.sold}</td>{/if}{#if visible("Inventory","Closing units")}<td>{s.closing}</td>{/if}{#if visible("Inventory","Unit cost")}<td>{formatted(s.cost)}</td>{/if}{#if visible("Inventory","Stock value")}<td>{formatted(s.value)}</td>{/if}<td><div class="row-actions"><button onclick={() => editStock(s)}>Edit</button><button class="danger-text" disabled={busy} onclick={() => saveStock(true,s)}>Delete</button></div></td></tr>{/each}</tbody><tfoot><tr>{#if visible("Inventory","Product")}<td>Total</td>{/if}{#if visible("Inventory","Opening units")}<td>{total(inventory,"opening")}</td>{/if}{#if visible("Inventory","Received")}<td>{total(inventory,"received")}</td>{/if}{#if visible("Inventory","Sold")}<td>{total(inventory,"sold")}</td>{/if}{#if visible("Inventory","Closing units")}<td>{total(inventory,"closing")}</td>{/if}{#if visible("Inventory","Unit cost")}<td></td>{/if}{#if visible("Inventory","Stock value")}<td>{formatted(total(inventory,"value"))}</td>{/if}<td></td></tr></tfoot></table></div>
  {:else if tab === "Payment history"}
    {@const columns = ["Date","Party","Amount","Reference","Method","Notes","Invoice number"].filter(c => visible("Payment History",c))}
    <div class="table-scroll"><table><thead><tr>{#each columns as c}<th>{c}</th>{/each}<th>Correction</th></tr></thead><tbody>
      {#each metadata.records as r}{#each r.payments as p}
        {@const fields: Record<string,string> = {Date:p.date,Party:r.party,Amount:formatted(p.amount),Reference:p.reference,Method:p.method,Notes:p.notes,"Invoice number":r.invoice}}
        <tr>{#each columns as c}<td>{fields[c]}</td>{/each}<td>{#if p.reversal_of}Reversal{:else if r.payments.some((x: any) => x.reversal_of === p.id)}Reversed{:else}<button disabled={busy} onclick={() => reverse(r,p)}>Reverse</button>{/if}</td></tr>
      {/each}{/each}</tbody></table></div>
  {:else if tab === "Settings"}<article class="card"><div class="panel-tools"><h3>Business settings</h3><button disabled={busy} onclick={manageSheets}>Manage worksheets</button><button onclick={editSettings}>Edit business details</button></div><dl class="business-settings">{#each [["Name",business.name],["Country",business.country],["Region",business.region],["Currency",business.currency],["Period",business.period],["Registration",business.registration],["Tax registration",business.tax_registration],["Accounting",business.accounting]] as [label,value]}{#if visible("Settings","Setting")}<dt>{label}</dt>{/if}{#if visible("Settings","Value")}<dd>{value || "—"}</dd>{/if}{/each}</dl><p>Use Start a new month to carry active employees, recurring rules and outstanding balances forward.</p></article>
  {:else if tab === "Rollover & year-end"}<div class="two-column"><article class="card"><h3>Import monthly workbooks</h3><p>Company identity, currency and accounting method must match. Matching records are deduplicated; conflicts stop the import for review.</p><label>Import treatment<select bind:value={rollover}><option value={true}>Opening outstanding balances only</option><option value={false}>Consolidate all transactions</option></select></label><button class="primary" disabled={busy} onclick={importFiles}>Choose up to 12 workbooks</button></article><article class="card"><h3>Period summary</h3><label>From<input type="month" bind:value={periodFrom} /></label><label>Through<input type="month" bind:value={periodTo} /></label><button disabled={busy || !periodFrom || !periodTo} onclick={reportPeriod}>Calculate report</button></article></div>
    {#if report}<div class="metric-grid">{#each Object.entries(report.summary) as [key,value]}<div class="metric"><span>{labels[key] || key}</span><strong>{formatted(value)}</strong></div>{/each}</div>{/if}{#if preview}<article class="card"><h3>Import preview · {preview.count} new records</h3><p>Source periods: {preview.sourcePeriods.join(", ")}</p><div class="table-scroll"><table><thead><tr><th>Party</th><th>Source</th><th>Category</th><th>Amount</th><th>Payments</th></tr></thead><tbody>{#each preview.records as r}<tr><td>{r.party}</td><td>{r.period}</td><td>{r.category}</td><td>{formatted(r.amount)}</td><td>{r.payments.length}</td></tr>{/each}</tbody></table></div><div class="row-actions"><button class="primary" disabled={busy} onclick={commitImport}>Confirm import</button><button onclick={() => preview = null}>Cancel</button></div></article>{/if}
  {:else if tab === "Worksheets"}<button class="primary" disabled={busy} onclick={manageSheets}>Manage worksheets · add a sheet</button><p>Every worksheet is available here. Use the entry menus above to change business data; the linked sheets update after each save.</p><div class="worksheet-grid">{#each doc.sheets as s,i}<button class="card" onclick={() => openSheet(i)}><strong>{s.name}</strong><small>Open worksheet →</small></button>{/each}</div>
  {:else}<article class="card"><h3>Workbook audit trail</h3><p>Workbook ID: {metadata.workbook_id}</p><p>Company ID: {business.company_id}</p>{#each metadata.audit as line}<p class="audit-line">{line}</p>{/each}</article>{/if}
</section>

{#if edit}
  <div class="modal-backdrop"><div tabindex="-1" class="modal business-modal" class:wide={edit === "employee" || edit === "sheets"} role="dialog" aria-modal="true" aria-labelledby="business-dialog-title">
    <header><h2 id="business-dialog-title">{edit === "transaction" ? (transaction.id ? "Edit transaction" : "Add transaction") : titles[edit]}</h2><button aria-label="Close" disabled={busy} onclick={() => edit = ""}>✕</button></header>
    {#if formError}<p class="form-error" role="alert">{formError}</p>{/if}
    {#if edit === "transaction"}<form onsubmit={(e) => { e.preventDefault(); saveTransaction(); }}>
      <label>Category<select bind:value={transaction.category}>{#each categories as c}<option>{c}</option>{/each}</select></label>
      {#if tx(transaction.category === "Fuel" ? "Petrol station" : transaction.category === "Vehicle repairs" ? "Repair shop" : "Customer / recipient")}<label>{transaction.category === "Fuel" ? "Petrol station name" : transaction.category === "Vehicle repairs" ? "Repair shop name" : "Customer or recipient"}<input required bind:value={transaction.party} /></label>{/if}{#if showDescription}<label>Description<input required={transaction.category !== "Fuel"} bind:value={transaction.description} /></label>{/if}{#if tx("Invoice number")}<label>Invoice number<input bind:value={transaction.invoice} /></label>{/if}
      {#if transaction.category === "Fuel" || transaction.category === "Vehicle repairs"}<div class="form-grid">{#if tx("Vehicle registration")}<label>Vehicle registration<input required bind:value={transaction.vehicle.registration} /></label>{/if}{#if tx("Employee")}<label>Employee name<input required bind:value={transaction.vehicle.employee} /></label>{/if}</div>{/if}
      {#if transaction.category === "Fuel"}{#if tx("Volume unit") || tx("Distance unit") || tx("Economy unit")}<label>Fuel measurements<select bind:value={transaction.vehicle.units}><option value="">Country default ({business.country})</option><option value="Metric">Litres · km · km/L</option><option value="US">US gallons · miles · mpg (US)</option><option value="UK">Litres · miles · mpg (Imperial)</option><option value="Imperial">Imperial gallons · miles · mpg (Imperial)</option></select></label>{/if}<div class="form-grid">{#if tx("Price per unit")}<label>Price per {unitInfo().price}<input required inputmode="decimal" bind:value={transaction.vehicle.price_per_litre} /></label>{/if}{#if tx("Fuel quantity")}<label>Total {unitInfo().volume} added<input required inputmode="decimal" bind:value={transaction.vehicle.litres} /></label>{/if}{#if tx("Start odometer") && tx("End odometer")}<label>Start odometer ({unitInfo().distance})<input inputmode="decimal" bind:value={transaction.vehicle.start_odometer} /></label>{/if}{#if tx("Start odometer") && tx("End odometer")}<label>End odometer ({unitInfo().distance})<input inputmode="decimal" bind:value={transaction.vehicle.end_odometer} /></label>{/if}</div>{#if (!tx("Start odometer") || !tx("End odometer")) && (tx("Distance") || tx("Fuel economy"))}<label>Tank distance ({unitInfo().distance})<input type="number" min="0" step="any" value={transaction.vehicle.end_odometer === "" ? "" : fuelDistance} oninput={e => { const distance=e.currentTarget.value; if(distance===""){transaction.vehicle.start_odometer="";transaction.vehicle.end_odometer="";}else{transaction.vehicle.start_odometer ||= "0";transaction.vehicle.end_odometer=String(Number(transaction.vehicle.start_odometer)+Number(distance));} }} /></label>{/if}
      {#if transaction.vehicle.start_odometer !== "" && transaction.vehicle.end_odometer !== ""}<p class="calculation-line">{#if tx("Distance")}Distance: {fuelDistance} {unitInfo().distance}{/if} {#if tx("Fuel economy")}Fuel economy: {fuelEconomy.toFixed(3)} {unitInfo().economy}{/if}</p>{/if}<p class="notice">Consumption assumes comparable full-tank fills. Fuel is marked paid automatically.</p>{/if}
      {#if transaction.category !== "Fuel" && transaction.category !== "Vehicle repairs"}{#if tx(transaction.category === "Income" ? "Gross amount" : "Amount")}<label>Gross amount ({business.currency})<input required inputmode="decimal" bind:value={transaction.pricing.gross} /></label>{/if}{/if}
      {#if (transaction.category === "Income" || transaction.category === "Fuel") && (tx("Discount percent") || tx("Discount fixed") || tx("Discount total"))}
        <details class="entry-options"><summary>Discount · {formatted(draftDiscount)}</summary><div class="form-grid">{#if tx("Discount percent")}<label>Discount percent<input inputmode="decimal" bind:value={transaction.pricing.discount_percent} /></label>{/if}{#if tx("Discount fixed")}<label>Discount fixed amount<input inputmode="decimal" bind:value={transaction.pricing.discount_fixed} /></label>{/if}</div></details>{/if}
      {#if transaction.category === "Vehicle repairs"}
        <div class="form-grid">{#if tx("Part name")}<label>Part name<input bind:value={transaction.vehicle.part_name} /></label>{/if}{#if tx("Part number")}<label>Part number<input bind:value={transaction.vehicle.part_number} /></label>{/if}</div>
        <fieldset><legend>Parts</legend>{#if tx("Parts paid / invoiced")}<label>Total paid for parts<input required inputmode="decimal" bind:value={transaction.vehicle.part_amount} /></label>{/if}<TaxFields visible={tx} vehicle prefix="Part" bind:value={transaction.vehicle.part} base={Number(transaction.vehicle.part_amount||0)} purchase={true} /></fieldset>
        <fieldset><legend>Labour</legend>{#if tx("Labour paid / invoiced")}<label>Total paid for labour<input required inputmode="decimal" bind:value={transaction.vehicle.labour_amount} /></label>{/if}<TaxFields visible={tx} vehicle prefix="Labour" bind:value={transaction.vehicle.labour} base={Number(transaction.vehicle.labour_amount||0)} purchase={true} /></fieldset>
        {#if tx(["Fuel","Vehicle repairs"].includes(transaction.category) ? "Total paid / invoiced" : "Amount")}<label>Total amount paid / invoiced ({business.currency})<input readonly value={transaction.amount} /></label>{/if}
      {:else}{#if tx(["Fuel","Vehicle repairs"].includes(transaction.category) ? "Total paid / invoiced" : "Amount")}<label>Total amount paid / invoiced ({business.currency})<input readonly inputmode="decimal" value={transaction.amount} /></label>{/if}<TaxFields visible={tx} vehicle={transaction.category === "Fuel"} bind:value={transaction.calculation} base={Number(transaction.amount||0)} purchase={transaction.category === "Fuel"} />{/if}
      {#if transaction.category !== "Fuel"}{#if tx("Due date")}<label>Due date<input type="date" bind:value={transaction.due} /></label>{/if}{/if}<div class="business-form-actions">
        {#if transaction.id && transaction.category === "Fuel"}<button type="button" class="danger-text" disabled={busy} onclick={() => removeTransaction(transaction)}>Delete purchase</button>{/if}
        <button class="primary" disabled={busy}>Save transaction</button>
      </div>
    </form>
    {:else if edit === "payment"}<form onsubmit={(e) => { e.preventDefault(); savePayment(); }}><p>{selected.party} · {selected.description}</p><label>Amount ({business.currency})<input required inputmode="decimal" bind:value={payment.amount} /></label><label>Date<input required type="date" bind:value={payment.date} /></label><label>Method<select bind:value={payment.method}><option>Bank transfer</option><option>Cash</option><option>Card</option><option>Other</option></select></label><label>Reference<input bind:value={payment.reference} /></label><label>Notes<textarea bind:value={payment.notes}></textarea></label><button class="primary" disabled={busy}>Record payment</button></form>
    {:else if edit === "employee"}<form onsubmit={(e) => { e.preventDefault(); saveEmployee(); }}>
      <div class="form-grid"><label>Name<input required bind:value={employee.name} /></label>{#if payColumn("Employee number") || visible("Timesheets","Employee number")}<label>Employee number<input required bind:value={employee.number} /></label>{/if}{#if visible("Payroll","Work site") || visible("Timesheets","Work site")}<label>Work site<input bind:value={employee.site} /></label>{/if}<label>Job title<input bind:value={employee.role} /></label>{#if visible("Payroll","Structure")}<label>Pay structure<select bind:value={employee.structure}><option>Hourly</option><option>Daily</option><option>Monthly</option></select></label>{/if}<label>{employee.structure} rate<input required inputmode="decimal" bind:value={employee.rate} /></label><label>Standard daily hours<input inputmode="decimal" bind:value={employee.standard_hours} /></label><label>Overtime multiplier<input inputmode="decimal" bind:value={employee.overtime_multiplier} /></label>{#if visible("Payroll","Employment status")}<label>Employment status<select bind:value={employee.status}><option>Active</option><option>Resigned</option><option>Fired</option></select></label>{/if}</div>
      <p class="notice">Active employees carry into the next month with their pay and contribution settings. Resigned or Fired employees remain in this month but do not carry forward.</p>
      {#if payColumn("Employee identifiers")}<h3>Optional employee identifiers</h3><p>Add a tax number, PSIRA number, or any other named identifier. Values are preserved in full as text.</p>{#each employee.identifiers as field,i}<div class="form-grid"><label>Identifier name<input required placeholder="Tax number / PSIRA number" bind:value={field.name} /></label><label>Full number / value<input required type="text" bind:value={field.value} /></label><button type="button" onclick={()=>employee.identifiers.splice(i,1)}>Remove identifier</button></div>{/each}<button type="button" onclick={()=>employee.identifiers.push({name:"",value:""})}>+ Add identifier</button>
      {/if}
      {#if payColumn("Employee tax","Tax") || visible("Payroll","Employer-paid tax") || visible("Employee Tax & Contributions","Employee amount")}<h3>Manual tax · percent + fixed</h3><div class="form-grid"><label>Tax percent<input inputmode="decimal" bind:value={employee.tax_percent} /></label><label>Tax fixed<input inputmode="decimal" bind:value={employee.manual_tax} /></label></div><label>Who bears this tax?<select bind:value={employee.tax_payer}><option value="Employee">Employee — deducted from gross pay</option><option value="Employer">Employer — added to employer cost</option></select></label><p class="calculation-line">Gross pay preview {formatted(draftGross)} · Tax {formatted(percentFixed(draftGross,employee.tax_percent,employee.manual_tax))}</p>
      {/if}
      {#if payColumn("Individual contributions") || visible("Employee Tax & Contributions","Employee amount")}<h3>Named contributions</h3><p>Enter total fixed + a percentage of gross pay, then split that pool between employee and employer. Optional fixed shares are added after the split.</p>
      {#each employee.contributions as c,i}<ContributionEditor bind:value={employee.contributions[i]} base={draftGross} currency={business.currency} remove={() => employee.contributions.splice(i,1)} />{/each}<button type="button" onclick={() => employee.contributions.push(newContribution())}>+ Custom contribution</button>
      {/if}
      <div class="business-form-actions">{#if metadata.employees.some((e: any) => e.id === employee.id)}<button type="button" class="danger-text" disabled={busy} onclick={() => removeEmployee(employee)}>Delete employee</button>{/if}<button class="primary" disabled={busy}>Save employee</button></div><small>Saving updates payroll, taxes, contributions and worksheets immediately.</small>
    </form>
    {:else if edit === "day"}<form onsubmit={(e) => { e.preventDefault(); saveDay(); }}><h3>{day.date}</h3>{#if attendance("Attendance")}<label>Attendance<select bind:value={day.status}>{#each ["Unmarked","Present","Absent","Paid leave","Unpaid leave","Public holiday","Sick leave","Exception"] as s}<option>{s}</option>{/each}</select></label>{/if}<div class="form-grid">{#if attendance("Ordinary hours")}<label>Ordinary paid hours<input inputmode="decimal" bind:value={day.hours} /></label>{/if}{#if attendance("Overtime hours")}<label>Overtime hours<input inputmode="decimal" bind:value={day.overtime} /></label>{/if}{#if attendance("Additional earnings")}<label>Additional earnings<input inputmode="decimal" bind:value={day.extra} /></label>{/if}{#if attendance("Fixed deduction")}<label>Fixed deduction<input inputmode="decimal" bind:value={day.deduction} /></label>{/if}</div>{#if attendance("Notes")}<label>Explanation / allowance / exception<textarea bind:value={day.note}></textarea></label>{/if}<button class="primary" disabled={busy}>Save day</button></form>
    {:else if edit === "tax"}<form onsubmit={(e)=>{e.preventDefault();saveTax();}}><TaxRanges bind:value={tax} income={Number(taxes.taxable_income||0)} /><button class="primary" disabled={busy}>Save employer tax</button></form>
    {:else if edit === "deduction"}<form onsubmit={(e)=>{e.preventDefault();saveDeduction();}}><label>Description<input required bind:value={deduction.name} /></label><div class="form-grid"><label>Percent of income<input inputmode="decimal" bind:value={deduction.percent} /></label><label>Fixed amount<input inputmode="decimal" bind:value={deduction.fixed} /></label></div><label>Apply to<select bind:value={deduction.target}>{#if visible(taxSheet,"Income deductions")}<option value="Income">Taxable income</option>{/if}{#if visible(taxSheet,"Tax credits")}<option value="Tax">Tax payable (credit)</option>{/if}</select></label><p class="notice">Use 0% for a fixed-only deduction. Do not repeat a deduction already entered on a transaction.</p><button class="primary" disabled={busy}>Save deduction</button></form>
    {:else if edit === "extra"}<form onsubmit={(e)=>{e.preventDefault();saveExtra();}}><div class="form-grid"><label>Employee number<input required bind:value={extra.number} /></label><label>Employee name<input required bind:value={extra.employee} /></label></div><label>Description<input required bind:value={extra.name} /></label><label>Type<select bind:value={extra.kind}><option>Tax</option><option>Contribution</option></select></label><label>Calculation base<input inputmode="decimal" bind:value={extra.base} /></label><div class="form-grid"><label>Percent<input inputmode="decimal" bind:value={extra.percent} /></label><label>Fixed<input inputmode="decimal" bind:value={extra.fixed} /></label></div><p class="notice">This adds a separate remittance. It does not change the employee's payroll tax, contributions or net salary.</p><button class="primary" disabled={busy}>Save employee item</button></form>
    {:else if edit === "contribution"}<form onsubmit={(e) => { e.preventDefault(); saveCompanyContribution(); }}><ContributionEditor bind:value={companyContribution} base={Number(companyContribution.base || 0)} currency={business.currency} company={true} remove={() => contributionIndex < 0 ? edit = "" : removeCompanyContribution(contributionIndex)} /><button class="primary" disabled={busy}>Save contribution</button></form>
    {:else if edit === "stock"}<form onsubmit={(e) => { e.preventDefault(); saveStock(); }}>{#if visible("Inventory","Product")}<label>Product<input required bind:value={stock.product} /></label>{/if}<div class="form-grid">{#if visible("Inventory","Opening units")}<label>Opening units<input inputmode="decimal" bind:value={stock.opening} /></label>{/if}{#if visible("Inventory","Received")}<label>Units received<input inputmode="decimal" bind:value={stock.received} /></label>{/if}{#if visible("Inventory","Sold")}<label>Units sold<input inputmode="decimal" bind:value={stock.sold} /></label>{/if}{#if visible("Inventory","Unit cost")}<label>Unit cost<input inputmode="decimal" bind:value={stock.cost} /></label>{/if}</div><p>{#if visible("Inventory","Closing units")}Closing stock: {Number(stock.opening || 0)+Number(stock.received || 0)-Number(stock.sold || 0)}{/if} {#if visible("Inventory","Stock value")}Value: {formatted((Number(stock.opening || 0)+Number(stock.received || 0)-Number(stock.sold || 0))*Number(stock.cost || 0))}{/if}</p><button class="primary" disabled={busy}>Save product</button></form>
    {:else if edit === "sheets"}<form onsubmit={e=>{e.preventDefault();saveSheets();}}><p>Choose additional worksheets or change which transaction sheets show Description.</p><TemplateBuilder bind:value={templateConfig} catalog={templateCatalog} locked={doc.sheets.map((s:any)=>s.name)} /><button class="primary" disabled={busy}>Save worksheet choices</button></form>
    {:else if edit === "settings"}<form onsubmit={(e) => { e.preventDefault(); saveSettings(); }}><label>Company name<input required bind:value={settings.name} /></label><label>Region<input bind:value={settings.region} /></label><label>Registration<input bind:value={settings.registration} /></label><label>Tax registration<input bind:value={settings.tax_registration} /></label><button class="primary" disabled={busy}>Save settings</button></form>{/if}
  </div></div>
{/if}

