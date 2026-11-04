// Inspect the installed WebView2 window on the disposable CI desktop.
// The debugging port is enabled by the test process, never by the application.
import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { installedMainTarget } from "./windows-webview-target.mjs";

const output = process.argv[2];
// Measured cold WebView2 startup reached 47 seconds on a fresh CI runner.
const started = Date.now();
const deadline = started + 90_000;
const pause = () => new Promise((resolve) => setTimeout(resolve, 500));
let page;
let endpointFailure = null;
while (!page && Date.now() < deadline) {
  try {
    const response = await fetch("http://127.0.0.1:9222/json/list", { signal: AbortSignal.timeout(2000) });
    const targets = await response.json();
    endpointFailure = { httpStatus: response.status, targetCount: Array.isArray(targets) ? targets.length : 0 };
    page = installedMainTarget(targets);
  } catch (reason) { endpointFailure = { errorKind: reason.name, code: reason.cause?.code ?? null }; }
  if (!page) await pause();
}
if (!page) {
  await writeFile(join(output, "startup-failure.json"), JSON.stringify({ phase: "main-target", elapsedMs: Date.now() - started, endpointFailure }, null, 2));
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
    if (!state.ready) await pause();
  } while (!state.ready && Date.now() < deadline);
  await writeFile(join(output, "installed-ui.json"), JSON.stringify({ ...state, errors }, null, 2));
  const screenshot = await call("Page.captureScreenshot", { format: "png" });
  await writeFile(join(output, "installed-webview.png"), Buffer.from(screenshot.data, "base64"));
  if (!state.ready) {
    await writeFile(join(output, "startup-failure.json"), JSON.stringify({ phase: "interface", elapsedMs: Date.now() - started, ready: false }, null, 2));
    throw new Error("The installed interface did not load within its bounded startup check.");
  }
  if (state.platform !== "windows") throw new Error("The installed interface did not select Windows.");
  if (!state.shortcuts.includes("F8")) throw new Error("The installed interface did not display F8.");
  if (state.shortcuts.some((shortcut) => /[⌘⌥⇧⌃]/.test(shortcut))) throw new Error("The Windows interface displayed Mac keyboard symbols.");
  console.log("Installed Windows interface loaded and displayed its Windows shortcut.");
} finally {
  socket.close();
}
