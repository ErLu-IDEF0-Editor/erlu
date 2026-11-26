import {test,expect} from '@playwright/test';
import {ready,call,clickUi} from './helpers.js';
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
 await clickUi(page,'menu.ai');
 await clickUi(page,'ai.enabled');
 await clickUi(page,'ai.origin');
 await page.keyboard.press('ControlOrMeta+A');
 await page.keyboard.insertText('http://127.0.0.1:8080');
 await expect.poll(()=>page.evaluate(()=>window.erluRuntime.connectionStatus())).toEqual({enabled:true,origin:'http://127.0.0.1:8080'});
 await page.mouse.click(600,200);
 const result=await page.evaluate(async()=>{const {connect}=await import('./sdk/erlu-agent.js');window.testAgent=await connect(window,location.origin,{timeout:3000});const specs=await window.testAgent.actions();const state=await window.testAgent.call('document_state');return{count:specs.actions.length,revision:state.revision};});expect(result.count).toBeGreaterThan(30);
 await clickUi(page,'menu.ai');
 await clickUi(page,'ai.enabled');
 await expect.poll(()=>page.evaluate(()=>window.erluRuntime.connectionStatus().enabled)).toBe(false);
 await page.mouse.click(600,200);
 const revoked=await page.evaluate(async()=>{try{await window.testAgent.call('document_state');return false;}catch{return true;}});expect(revoked).toBe(true);
});
test('offline reload uses precached wasm assets',async({page,context})=>{
 await ready(page);await page.evaluate(()=>navigator.serviceWorker.ready);await context.setOffline(true);await page.reload();await page.waitForFunction(()=>window.erluRuntime?.actionSpecs?.()?.length>30);await context.setOffline(false);
});

test('GUI ports create arrows and middle-click opens inherited interfaces',async({page})=>{
 await ready(page);await call(page,'block_update',{id:2,title:'Process'});await page.waitForTimeout(400);
 const originalBlock=(await call(page,'object_get',{id:2})).response.result.object.pos;
 // Headed engines may resize the real canvas beyond the requested viewport.
 const bounds=await page.locator('#erlu').boundingBox();expect(bounds).not.toBeNull();
 const pixels=await page.evaluate(()=>{const c=document.getElementById('erlu');return{width:c.width,height:c.height,dpr:devicePixelRatio};});
 expect(pixels.width/pixels.dpr).toBeCloseTo(bounds.width,0);
 expect(pixels.height/pixels.dpr).toBeCloseTo(bounds.height,0);
 const scale=Math.min((bounds.width-56)/297,(bounds.height-106)/210);
 const at=(x,y)=>({x:bounds.x+bounds.width/2+x*scale,y:bounds.y+bounds.height/2+y*scale});
 async function drag(a,b){await page.mouse.move(a.x,a.y);await page.mouse.down();await page.mouse.move(b.x,b.y,{steps:12});await page.mouse.up();await page.waitForTimeout(250);}
 await drag(at(-148.5,4.2),at(-26.5,4.2));await drag(at(26.5,4.2),at(148.5,4.2));
 let state=(await call(page,'document_state')).response.result;
 expect(Object.keys(state.document.diagrams['1'].arrows)).toHaveLength(2);
 expect(state.document.diagrams['1'].blocks['2'].pos).toEqual(originalBlock);
 const center=at(0,4.2);await page.mouse.click(center.x,center.y,{button:'middle'});await page.waitForTimeout(250);
 state=(await call(page,'document_state')).response.result;expect(state.view.diagram).not.toBe(1);
 expect(Object.values(state.document.diagrams[state.view.diagram].arrows).filter(a=>a.interface?.stub)).toHaveLength(2);
 await call(page,'block_create',{pos:{x:-30,y:0},title:'Subprocess'});await page.waitForTimeout(250);
 await drag(at(-148.5,4.2),at(-30,13.25));await drag(at(23,13.25),at(148.5,4.2));
 state=(await call(page,'document_state')).response.result;
 expect(Object.values(state.document.diagrams[state.view.diagram].arrows).filter(a=>a.interface?.stub)).toHaveLength(0);
 await page.keyboard.press('Control+ArrowUp');await page.waitForTimeout(250);expect((await call(page,'document_state')).response.result.view.diagram).toBe(1);
});

