// Run with an isolated, initially empty DSS_TEST_PROFILE and a loopback CDP endpoint.
const { chromium } = require("playwright-core");
const assert = require("node:assert/strict");

async function run(browser) {
  const page = browser.contexts().flatMap(c => c.pages()).find(p => p.url().includes("tauri"));
  assert(page, "Expected the running desktop app");
  page.setDefaultTimeout(20000);
  const failures = [];
  page.on("pageerror", error => failures.push(error.message));
  const button = name => page.getByRole("button", {name,exact:true});
  const library = page.getByTitle("My workbooks", {exact:true});
  const formula = page.getByLabel("Formula or cell contents", {exact:true});
  await button("+ New workbook").click();
  await page.locator("section.modal").getByRole("button",{name:"Blank workbook",exact:true}).click();
  await formula.fill("Manual save");
  await button("Save workbook").click();
  await page.getByText("Saved in My workbooks", {exact:true}).waitFor();
  await page.getByLabel("Close workbook",{exact:true}).click();
  await library.click();
  await page.locator(".recent button").first().click();
  await page.getByTitle("Manual save",{exact:true}).first().waitFor();
  await formula.fill("Autosaved in background");
  await formula.press("Enter");
  await page.getByTitle("Autosaved in background", {exact:true}).first().waitFor();
  await button("+ New workbook").click();
  await page.locator("section.modal").getByRole("button",{name:"Blank workbook",exact:true}).click();
  await page.locator(".workbook-tabs > button.active").filter({hasText:"(2).xlsx"}).waitFor();
  await formula.fill("Autosaved draft");
  // Use the real one-minute timer. The second cell remains in edit mode.
  await page.waitForFunction(() => document.body.innerText.includes("Autosaved at "), null, {timeout:75000});
  await page.waitForFunction(() => !Array.from(document.querySelectorAll(".workbook-tabs > button")).some(b=>b.textContent.includes("●")));
  for (const remaining of [1,0]) {
    await page.getByLabel("Close workbook",{exact:true}).first().click();
    await page.waitForFunction(n => document.querySelectorAll('[aria-label="Close workbook"]').length === n, remaining);
  }
  await library.click();
  const entries = page.locator(".recent button");
  await entries.nth(1).waitFor();
  assert.equal(await entries.count(),2);
  for(const [pattern,value] of [[/\(2\)\.xlsx/,"Autosaved draft"],[/Blank workbook\.xlsx/,"Autosaved in background"]]) {
    await entries.filter({hasText:pattern}).click();
    await page.getByTitle(value,{exact:true}).first().waitFor();
    await library.click();
  }
  console.log("PASS actual minute timer saved two workbooks, including the active draft; sidebar reopened both");
  await page.getByTitle("Home and templates").click();
  await page.locator(".template-card").filter({has:page.getByText("Custom Template",{exact:true})}).click();
  await page.getByLabel("Template name",{exact:true}).fill("Fleet QA template");
  async function configure(name, disabled) {
    const item=page.locator(".template-options details").filter({has:page.getByRole("switch",{name:new RegExp("^"+name+".*Linked$")})});
    await item.getByRole("switch").first().check();
    await item.evaluate(el=>el.open=true);
    if (name === "Fuel" || name === "Income") {
      for (const label of ["Tax", "Deductibles", "Discount"]) assert.equal(await item.getByRole("checkbox", {name:label,exact:true}).count(), 1);
      assert.equal(await item.getByRole("checkbox",{name:/tax percent|deductible fixed|discount percent|discount total/i}).count(),0);
    }
    for(const label of disabled) await item.getByRole("checkbox",{name:label,exact:true}).uncheck();
  }
  await configure("Fuel",["Tax","Deductibles"]);
  await configure("Income",["Deductibles","Invoice number"]);
  await configure("Inventory",["Unit cost","Stock value"]);
  await button("Continue").click();
  await page.getByLabel("Workbook name",{exact:true}).fill("Fleet QA");
  await button("Create workbook").click();
  await page.getByRole("heading",{name:"Fleet QA",exact:true}).waitFor();
  const nav = page.locator(".subnav");
  await nav.getByRole("button",{name:"Transactions",exact:true}).click();
  const modal = page.locator(".business-modal");
  const field = name => modal.getByLabel(name,{exact:true});
  await page.getByLabel("Transaction category",{exact:true}).selectOption("Fuel");
  await button("+ Add transaction").click();
  const fuelForm = await modal.innerText();
  assert(!/\btax\b|deductible|deduction/i.test(fuelForm), fuelForm);
  for(const [label,value] of [["Petrol station name","Test station"],["Description","Test tank"],["Vehicle registration","TEST"],["Employee name","Driver"],["Price per litre","20"],["Total litres added","30"],["Start odometer (km)","1000"],["End odometer (km)","1300"]]) await field(label).fill(value);
  await modal.locator("summary").filter({hasText:"Discount"}).click();
  await field("Discount percent").fill("5");
  await field("Discount fixed amount").fill("10");
  assert.equal(await field("Total amount paid / invoiced (ZAR)").inputValue(),"560.00");
  await button("Save transaction").click();
  await modal.waitFor({state:"hidden"});
  await page.getByRole("cell",{name:/^300(?:\.0+)? km$/}).waitFor();
  await page.getByRole("cell",{name:/^10(?:\.0+)? km\/L$/}).waitFor();
  const table = page.getByRole("table",{name:"Fuel purchases"});
  const expected = ["Station","Filled by","Vehicle registration","Fuel economy","Distance travelled","Total price"];
  assert.deepEqual(await table.getByRole("columnheader").allTextContents(), expected);
  let row = table.locator("tbody tr").first();
  assert.match(await row.innerText(), /560[.,]00/);
  assert.equal(await row.getByRole("cell").count(), 6);
  await row.getByRole("button",{name:"Edit fuel purchase at Test station"}).click();
  assert.equal(await field("Total amount paid / invoiced (ZAR)").inputValue(),"560.00");
  await modal.getByRole("button",{name:"Close",exact:true}).click();
  await page.getByLabel("Transaction category",{exact:true}).selectOption("All");
  assert.deepEqual(await table.getByRole("columnheader").allTextContents(),expected);
  await page.getByLabel("Transaction category",{exact:true}).selectOption("Income");
  await button("+ Add transaction").click();
  assert.equal(await field("Tax fixed").count(),1);
  assert.equal(await field("Tax percent").count(),1);
  assert.equal(await field("Deductible percent").count(),0);
  assert.equal(await field("Deductible fixed").count(),0);
  assert.equal(await field("Invoice number").count(),0);
  await field("Customer or recipient").fill("Customer");
  await field("Description").fill("Service");
  await field("Gross amount (ZAR)").fill("100");
  await field("Tax percent").fill("10");
  await field("Tax fixed").fill("2");
  await button("Save transaction").click();
  await modal.waitFor({state:"hidden"});
  const incomeTable = page.getByRole("table",{name:"Income transactions"});
  await incomeTable.getByRole("cell",{name:"Customer",exact:true}).waitFor();
  const headers = await incomeTable.getByRole("columnheader").allTextContents();
  assert(headers.includes("Tax"));
  assert(!headers.some(h=>/discount|deduct|tax percent|tax fixed/i.test(h)));
  assert.equal(Number(await incomeTable.locator("tbody tr").first().getByRole("cell").nth(headers.indexOf("Tax")).innerText()),12);
  const invoke=(cmd,args={})=>page.evaluate(({cmd,args})=>window.__TAURI_INTERNALS__.invoke(cmd,args),{cmd,args});
  const entriesOnDisk = await invoke("list_workbooks");
  const stored=await invoke("open_workbook",{path:entriesOnDisk.find(item=>item.name.includes("Fleet QA")).path});
  const sheet = stored.sheets.findIndex(sh=>sh.name==="Fuel");
  const viewport=await invoke("viewport",{id:stored.id,sheet,row:3,column:1,width:40,height:2});
  const sheetCells=viewport.cells.filter(c=>c.r===3);
  for (const [header,value] of [["Discount percent",5],["Discount fixed",10],["Discount total",40],["Total paid / invoiced",560],["Outstanding",0]]) {
    const col=sheetCells.find(c=>c.text===header)?.c;
    assert(col,header); assert.equal(Number(viewport.cells.find(c=>c.r===4 && c.c===col).text.replaceAll(",","")),value);
  }
  assert(!sheetCells.some(c=>/tax|deduct/i.test(c.text)));
  console.log("PASS six fuel fields, paid price after a combined discount, independent switches, transaction tax total and detailed worksheet");
  await nav.getByRole("button",{name:"Inventory",exact:true}).click();
  assert.equal(await page.getByRole("columnheader",{name:"Unit cost",exact:true}).count(),0);
  await button("+ Add product").click();
  assert.equal(await field("Unit cost").count(),0);
  await modal.getByRole("button",{name:"Close",exact:true}).click();
  await button("Save workbook").click();
  await page.getByText("Saved in My workbooks",{exact:true}).waitFor();
  assert.deepEqual(failures,[]);
  const standard=await invoke("create_workbook",{template:"General Business",business:{...stored.metadata.business,name:"Standard fleet",company_id:"standard-fleet-qa"}});
  const standardMeta=standard.metadata;
  const fill=structuredClone(stored.metadata.records.find(r=>r.category==="Fuel"));
  fill.id="standard-fuel";
  fill.payments=[];
  fill.source_workbook=standardMeta.workbook_id;
  standardMeta.records.push(fill);
  await invoke("update_business",{id:standard.id,metadata:standardMeta});
  await library.click();
  await page.locator(".recent button").filter({hasText:"Standard fleet"}).click();
  await page.locator(".workbook-tabs > button.active").filter({hasText:"Standard fleet"}).waitFor();
  await page.getByTitle("Business workspace",{exact:true}).click();
  await page.getByRole("heading",{name:"Standard fleet",exact:true}).waitFor();
  await nav.getByRole("button",{name:"Transactions",exact:true}).click();
  await table.getByRole("cell",{name:/^300(?:\.0+)? km$/}).waitFor();
  assert.deepEqual(await table.getByRole("columnheader").allTextContents(),expected);
  console.log("PASS six fuel fields in the standard workbook's All transactions view");
  await page.getByTitle("Home and templates").click();
  await page.evaluate(()=>localStorage.setItem("dss-custom-templates",JSON.stringify({"Earlier selection":{name:"Earlier selection",sheets:{Fuel:{enabled:true,columns:{"Tax percent":false,"Tax included":true,"Tax fixed":true}}}}})));
  await page.reload();
  await page.locator(".template-card").filter({has:page.getByText("Saved: Earlier selection",{exact:true})}).click();
  await page.getByLabel("Template name",{exact:true}).waitFor();
  const fuel = page.locator(".template-options details").filter({has:page.getByRole("switch",{name:/^Fuel.*Linked$/})});
  await fuel.evaluate(el=>el.open=true);
  const mixed = fuel.getByRole("checkbox",{name:/^Tax Partly selected$/});
  assert(await mixed.evaluate(el=>el.indeterminate));
  await mixed.click();
  assert(await fuel.getByRole("checkbox",{name:"Tax",exact:true}).isChecked());
  assert(await fuel.getByRole("checkbox",{name:"Deductibles",exact:true}).isChecked());
  console.log("PASS older partial selections and inventory switches");
}

(async()=>{
  const browser=await chromium.connectOverCDP(process.argv[2] || "http://127.0.0.1:9246");
  try { await run(browser); } finally { await browser.close(); }
})().catch(error=>{ console.error(error); process.exitCode=1; });

