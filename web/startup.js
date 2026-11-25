// Own startup end-to-end so failed module downloads cannot silently blank the editor.
const messages={
 en:{loading:'Loading erlu…',compiling:'Preparing the editor…',starting:'Starting the editor…',failed:'The editor could not start.',retry:'Retry',language:'Language',update:'Update available',check:'Check for updates',current:'You have the latest version.',confirm:'Save your model before updating. Reload now?',slow:'The download is taking longer. Keep this tab open.',webgl:'WebGL is unavailable. Check graphics acceleration in your browser.'},
 ru:{loading:'Загрузка erlu…',compiling:'Подготовка редактора…',starting:'Запуск редактора…',failed:'Не удалось запустить редактор.',retry:'Повторить',language:'Язык',update:'Доступно обновление',check:'Проверить обновления',current:'Установлена актуальная версия.',confirm:'Сохраните модель перед обновлением. Перезагрузить сейчас?',slow:'Загрузка занимает больше времени. Оставьте вкладку открытой.',webgl:'WebGL недоступен. Проверьте аппаратное ускорение в браузере.'},
 fr:{loading:'Chargement de erlu…',compiling:'Préparation de l’éditeur…',starting:'Démarrage de l’éditeur…',failed:'Impossible de démarrer l’éditeur.',retry:'Réessayer',language:'Langue',update:'Mise à jour disponible',check:'Vérifier les mises à jour',current:'La version est à jour.',confirm:'Enregistrez votre modèle avant la mise à jour. Recharger maintenant ?',slow:'Le téléchargement prend du temps. Gardez cet onglet ouvert.',webgl:'WebGL est indisponible. Vérifiez l’accélération graphique du navigateur.'},
 es:{loading:'Cargando erlu…',compiling:'Preparando el editor…',starting:'Iniciando el editor…',failed:'No se pudo iniciar el editor.',retry:'Reintentar',language:'Idioma',update:'Actualización disponible',check:'Buscar actualizaciones',current:'La versión está actualizada.',confirm:'Guarda el modelo antes de actualizar. ¿Recargar ahora?',slow:'La descarga está tardando. Mantén esta pestaña abierta.',webgl:'WebGL no está disponible. Revisa la aceleración gráfica del navegador.'},
 ar:{loading:'جار تحميل erlu…',compiling:'جار إعداد المحرر…',starting:'جار تشغيل المحرر…',failed:'تعذر تشغيل المحرر.',retry:'إعادة المحاولة',language:'اللغة',update:'يتوفر تحديث',check:'التحقق من التحديثات',current:'الإصدار الحالي هو الأحدث.',confirm:'احفظ النموذج قبل التحديث. هل تريد إعادة التحميل الآن؟',slow:'يستغرق التنزيل وقتًا أطول. أبقِ علامة التبويب مفتوحة.',webgl:'WebGL غير متاح. تحقق من تسريع الرسومات في المتصفح.'},
 zh:{loading:'正在加载 erlu…',compiling:'正在准备编辑器…',starting:'正在启动编辑器…',failed:'编辑器无法启动。',retry:'重试',language:'语言',update:'有可用更新',check:'检查更新',current:'当前已是最新版本。',confirm:'更新前请保存模型。现在重新加载吗？',slow:'下载时间较长，请保持此标签页打开。',webgl:'WebGL 不可用，请检查浏览器的图形加速设置。'},
};
const languages={en:'English',ru:'Русский',fr:'Français',es:'Español',ar:'العربية',zh:'中文'};
const requested=new URL(location.href).searchParams.get('lang');
let saved;try{saved=localStorage.getItem('erlu.editor.language');}catch{}
let language=Object.hasOwn(messages,requested)?requested:(Object.hasOwn(messages,saved)?saved:(navigator.language??'en').split('-')[0]);
if(!Object.hasOwn(messages,language))language='en';
const panel=document.getElementById('loading'),caption=document.getElementById('loading-message'),detail=document.getElementById('loading-detail'),progress=document.getElementById('loading-progress'),retry=document.getElementById('retry');
let ready=false,registration,assets;
window.erluReady=false;
const text=key=>messages[language][key];
const controls=document.createElement('aside');controls.id='web-controls';
controls.style.cssText='position:fixed;bottom:4px;right:8px;z-index:8;background:#fff;color:#172b49;border:1px solid #cdd7e6;border-radius:5px;padding:3px 6px;font:14px sans-serif;display:flex;align-items:center;gap:6px;max-width:calc(100% - 30px)';
const picker=document.createElement('select');picker.id='editor-language';picker.style.font='inherit';
for(const [code,name] of Object.entries(languages)){const option=document.createElement('option');option.value=code;option.textContent=name;option.lang=code;picker.append(option);}
picker.value=language;
const update=document.createElement('button');update.id='editor-update';update.style.font='inherit';
const build=document.createElement('span');build.id='editor-version';build.style.fontSize='11px';
controls.append(picker,update,build);document.body.append(controls);
function labels(){picker.setAttribute('aria-label',text('language'));update.textContent=registration?.waiting?text('update'):text('check');retry.textContent=text('retry');panel.dir=language==='ar'?'rtl':'ltr';}
labels();caption.textContent=text('loading');
retry.onclick=()=>location.reload();
picker.onchange=async()=>{
 language=picker.value;labels();
 const url=new URL(location.href);url.searchParams.set('lang',language);history.replaceState(null,'',url);
 if(ready)await window.erluRuntime.call('language_set',{language});
 else caption.textContent=text('loading');
};
function showError(error){
 window.erluStartupError=String(error);panel.hidden=false;
 caption.textContent=text('failed');detail.textContent=/webgl/i.test(String(error))?text('webgl'):String(error);
 progress.hidden=true;retry.hidden=false;
}
addEventListener('error',event=>{if(!ready)showError(event.error??event.message);});
addEventListener('unhandledrejection',event=>{if(!ready)showError(event.reason);});
async function reloadUpdated(){
 const state=ready?await window.erluRuntime.call('document_state',{}):null;
 if(state?.response?.dirty&&!confirm(text('confirm')))return;
 if(registration?.waiting){
  navigator.serviceWorker.addEventListener('controllerchange',()=>location.reload(),{once:true});
  registration.waiting.postMessage({type:'activate-update'});
 }else location.reload();
}
update.onclick=async()=>{
 try{
  if(registration?.waiting){await reloadUpdated();return;}
  await registration?.update();
  const response=await fetch('./version.json',{cache:'no-store'});
  if(!response.ok)throw new Error(`HTTP ${response.status}`);
  const latest=await response.json();
  if(registration?.waiting||latest.version!==assets.version)await reloadUpdated();
  else {build.textContent=text('current');setTimeout(()=>{build.textContent=assets.version.slice(0,7);},5000);}
 }catch(error){build.textContent=String(error);}
};
async function configureUpdates(){
 if(!('serviceWorker'in navigator))return;
 try{
  registration=await navigator.serviceWorker.register('./sw.js',{updateViaCache:'none'});
  const waiting=()=>{if(registration.waiting)labels();};waiting();
  registration.addEventListener('updatefound',()=>registration.installing?.addEventListener('statechange',waiting));
  await registration.update();waiting();
 }catch(error){console.warn('Offline cache unavailable',error);}
}
try{
 assets=JSON.parse(document.getElementById('erlu-assets').textContent);
 build.textContent=assets.version.slice(0,7);
 const {createRuntime}=await import(assets.runtime);
 window.erluRuntime=createRuntime();
 const bindings=await import(assets.module);
 const response=await fetch(assets.wasm);
 if(!response.ok)throw new Error(`WASM: HTTP ${response.status}`);
 let loaded=0;const reader=response.body.getReader();
 const tracked=new Response(new ReadableStream({async pull(controller){
  try{const {done,value}=await reader.read();if(done){controller.close();caption.textContent=text('compiling');return;}
   loaded+=value.byteLength;progress.value=Math.min(100,Math.floor(loaded/assets.bytes*100));
   detail.textContent=`${progress.value}% · ${(loaded/1048576).toFixed(1)} / ${(assets.bytes/1048576).toFixed(1)} MB`;controller.enqueue(value);
  }catch(error){controller.error(error);}
 },cancel(reason){return reader.cancel(reason);}}),{headers:{'Content-Type':'application/wasm'}});
 const slow=setTimeout(()=>{if(!ready)caption.textContent=text('slow');},30000);
 try{await bindings.default({module_or_path:tracked});}finally{clearTimeout(slow);}
 window.wasmBindings=bindings;caption.textContent=text('starting');
 const handle=new bindings.WebHandle();await handle.start(document.getElementById('erlu'));
 const chosen=picker.value;
 const result=JSON.parse(handle.call('language_set',JSON.stringify({language:chosen}),undefined));
 if(!result.ok)throw new Error(result.error.message);
 window.erluRuntime.attach(handle);
 // start() initializes WebGL before the first egui frame and font upload.
 // Keep startup visible until that initial work has actually been painted.
 await new Promise((resolve,reject)=>{
  let deadline=performance.now()+15000;
  function painted(){
   if(document.hidden){deadline=performance.now()+15000;setTimeout(painted,250);return;}
   if(window.erluRuntime.uiSnapshot().frame>=2){requestAnimationFrame(()=>requestAnimationFrame(resolve));return;}
   if(performance.now()>deadline){reject(new Error(text('webgl')));return;}
   setTimeout(painted,25);
  }
  painted();
 });
 document.getElementById('erlu').focus();
 ready=true;window.erluReady=true;panel.hidden=true;
 // The app's language menu and the browser selector stay synchronized.
 const onState=window.erluRuntime.onState;
 window.erluRuntime.onState=json=>{onState(json);language=JSON.parse(json).view.language;picker.value=language;labels();try{localStorage.setItem('erlu.editor.language',language);}catch{}};
 await configureUpdates();
}catch(error){showError(error);}
