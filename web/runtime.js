// Transport and storage adapters. All document edits execute in Rust.
const PROTOCOL = 'erlu.ai.v1';
const MAX_MESSAGE = 4 * 1024 * 1024;
export function createRuntime() {
  let handle, enabled = false, allowedOrigin = '', dirty = false, pendingCheckpoint, checkpointTimer;
  const ports = new Set(), plugins = new Map(), results=[];
  const database = new Promise((resolve, reject) => {
    const request = indexedDB.open('erlu', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('recovery');
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
  database.catch(console.error);
  function failure(code,message) { return {ok:false,error:{code,message}}; }
  function revoke() {for (const port of ports) {port.postMessage({protocol:PROTOCOL,type:'revoked'});port.close();}ports.clear();}
  async function call(name,args={},revision) {
    if (!handle) return failure('not_ready','Editor is not ready');
    if(name==='plugin_list'){const state=JSON.parse(handle.call('document_state','{}',undefined)).response;return {ok:true,response:{revision:state.revision,dirty:state.dirty,result:Array.from(plugins.values(),p=>p.manifest)}};}
    if(name==='plugin_load'){try{await installPlugin(args.source);const state=JSON.parse(handle.call('document_state','{}',undefined)).response;return {ok:true,response:{revision:state.revision,dirty:state.dirty,result:{}}};}catch(error){return failure('plugin_load',String(error));}}
    if (name.startsWith('plugin_')) {
      for (const [id,plugin] of plugins) {
        const spec=plugin.manifest.actions.find(a=>`plugin_${id}_${a.name.replaceAll('.','_')}`===name);
        if(spec) return invokePlugin(id,spec.name,args,undefined,revision);
      }
      return failure('not_found','Plugin action not found');
    }
    return JSON.parse(handle.call(name,JSON.stringify(args),revision===undefined?undefined:BigInt(revision)));
  }
  function checkpoint(json) {
    pendingCheckpoint=json;clearTimeout(checkpointTimer);
    checkpointTimer=setTimeout(()=>flushCheckpoint(),200);
  }
  async function flushCheckpoint(){if(!pendingCheckpoint)return;const value=pendingCheckpoint;pendingCheckpoint=null;try{const db=await database;const tx=db.transaction('recovery','readwrite');tx.objectStore('recovery').put(value,'current');}catch(error){console.error('Recovery unavailable',error);}}
  function configureAI(next,origin) {
    // Require an exact origin. No wildcard, opaque origin, or insecure remote origin.
    let valid=false;
    try{const url=new URL(origin);valid=url.origin===origin&&(url.protocol==='https:'||['localhost','127.0.0.1'].includes(url.hostname));}catch{}
    const newEnabled=Boolean(next&&valid);
    if(enabled!==newEnabled||allowedOrigin!==origin)revoke();enabled=newEnabled;allowedOrigin=origin;
  }
  addEventListener('message',event=>{
    if(!enabled||event.origin!==allowedOrigin||!event.source||event.data?.protocol!==PROTOCOL||event.data?.type!=='connect'||event.ports.length!==1)return;
    const port=event.ports[0],session=crypto.randomUUID();ports.add(port);
    port.onmessage=async message=>{
      const request=message.data;
      if(!ports.has(port)||!enabled||request?.protocol!==PROTOCOL||request.session!==session)return;
      try {
        if(JSON.stringify(request).length>MAX_MESSAGE)throw new Error('Message exceeds 4 MiB');
        const result=request.type==='actions'?{ok:true,response:{actions:actionSpecs()}}:await call(request.action,request.arguments,request.expected_revision);
        if(ports.has(port))port.postMessage({protocol:PROTOCOL,type:'result',id:request.id,...result});
      }catch(error){if(ports.has(port))port.postMessage({protocol:PROTOCOL,type:'result',id:request?.id,...failure('connector_error',String(error))});}
    };
    port.start();port.postMessage({protocol:PROTOCOL,type:'connected',session,version:1});
  });
  function actionSpecs(){if(!handle)return [];return [...JSON.parse(handle.actions()),...Array.from(plugins,([id,p])=>p.manifest.actions.map(a=>({name:`plugin_${id}_${a.name.replaceAll('.','_')}`,description:a.title,input_schema:a.input_schema,read_only:a.kind!=='command'}))).flat()];}
  function workerRequest(plugin,action,context) {
    // A fresh worker prevents persistent untrusted state and makes timeout cancellation reliable.
    return new Promise((resolve,reject)=>{
      const worker=new Worker(new URL('./plugin-worker.js',import.meta.url),{type:'module'});
      const timer=setTimeout(()=>{worker.terminate();reject(new Error('Plugin exceeded 2 seconds'));},2000);
      worker.onmessage=event=>{clearTimeout(timer);worker.terminate();event.data.ok?resolve(event.data.output):reject(new Error(event.data.error));};
      worker.onerror=error=>{clearTimeout(timer);worker.terminate();reject(new Error(error.message));};
      worker.postMessage({module:plugin.module,action,context});
    });
  }
  async function invokePlugin(id,action,args,providedContext,expected) {
    try{
      const plugin=plugins.get(id);if(!plugin)throw new Error(`Plugin ${id} is not loaded`);
      const spec=plugin.manifest.actions.find(a=>a.name===action);if(!spec)throw new Error('Action not registered');
      const state=JSON.parse(handle.call('document_state','{}',undefined)).response;
      if(expected!==undefined&&Number(expected)!==state.revision)return failure('revision_conflict','Document changed');
      const context=providedContext?JSON.parse(providedContext):{api_version:1,...state.result,revision:state.revision};context.arguments=typeof args==='string'?JSON.parse(args):args;
      const output=await workerRequest(plugin,action,JSON.stringify(context));
      if(JSON.stringify(output).length>MAX_MESSAGE)throw new Error('Plugin response exceeds 4 MiB');
      if(spec.kind!=='command'&&output.commands?.length)throw new Error('Read-only action proposed edits');
      const applied=JSON.parse(handle.plugin_apply(JSON.stringify(output.commands??[]),BigInt(context.revision)));
      if(applied.ok)applied.response.result=output.result??{};return applied;
    }catch(error){return failure('plugin_error',String(error));}
  }
  async function installPlugin(manifestUrl) {
    const url=new URL(manifestUrl,location.href);
    if(url.origin!==location.origin)throw new Error('Plugins must be served from this application origin');
    const response=await fetch(url);if(!response.ok)throw new Error(`Plugin manifest: HTTP ${response.status}`);
    const manifest=await response.json();
    if(manifest.api_version!==1||!/^\w+$/.test(manifest.id)||!Array.isArray(manifest.actions)||!manifest.web)throw new Error('Invalid plugin manifest');
    const names=new Set();for(const action of manifest.actions){if(!action.name||!/^[A-Za-z0-9_.]+$/.test(action.name)||!['command','validation','export'].includes(action.kind)||!action.input_schema||typeof action.input_schema!=='object')throw new Error('Invalid action declaration');const name=`plugin_${manifest.id}_${action.name.replaceAll('.','_')}`;if(names.has(name))throw new Error('Plugin action name collision');names.add(name);}
    for(const [id,existing]of plugins){if(id===manifest.id)continue;for(const action of existing.manifest.actions){if(names.has(`plugin_${id}_${action.name.replaceAll('.','_')}`))throw new Error('Plugin tool name collision');}}
    const module=new URL(manifest.web,url);if(module.origin!==location.origin)throw new Error('Invalid plugin module origin');
    plugins.set(manifest.id,{manifest,module:module.href});
  }
  async function loadPlugin(){const path=prompt('Plugin manifest URL (same application origin)');if(!path)return;try{await installPlugin(path);}catch(error){alert(String(error));}}
  function onState(json){const state=JSON.parse(json);for(const port of ports)port.postMessage({protocol:PROTOCOL,type:'state',state});}
  addEventListener('beforeunload',event=>{flushCheckpoint();if(dirty){event.preventDefault();event.returnValue='';}});
  document.addEventListener('visibilitychange',()=>{if(document.hidden)flushCheckpoint();});
  async function recovery(){try{const db=await database;const request=db.transaction('recovery').objectStore('recovery').get('current');request.onsuccess=()=>{if(!request.result)return;const saved=JSON.parse(request.result);if(!saved.dirty)return;const panel=document.createElement('div');panel.id='recovery';panel.textContent='An unsaved document is available. ';for(const [label,restore]of [['Restore',true],['Discard',false]]){const button=document.createElement('button');button.textContent=label;button.onclick=()=>{if(restore){const r=JSON.parse(handle.restore(JSON.stringify(saved)));if(!r.ok){alert(r.error.message);return;}}if(!restore){db.transaction('recovery','readwrite').objectStore('recovery').delete('current');}panel.remove();};panel.append(button);}document.body.append(panel);};}catch(error){console.error(error);}}
  return {attach(value){handle=value;installPlugin('./plugins/colors/plugin.json').catch(console.error);recovery();if('serviceWorker'in navigator)navigator.serviceWorker.register('./sw.js').catch(console.error);},checkpoint,configureAI,loadPlugin,invokePlugin,onState,pluginSpecs(){return JSON.stringify(actionSpecs().filter(s=>s.name.startsWith("plugin_")&&!["plugin_load","plugin_list"].includes(s.name)));},invokePluginUI(...args){invokePlugin(...args).then(result=>results.push(result));},invokeRegistered(name,args,revision){call(name,JSON.parse(args),revision).then(result=>results.push(result));},pollResults(){return JSON.stringify(results.splice(0));},setDirty(value){dirty=value;},call,actionSpecs};
}
