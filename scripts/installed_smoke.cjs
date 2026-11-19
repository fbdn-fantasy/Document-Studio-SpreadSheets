// Windows-only integration tests against the real installed Tauri WebView.
// Launch the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9224
const { chromium } = require('playwright-core');
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');

(async()=>{
 const output=path.resolve(process.argv[2]);const exe=process.argv[3];fs.mkdirSync(output,{recursive:true});
 const browser=await chromium.connectOverCDP('http://127.0.0.1:9224');
 const page=browser.contexts()[0].pages().find(p=>p.url().includes('tauri.localhost'));
 assert(page,'Installed Tauri page is present');page.setDefaultTimeout(10000);
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const checks=[];const pass=name=>{checks.push(name);console.log('PASS',name);};
 const invoke=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
 await page.waitForLoadState('domcontentloaded');pass('Installed application startup and rendered interface');
 if(await page.getByRole('button',{name:'Close dialog',exact:true}).count())await page.getByRole('button',{name:'Close dialog',exact:true}).click();const startup=await invoke('startup');
 const savedEarlier=path.join(output,'installed-smoke.xlsx');const recovered=startup.recovery[0]||await invoke('open_workbook',{path:savedEarlier});if(startup.recovery.length){await page.getByRole('button',{name:'Restore',exact:true}).first().click();}else{spawn(exe,[savedEarlier],{stdio:'ignore'});}
 await page.getByRole('grid',{name:'Spreadsheet',exact:true}).waitFor();
 if(startup.recovery.length){assert.equal((await invoke('cell',{id:recovered.id,sheet:0,row:1,column:1})).text,'60');pass('Crash recovery restores the real unsaved formula');}
 await page.getByRole('textbox',{name:'Formula or cell contents',exact:true}).fill('=SUM(7,8,9)');
 await page.getByRole('textbox',{name:'Formula or cell contents',exact:true}).press('Enter');
 await page.waitForFunction(()=>[...document.querySelectorAll('[role=gridcell]')].some(e=>e.textContent==='24'));
 assert.equal((await invoke('cell',{id:recovered.id,sheet:0,row:1,column:1})).text,'24');pass('UI formula editing invokes Rust and displays calculated result');
 const saved=path.join(output,'installed-smoke.xlsx');await invoke('save_workbook',{id:recovered.id,path:saved});assert(fs.existsSync(saved));pass('Installed backend saves a genuine XLSX');
 spawn(exe,[saved],{detached:false,stdio:'ignore'});
 await page.getByRole('button',{name:/installed-smoke.xlsx/}).waitFor();pass('Single-instance file activation opens the saved workbook');
 if(await page.getByRole('button',{name:'Simple mode ▾',exact:true}).count())await page.getByRole('button',{name:'Simple mode ▾',exact:true}).click();await page.getByRole('button',{name:'Advanced mode ▾',exact:true}).waitFor();pass('Simple/Advanced mode switch');
 await page.getByTitle('Preferences',{exact:true}).click();await page.locator('.modal label').filter({hasText:/^Theme/}).locator('select').selectOption('light');assert.equal(await page.locator('html').getAttribute('data-theme'),'light');await page.locator('.modal label').filter({hasText:/^Theme/}).locator('select').selectOption('plum');await page.getByRole('button',{name:'Close dialog',exact:true}).click();pass('Light/plum theme preference controls');
 await page.screenshot({path:path.join(output,'installed-spreadsheet.png')});
 const b={company_id:'ui-verification',name:'Synthetic UI Company',country:'South Africa',region:'Gauteng',currency:'ZAR',kind:'General',period:'2026-11',year_start:3,accounting:'Accrual',registration:'',tax_registration:''};
 let business=await invoke('create_workbook',{template:'General Business',business:b});
 business.metadata.records.push({id:'synthetic-invoice',revision:1,period:'2026-11',category:'Income',party:'Synthetic customer',description:'Integration test',amount:'10000',due:'2026-11-30',payments:[{id:'synthetic-receipt',date:'2026-11-12',amount:'4000',reference:'TEST',method:'Test',notes:'No real payment',reversal_of:null}],disputed:false,source_workbook:business.metadata.workbook_id});
 business=await invoke('update_business',{id:business.id,metadata:business.metadata});
 let fv=await invoke('financial_view',{id:business.id,from:null,to:null});assert.equal(fv.summary.receivables,'6000');assert.equal(fv.summary.revenue,'10000');pass('Installed ledger preserves decimal partial receipt and balance');
 const bp=path.join(output,'installed-business.xlsx');await invoke('save_workbook',{id:business.id,path:bp});spawn(exe,[bp],{stdio:'ignore'});await page.getByRole('button',{name:/installed-business.xlsx/}).waitFor();await page.getByTitle('Business workspace',{exact:true}).click();await page.getByRole('heading',{name:'Synthetic UI Company',exact:true}).waitFor();pass('Business overview renders real ledger data');
 await page.getByRole('button',{name:'Payroll',exact:true}).click();await page.getByRole('button',{name:'+ Add employee',exact:true}).click();await page.getByLabel('Name',{exact:true}).fill('Synthetic Employee');await page.getByLabel('Monthly rate',{exact:true}).fill('10000');await page.getByRole('button',{name:'Save employee',exact:true}).click();await page.getByRole('button',{name:/Synthetic Employee/}).first().waitFor();await page.getByRole('button',{name:/Synthetic Employee/}).first().click();assert.equal(await page.locator('.calendar button').count(),30);pass('Employee UI creates payroll and the correct November calendar');
 await page.screenshot({path:path.join(output,'installed-payroll.png')});
 await invoke('save_workbook',{id:business.id,path:bp});
 assert.deepEqual(errors,[]);pass('No JavaScript runtime errors during exercised workflows');
 fs.writeFileSync(path.join(output,'installed-ui-tests.json'),JSON.stringify({checks,errors,installedExecutable:exe},null,2));
 await browser.close();
})().catch(e=>{console.error(e);process.exit(1);});
