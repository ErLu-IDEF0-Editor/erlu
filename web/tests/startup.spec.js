import {test,expect} from '@playwright/test';
import {ready,call,clickUi} from './helpers.js';

test('a failed WASM download shows an actionable error instead of a blank or endless loading screen',async({page})=>{
 await page.route('**/*_bg.wasm',route=>route.abort('failed'));
 await page.goto('./editor.html?lang=ru');
 await expect(page.locator('#loading-message')).toHaveText('Не удалось запустить редактор.');
 await expect(page.locator('#retry')).toBeVisible();
 await expect(page.locator('#loading-detail')).not.toHaveText('');
 expect(await page.locator('#editor-language option').count()).toBe(6);
});

test('the app language menu lists and selects all six UN languages while rendering keeps running',async({page})=>{
 await ready(page);
 await call(page,'block_update',{id:2,title:'Language menu test'});
 for(const language of ['en','ru','fr','es','ar','zh']){
  await clickUi(page,'menu.language');
  const options=await page.evaluate(()=>Object.keys(window.erluRuntime.uiSnapshot().targets).filter(id=>id.startsWith('language.')).sort());
  expect(options).toEqual(['ar','en','es','fr','ru','zh'].map(code=>`language.${code}`).sort());
  await clickUi(page,`language.${language}`);
  await expect.poll(async()=>(await call(page,'document_state')).response.result.view.language).toBe(language);
  await expect(page.locator('#editor-language')).toHaveValue(language);
 }
 const frame=await page.evaluate(()=>window.erluRuntime.uiSnapshot().frame);
 await page.mouse.move(500,400);
 await expect.poll(()=>page.evaluate(()=>window.erluRuntime.uiSnapshot().frame)).toBeGreaterThan(frame);
});

test('online navigation replaces a stale cached editor and preserves six languages',async({page})=>{
 await ready(page);await page.evaluate(()=>navigator.serviceWorker.ready);
 await page.reload();
 await page.waitForFunction(()=>Boolean(navigator.serviceWorker.controller)&&window.erluRuntime?.actionSpecs().length>30);
 await page.evaluate(async()=>{
  const names=await caches.keys();
  for(const name of names)if(name.startsWith('erlu-')){
   const cache=await caches.open(name);
   for(const path of ['editor.html','editor.html?lang=ru'])await cache.put(new URL(path,location.href),new Response('<h1>Stale editor</h1>',{headers:{'Content-Type':'text/html'}}));
  }
 });
 await page.goto('./editor.html?lang=ru');
 await expect(page.locator('#editor-language option')).toHaveCount(6);
 await page.waitForFunction(()=>window.erluRuntime?.actionSpecs().length>30);
 expect((await call(page,'document_state')).response.result.view.language).toBe('ru');
});
