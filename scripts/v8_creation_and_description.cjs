// Launch the packaged app with an empty DSS_TEST_PROFILE and a loopback CDP port.
const { chromium } = require("playwright-core");
const assert = require("node:assert/strict");

async function run(page) {
  page.setDefaultTimeout(15000);
  const button = (name, root=page) => root.getByRole("button",{name,exact:true});
  const dialog = page.locator("section.modal");
  const editor = page.locator(".business-modal");
  const label = (name, root=page) => root.getByLabel(new RegExp("^"+name.replace(/[.*+?^{}()|[\]\\]/g,"\\$&")));
  const call = (cmd,args={}) => page.evaluate(({cmd,args})=>window.__TAURI_INTERNALS__.invoke(cmd,args),{cmd,args});
  const errors=[]; page.on("pageerror",e=>errors.push(e.message));
  async function newBook(){
    await button("+ New workbook").click();
    await dialog.getByRole("heading",{name:"New workbook",exact:true}).waitFor();
  }
  async function sheet(name) {
    const group=dialog.locator("details").filter({has:page.getByRole("switch",{name:new RegExp("^"+name+"(?:Linked|Spreadsheet|Already included|\\s)")})});
    await group.getByRole("switch").check();
    await group.evaluate(e=>e.open=true);
    return group;
  }
  async function active(name){
    await page.locator(".workbook-tabs > button.active").filter({hasText:name}).waitFor();
  }
  async function documentNamed(name) {
    const entries=await call("list_workbooks");
    const file=entries.find(f=>f.name.includes(name));
    assert(file,name);
    return call("open_workbook",{path:file.path});
  }
  async function viewport(doc,name,row=3,height=1) {
    return (await call("viewport",{id:doc.id,sheet:doc.sheets.findIndex(s=>s.name===name),row,column:1,width:40,height})).cells;
  }
  await newBook();
  assert.equal(await page.getByText(/Offline workspace/i).count(),0);
  const startup=await call("startup");
  for(const template of startup.templates.filter(t=>!["Custom Template","Blank workbook"].includes(t))) {
    assert.equal(await button(template,dialog).count(),1,template);
  }
  await label("Workbook type",dialog).selectOption("School & education");
  await button("Gradebook",dialog).click();
  await active("Gradebook");
  assert.equal(await page.getByTitle("Business workspace",{exact:true}).count(),0);
  assert.equal((await documentNamed("Gradebook")).metadata.business,null);
  console.log("PASS full template catalog and a school workbook from New workbook");

  await newBook(); await button("Build a custom workbook",dialog).click();
  await label("Template name",dialog).fill("Home and school");
  await label("Show worksheets for",dialog).selectOption("Personal & household");
  await sheet("Savings Tracker");
  await label("Show worksheets for",dialog).selectOption("School & education");
  await sheet("Gradebook");
  await button("Create workbook",dialog).click();
  await active("Home and school");
  const personal=await documentNamed("Home and school");
  assert.deepEqual(personal.sheets.map(s=>s.name).sort(),["Gradebook","Savings Tracker"]);
  assert.equal(personal.metadata.business,null);
  assert((await viewport(personal,"Gradebook",4)).some(c=>c.input.startsWith("=")));
  await page.getByLabel("Close workbook",{exact:true}).last().click();
  await page.getByTitle("My workbooks").click();
  await page.locator(".recent button").filter({hasText:"Home and school"}).click(); await active("Home and school");
  assert.equal(await page.getByTitle("Business workspace",{exact:true}).count(),0);
  console.log("PASS custom mixed school/household workbook, formulas, naming, and reopen");

  await newBook(); await button("Build a custom workbook",dialog).click();
  await label("Template name",dialog).fill("Description QA");
  await label("Show worksheets for",dialog).selectOption("Business");
  for (const name of ["Fuel","Expenses","Vehicle Repairs","Assets","Refunds","Income"]) {
    const section=await sheet(name);
    const description=section.getByRole("checkbox",{name:"Description",exact:true});
    assert(await description.isChecked());
    if(name!=="Income")await description.uncheck();
  }
  await button("Continue",dialog).click();
  await button("Create workbook",dialog).click(); await active("Description QA");
  await page.getByRole("heading",{name:"Description QA",exact:true}).waitFor();
  const nav=page.locator(".subnav");
  await button("Transactions",nav).click();
  await label("Transaction category").selectOption("Expense");
  await button("+ Add transaction").click();
  const category=label("Category",editor);
  for(const [kind,count] of [["Expense",0],["Fuel",0],["Vehicle repairs",0],["Other payable",0],["Asset",0],["Refund",0],["Income",1],["Expense",0]]) {
    await category.selectOption(kind);
    assert.equal(await label("Description",editor).count(),count,kind);
  }
  await label("Customer or recipient",editor).fill("Shop");
  await label("Gross amount (ZAR)",editor).fill("125");
  await button("Save transaction",editor).click();
  await editor.waitFor({state:"hidden"});
  await page.getByRole("cell",{name:"Shop",exact:true}).waitFor();
  await button("Edit",page.getByRole("table",{name:"Expense transactions"})).click();
  assert.equal(await label("Description",editor).count(),0);
  await button("Close",editor).click();
  const doc=await documentNamed("Description QA");
  for (const name of ["Expenses","Fuel","Vehicle Repairs","Assets","Refunds","Income"]) {
    const has=(await viewport(doc,name)).some(c=>c.text==="Description");
    assert.equal(has,name==="Income",name);
  }
  await button("Save workbook").click();
  await page.getByText("Saved in My workbooks",{exact:true}).waitFor();
  await page.getByLabel("Close workbook",{exact:true}).last().click();
  await page.getByTitle("My workbooks").click();
  await page.locator(".recent button").filter({hasText:"Description QA"}).click(); await active("Description QA");
  await page.getByTitle("Business workspace",{exact:true}).click();
  await button("Transactions",nav).click();
  await label("Transaction category").selectOption("Expense");
  await button("Edit",page.getByRole("table",{name:"Expense transactions"})).click();
  assert.equal(await label("Description",editor).count(),0);
  await button("Close",editor).click();
  await button("Worksheets",nav).click();
  await button("Manage worksheets · add a sheet").click();
  const fuelOptions=editor.locator("details").filter({has:page.getByRole("switch",{name:/^Fuel.*Already included$/})});
  await fuelOptions.evaluate(e=>e.open=true);
  await fuelOptions.getByRole("checkbox",{name:"Description",exact:true}).check();
  await button("Save worksheet choices",editor).click();
  await editor.waitFor({state:"hidden"});
  await button("Transactions",nav).click(); await label("Transaction category").selectOption("Fuel");
  await button("+ Add transaction").click();
  await label("Description",editor).fill("Morning fill");
  for(const [field,value] of [["Petrol station name","Shell"],["Vehicle registration","XYZ"],["Employee name","Sam"],["Total litres added","50"],["Price per litre","32"]])await label(field,editor).fill(value);
  await button("Save transaction",editor).click();await editor.waitFor({state:"hidden"});
  const reopened=await documentNamed("Description QA");
  const cells=await viewport(reopened,"Fuel",3,2);
  const col=cells.find(c=>c.r===3&&c.text==="Description").c;
  assert.equal(cells.find(c=>c.r===4&&c.c===col).text,"Morning fill");
  assert.equal(await page.getByRole("table",{name:"Fuel purchases"}).getByRole("columnheader").count(),6);
  assert.deepEqual(errors,[]);
  console.log("PASS description switches on add, category change, edit, worksheet, disk reopen, and existing workbook settings");
}
(async()=>{const b=await chromium.connectOverCDP(process.argv[2]||"http://127.0.0.1:9258");try{
 const p=b.contexts().flatMap(c=>c.pages()).find(p=>p.url().includes("tauri"));assert(p); try { await run(p); } catch(error) { console.error((await p.locator("body").innerText()).slice(-2300));console.error(await p.locator("input:invalid").evaluateAll(inputs=>inputs.map(i=>[i.outerHTML,i.validationMessage]))); throw error; }
}finally{await b.close();}})().catch(e=>{console.error(e);process.exitCode=1});