test('Responsibility Map uses the same WASM component for RACI, exports and undo',async({page})=>{
 await ready(page);
 await page.waitForFunction(()=>window.erluRuntime.actionSpecs().some(s=>s.name==='plugin_responsibility_map_matrix_export'));
 const plugin=(action,args={})=>call(page,`plugin_responsibility_map_${action}`,args);
 await call(page,'selection_set',{ids:[2]});
 for(const name of ['Operations','Manager'])expect((await plugin('role_add',{name})).ok).toBe(true);
 expect((await plugin('assignment_set',{role:'Operations',responsibility:'R'})).ok).toBe(true);
 expect((await plugin('assignment_set',{role:'Manager',responsibility:'A'})).ok).toBe(true);
 expect((await plugin('validate')).response.result.valid).toBe(true);
 const revision=(await call(page,'document_state')).response.revision;
 const matrix=await plugin('matrix_export',{format:'csv',scope:'document'});
 expect(matrix.ok,matrix.error?.message).toBe(true);
 expect(matrix.response.result.text).toContain('Operations');
 expect(matrix.response.result.filename).toBe('responsibility-map.csv');
 expect((await call(page,'document_state')).response.revision).toBe(revision);
 expect((await plugin('assignment_set',{role:'Operations',responsibility:'A'})).ok).toBe(true);
 expect((await plugin('validate')).response.result.diagnostics.some(d=>d.code==='multiple_accountable')).toBe(true);
 await call(page,'undo');expect((await plugin('validate')).response.result.valid).toBe(true);
 expect((await plugin('role_rename',{name:'Manager',new_name:'Owner'})).ok).toBe(true);
 const serialized=(await call(page,'document_serialize')).response.result.json;
 expect((await call(page,'document_open',{json:serialized,discard:true})).ok).toBe(true);
 expect((await plugin('validate')).response.result.valid).toBe(true);
 await call(page,'selection_set',{ids:[2]});
 const result=await plugin('matrix_export',{format:'json',scope:'selection'});
 expect(JSON.parse(result.response.result.text).roles.some(role=>role.name==='Owner')).toBe(true);
});

