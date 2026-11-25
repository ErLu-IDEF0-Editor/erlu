import {test,expect} from '@playwright/test';
import {translations} from '../site/translations.js';

test('homepage supports all six languages, shareable URLs and RTL without overflow',async({page})=>{
 const errors=[];page.on('pageerror',error=>errors.push(error.message));
 for(const [language,text] of Object.entries(translations)){
  await page.goto(`./?lang=${language}`);
  await expect(page.locator('html')).toHaveAttribute('lang',language);
  await expect(page.locator('html')).toHaveAttribute('dir',language==='ar'?'rtl':'ltr');
  await expect(page.locator('h1')).toHaveText(text.heroTitle);
  await expect(page).toHaveTitle(text.title);
  await expect(page.locator('.hero .primary')).toHaveAttribute('href',`./editor.html?lang=${language}`);
  const missing=await page.locator('[data-i18n]').evaluateAll(nodes=>nodes.filter(n=>!n.textContent||n.textContent==='undefined').map(n=>n.dataset.i18n));
  expect(missing).toEqual([]);
  for(const width of [1280,375]){
   await page.setViewportSize({width,height:900});
   expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`${language} at ${width}px`).toBe(true);
  }
 }
 expect(errors).toEqual([]);
});

test('language follows browser preferences, persists selection and accepts an explicit URL override',async({browser,baseURL})=>{
 const context=await browser.newContext({locale:'ru-RU'});
 try{
  const page=await context.newPage();await page.goto(baseURL);
  await expect(page.locator('html')).toHaveAttribute('lang','ru');
  await page.locator('#language').selectOption('fr');
  await expect(page).toHaveURL(/lang=fr/);
  await page.goto(new URL('./',page.url()).href);
  await expect(page.locator('html')).toHaveAttribute('lang','fr');
  await page.goto(new URL('?lang=ar',page.url()).href);
  await expect(page.locator('html')).toHaveAttribute('dir','rtl');
  await page.goto(new URL('?lang=unsupported',page.url()).href);
  await expect(page.locator('html')).toHaveAttribute('lang','fr');
 }finally{await context.close();}
});

test('try online opens the editor with the selected language and all bundled plugins',async({page,context})=>{
 await page.goto('./?lang=ru');
 await page.locator('.hero .primary').click();
 await page.waitForFunction(()=>window.erluRuntime?.actionSpecs().some(s=>s.name==='plugin_semantic_dictionary_concept_add'));
 const state=await page.evaluate(async()=>({state:await window.erluRuntime.call('document_state',{}),actions:window.erluRuntime.actionSpecs().map(s=>s.name)}));
 expect(state.state.response.result.view.language).toBe('ru');
 for(const prefix of ['plugin_colors_','plugin_responsibility_map_','plugin_semantic_dictionary_'])expect(state.actions.some(name=>name.startsWith(prefix))).toBe(true);
 await page.evaluate(()=>navigator.serviceWorker.ready);
 const manifest=await (await page.request.get('./manifest.webmanifest')).json();
 expect(manifest.start_url).toBe('./editor.html');
 await context.setOffline(true);
 await page.reload();
 await page.waitForFunction(()=>window.erluRuntime?.actionSpecs().some(s=>s.name==='plugin_semantic_dictionary_concept_add'));
 await page.goto(new URL('./?lang=ar',page.url()).href);
 await expect(page.locator('html')).toHaveAttribute('dir','rtl');
});
