import {test,expect} from '@playwright/test';
import {ready,call,clickUi} from './helpers.js';
async function coordinates(page){
 const state=(await call(page,'document_state')).response.result;
 const size={A4:[297,210],A3:[420,297]}[state.document.meta.page_size]??[297,210];
 await page.waitForFunction(()=>window.erluRuntime.uiSnapshot().targets.canvas);
 const rect=await page.evaluate(()=>window.erluRuntime.uiSnapshot().targets.canvas);
 const scale=Math.max(.1,Math.min((rect.width-40)/size[0],(rect.height-40)/size[1]))*state.view.zoom;
 return {scale,point:(x,y)=>({x:rect.x+rect.width/2+(x-state.view.center.x)*scale,y:rect.y+rect.height/2+(y-state.view.center.y)*scale})};
}
async function drag(page,start,end){await page.mouse.move(start.x,start.y);await page.mouse.down();await page.mouse.move(end.x,end.y,{steps:12});await page.mouse.up();const frame=await page.evaluate(()=>window.erluRuntime.uiSnapshot().frame);await page.waitForFunction(frame=>window.erluRuntime.uiSnapshot().frame>frame,frame);}

test('near-border GUI connections become exact straight lines and an end-segment drag slides both ports',async({page})=>{
 await ready(page);
 await call(page,'objects_move',{positions:{2:{x:-40,y:0}}});
 const {point}=await coordinates(page);
 await drag(page,point(13,13.25),point(148.5,14.25));
 let doc=(await call(page,'document_state')).response.result.document;
 let arrow=Object.values(doc.diagrams['1'].arrows)[0];expect(arrow).toBeTruthy();
 const originalY=26.5*arrow.from.anchor.t;
 expect(arrow.to.anchor.t).toBeCloseTo((originalY+79.8)/168,8);
 await drag(page,point(70,13.25),point(70,18.25));
 doc=(await call(page,'document_state')).response.result.document;arrow=doc.diagrams['1'].arrows[arrow.id];
 const movedY=26.5*arrow.from.anchor.t;
 expect(Math.abs(movedY-originalY-5)).toBeLessThan(.5);
 expect(arrow.to.anchor.t).toBeCloseTo((movedY+79.8)/168,8);
 expect(arrow.bends).toEqual([]);
 const svg=(await call(page,'export_svg',{light:true})).response.result.text;
 expect(svg).toContain('<svg');
 await call(page,'undo');
 doc=(await call(page,'document_state')).response.result.document;
 expect(doc.diagrams['1'].arrows[arrow.id].from.anchor.t).toBeCloseTo(.5,2);
});

test('grid menu and G shortcut enable snapping without editing the model',async({page})=>{
 await ready(page);await call(page,'block_update',{id:2,title:'Grid test'});
 const before=(await call(page,'document_state')).response;
 await clickUi(page,'menu.view');await clickUi(page,'view.grid');
 await expect.poll(async()=>(await call(page,'document_state')).response.result.view.grid).toEqual({enabled:true,spacing:5});
 let state=(await call(page,'document_state')).response;
 expect(state.revision).toBe(before.revision);expect(state.result.document).toEqual(before.result.document);
 await page.keyboard.press('Escape');
 const {point}=await coordinates(page);
 const old=state.result.document.diagrams['1'].blocks['2'].pos;
 await drag(page,point(old.x+26.5,old.y+13.25),point(old.x+34.1,old.y+20.8));
 state=(await call(page,'document_state')).response;
 const moved=state.result.document.diagrams['1'].blocks['2'].pos;
 expect(moved.x/5).toBeCloseTo(Math.round(moved.x/5),6);expect(moved.y/5).toBeCloseTo(Math.round(moved.y/5),6);
 await page.keyboard.press('g');
 await expect.poll(async()=>(await call(page,'document_state')).response.result.view.grid.enabled).toBe(false);
 await call(page,'undo');
 expect((await call(page,'document_state')).response.result.document).toEqual(before.result.document);
});

test('grid checkbox survives a slow rendered frame without losing a short mouse click',async({page})=>{
 await ready(page);await call(page,'block_update',{id:2,title:'Slow frame test'});
 const before=(await call(page,'document_state')).response;
 await clickUi(page,'menu.view');
 await page.locator('#erlu').evaluate(canvas=>{
  canvas.addEventListener('mousedown',()=>{
   requestAnimationFrame(()=>{
    const until=performance.now()+1100;
    while(performance.now()<until){} // Reproduce a shader/font upload exceeding egui's 0.8 s click limit.
    window.erluSlowFrameFinished=true;
   });
  },{once:true,capture:true});
 });
 await clickUi(page,'view.grid');
 await expect.poll(()=>page.evaluate(()=>window.erluSlowFrameFinished)).toBe(true);
 await expect.poll(async()=>(await call(page,'document_state')).response.result.view.grid.enabled).toBe(true);
 const after=(await call(page,'document_state')).response;
 expect(after.revision).toBe(before.revision);expect(after.result.document).toEqual(before.result.document);
});
