import {translations} from './translations.js';

const picker=document.getElementById('language');
function supported(value){return typeof value==='string'&&Object.hasOwn(translations,value.toLowerCase().split('-')[0])?value.toLowerCase().split('-')[0]:null;}
function savedLanguage(){try{return supported(localStorage.getItem('erlu.site.language'));}catch{return null;}}
function render(language){
 const text=translations[language];
 document.documentElement.lang=language;
 document.documentElement.dir=language==='ar'?'rtl':'ltr';
 document.title=text.title;
 document.querySelector('meta[name="description"]').content=text.description;
 for(const node of document.querySelectorAll('[data-i18n]'))node.textContent=text[node.dataset.i18n];
 for(const node of document.querySelectorAll('[data-i18n-aria]'))node.setAttribute('aria-label',text[node.dataset.i18nAria]);
 for(const node of document.querySelectorAll('[data-i18n-alt]'))node.alt=text[node.dataset.i18nAlt];
 picker.value=language;
 for(const link of document.querySelectorAll('a[href*="editor.html"]'))link.href=`./editor.html?lang=${language}`;
}
const url=new URL(location.href);
const preferred=supported(url.searchParams.get('lang'))??savedLanguage()??(navigator.languages??[navigator.language]).map(supported).find(Boolean)??'en';
render(preferred);
picker.addEventListener('change',()=>{
 const language=supported(picker.value)??'en';
 render(language);
 try{localStorage.setItem('erlu.site.language',language);}catch{}
 const next=new URL(location.href);next.searchParams.set('lang',language);history.replaceState(null,'',next);
});
