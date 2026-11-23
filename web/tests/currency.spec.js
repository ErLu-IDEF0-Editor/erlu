import {test,expect} from '@playwright/test';
import {ready,call} from './helpers.js';
test('currency menu shows names while models keep only the selected symbol',async({page})=>{
 await ready(page);
 expect((await call(page,'block_update',{id:2,title:'Costed function',price:12.5})).ok).toBe(true);
 await page.waitForTimeout(300);
 for(const [row,symbol] of [[150,'€'],[287,'$'],[389,'¤']]) {
  await page.waitForTimeout(250);
  await page.mouse.dblclick(640,70,{delay:100});await page.waitForTimeout(250);
  await page.mouse.click(140,68);await page.waitForTimeout(200);
  await page.mouse.click(460,100);await page.waitForTimeout(200);
  await page.screenshot({path:`test-results/currency-menu-${symbol.codePointAt(0)}.png`});
  await page.mouse.click(540,row);await page.waitForTimeout(150);
  await page.mouse.click(40,187);
  await expect.poll(async()=>(await call(page,'document_state')).response.result.document.meta.currency).toBe(symbol);
  const svg=(await call(page,'export_svg',{light:true})).response.result.text;
  expect(svg).toContain(`12.50${symbol}`);
  expect(svg).not.toContain('Canadian dollar');
  expect(svg).not.toContain('Conventional units');
 }
});
