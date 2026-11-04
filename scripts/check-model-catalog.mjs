import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { build } from "esbuild";
import { JSDOM } from "jsdom";
import React, { act } from "react";

const project = path.resolve(import.meta.dirname, "..");
const cache = path.join(project, "node_modules", ".cache");
await fs.mkdir(cache, { recursive: true });
const temporary = await fs.mkdtemp(path.join(cache, "model-catalog-test-"));
const dom = new JSDOM("<body></body>", { url: "http://localhost/", pretendToBeVisual: true });
for (const name of ["window", "document", "HTMLElement", "Node", "Event", "MouseEvent", "KeyboardEvent", "FocusEvent", "navigator"]) {
  Object.defineProperty(globalThis, name, { configurable: true, value: name === "window" ? dom.window : dom.window[name] });
}
globalThis.IS_REACT_ACT_ENVIRONMENT = true;
dom.window.HTMLElement.prototype.scrollIntoView = () => {};

const requests = [];
dom.window.__TAURI_INTERNALS__ = {
  invoke: (command) => {
    if (command !== "get_model_catalog") return Promise.resolve(1);
    let resolve, reject;
    const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
    requests.push({ command, resolve, reject });
    return promise;
  },
  transformCallback: () => 1,
  unregisterCallback: () => {},
};
dom.window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };

const catalog = JSON.parse(await fs.readFile(path.join(project, "catalog/transcription-models.json"), "utf8"));
const report = (available) => ({ models: catalog.models.map((model) => ({ ...model, available, isNew: false })), catalogUpdated: true });
const settings = {
  onboardingComplete: true, uiLanguage: "en", language: "en", model: "gpt-transcribe", modelDetails: catalog.models[0],
  vocabularyEnabled: true, vocabularyTerms: [], autoPaste: true, saveAudio: true, saveText: true, historyEnabled: true,
  outputDirectory: null, launchAtLogin: false, microphoneName: null, automaticMicrophoneFallback: true,
  dictationShortcut: { kind: "key", code: "alt_gr", modifiers: [] },
};
const originalDisplayNames = Intl.DisplayNames;
const fixture = {
  settings, history: [], failedRecording: null, microphones: ["Fixture microphone"], hasApiKey: true,
  apiKeyStatus: { phase: "verified", checkedAt: null, lastSuccessAt: null }, accessibilityGranted: true,
  appVersion: "Regression fixture", defaultOutputDirectory: "/tmp",
  recording: { state: "idle", progress: { percent: 0, stage: "idle", determinate: false }, elapsedSeconds: 0, error: null },
};

try {
  const compiled = path.join(temporary, "components.mjs");
  await build({
    absWorkingDir: project,
    stdin: { contents: 'export { useModelCatalog } from "./src/hooks/useModelCatalog.ts"; export { Onboarding } from "./src/components/Onboarding.tsx";', resolveDir: project, loader: "tsx" },
    outfile: compiled, bundle: true, platform: "node", format: "esm", jsx: "automatic", packages: "external",
  });
  const { createRoot } = await import("react-dom/client");
  const { useModelCatalog, Onboarding } = await import(pathToFileURL(compiled).href);
  let state;
  function Probe(props) { state = useModelCatalog(props.hasKey, props.checkedAt); return null; }
  const mount = document.createElement("div");
  document.body.append(mount);
  const root = createRoot(mount);
  const render = (hasKey, checkedAt) => act(async () => { root.render(React.createElement(Probe, { hasKey, checkedAt })); });

  await render(false, null);
  assert.equal(state.loading, false);
  assert.equal(requests.length, 0, "No credential means no model discovery request");

  await render(true, "check-1");
  assert.equal(state.loading, true);
  assert.equal(requests.length, 1);
  await render(false, null);
  assert.equal(state.loading, false, "Deleting a key must stop loading immediately");
  assert.ok(state.models.every((model) => model.available === null));
  await act(async () => { requests[0].resolve(report(false)); });
  assert.equal(state.loading, false);
  assert.ok(state.models.every((model) => model.available === null), "A deleted key's late account response must be ignored");

  await render(true, "check-2");
  assert.equal(state.loading, true);
  assert.equal(requests.length, 2);
  await render(true, "check-3");
  assert.equal(requests.length, 3, "A new key check must refresh the model report");
  await act(async () => { requests[1].resolve(report(false)); });
  assert.equal(state.loading, true, "An older check must not finish the newer check's loading state");
  assert.ok(state.models.every((model) => model.available === null));
  await act(async () => { requests[2].resolve(report(true)); });
  assert.equal(state.loading, false);
  assert.ok(state.models.every((model) => model.available === true));

  await render(true, "check-4");
  await act(async () => { requests[3].reject(new Error("Offline fixture")); });
  assert.equal(state.loading, false, "A failed discovery must stop loading");
  await act(async () => { root.unmount(); });
  mount.remove();

  const dialogMount = document.createElement("div");
  document.body.append(dialogMount);
  const dialogRoot = createRoot(dialogMount);
  // Some WebKit locale data returns the language code instead of its name.
  Intl.DisplayNames = class extends originalDisplayNames {
    of(code) { return ["ba", "bo"].includes(code) ? code : super.of(code); }
  };
  let closed = 0;
  function DialogProbe() {
    const [opened, setOpened] = React.useState(true);
    return opened ? React.createElement(Onboarding, {
      data: fixture, language: "en", isBusy: false, onData: () => {}, onPersist: async (draft) => draft,
      onRefresh: async () => fixture, onClose: () => { closed += 1; setOpened(false); }, onToast: () => {},
    }) : null;
  }
  await act(async () => { dialogRoot.render(React.createElement(DialogProbe)); });
  for (let step = 0; step < 3; step += 1) {
    const next = [...dialogMount.querySelectorAll("button")].find((button) => button.textContent === "Continue");
    assert.ok(next, "The real Onboarding must expose Continue");
    await act(async () => { next.click(); });
  }
  const input = dialogMount.querySelector('[role="combobox"]');
  assert.ok(input, "The real Onboarding's language step must render its combobox");
  await act(async () => { input.focus(); });
  assert.equal(input.getAttribute("aria-expanded"), "true");
  const options = [...dialogMount.querySelectorAll('[role="option"]')].map((option) => option.textContent);
  assert.ok(options.includes("Bashkirba"), "Missing locale data must fall back to Bashkir's full catalog name");
  assert.ok(options.includes("Tibetanbo"), "Missing locale data must fall back to Tibetan's full catalog name");
  const escape = () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await act(async () => { escape(); });
  assert.equal(closed, 0, "The popup's first Escape must keep Onboarding open");
  assert.ok(dialogMount.querySelector('[role="dialog"]'));
  assert.equal(input.getAttribute("aria-expanded"), "false");
  await act(async () => { escape(); });
  assert.equal(closed, 1, "The second Escape must retain the existing dialog dismissal behavior");
  assert.equal(dialogMount.querySelector('[role="dialog"]'), null);
  await act(async () => { dialogRoot.unmount(); });
  dialogMount.remove();
  console.log("Model catalog React regressions passed: key removal, late response, new checks, errors, language-name fallback, and nested Onboarding Escape.");
} finally {
  Intl.DisplayNames = originalDisplayNames;
  dom.window.close();
  await fs.rm(temporary, { recursive: true, force: true });
}
