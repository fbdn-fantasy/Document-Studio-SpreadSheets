// Exercise the actual Windows application through its local WebView2 debugger.
// Usage: node scripts/v2_installed_smoke.cjs <synthetic-output-directory> <installed-exe>
// The app must be running with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9224.
const { chromium } = require('playwright-core');
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');

const output = path.resolve(process.argv[2] || 'v2-installed-tests');
const exe = process.argv[3];
assert(exe, 'Pass the installed application executable as argument 3');
fs.mkdirSync(output, { recursive: true });
const runId = Date.now().toString();
const checks = [], errors = [];
let browser, page, syntheticVisible = false;
function pass(name) { checks.push(name); console.log('PASS', name); }
const number = value => Number(String(value).replace(/,/g, ''));

(async () => {
  browser = await chromium.connectOverCDP('http://127.0.0.1:9224');
  page = browser.contexts().flatMap(c => c.pages()).find(p => /tauri(?:\.localhost|:\/\/localhost)/.test(p.url()));
  assert(page, 'A real Tauri application WebView is attached');
  page.setDefaultTimeout(15000);
  page.on('pageerror', e => errors.push(e.message));
  await page.waitForLoadState('domcontentloaded');
  // A prior failed synthetic run can leave its employee/transaction form open.
  // Close that draft before creating the next isolated synthetic workbook.
  const oldBusinessDialog = page.locator('.business-modal').getByRole('button', { name: 'Close', exact: true });
  if (await oldBusinessDialog.count()) await oldBusinessDialog.click();
  const oldGenericDialog = page.getByRole('button', { name: 'Close dialog', exact: true });
  if (await oldGenericDialog.count()) await oldGenericDialog.click();
  const invoke = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args });
  const fixtureBusiness = {
    company_id: `v2-installed-${runId}`, name: 'V2 Synthetic Company', country: 'South Africa',
    region: 'Gauteng', currency: 'ZAR', kind: 'General', period: '2026-11', year_start: 3,
    accounting: 'Accrual', registration: '', tax_registration: '',
  };
  let doc = await invoke('create_workbook', { template: 'General Business', business: fixtureBusiness, sourceId: null });
  const employee = (id, name, rate, status = 'Active') => ({
    id, name, role: 'Synthetic test employee', structure: 'Monthly', rate,
    standard_hours: '8', overtime_multiplier: '1.5', days: [], contributions: [],
    manual_tax: '0', tax_percent: '0', tax_payer: 'Employee', status,
  });
  const ari = employee('v2-ari', 'V2 Ari', '10000');
  ari.manual_tax = '50'; ari.tax_percent = '10';
  ari.contributions.push({ name: 'V2 Pension', total_percent: '10', total_fixed: '200',
    employee_percent: '40', employer_percent: '60', employee_fixed: '25', employer_fixed: '35',
    base: '0', cap: null, split_pool: true });
  const bea = employee('v2-bea', 'V2 Bea', '5000');
  bea.tax_percent = '5'; bea.manual_tax = '25'; bea.tax_payer = 'Employer';
  doc.metadata.employees = [ari, bea, employee('v2-resigned', 'V2 Resigned', '0', 'Resigned'), employee('v2-fired', 'V2 Fired', '0', 'Fired')];
  doc.metadata.business_taxes = [{ id: 'v2-company-tax', name: 'V2 Company tax', base: '10000', percent: '20', fixed: '100' }];
  doc.metadata.business_contributions = [{ name: 'V2 Employer benefit', total_percent: '0', total_fixed: '50',
    employee_percent: '0', employer_percent: '100', employee_fixed: '0', employer_fixed: '25', base: '0', cap: null, split_pool: true }];
  function record(id, category, amount, description, calculation) {
    return { id, revision: 1, period: '2026-11', category, party: 'V2 Synthetic party', description, amount,
      due: '2026-11-30', payments: [], disputed: false, source_workbook: doc.metadata.workbook_id,
      managed_key: '', calculation };
  }
  const noTax = { tax_percent: '0', tax_fixed: '0', deductible_percent: '0', deductible_fixed: '0' };
  doc.metadata.records.push(record('v2-income', 'Income', '1000', 'V2 synthetic income', { ...noTax, tax_percent: '10', tax_fixed: '5' }));
  doc.metadata.records.push(record('v2-expense', 'Expense', '200', 'V2 deductible purchase', { ...noTax, deductible_percent: '10', deductible_fixed: '5' }));
  doc = await invoke('update_business', { id: doc.id, metadata: doc.metadata });
  const view = () => invoke('financial_view', { id: doc.id, from: null, to: null });
  let fv = await view();
  assert.equal(number(fv.payroll[0].net), 8445);
  assert.equal(number(fv.payroll[0].employee_contributions), 505);
  assert.equal(number(fv.payroll[0].employer_contributions), 755);
  assert.equal(number(fv.payroll[1].net), 5000, 'Employer-paid tax must not reduce employee salary');
  assert.equal(number(fv.taxes.employer_tax), 2480);
  assert.equal(number(fv.taxes.deductions), 25);
  assert.equal(number(fv.taxes.employer_due), 3285);
  assert.equal(number(fv.taxes.employee_due), 1555);
  assert(doc.sheets.length >= 14);
  pass('Real backend calculates manual percent-plus-fixed taxes and pooled contribution shares');

  const cell = (sheet, row, column) => invoke('cell', { id: doc.id, sheet, row, column });
  assert.equal(number((await cell(3, 4, 6)).text), 8445);
  assert.equal(number((await cell(1, 5, 5)).text), 1000, 'Income total must evaluate');
  assert.equal(number((await cell(1, 4, 11)).text), 105);
  assert.equal(number((await cell(2, 4, 12)).text), 25);
  const employerGrid = await invoke('print_data', { id: doc.id, sheet: 4, rows: 40, columns: 6 });
  assert(JSON.stringify(employerGrid).includes('V2 Pension'));
  const employeeGrid = await invoke('print_data', { id: doc.id, sheet: 12, rows: 40, columns: 6 });
  assert(JSON.stringify(employeeGrid).includes('V2 Pension'));
  pass('Live sheets contain calculated totals, named contributions, tax and deductible amounts');

  doc = await invoke('save_inventory', { id: doc.id, row: 0, product: 'V2 Widget', opening: '10', received: '4', sold: '3', cost: '12.50', remove: false });
  let inventory = await invoke('inventory_view', { id: doc.id });
  const widget = inventory.rows.find(r => r.product === 'V2 Widget');
  assert.equal(number(widget.closing), 11); assert.equal(number(widget.value), 137.5);
  assert.equal(number((await cell(8, 105, 7)).text), 137.5);
  doc = await invoke('save_inventory', { id: doc.id, row: widget.row, product: 'V2 Widget', opening: '10', received: '4', sold: '4', cost: '12.50', remove: false });
  assert.equal(number((await cell(8, 105, 7)).text), 125);
  doc = await invoke('save_inventory', { id: doc.id, row: widget.row, product: '', opening: '0', received: '0', sold: '0', cost: '0', remove: true });
  inventory = await invoke('inventory_view', { id: doc.id });
  assert(!inventory.rows.some(r => r.product === 'V2 Widget'));
  assert.equal(number((await cell(8, 105, 7)).text), 0);
  pass('Inventory add, edit, remove and worksheet totals use the actual backend');

  const workbook = path.join(output, `v2-installed-${runId}.xlsx`);
  doc = await invoke('save_workbook', { id: doc.id, path: workbook });
  const closeDialog = page.getByRole('button', { name: 'Close dialog', exact: true });
  if (await closeDialog.count()) await closeDialog.click();
  const activation = spawn(exe, [workbook], { stdio: 'ignore', windowsHide: true });
  activation.on('error', e => errors.push(`File activation: ${e.message}`));
  await page.getByRole('button', { name: new RegExp(`v2-installed-${runId}\\.xlsx`) }).waitFor();
  await page.getByTitle('Business workspace', { exact: true }).click();
  await page.getByRole('heading', { name: 'V2 Synthetic Company', exact: true }).waitFor();
  syntheticVisible = true;
  pass('Installed application opens the synthetic workbook through single-instance file activation');

  // UI assertions below intentionally address visible controls instead of invoking their handlers.
  const nav = page.locator('.business-panel .subnav');
  await nav.getByRole('button', { name: 'Transactions', exact: true }).click();
  const incomeRow = page.locator('.business-panel tbody tr').filter({ hasText: 'V2 synthetic income' });
  await incomeRow.getByRole('button', { name: /Mark paid/i }).click();
  await incomeRow.getByRole('button', { name: /Mark (unpaid|due)/i }).waitFor();
  fv = await view();
  let balance = fv.balances.find(b => b.id === 'v2-income');
  assert.equal(number(balance.paid), 1000);
  const today = await page.evaluate(() => { const d = new Date(); return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`; });
  assert.equal(balance.paid_date, today);
  assert.equal((await cell(1, 4, 10)).text, today);
  await incomeRow.getByRole('button', { name: /Mark (unpaid|due)/i }).click();
  await incomeRow.getByRole('button', { name: /Mark paid/i }).waitFor();
  fv = await view(); balance = fv.balances.find(b => b.id === 'v2-income');
  assert.equal(number(balance.paid), 0); assert.equal(balance.paid_date, '');
  assert.equal((await cell(1, 4, 10)).text, '');
  doc = await invoke('save_workbook', { id: doc.id, path: workbook });
  const history = doc.metadata.records.find(r => r.id === 'v2-income').payments;
  assert.equal(history.length, 2); assert.equal(history[1].reversal_of, history[0].id);
  pass('Visible paid/unpaid controls stamp the payment date and preserve reversal history');
  await incomeRow.getByRole('button', { name: 'Edit', exact: true }).click();
  assert.equal(await page.getByLabel('Tax percent', { exact: true }).inputValue(), '10');
  assert.equal(await page.getByLabel('Tax fixed', { exact: true }).inputValue(), '5');
  assert.equal(await page.getByLabel('Deductible percent', { exact: true }).inputValue(), '0');
  assert.equal(await page.getByLabel('Deductible fixed', { exact: true }).inputValue(), '0');
  assert(!(await page.getByRole('combobox', { name: /^Category/ }).locator('option').allTextContents()).includes('Tax'));
  await page.screenshot({ path: path.join(output, 'v2-transaction-inputs.png'), fullPage: true });
  await page.getByRole('button', { name: 'Save transaction', exact: true }).click();
  await page.locator('.modal-backdrop').waitFor({ state: 'hidden' });
  pass('Transaction form retains tax and deductible percent/fixed inputs without a standalone tax category');

  await nav.getByRole('button', { name: /Employer.*tax/i }).click();
  await page.getByText('V2 Company tax', { exact: true }).first().waitFor();
  await page.getByText('V2 Pension', { exact: true }).first().waitFor();
  await page.screenshot({ path: path.join(output, 'v2-employer-taxes.png'), fullPage: true });
  await page.getByRole('button', { name: '+ Add employer tax', exact: true }).click();
  await page.getByLabel('Tax name', { exact: true }).fill('V2 UI tax');
  await page.getByLabel('Taxable base (ZAR)', { exact: true }).fill('1000');
  await page.getByLabel('Percent', { exact: true }).fill('2');
  await page.getByLabel('Fixed', { exact: true }).fill('5');
  await page.getByRole('button', { name: 'Save employer tax', exact: true }).click();
  await page.locator('.modal-backdrop').waitFor({ state: 'hidden' });
  assert.equal(number((await view()).taxes.employer_due), 3310);
  const temporaryTax = page.locator('.business-panel tbody tr').filter({ hasText: 'V2 UI tax' }).filter({ has: page.getByRole('button', { name: 'Delete', exact: true }) });
  await temporaryTax.getByRole('button', { name: 'Delete', exact: true }).click();
  await temporaryTax.waitFor({ state: 'hidden' });
  assert.equal(number((await view()).taxes.employer_due), 3285);
  pass('Employer tax form adds and removes percent-plus-fixed obligations live');
  await nav.getByRole('button', { name: /Employee.*tax/i }).click();
  await page.getByText('V2 Pension', { exact: true }).first().waitFor();
  await page.screenshot({ path: path.join(output, 'v2-employee-taxes.png'), fullPage: true });
  pass('Employer and employee tax menus expose named contribution details separately');

  await nav.getByRole('button', { name: 'Worksheets', exact: true }).click();
  assert.equal(await page.locator('.worksheet-grid button').count(), doc.sheets.length);
  await page.locator('.worksheet-grid button').filter({ hasText: 'Employee Tax & Contributions' }).click();
  await page.getByRole('grid', { name: 'Spreadsheet', exact: true }).waitFor();
  await page.screenshot({ path: path.join(output, 'v2-tax-worksheet.png') });
  await page.getByTitle('Business workspace', { exact: true }).click();
  pass('Every worksheet has a menu shortcut and the employee tax sheet opens in the grid');

  await nav.getByRole('button', { name: 'Payroll', exact: true }).click();
  await page.getByRole('button', { name: '+ Add employee', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('V2 UI Employee');
  await page.getByRole('combobox', { name: /^Pay structure/ }).selectOption('Hourly');
  await page.getByLabel('Hourly rate', { exact: true }).fill('12');
  await page.getByRole('button', { name: 'Save employee', exact: true }).click();
  await page.getByRole('button', { name: /V2 UI Employee/ }).first().waitFor();
  await page.getByRole('button', { name: /V2 UI Employee/ }).first().click();
  assert.equal(await page.locator('.calendar button').count(), 30);
  await page.locator('.calendar button').first().click();
  await page.getByRole('combobox', { name: /^Attendance/ }).selectOption('Present');
  await page.getByLabel('Ordinary paid hours', { exact: true }).fill('8');
  await page.getByRole('button', { name: 'Save day', exact: true }).click();
  await page.locator('.modal-backdrop').waitFor({ state: 'hidden' });
  fv = await view();
  assert.equal(number(fv.payroll.at(-1).gross), 96);
  assert.equal(number((await cell(3, 8, 3)).text), 96);
  assert.equal(number((await cell(13, 4, 4)).text), 8);
  await page.screenshot({ path: path.join(output, 'v2-payroll.png'), fullPage: true });
  pass('Employee form and attendance save update actual payroll and Timesheets immediately');
  const uiEmployee = page.locator('.employee').filter({ hasText: 'V2 UI Employee' });
  await uiEmployee.getByRole('button', { name: 'Edit employee', exact: true }).click();
  await page.getByRole('button', { name: '+ Custom contribution', exact: true }).click();
  await page.getByLabel('Contribution name', { exact: true }).fill('V2 Temporary contribution');
  await page.getByLabel('Total fixed', { exact: true }).fill('20');
  await page.getByLabel('Total percent', { exact: true }).fill('10');
  await page.getByLabel('Employee percent', { exact: true }).fill('40');
  await page.getByLabel('Employer percent', { exact: true }).fill('60');
  await page.getByLabel('Employee fixed', { exact: true }).fill('1');
  await page.getByLabel('Employer fixed', { exact: true }).fill('2');
  await page.screenshot({ path: path.join(output, 'v2-contribution-inputs.png'), fullPage: true });
  await page.getByRole('button', { name: 'Save employee', exact: true }).click();
  await page.locator('.modal-backdrop').waitFor({ state: 'hidden' });
  fv = await view();
  assert.equal(number(fv.payroll.at(-1).employee_contributions), 12.84);
  assert.equal(number(fv.payroll.at(-1).employer_contributions), 19.76);
  assert.equal(number(fv.payroll.at(-1).net), 83.16);
  let namedSheet = await invoke('print_data', { id: doc.id, sheet: 12, rows: 50, columns: 6 });
  assert(JSON.stringify(namedSheet).includes('V2 Temporary contribution'));
  await uiEmployee.getByRole('button', { name: 'Edit employee', exact: true }).click();
  await page.getByRole('button', { name: 'Remove contribution', exact: true }).click();
  await page.getByRole('button', { name: 'Save employee', exact: true }).click();
  await page.locator('.modal-backdrop').waitFor({ state: 'hidden' });
  fv = await view();
  assert.equal(number(fv.payroll.at(-1).net), 96);
  namedSheet = await invoke('print_data', { id: doc.id, sheet: 12, rows: 50, columns: 6 });
  assert(!JSON.stringify(namedSheet).includes('V2 Temporary contribution'));
  pass('All six contribution inputs calculate correctly and deleting a contribution removes stale worksheet rows');

  doc = await invoke('save_workbook', { id: doc.id, path: workbook });
  const next = await invoke('create_workbook', { template: 'General Business', business: { ...fixtureBusiness, period: '2026-12' }, sourceId: doc.id });
  assert.deepEqual(next.metadata.employees.map(e => e.name).sort(), ['V2 Ari', 'V2 Bea', 'V2 UI Employee']);
  assert(next.metadata.employees.every(e => e.days.length === 0));
  assert.equal(next.metadata.business_taxes.length, 1);
  assert.equal(next.metadata.business_contributions.length, 1);
  assert(next.metadata.records.some(r => r.id === 'v2-income'), 'Outstanding income rolls into the next month');
  await invoke('close_workbook', { id: next.id });
  pass('New month retains active staff and their configured taxes/contributions, excluding resigned and fired staff');

  for (const extension of ['xlsx', 'ods']) {
    const saved = path.join(output, `v2-saved-${runId}.${extension}`);
    const reloaded = path.join(output, `v2-reopened-${runId}.${extension}`);
    await invoke('save_workbook', { id: doc.id, path: saved });
    fs.copyFileSync(saved, reloaded);
    const loaded = await invoke('open_workbook', { path: reloaded });
    const loadedView = await invoke('financial_view', { id: loaded.id, from: null, to: null });
    assert.equal(number(loadedView.taxes.employer_due), 3285);
    assert.equal(number(loadedView.taxes.employee_due), 1555);
    assert.equal(loaded.metadata.records.find(r => r.id === 'v2-income').payments.length, 2);
    assert.equal(number((await invoke('cell', { id: loaded.id, sheet: 1, row: 5, column: 5 })).text), 1000);
    assert(loaded.sheets.length >= 14);
    await invoke('close_workbook', { id: loaded.id });
    pass(`${extension.toUpperCase()} save/reopen preserves live calculations, worksheets and payment history`);
  }
  await invoke('save_workbook', { id: doc.id, path: workbook });
  assert.deepEqual(errors, []);
  pass('No JavaScript runtime errors during all exercised workflows');
})().catch(async e => {
  errors.push(e.stack || String(e));
  console.error(e);
  if (page && syntheticVisible) await page.screenshot({ path: path.join(output, 'v2-failure.png'), fullPage: true }).catch(() => {});
  process.exitCode = 1;
}).finally(async () => {
  fs.writeFileSync(path.join(output, 'v2-installed-test-results.json'), JSON.stringify({ checks, errors, installedExecutable: exe, completedAt: new Date().toISOString() }, null, 2));
  if (browser) await browser.close();
});
