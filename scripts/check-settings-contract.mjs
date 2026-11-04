import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { build } from "esbuild";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
// Execute the actual model selector callback, including its catalogue-only metadata.
const cache = path.join(root, "node_modules/.cache");
await fs.mkdir(cache, { recursive: true });
const temporary = await fs.mkdtemp(path.join(cache, "settings-contract-test-"));
try {
const output = path.join(temporary, "selector.mjs");
await build({ entryPoints: [path.join(root, "src/components/ModelSelector.tsx")], bundle: true, platform: "node", format: "esm", outfile: output, packages: "external", jsx: "automatic" });
const { ModelSelector } = await import(pathToFileURL(output).href);
const contractOutput = path.join(temporary, "serializer.mjs");
await build({ entryPoints: [path.join(root, "src/lib/settings-contract.ts")], bundle: true, platform: "node", format: "esm", outfile: contractOutput, packages: "external" });
const { settingsForCommand } = await import(pathToFileURL(contractOutput).href);
const catalog = JSON.parse(await fs.readFile(path.join(root, "catalog/transcription-models.json"), "utf8"));
const fields = Object.keys(catalog.models[0]).sort();
function findSelect(element) {
  if (!element?.props) return undefined;
  if (element?.type === "select") return element;
  for (const child of [element?.props?.children].flat(Infinity)) {
    const found = findSelect(child);
    if (found) return found;
  }
}
const samples = [];
for (const language of ["bg", "en"]) {
  for (const model of catalog.models) {
    let saved;
    const settings = { model: "gpt-transcribe", language: "bg", modelDetails: catalog.models[0] };
    const models = catalog.models.map((m) => ({ ...m, available: true, isNew: true }));
    const element = ModelSelector({ settings, models, language, disabled: false, loading: false, onChange: (next) => { saved = next; } });
    const select = findSelect(element);
    assert.ok(select, "actual model picker must render a select");
    select.props.onChange({ target: { value: model.id } });
    assert.equal(saved.model, model.id);
    // Cover both the picker and the final command boundary, including older contaminated drafts.
    assert.deepEqual(Object.keys(saved.modelDetails).sort(), fields);
    const sent = settingsForCommand(saved);
    samples.push(sent);
    const contaminated = { ...saved, modelDetails: { ...saved.modelDetails, available: false, isNew: true } };
    const normalized = settingsForCommand(contaminated);
    assert.deepEqual(Object.keys(normalized.modelDetails).sort(), fields);
    assert.deepEqual(normalized, sent);
    assert.equal(contaminated.modelDetails.available, false, "must not mutate the UI model");
  }
}
const outputIndex = process.argv.indexOf("--output");
if (outputIndex >= 0) await fs.writeFile(process.argv[outputIndex + 1], JSON.stringify(samples));
if (!process.argv.includes("--capture-only")) {
  for (const settings of samples) assert.deepEqual(Object.keys(settings.modelDetails).sort(), fields, `update_settings modelDetails must match the native CatalogModel for ${settings.model}`);
  console.log(`Settings IPC contract passed (${samples.length} actual model selections).`);
}

} finally {
  await fs.rm(temporary, { recursive: true, force: true });
}
