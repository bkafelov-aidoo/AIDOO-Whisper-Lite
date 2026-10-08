import { spawnSync } from "node:child_process";

if (!process.env.npm_execpath) throw new Error("Start the Windows installer build with npm run bundle:windows.");
const result = spawnSync(process.execPath, [process.env.npm_execpath, "run", "build"], {
  env: { ...process.env, VITE_APP_PLATFORM: "windows" },
  stdio: "inherit",
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
