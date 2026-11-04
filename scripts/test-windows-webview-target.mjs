import assert from "node:assert/strict";
import { installedMainTarget } from "./windows-webview-target.mjs";

const blank = { type: "page", url: "about:blank", id: "initializing" };
const overlay = { type: "page", url: "http://tauri.localhost/?window=overlay", id: "overlay" };
const main = { type: "page", url: "http://tauri.localhost/", id: "main" };
assert.equal(installedMainTarget([blank]), null, "Wait for navigation; the cold-start blank target is not the main application.");
assert.equal(installedMainTarget([blank, overlay, main]), main, "The real installer probe must choose main even when blank/overlay targets appear first.");
assert.equal(installedMainTarget([overlay]), null);
assert.equal(installedMainTarget([{ type: "page", url: "https://example.com/" }]), null);
assert.equal(installedMainTarget([{ type: "page", url: "not a URL" }]), null);
assert.equal(installedMainTarget([{ type: "worker", url: "http://tauri.localhost/" }]), null);
assert.equal(installedMainTarget([{ type: "page", url: "https://tauri.localhost/index.html?lang=bg" }])?.url, "https://tauri.localhost/index.html?lang=bg");
console.log("Installed Windows main-target regressions passed: blank startup, overlay, unrelated pages, invalid URLs, and both app schemes.");
