// New versions wait for all existing tabs to close. Never reload an editing tab.
const CACHE='erlu-0.1.0';
self.addEventListener('install',event=>event.waitUntil(fetch('./precache.json',{cache:'no-store'}).then(r=>r.json()).then(files=>caches.open(CACHE).then(cache=>cache.addAll(files)))));
self.addEventListener('activate',event=>event.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(k=>k.startsWith('erlu-')&&k!==CACHE).map(k=>caches.delete(k))))));
// Only explicit user action activates an update while an editing tab is open.
self.addEventListener('message',event=>{if(event.data?.type==='activate-update')self.skipWaiting();});
self.addEventListener('fetch',event=>{
 if(event.request.method!=='GET'||new URL(event.request.url).origin!==self.location.origin)return;
 if(event.request.cache==='no-store')return;
 // Online navigations must see new releases instead of an indefinitely cached UI.
 if(event.request.mode==='navigate'){
  event.respondWith(fetch(event.request).catch(()=>caches.match(event.request,{ignoreSearch:true})));return;
 }
 event.respondWith(caches.match(event.request).then(cached=>cached??fetch(event.request).then(response=>{
  if(response.ok){const clone=response.clone();caches.open(CACHE).then(cache=>cache.put(event.request,clone));}return response;
 })));
});
