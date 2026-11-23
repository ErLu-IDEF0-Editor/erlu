import {defineConfig} from '@playwright/test';
const basePath=process.env.ERLU_BASE_PATH??'/';
if(!/^\/(?:[A-Za-z0-9_.-]+\/)*$/.test(basePath))throw new Error('Invalid ERLU_BASE_PATH');
const baseURL=`http://127.0.0.1:8080${basePath}`;
export default defineConfig({testDir:'./tests',timeout:60000,fullyParallel:false,use:{baseURL,viewport:{width:1280,height:900},headless:true,launchOptions:process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH,args:['--no-sandbox','--enable-webgl','--use-gl=angle','--use-angle=swiftshader']}:{args:['--enable-webgl','--use-gl=angle','--use-angle=swiftshader']}},projects:[{name:'chromium',use:{browserName:'chromium'}},...(!process.env.CHROMIUM_PATH?[{name:'firefox',use:{browserName:'firefox'}},{name:'webkit',use:{browserName:'webkit'}}]:[])],webServer:{command:'python3 ../scripts/serve_web.py',url:baseURL,reuseExistingServer:!process.env.CI},reporter:'list'});