test('Semantic Dictionary previews inherited label changes and imports portable catalogs',async({page})=>{
 await ready(page);
 await page.waitForFunction(()=>window.erluRuntime.actionSpecs().some(s=>s.name==='plugin_semantic_dictionary_normalize_preview'));
 const dictionary=(action,args={},revision)=>call(page,`plugin_semantic_dictionary_${action}`,args,revision);
 const ep=(id,port)=>({anchor:{kind:'block',id,port,t:.5}});
 const boundary=port=>({anchor:{kind:'boundary',port,t:.5}});
 const created=await call(page,'arrow_create',{from:boundary('input'),to:ep(2,'input'),label:'req'});expect(created.ok).toBe(true);
 const source=created.response.result.id;
 await call(page,'decompose',{block:2});
 const a=(await call(page,'block_create',{pos:{x:-60,y:0},title:'Receive request'})).response.result.id;
 const b=(await call(page,'block_create',{pos:{x:20,y:0},title:'Process request'})).response.result.id;
 let state=(await call(page,'document_state')).response;
 const diagram=state.result.view.diagram;
 const inherited=Object.values(state.result.document.diagrams[diagram].arrows).find(a=>a.interface).id;
 expect((await call(page,'interface_connect',{id:inherited,endpoint:ep(a,'input')})).ok).toBe(true);
 const branch=await call(page,'arrow_branch',{id:inherited,point:{x:-80,y:0},target:ep(b,'input')});expect(branch.ok).toBe(true);
 const forward=branch.response.result.forward;
 const independent=(await call(page,'arrow_create',{from:ep(b,'output'),to:boundary('output'),label:'Other result'})).response.result.id;
 expect((await dictionary('concept_add',{code:'request',term:'Заявка клиента',definition:'Запрос клиента'})).ok).toBe(true);
 expect((await dictionary('alias_add',{code:'request',alias:'req'})).ok).toBe(true);
 await call(page,'selection_set',{ids:[inherited]});expect((await dictionary('binding_set',{code:'request'})).ok).toBe(true);
 const before=(await call(page,'document_state')).response;
 const preview=await dictionary('normalize_preview',{scope:'selection'});expect(preview.ok,preview.error?.message).toBe(true);
 expect(preview.response.result.changes[0].root).toBe(source);
 expect(preview.response.result.changes[0].uses.some(use=>use.arrow===forward)).toBe(true);
 expect((await call(page,'document_state')).response.revision).toBe(before.revision);
 const stale=preview.response.result.confirm_action;
 await dictionary('concept_update',{code:'request',field:'owner',value:'Operations'});
 expect((await call(page,stale.name,stale.arguments,stale.expected_revision)).error.code).toBe('revision_conflict');
 const fresh=await dictionary('normalize_preview',{scope:'selection'});const confirm=fresh.response.result.confirm_action;
 await call(page,'selection_set',{ids:[independent]});
 expect((await call(page,confirm.name,confirm.arguments,confirm.expected_revision)).ok).toBe(true);
 state=(await call(page,'document_state')).response;
 expect(state.result.document.diagrams['1'].arrows[source].label).toBe('Заявка клиента');
 expect(state.result.document.diagrams[diagram].arrows[forward].label).toBe('Заявка клиента');
 expect(state.result.document.diagrams[diagram].arrows[forward].label_source).toBe(inherited);
 expect(state.result.document.diagrams[diagram].arrows[independent].label).toBe('Other result');
 await call(page,'undo');expect((await call(page,'document_state')).response.result.document.diagrams['1'].arrows[source].label).toBe('req');
 await call(page,'redo');
 const serialized=(await call(page,'document_serialize')).response.result.json;
 await call(page,'document_open',{json:serialized,discard:true});
 const usage=await dictionary('usage',{code:'request'});expect(usage.response.result.uses.length).toBeGreaterThan(1);
 const exported=await dictionary('catalog_export',{format:'json'});const catalog=JSON.parse(exported.response.result.text);
 expect(catalog.bindings).toBeUndefined();
 await call(page,'document_new',{discard:true});
 const imported=await dictionary('catalog_import_preview',{catalog});expect(imported.ok,imported.error?.message).toBe(true);
 const apply=imported.response.result.confirm_action;
 expect((await call(page,apply.name,apply.arguments,apply.expected_revision)).ok).toBe(true);
 const restored=(await call(page,'document_state')).response.result.document.extensions.semantic_dictionary.data;
 expect(restored.bindings).toEqual({});
 expect(Object.values(restored.concepts)[0].term).toBe('Заявка клиента');
});

test('six interface languages preserve document data and localize browser recovery',async({page})=>{
 await ready(page);
 await call(page,'block_update',{id:2,title:'Model — نموذج — 模型'});
 const original=(await call(page,'document_state')).response.result.document;
 for(const [language,save] of [['en','Save'],['ru','Сохранить'],['fr','Enregistrer'],['es','Guardar'],['ar','حفظ'],['zh','保存']]) {
  expect((await call(page,'language_set',{language})).ok).toBe(true);
  await expect.poll(()=>page.locator('html').getAttribute('lang')).toBe(language);
  expect(await page.evaluate(()=>window.wasmBindings?window.erluRuntime.translate('Save'):null)).toBe(save);
  expect((await call(page,'document_state')).response.result.document).toEqual(original);
  await page.screenshot({path:`test-results/interface-${language}.png`});
 }
 await page.waitForTimeout(400);
 await page.reload();await page.waitForFunction(()=>window.erluRuntime?.actionSpecs?.()?.length>30);
 await expect(page.locator('#recovery')).toBeVisible();
 await call(page,'language_set',{language:'ar'});
 await expect(page.locator('#recovery')).toHaveAttribute('dir','rtl');
 await expect(page.locator('#recovery')).toContainText('يتوفر مستند غير محفوظ.');
 await expect(page.locator('#recovery button').first()).toHaveText('استعادة');
 await call(page,'language_set',{language:'zh'});
 await expect(page.locator('#recovery button').first()).toHaveText('恢复');
});
