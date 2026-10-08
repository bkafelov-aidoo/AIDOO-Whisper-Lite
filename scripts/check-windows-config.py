#!/usr/bin/env python3
"""Validate the Windows installer, frontend and native test boundaries."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
config = json.loads((ROOT / "src-tauri/tauri.windows.conf.json").read_text())
package = json.loads((ROOT / "package.json").read_text())
errors = []

def require(condition, message):
    if not condition:
        errors.append(message)

require(config["bundle"]["targets"] == ["nsis"], "Windows must build one NSIS installer")
require(config["bundle"]["icon"] == ["icons/icon.ico"], "Windows icon is missing")
require((ROOT / "src-tauri/icons/icon.ico").is_file(), "Windows icon file is missing")
require(config["bundle"]["windows"]["nsis"]["installMode"] == "currentUser", "Install per user")
require(config["bundle"]["windows"]["nsis"]["languages"] == ["Bulgarian", "English"], "Installer languages differ")
require(config["bundle"]["windows"]["webviewInstallMode"]["type"] == "downloadBootstrapper", "WebView2 bootstrapper is missing")
require(config["app"]["macOSPrivateApi"] is False, "Mac private APIs must be disabled on Windows")
windows = {item["label"]: item for item in config["app"]["windows"]}
require(set(windows) == {"main", "overlay"}, "Expected main and overlay windows")
require(windows["main"]["decorations"] is True, "Use the native Windows title bar")
require("titleBarStyle" not in windows["main"], "Mac title bar configuration leaked into Windows")
for key, value in {"focus": False, "focusable": False, "alwaysOnTop": True, "visible": False, "skipTaskbar": True}.items():
    require(windows["overlay"].get(key) is value, f"Overlay {key} differs")
require(config["build"]["beforeBuildCommand"] == "node scripts/build-windows-frontend.mjs", "Windows frontend build hook differs")
require("x86_64-pc-windows-msvc" in package["scripts"]["bundle:windows"], "Native Windows target is missing")
require("--locked" in package["scripts"]["bundle:windows"], "The installer must use the lockfile")
workflow = (ROOT / ".github/workflows/ci-windows.yml").read_text()
for guard in ["runs-on: windows-2022", "npm run bundle:windows", "cargo test --locked --release --target x86_64-pc-windows-msvc", "cargo clippy --locked --release --target x86_64-pc-windows-msvc", "scripts/test-windows-installer.ps1", "scripts/test-windows-input.ps1", "scripts/audit-windows-dependencies.py", "if-no-files-found: error"]:
    require(guard in workflow, f"Windows workflow guard is missing: {guard}")
if errors:
    raise SystemExit("\n".join(errors))
print("Windows configuration validation passed.")
