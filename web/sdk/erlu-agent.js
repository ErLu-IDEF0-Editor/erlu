export const PROTOCOL='erlu.ai.v1';
/** Connect from an explicitly allowed parent page or extension content script. */
export function connect(targetWindow,targetOrigin,{timeout=10000}={}){
  return new Promise((resolve,reject)=>{
    const {port1:port,port2}=new MessageChannel();const pending=new Map();const listeners=new Set();let counter=0,session,closed=false;
    const timer=setTimeout(()=>{port.close();reject(new Error('Enable AI in erlu and allow the connecting origin'));},timeout);
    function failAll(message){closed=true;for(const entry of pending.values()){clearTimeout(entry.timer);entry.reject(new Error(message));}pending.clear();}
    port.onmessage=event=>{const data=event.data;if(data?.protocol!==PROTOCOL)return;if(data.type==='connected'){clearTimeout(timer);session=data.session;resolve({call(action,args={},expected_revision){return request({type:'call',action,arguments:args,expected_revision});},actions(){return request({type:'actions'});},subscribe(listener){listeners.add(listener);return()=>listeners.delete(listener);},close(){failAll('Connection closed');port.close();}});}else if(data.type==='result'){const entry=pending.get(data.id);if(entry){pending.delete(data.id);clearTimeout(entry.timer);data.ok?entry.resolve(data.response):entry.reject(Object.assign(new Error(data.error.message),data.error));}}else if(data.type==='state'){for(const fn of listeners)fn(data.state);}else if(data.type==='revoked'){failAll('Access revoked by user');port.close();}};
    function request(payload){return new Promise((resolve,reject)=>{if(closed){reject(new Error("Connection closed or access revoked"));return;}const id=++counter;const timer=setTimeout(()=>{pending.delete(id);reject(new Error('Agent request timed out; read state before retrying edits'));},timeout);pending.set(id,{resolve,reject,timer});port.postMessage({protocol:PROTOCOL,session,id,...payload});});}
    port.start();targetWindow.postMessage({protocol:PROTOCOL,type:'connect'},targetOrigin,[port2]);
  });
}
