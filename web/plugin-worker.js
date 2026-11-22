// Only SDK-generated component bindings from this application origin are loaded.
// The WASM component has no WASI imports and receives only a JSON snapshot.
self.onmessage=async event=>{
  try{
    const {module,action,context}=event.data;
    const plugin=await import(module);
    const output=await plugin.run(action,context);
    self.postMessage({ok:true,output:JSON.parse(output)});
  }catch(error){self.postMessage({ok:false,error:String(error)});}
};
