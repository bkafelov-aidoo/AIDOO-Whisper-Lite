import { execFileSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { installedMainTarget } from './windows-webview-target.mjs';
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const result = { tests: [], noMicrophoneOrApi: true };
let socket;
try {
  let page; const deadline = Date.now() + 90_000;
  while (!page && Date.now() < deadline) {
    try { page = installedMainTarget(await (await fetch('http://127.0.0.1:9222/json/list', { signal: AbortSignal.timeout(2000) })).json()); } catch {}
    if (!page) await pause(250);
  }
  if (!page) throw new Error('Main test app target unavailable.');
  socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve,reject) => { socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true}); });
  let sequence=0; const pending=new Map();
  socket.addEventListener('message',({data})=>{const m=JSON.parse(data);const q=pending.get(m.id);if(q){clearTimeout(q.timer);pending.delete(m.id);m.error?q.reject(new Error('CDP command failed')):q.resolve(m.result);}});
  const call=(method,params={})=>new Promise((resolve,reject)=>{const id=++sequence;const timer=setTimeout(()=>reject(new Error('CDP timeout')),10000);pending.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params}));});
  const evaluate=async expression=>{const r=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw new Error('Fixture evaluation failed');return r.result.value;};
  await evaluate(`(async()=>{window.__shortcutProbe={events:[]};const api=window.__TAURI_INTERNALS__;for(const event of ['shortcut:captured','shortcut:capture-error','recording:state']){const handler=api.transformCallback(e=>window.__shortcutProbe.events.push({event,payload:e.payload}));await api.invoke('plugin:event|listen',{event,target:{kind:'Any'},handler});}return true;})()`);
  const native=command=>evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)})`);
  const press=code=>execFileSync('pwsh',['-NoProfile','-NonInteractive','-File',new URL('./shortcut-input-fixture.ps1',import.meta.url).pathname.replace(/^\/(\w:)/,'$1'),'-Code',code],{stdio:'pipe'});
  const initial=await native('bootstrap');
  if(initial.settings.onboardingComplete || initial.hasApiKey)throw new Error('Fixture must have no configured dictation or real API key.');
  await evaluate('window.__shortcutProbe.events=[]');press('f8');await pause(600);
  const f8=await evaluate(`window.__shortcutProbe.events.some(e=>e.event==='recording:state' && e.payload==='error')`);
  result.tests.push({case:'foreground_f8_reaches_native_readiness_guard',passed:f8});
  for(const code of ['f9','control_right','kp_0']) {
    await evaluate('window.__shortcutProbe.events=[]');await native('begin_shortcut_capture');press(code);await pause(500);
    const observed=await evaluate(`window.__shortcutProbe.events.filter(e=>e.event.startsWith('shortcut:')).map(e=>({event:e.event,code:e.payload?.binding?.code??null,validationRejected:e.event==='shortcut:capture-error'}))`);
    result.tests.push({case:'capture_'+code,passed:observed.some(e=>e.event==='shortcut:captured'),observed});
    await native('cancel_shortcut_capture');
  }
} catch(reason) { result.fixtureError=reason.message; }
finally { if(socket)socket.close();await writeFile('release/shortcut-repro/results.json',JSON.stringify(result,null,2)); }
console.log(JSON.stringify(result));
if(result.fixtureError || result.tests.some(test=>!test.passed))process.exitCode=1;
