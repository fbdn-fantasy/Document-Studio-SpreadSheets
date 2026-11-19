const {chromium}=require('playwright-core');
const fs=require('node:fs');const path=require('node:path');const assert=require('node:assert/strict');
(async()=>{
 const output=path.resolve(process.argv[2]);const browser=await chromium.connectOverCDP('http://127.0.0.1:9224');const page=browser.contexts()[0].pages()[0];
 const invoke=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
 await page.getByRole('heading',{name:/Make room for/}).waitFor();
 const startup=await invoke('startup');assert.equal(startup.recovery.length,0);assert.equal(await page.getByText('Unsaved work recovered after an unexpected closure').count(),0);
 assert.equal(startup.settings.mode,'Advanced');assert.equal(startup.settings.theme,'plum');
 const full=path.join(output,'installed-smoke.xlsx');const first=await invoke('open_workbook',{path:full});const alias=await invoke('open_workbook',{path:full.replaceAll('\\','/')});assert.equal(first.id,alias.id);
 const printed=await invoke('print_data',{id:first.id,sheet:0,rows:5,columns:3});assert.equal(printed[0][0],'24');assert.equal(printed.length,5);
 const other=await invoke('create_workbook',{template:'Blank workbook',business:null});let rejected=false;try{await invoke('save_workbook',{id:other.id,path:full});}catch(e){rejected=String(e).includes('already open');}assert(rejected);
 await invoke('close_workbook',{id:other.id});await invoke('close_workbook',{id:first.id});
 const checks=['No recovery prompt after clean shutdown','Theme and interface-mode preferences restored','Alternate file-path spellings reuse the same workbook','Save refuses a destination open in another workbook','Print data contains calculated values and requested area','Installed window close command permitted'];
 fs.writeFileSync(path.join(output,'final-verification.json'),JSON.stringify({checks},null,2));console.log(checks.map(x=>'PASS '+x).join('\n'));
 await invoke('plugin:window|close',{label:'main'}).catch(e=>{if(!String(e).includes('closed'))throw e;});await browser.close().catch(()=>{});
})().catch(e=>{console.error(e);process.exit(1);});
