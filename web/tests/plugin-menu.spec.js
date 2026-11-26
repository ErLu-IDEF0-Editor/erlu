import {test,expect} from '@playwright/test';
import {ready,call,clickUi} from './helpers.js';

for(const [count,title] of [[3,undefined],[4,'External colors'],[4,undefined]])test(`external plugin with ${count} visible actions ${title?'with title':'without title'} uses the appropriate menu`,async({page})=>{
 await ready(page);
 expect((await call(page,'block_update',{id:2,title:'External plugin test'})).ok).toBe(true);
 const id=`external_${count}`;
 const manifest={id,api_version:1,version:'0.1.0',component:'unused.wasm',
  web:new URL('plugins/colors/dist/colors.js',page.url()).href,
  ...(title?{title}:{}),
  actions:Array.from({length:count},(_,i)=>({name:i===0?'colors.clear':`unused.${i}`,title:`External action ${i}`,kind:'command',input_schema:{type:'object',properties:{}}})),
 };
 // Confirmation-only actions must not push a three-item plugin into a submenu.
 manifest.actions.push({name:'confirmation',title:'Hidden confirmation',kind:'command',input_schema:{type:'object','x-erlu-confirm-only':true}});
 await page.route(`**/${id}/plugin.json`,route=>route.fulfill({json:manifest}));
 await clickUi(page,'menu.plugins');
 page.once('dialog',dialog=>dialog.accept(new URL(`${id}/plugin.json`,page.url()).href));
 await clickUi(page,'plugin.load');
 await page.waitForFunction(id=>window.erluRuntime.actionSpecs().some(s=>s.name===`plugin_${id}_colors_clear`),id,{timeout:10000});
 await call(page,'selection_set',{ids:[2]});
 expect((await call(page,'plugin_colors_colors_set',{color:'#00ff00'})).ok).toBe(true);
 await clickUi(page,'menu.plugins');
 const action=`plugin.action.plugin_${id}_colors_clear`;
 if(count===4) {
  await page.waitForFunction(id=>Boolean(window.erluRuntime.uiSnapshot().targets[`plugin.menu.${id}`]),id,{timeout:10000});
  expect(await page.evaluate(id=>window.erluRuntime.uiSnapshot().targets[`plugin.menu.${id}`].value,id)).toBe(title??id);
  expect(await page.evaluate(action=>Boolean(window.erluRuntime.uiSnapshot().targets[action]),action)).toBe(false);
  await clickUi(page,`plugin.menu.${id}`);
 } else {
  await page.waitForFunction(action=>Boolean(window.erluRuntime.uiSnapshot().targets[action]),action,{timeout:10000});
  expect(await page.evaluate(id=>Boolean(window.erluRuntime.uiSnapshot().targets[`plugin.menu.${id}`]),id)).toBe(false);
 }
 expect(await page.evaluate(id=>Boolean(window.erluRuntime.uiSnapshot().targets[`plugin.action.plugin_${id}_confirmation`]),id)).toBe(false);
 await clickUi(page,action);
 await clickUi(page,'plugin.form.run');
 await expect.poll(async()=>(await call(page,'object_get',{id:2})).response.result.object.color).toBeNull();
});

for(const width of [1280,570])test(`bundled plugins use compact submenus at ${width}px`,async({page})=>{
 await page.setViewportSize({width,height:900});
 await ready(page);
 await call(page,'language_set',{language:'ru'});
 await page.waitForFunction(()=>window.erluRuntime.actionSpecs().some(s=>s.name==='plugin_semantic_dictionary_concept_add'));
 // Dismiss startup with a real document change before opening the menus.
 await call(page,'block_update',{id:2,title:'Menu test'});
 await clickUi(page,'menu.plugins');
 for(const id of ['responsibility_map','semantic_dictionary']) {
  await page.waitForFunction(id=>Boolean(window.erluRuntime.uiSnapshot().targets[`plugin.menu.${id}`]),id);
 }
 const action='plugin.action.plugin_semantic_dictionary_concept_add';
 expect(await page.evaluate(action=>Boolean(window.erluRuntime.uiSnapshot().targets[action]),action)).toBe(false);
 await clickUi(page,'plugin.menu.semantic_dictionary');
 await page.waitForFunction(action=>Boolean(window.erluRuntime.uiSnapshot().targets[action]),action);
 expect(await page.evaluate(action=>window.erluRuntime.uiSnapshot().targets[action].value,action)).toBe('Добавить понятие…');
 await clickUi(page,action);
 await page.waitForFunction(()=>Boolean(window.erluRuntime.uiSnapshot().targets['plugin.form.run']));
});
