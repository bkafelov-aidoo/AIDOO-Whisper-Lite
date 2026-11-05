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
const writes=[];let failSave=false;let gate=null;let launchAtLogin=false;const startup=[];let sequence=0;const callbacks=new Map();
dom.window.__TAURI_INTERNALS__={transformCallback:fn=>{callbacks.set(++sequence,fn);return sequence;},unregisterCallback:id=>callbacks.delete(id),invoke:async(command,args)=>{
 if(command==='bootstrap')return {settings:structuredClone(settings),history:[],failedRecording:null,microphones:['Fixture microphone'],hasApiKey:false,apiKeyStatus:{phase:'missing',checkedAt:null,lastSuccessAt:null},accessibilityGranted:true,appVersion:'TEST FIXTURE',defaultOutputDirectory:'/tmp',recording:{state:'idle',progress:{percent:0,stage:'idle',determinate:false},elapsedSeconds:0,error:null}};
 if(command==='plugin:autostart|is_enabled')return launchAtLogin;
 if(command==='plugin:autostart|enable'){startup.push(true);launchAtLogin=true;return;}
 if(command==='plugin:autostart|disable'){startup.push(false);launchAtLogin=false;return;}
 if(command==='plugin:event|listen')return ++sequence;
 if(command==='plugin:event|unlisten')return;
 if(command==='update_settings'){if(gate)await gate;if(failSave)throw new Error('Fixture write failed');settings=structuredClone(args.settings);writes.push(settings);return structuredClone(settings);}
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

 assert.ok(document.querySelector('.toast.success'),'Show acceptance only after a successful write.');
 // Two handlers from one React render must merge their patches rather than replace each other.
 const switches=[...document.querySelectorAll('[role="switch"]')].filter(node=>node!==toggle);
 assert.ok(switches.length>=2);
 await act(async()=>{switches[0].click();switches[1].click();});
 const recent=writes.at(-1);assert.ok(recent);
 assert.equal(recent.vocabularyEnabled,false);assert.equal(recent.automaticMicrophoneFallback,false);
 // A rejected persistence call keeps the draft and offers Retry, with no success toast.
 failSave=true;
 await act(async()=>toggle.click());
 assert.ok(document.querySelector('.toast.error'));
 assert.equal(document.querySelector('.toast.success'),null);
 assert.equal(toggle.getAttribute('aria-checked'),'true','Failed saves must keep the changed draft visible.');
 failSave=false;
 const retry=[...document.querySelectorAll('.field-error button')][0];assert.ok(retry);
 await act(async()=>retry.click());
 assert.equal(settings.autoPaste,true);assert.ok(document.querySelector('.toast.success'));
 // Root-owned queue survives navigating away before the native response.
 let release;gate=new Promise(resolve=>{release=resolve;});
 await act(async()=>toggle.click());
 const history=[...document.querySelectorAll('nav button')].find(button=>button.textContent.includes('History'));
 await act(async()=>history.click());
 await act(async()=>{gate=null;release();});
 assert.equal(settings.autoPaste,false);
 await act(async()=>settingsButton.click());
 assert.equal(document.querySelector('[role="switch"][aria-label="Paste automatically"]').getAttribute('aria-checked'),'false');
 // OS login preference is rolled back if writing settings fails.
 failSave=true;
 await act(async()=>document.querySelector('[role="switch"][aria-label="Launch when you sign in"]').click());
 assert.equal(launchAtLogin,false);assert.deepEqual(startup,[true,false]);
 failSave=false;
 await act(async()=>[...document.querySelectorAll('.field-error button')][0].click());
 assert.equal(launchAtLogin,true);assert.equal(settings.launchAtLogin,true);
 console.log('Actual App autosave passed: merged changes, acceptance toast, error/retry, navigation, and OS login rollback.');
 // Exercise the writer under delayed responses, canonicalized vocabulary, and refresh races.
 const writerFile=path.join(temporary,'writer.mjs');
 await build({absWorkingDir:project,entryPoints:['src/lib/settings-autosave.ts'],outfile:writerFile,bundle:true,platform:'node',format:'esm',jsx:'automatic',packages:'external'});
 const {SettingsAutosave}=await import(pathToFileURL(writerFile).href);
 const initial=structuredClone(settings);const pendingWrites=[];const reports=[];
 const store=new SettingsAutosave(value=>new Promise((resolve,reject)=>pendingWrites.push({value,resolve,reject})),error=>reports.push(error??'saved'));
 store.receive(initial,0);const base=store.draft;
 store.change({...base,autoPaste:!base.autoPaste},base);await Promise.resolve();
 store.change({...base,historyEnabled:!base.historyEnabled},base);
 assert.equal(pendingWrites.length,1,'Only one write may run at once.');
 pendingWrites[0].resolve(pendingWrites[0].value);await new Promise(resolve=>setTimeout(resolve,0));
 assert.equal(pendingWrites.length,2);assert.equal(pendingWrites[1].value.historyEnabled,!base.historyEnabled);assert.equal(pendingWrites[1].value.autoPaste,!base.autoPaste);
 pendingWrites[1].resolve(pendingWrites[1].value);await store.flush();
 assert.deepEqual(reports,['saved']);
 const oldRevision=store.revision;
 store.change({...store.draft,vocabularyTerms:[' кофердам ','','кофердам']},store.draft);await Promise.resolve();
 assert.deepEqual(pendingWrites[2].value.vocabularyTerms,['кофердам']);
 pendingWrites[2].resolve(pendingWrites[2].value);await store.flush();
 assert.deepEqual(store.draft.vocabularyTerms,[' кофердам ','','кофердам'],'Preserve textarea newlines/cursor formatting.');
 store.receive(initial,oldRevision);assert.deepEqual(store.saved.vocabularyTerms,['кофердам'],'Ignore refreshes started before a newer change.');
 store.change({...store.draft,vocabularyTerms:['<invalid>']},store.draft);await store.flush();
 assert.equal(pendingWrites.length,3,'Invalid vocabulary must not be sent.');
 store.change({...store.draft,saveAudio:!store.draft.saveAudio},store.draft);await Promise.resolve();
 assert.deepEqual(pendingWrites[3].value.vocabularyTerms,['кофердам'],'Unrelated settings still save with the last valid vocabulary.');
 pendingWrites[3].reject(new Error('Fixture writer rejected'));await assert.rejects(store.flush());
 assert.equal(store.draft.saveAudio,!initial.saveAudio);assert.ok(store.error);
 const retryWrite=store.flush();await Promise.resolve();pendingWrites[4].resolve(pendingWrites[4].value);await retryWrite;assert.equal(store.error,null);
 console.log('Settings writer regressions passed: serial writes, stale responses, vocabulary editing, independent validation, and retry.');
}finally{if(root)await act(async()=>root.unmount());dom.window.close();await fs.rm(temporary,{recursive:true,force:true});}
