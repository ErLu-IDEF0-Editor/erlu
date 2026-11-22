import {test,expect} from '@playwright/test';
async function ready(page){await page.goto('/');await page.waitForFunction(()=>window.erluRuntime?.actionSpecs?.()?.length>30);}
async function call(page,action,args={},revision){return page.evaluate(async({action,args,revision})=>window.erluRuntime.call(action,args,revision),{action,args,revision});}
test('browser engine edits, decomposition, history and PDF',async({page})=>{
 const errors=[];page.on('pageerror',e=>errors.push(e.message));await ready(page);
 const initial=await call(page,'document_state');expect(initial.ok).toBe(true);expect(initial.response.result.document.diagrams['1'].blocks['2'].number).toBe('A0');
 const child=await call(page,'decompose',{block:2},initial.response.revision);expect(child.ok).toBe(true);
 const b=await call(page,'block_create',{pos:{x:0,y:0},title:'Моделировать процесс'});expect(b.ok).toBe(true);
 await call(page,'navigate_up');await call(page,'undo');
 const doc=await call(page,'document_state');expect(Object.keys(doc.response.result.document.diagrams[child.response.result.diagram].blocks)).toHaveLength(0);
 const pdf=await call(page,'export_pdf',{all:true,number_pages:true,light:true});expect(pdf.ok).toBe(true);expect(String.fromCharCode(...pdf.response.result.bytes.slice(0,4))).toBe('%PDF');expect(errors).toEqual([]);
});
test('same component plugin applies a single undoable transaction',async({page})=>{
 await ready(page);await page.waitForFunction(()=>window.erluRuntime.actionSpecs().some(s=>s.name==='plugin_colors_colors_set'));
 await call(page,'selection_set',{ids:[2]});const result=await call(page,'plugin_colors_colors_set',{color:'#00ff00'});expect(result.ok,result.error?.message).toBe(true);
 expect((await call(page,'object_get',{id:2})).response.result.object.color).toBe('#00ff00');await call(page,'undo');expect((await call(page,'object_get',{id:2})).response.result.object.color).toBeNull();
});
test('stale revisions reject edits, optional future fields roundtrip',async({page})=>{
 await ready(page);const first=await call(page,'block_create',{pos:{x:30,y:0},title:'Work'},0);expect(first.ok).toBe(true);expect((await call(page,'objects_delete',{ids:[2]},0)).error.code).toBe('revision_conflict');
 const state=(await call(page,'document_state')).response;const doc=state.result.document;doc.future={keep:[1,2]};doc.format_version.minor=8;doc.features.optional.push('future');await call(page,'document_open',{json:JSON.stringify(doc),discard:true});expect(JSON.parse((await call(page,'document_serialize')).response.result.json).future).toEqual({keep:[1,2]});
});
test('GUI right-click creates a block and displays the document',async({page})=>{
 await ready(page);await call(page,'block_update',{id:2,title:'Process'});await page.waitForTimeout(400);
 const before=Object.keys((await call(page,'document_state')).response.result.document.diagrams['1'].blocks).length;
 await page.mouse.click(900,600,{button:'right'});await page.waitForTimeout(300);
 const after=Object.keys((await call(page,'document_state')).response.result.document.diagrams['1'].blocks).length;
 expect(after).toBe(before+1);await page.screenshot({path:'test-results/editor.png'});
});
test('AI bridge requires opt-in, origin, and supports revocation',async({page})=>{
 await ready(page);
 const disabled=await page.evaluate(()=>new Promise(resolve=>{const c=new MessageChannel();let responded=false;c.port1.onmessage=()=>{responded=true;};window.postMessage({protocol:'erlu.ai.v1',type:'connect'},location.origin,[c.port2]);setTimeout(()=>{c.port1.close();resolve(responded);},250);}));expect(disabled).toBe(false);
 await call(page,'block_update',{id:2,title:'Process'});await page.waitForTimeout(250);
 await page.mouse.click(234,11);await page.waitForTimeout(200);await page.mouse.click(238,39);await page.waitForTimeout(150);await page.mouse.click(360,80);await page.keyboard.type('http://127.0.0.1:8080',{delay:25});await page.mouse.click(600,200);await page.waitForTimeout(250);
 const result=await page.evaluate(async()=>{const {connect}=await import('./sdk/erlu-agent.js');window.testAgent=await connect(window,location.origin,{timeout:3000});const specs=await window.testAgent.actions();const state=await window.testAgent.call('document_state');return{count:specs.actions.length,revision:state.revision};});expect(result.count).toBeGreaterThan(30);
 await page.mouse.click(234,11);await page.waitForTimeout(200);await page.mouse.click(238,39);await page.waitForTimeout(150);await page.mouse.click(600,200);await page.waitForTimeout(250);
 const revoked=await page.evaluate(async()=>{try{await window.testAgent.call('document_state');return false;}catch{return true;}});expect(revoked).toBe(true);
});
test('offline reload uses precached wasm assets',async({page,context})=>{
 await ready(page);await page.evaluate(()=>navigator.serviceWorker.ready);await context.setOffline(true);await page.reload();await page.waitForFunction(()=>window.erluRuntime?.actionSpecs?.()?.length>30);await context.setOffline(false);
});

test('GUI ports create arrows and middle-click opens inherited interfaces',async({page})=>{
 await ready(page);await call(page,'block_update',{id:2,title:'Process'});await page.waitForTimeout(400);
 const scale=(900-24-24-16-40)/210;
 const at=(x,y)=>({x:640+x*scale,y:450+y*scale});
 async function drag(a,b){await page.mouse.move(a.x,a.y);await page.mouse.down();await page.mouse.move(b.x,b.y,{steps:12});await page.mouse.up();await page.waitForTimeout(250);}
 await drag(at(-148.5,4.2),at(-26.5,4.2));await drag(at(26.5,4.2),at(148.5,4.2));
 let state=(await call(page,'document_state')).response.result;
 expect(Object.keys(state.document.diagrams['1'].arrows)).toHaveLength(2);
 await page.mouse.click(640,450,{button:'middle'});await page.waitForTimeout(250);
 state=(await call(page,'document_state')).response.result;expect(state.view.diagram).not.toBe(1);
 expect(Object.values(state.document.diagrams[state.view.diagram].arrows).filter(a=>a.interface?.stub)).toHaveLength(2);
 await call(page,'block_create',{pos:{x:-30,y:0},title:'Subprocess'});await page.waitForTimeout(250);
 await drag(at(-148.5,4.2),at(-30,13.25));await drag(at(23,13.25),at(148.5,4.2));
 state=(await call(page,'document_state')).response.result;
 expect(Object.values(state.document.diagrams[state.view.diagram].arrows).filter(a=>a.interface?.stub)).toHaveLength(0);
 await page.keyboard.press('Control+ArrowUp');await page.waitForTimeout(250);expect((await call(page,'document_state')).response.result.view.diagram).toBe(1);
});
