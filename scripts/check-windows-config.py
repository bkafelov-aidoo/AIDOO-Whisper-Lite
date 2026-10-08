#!/usr/bin/env python3
"""Validate the Windows installer, frontend and native test boundaries."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
config = json.loads((ROOT / "src-tauri/tauri.windows.conf.json").read_text(encoding="utf-8"))
package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
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
workflow = (ROOT / ".github/workflows/ci-windows.yml").read_text(encoding="utf-8")
for guard in ["runs-on: windows-2022", "npm run bundle:windows", "cargo test --locked --release --target x86_64-pc-windows-msvc", "cargo clippy --locked --release --target x86_64-pc-windows-msvc", "scripts/test-windows-installer.ps1", "scripts/test-windows-input.ps1", "scripts/audit-windows-dependencies.py", "if-no-files-found: error"]:
    require(guard in workflow, f"Windows workflow guard is missing: {guard}")
signing_config = json.loads((ROOT / "src-tauri/tauri.windows-signing.conf.json").read_text(encoding="utf-8"))
require(set(signing_config) == {"$schema", "bundle"}, "Signing override must not alter app behavior")
require(signing_config["bundle"]["publisher"] == "Aidoo LTD", "Signed Windows publisher differs")
require(signing_config["bundle"]["windows"]["signCommand"] == {
    "cmd": "pwsh",
    "args": ["-NoProfile", "-NonInteractive", "-File", "../scripts/sign-windows-artifact.ps1", "%1"],
}, "The Tauri signing hook differs")
require("signCommand" not in config["bundle"]["windows"], "Unsigned CI must not acquire signing access")
signed_command = package["scripts"].get("bundle:windows:signed", "")
require("--config src-tauri/tauri.windows-signing.conf.json -- --locked" in signed_command, "Signing config must be a Tauri argument before Cargo arguments")
signed_job = workflow.split("\n  signed-windows:\n", 1)[-1]
require(package["scripts"].get("build:windows:check") ==
    "tauri build --target x86_64-pc-windows-msvc --no-bundle --ci -- --locked",
    "Prepare native Windows features and assets through Tauri without creating an installer")
prepare_step = signed_job.find("npm run build:windows:check")
test_step = signed_job.find("cargo test --locked --release --target x86_64-pc-windows-msvc")
login_step = signed_job.find("Sign in to Azure")
require(0 <= prepare_step < test_step < login_step,
    "A fresh signing runner must prepare the Windows build before Cargo tests and Azure login")
for guard in [
    "github.event_name == 'workflow_dispatch' && inputs.signed && github.ref == 'refs/heads/codex/whisper-lite-windows'",
    "environment: windows-signing", "id-token: write", "contents: read",
    "secrets.AZURE_CLIENT_ID", "secrets.AZURE_TENANT_ID", "allow-no-subscriptions: true",
    "ArtifactSigning -RequiredVersion 0.1.20", "npm run bundle:windows:signed",
    "scripts/test-windows-installer.ps1 -RequireSignature",
    "AIDOO-Whisper-Lite-Windows-x64-signed",
]:
    require(guard in signed_job, f"Signed Windows workflow guard is missing: {guard}")
for forbidden in ["actions/cache", "cache: npm", "AZURE_CLIENT_SECRET", "AZURE_PASSWORD", "if: always()", "--no-sign", "continue-on-error"]:
    require(forbidden not in signed_job, f"Signed Windows job must not use {forbidden}")
sign_script = (ROOT / "scripts/sign-windows-artifact.ps1").read_text(encoding="utf-8")
for guard in ["https://neu.codesigning.azure.net/", "aidooartifactsigning", "aidoo-whisper-lite", "ExcludeAzureCliCredential = $false", "ExcludeInteractiveBrowserCredential = $true", "FileDigest = 'SHA256'", "TimestampDigest = 'SHA256'", "Assert-AidooWindowsSignature"]:
    require(guard in sign_script, f"Windows signing script guard is missing: {guard}")
if errors:
    raise SystemExit("\n".join(errors))
print("Windows configuration validation passed.")
