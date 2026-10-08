#!/usr/bin/env python3
"""Reject all RustSec advisories affecting the actual Windows build graph."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
manifest = str(ROOT / "src-tauri/Cargo.toml")
tree = subprocess.run(["cargo", "tree", "--locked", "--manifest-path", manifest,
                       "--target", "x86_64-pc-windows-msvc", "--edges", "normal,build",
                       "--prefix", "none", "--format", "{p}"],
                      capture_output=True, text=True, check=True)
packages = {line.removesuffix(" (*)") for line in tree.stdout.splitlines()}
audit = subprocess.run(["cargo", "audit", "--json", "--file", str(ROOT / "src-tauri/Cargo.lock")],
                       capture_output=True, text=True)
if audit.returncode not in (0, 1):
    raise SystemExit(audit.stderr)
report = json.loads(audit.stdout)
if "vulnerabilities" not in report:
    raise SystemExit("The RustSec audit report is incomplete")
blocking, excluded = [], []
for item in report["vulnerabilities"]["list"]:
    package = item["package"]
    name = f"{package['name']} v{package['version']}"
    detail = f"{name}: {item['advisory']['id']}"
    (blocking if name in packages else excluded).append(detail)
output = ROOT / "release/windows-ci"
output.mkdir(parents=True, exist_ok=True)
(output / "dependency-audit.json").write_text(json.dumps({
    "target": "x86_64-pc-windows-msvc", "blocking": blocking,
    "excluded_from_shipped_graph": excluded, "rustsec": report,
}, indent=2) + "\n")
if blocking:
    raise SystemExit("Windows dependency vulnerabilities:\n" + "\n".join(blocking))
print(f"Windows dependency audit passed ({len(excluded)} advisories outside the shipped graph).")
