import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { build } from 'esbuild';
import { JSDOM } from 'jsdom';
import React, { act } from 'react';
const project=path.resolve(import.meta.dirname,'..');
const cache=path.join(project,'node_modules/.cache');await fs.mkdir(cache,{recursive:true});
const temporary=await fs.mkdtemp(path.join(cache,'settings-autosave-'));
const dom=new JSDOM('<body></body>',{url:'http://localhost/',pretendToBeVisual:true});
for(const name of ['window','document','HTMLElement','Element','Node','Event','MouseEvent','KeyboardEvent','FocusEvent','navigator'])Object.defineProperty(globalThis,name,{configurable:true,value:name==='window'?dom.window:dom.window[name]});
globalThis.IS_REACT_ACT_ENVIRONMENT=true;
dom.window.HTMLElement.prototype.scrollIntoView=()=>{};
const catalog=JSON.parse(await fs.readFile(path.join(project,'catalog/transcription-models.json'),'utf8'));
let settings={onboardingComplete:true,uiLanguage:'en',language:'bg',model:'gpt-transcribe',modelDetails:catalog.models[0],vocabularyEnabled:true,vocabularyTerms:[],autoPaste:true,saveAudio:false,saveText:false,historyEnabled:true,outputDirectory:null,launchAtLogin:false,microphoneName:null,automaticMicrophoneFallback:true,dictationShortcut:{kind:'key',code:'f8',modifiers:[]}};
const writes=[];let sequence=0;const callbacks=new Map();
dom.window.__TAURI_INTERNALS__={transformCallback:fn=>{callbacks.set(++sequence,fn);return sequence;},unregisterCallback:id=>callbacks.delete(id),invoke:async(command,args)=>{
 if(command==='bootstrap')return {settings:structuredClone(settings),history:[],failedRecording:null,microphones:['Fixture microphone'],hasApiKey:false,apiKeyStatus:{phase:'missing',checkedAt:null,lastSuccessAt:null},accessibilityGranted:true,appVersion:'TEST FIXTURE',defaultOutputDirectory:'/tmp',recording:{state:'idle',progress:{percent:0,stage:'idle',determinate:false},elapsedSeconds:0,error:null}};
 if(command==='plugin:autostart|is_enabled')return false;
 if(command==='plugin:event|listen')return ++sequence;
 if(command==='plugin:event|unlisten')return;
 if(command==='update_settings'){settings=structuredClone(args.settings);writes.push(settings);return structuredClone(settings);}
 throw new Error('Unexpected test IPC '+command);
}};
dom.window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
let root;
try{
 const outfile=path.join(temporary,'App.mjs');
 await build({absWorkingDir:project,entryPoints:['src/App.tsx'],outfile,bundle:true,platform:'node',format:'esm',jsx:'automatic',packages:'external',define:{'import.meta.env.VITE_APP_PLATFORM':'"windows"'}});
 const {createRoot}=await import('react-dom/client');const {default:App}=await import(pathToFileURL(outfile).href);
 const mount=document.createElement('div');document.body.append(mount);root=createRoot(mount);
 await act(async()=>{root.render(React.createElement(App));});
 const settingsButton=Array.from(document.querySelectorAll('nav button')).find(button=>button.textContent.includes('Settings'));assert.ok(settingsButton);
 await act(async()=>settingsButton.click());
 const toggle=document.querySelector('[role="switch"][aria-label="Paste automatically"]');assert.ok(toggle);
 await act(async()=>{toggle.click();await new Promise(resolve=>setTimeout(resolve,20));});
 assert.ok(writes.some(value=>value.autoPaste===false),'A settings change must persist without pressing Save settings.');
 assert.equal(Array.from(document.querySelectorAll('button')).some(button=>button.textContent==='Save settings'),false,'The manual Save settings action must be removed.');
 console.log('Actual App settings autosave regression passed.');
}finally{if(root)await act(async()=>root.unmount());dom.window.close();await fs.rm(temporary,{recursive:true,force:true});}
