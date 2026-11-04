// Inspect the installed WebView2 window on the disposable CI desktop.
// The debugging port is enabled by the test process, never by the application.
import { writeFile } from "node:fs/promises";
import { join } from "node:path";

const output = process.argv[2];
const started = Date.now();
const deadline = Date.now() + 30_000;
const kind = (value) => {
  try {
    const url = new URL(value);
    if (url.protocol === "about:") return "blank";
    if (url.hostname !== "tauri.localhost") return "other";
    return url.searchParams.get("window") === "overlay" ? "overlay" : "main";
  } catch { return "invalid"; }
};
const pause = () => new Promise((resolve) => setTimeout(resolve, 500));
let page;
let lastEndpointObservation = null;
let endpointAttempts = 0;
while (!page && Date.now() < deadline) {
  endpointAttempts++;
  try {
    const response = await fetch("http://127.0.0.1:9222/json/list");
    const targets = await response.json();
    lastEndpointObservation = { status: response.status, array: Array.isArray(targets), targets: Array.isArray(targets) ? targets.map(target => ({ type: target.type, kind: kind(target.url) })) : [] };
    page = targets.find((target) => target.type === "page" && !target.url.includes("window=overlay"));
  } catch (reason) { lastEndpointObservation = { errorKind: reason.name, code: reason.cause?.code ?? null }; }
  if (!page) await pause();
}
if (!page) {
  await writeFile(join(output, "endpoint-wait.json"), JSON.stringify({ endpointAttempts, lastEndpointObservation }, null, 2));
  throw new Error("The installed WebView2 did not open its test endpoint.");
}
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});
let sequence = 0;
const pending = new Map();
const errors = [];
let uiObservation = null;
socket.addEventListener("message", ({ data }) => {
  const message = JSON.parse(data);
  if (message.id) {
    const request = pending.get(message.id);
    if (request) {
      clearTimeout(request.timer);
      pending.delete(message.id);
      if (message.error) request.reject(new Error(JSON.stringify(message.error)));
      else request.resolve(message.result);
    }
  } else if (message.method === "Runtime.exceptionThrown" || message.method === "Log.entryAdded") {
    errors.push(message);
  }
});
function call(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`${method} timed out`)); }, 10_000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
try {
  await call("Runtime.enable");
  await call("Log.enable");
  let state;
  do {
    const response = await call("Runtime.evaluate", {
      expression: `JSON.stringify({ ready: Boolean(document.querySelector('.app-shell')), platform: document.documentElement.dataset.platform, text: document.body.innerText, shortcuts: Array.from(document.querySelectorAll('kbd'), item => item.textContent), url: location.href })`,
      returnByValue: true,
    });
    state = JSON.parse(response.result.value);
    uiObservation = { ready: state.ready, platformWindows: state.platform === "windows", documentKind: kind(state.url), shortcutCount: state.shortcuts.length };
    if (!state.ready) await pause();
  } while (!state.ready && Date.now() < deadline);
  await writeFile(join(output, "installed-ui.json"), JSON.stringify({ ...state, errors }, null, 2));
  const screenshot = await call("Page.captureScreenshot", { format: "png" });
  await writeFile(join(output, "installed-webview.png"), Buffer.from(screenshot.data, "base64"));
  if (!state.ready) throw new Error(`The installed interface did not load: ${JSON.stringify({ state, errors })}`);
  if (state.platform !== "windows") throw new Error("The installed interface did not select Windows.");
  if (!state.shortcuts.includes("F8")) throw new Error("The installed interface did not display F8.");
  if (state.shortcuts.some((shortcut) => /[⌘⌥⇧⌃]/.test(shortcut))) throw new Error("The Windows interface displayed Mac keyboard symbols.");
  console.log("Installed Windows interface loaded and displayed its Windows shortcut.");
} finally {
  await writeFile(join(output, "endpoint-wait.json"), JSON.stringify({ elapsedMs: Date.now() - started, endpointAttempts, selectedTargetKind: kind(page.url), lastEndpointObservation, uiObservation }, null, 2));
  socket.close();
}
