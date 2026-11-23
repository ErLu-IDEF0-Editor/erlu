import {test,expect} from '@playwright/test';
import {ready,call,clickUi} from './helpers.js';
test('currency menu shows names while models keep only the selected symbol',async({page})=>{
 await ready(page);
 expect((await call(page,'block_update',{id:2,title:'Costed function',price:12.5})).ok).toBe(true);
 for(const [code,symbol,name] of [['EUR','€','Euro'],['CAD','$','Canadian dollar'],['¤','¤','Conventional units']]) {
  await clickUi(page,'menu.edit');
  await clickUi(page,'menu.properties');
  await clickUi(page,'metadata.tab.1');
  await clickUi(page,'metadata.currency.open');
  const target=`metadata.currency.${code}`;
  await page.waitForFunction(id=>Boolean(window.erluRuntime.uiSnapshot().targets[id]),target);
  const caption=await page.evaluate(id=>window.erluRuntime.uiSnapshot().targets[id].value,target);
  expect(caption).toContain(name);
  if(code!=='¤')expect(caption).toContain(code);
  await clickUi(page,target);
  await expect.poll(()=>page.evaluate(()=>window.erluRuntime.uiSnapshot().targets['metadata.currency.input']?.value)).toBe(symbol);
  await clickUi(page,'metadata.confirm');
  await page.waitForFunction(()=>!window.erluRuntime.uiSnapshot().targets['metadata.confirm']);
  await expect.poll(async()=>(await call(page,'document_state')).response.result.document.meta.currency).toBe(symbol);
  const svg=(await call(page,'export_svg',{light:true})).response.result.text;
  expect(svg).toContain(`12.50${symbol}`);
  expect(svg).not.toContain('Canadian dollar');
  expect(svg).not.toContain('Conventional units');
 }
});
