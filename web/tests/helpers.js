export async function ready(page) {
 const errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
 await page.goto('./editor.html');
 try {
  await page.waitForFunction(()=>window.erluStartupError||(window.erluReady&&window.erluRuntime?.actionSpecs?.()?.length>30),null,{timeout:30000});
  const failure=await page.evaluate(()=>window.erluStartupError);
  if(failure)throw new Error(failure);
 } catch(error) {
  const details=await page.evaluate(()=>{
   const canvas=document.createElement('canvas');
   return {loading:document.getElementById('loading')?.textContent,webgl2:Boolean(canvas.getContext('webgl2'))};
  }).catch(()=>({}));
  throw new Error(`Editor startup failed: ${JSON.stringify(details)}\n${errors.join('\n')}\n${error.message}`);
 }
}
export async function call(page,action,args={},revision) {
 return page.evaluate(async({action,args,revision})=>window.erluRuntime.call(action,args,revision),{action,args,revision});
}

// Read-only egui geometry; actions still travel through real mouse events.
export async function clickUi(page,id) {
 await expectTarget(page,id);
 const target=await page.evaluate(id=>window.erluRuntime.uiSnapshot().targets[id],id);
 const canvas=await page.locator('canvas').boundingBox();
 await page.mouse.move(canvas.x+target.x+target.width/2,canvas.y+target.y+target.height/2);
 const frame=await page.evaluate(()=>window.erluRuntime.uiSnapshot().frame);
 await page.waitForFunction(frame=>window.erluRuntime.uiSnapshot().frame>frame,frame);
 await expectTarget(page,id);
 const current=await page.evaluate(id=>window.erluRuntime.uiSnapshot().targets[id],id);
 // Without a delay Playwright queues press and release together. Waiting for
 // a rendered frame between them can turn a short click into a >0.8 s hold
 // during shader/font work, which egui correctly rejects as a click.
 await page.mouse.click(canvas.x+current.x+current.width/2,canvas.y+current.y+current.height/2);
 const clicked=await page.evaluate(()=>window.erluRuntime.uiSnapshot().frame);
 await page.waitForFunction(frame=>window.erluRuntime.uiSnapshot().frame>frame,clicked,{timeout:10000});
}
async function expectTarget(page,id) {
 await page.waitForFunction(id=>{
  const target=window.erluRuntime.uiSnapshot().targets[id];
  return target?.enabled&&target.width>0&&target.height>0;
 },id);
}
