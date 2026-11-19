const {chromium}=require('playwright-core');
(async()=>{
 const browser=await chromium.connectOverCDP('http://127.0.0.1:9224');const page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(10000);
 await page.getByRole('button',{name:'File',exact:true}).click();await page.getByRole('button',{name:'Print preview…',exact:true}).click();
 await page.getByRole('button',{name:'Preview selected worksheet',exact:true}).click();await page.locator('.paper').waitFor();
 if(!(await page.locator('.paper').innerText()).includes('24'))throw Error('Preview missing calculated cell');
 console.log('PASS Print preview displays calculated workbook data');
 await page.getByRole('button',{name:'Print using system dialog',exact:true}).click();
 console.log('Requested native Windows system print dialog; cancel manually after inspection');
 await browser.close();
})().catch(e=>{console.error(e);process.exit(1);});
