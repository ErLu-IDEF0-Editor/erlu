export async function ready(page) {
 const errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
 await page.goto('./');
 try {
  await page.waitForFunction(()=>window.erluStartupError||window.erluRuntime?.actionSpecs?.()?.length>30,null,{timeout:30000});
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
