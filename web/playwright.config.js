import {defineConfig} from '@playwright/test';

const basePath=process.env.ERLU_BASE_PATH??'/';
if(!/^\/(?:[A-Za-z0-9_.-]+\/)*$/.test(basePath))throw new Error('Invalid ERLU_BASE_PATH');
const baseURL=`http://127.0.0.1:8080${basePath}`;
const chromiumPath=process.env.CHROMIUM_PATH;
const firefoxPath=process.env.FIREFOX_PATH;
const firefoxHeaded=process.env.ERLU_FIREFOX_HEADED==='1';

export default defineConfig({
 testDir:'./tests',timeout:60000,fullyParallel:false,
 workers:process.env.CI?1:undefined,
 use:{
  baseURL,locale:'en-US',colorScheme:'light',deviceScaleFactor:1,
  viewport:{width:1280,height:900},headless:true,
  trace:'retain-on-failure',screenshot:'only-on-failure',
 },
 projects:[
  {name:'chromium',use:{browserName:'chromium',launchOptions:{
   ...(chromiumPath?{executablePath:chromiumPath}:{}),
   args:['--no-sandbox','--enable-webgl','--use-gl=angle','--use-angle=swiftshader'],
  }}},
  ...(!chromiumPath||process.env.ERLU_ALL_BROWSERS==='1' ?[
   {name:'firefox',use:{browserName:'firefox',headless:!firefoxHeaded,launchOptions:{...(firefoxPath?{executablePath:firefoxPath}:{}),firefoxUserPrefs:{'webgl.force-enabled':true,'layout.css.devPixelsPerPx':'1.0'}}}},
   {name:'webkit',use:{browserName:'webkit'}},
  ]:[]),
 ],
 webServer:{command:'python3 ../scripts/serve_web.py',url:baseURL,reuseExistingServer:!process.env.CI},
 reporter:process.env.CI?[['list'],['html',{open:'never'}]]:'list',
});
