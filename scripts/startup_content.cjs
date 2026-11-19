const {chromium}=require('playwright-core');
const assert=require('node:assert/strict');
(async()=>{
 const browser=await chromium.connectOverCDP('http://127.0.0.1:9224');
 const page=browser.contexts().flatMap(c=>c.pages()).find(p=>/tauri/.test(p.url()));
 assert(page,'Application webview exists');
 const info=await page.evaluate(()=>({text:document.body.innerText,theme:document.documentElement.dataset.theme}));
 assert(info.text.includes('Ready · all files stay on this computer'), 'Initialized status rendered when visible');
 assert(info.text.includes('General Business'),'Templates loaded when visible');
 assert(info.theme,'Theme applied');
 console.log('PASS initialized content and theme at first visible window');
 await browser.close();
})().catch(e=>{console.error(e);process.exit(1)});
