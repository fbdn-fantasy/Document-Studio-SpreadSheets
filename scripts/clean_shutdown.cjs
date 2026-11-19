const {chromium}=require('playwright-core');
const {spawn}=require('node:child_process');
const path=require('node:path');
(async()=>{
 const browser=await chromium.connectOverCDP('http://127.0.0.1:9224');
 const page=browser.contexts()[0].pages()[0];
 const saved=path.resolve(process.argv[2],'installed-business.xlsx');
 spawn(process.argv[3],[saved],{stdio:'ignore'});
 await page.waitForFunction(()=>!document.querySelector('.workbook-tabs')?.textContent.includes('●'),null,{timeout:10000});
 console.log('All visible synthetic workbooks are saved');
 await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('plugin:window|close',{label:'main'})).catch(e=>{if(!String(e).includes('closed'))throw e;});
 await browser.close().catch(()=>{});
 console.log('Requested normal window close');
})().catch(e=>{console.error(e);process.exit(1);});
