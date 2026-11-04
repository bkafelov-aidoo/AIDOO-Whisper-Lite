#!/usr/bin/env python3
"""Collect completed release DMGs without changing older installer bytes."""
from pathlib import Path
import hashlib
import json
import shutil

ROOT = Path(__file__).resolve().parent.parent
DESTINATION = ROOT / "DMG"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    DESTINATION.mkdir(exist_ok=True)
    collected = 0
    for manifest in sorted((ROOT / "release").glob("*/release-manifest.json")):
        data = json.loads(manifest.read_text())
        version = data["version"]
        if manifest.parent.name != version:
            raise ValueError(f"Release directory does not match version: {manifest}")
        name = f"AIDOO Whisper Lite_{version}_aarch64.dmg"
        entry = next(item for item in data["files"] if item["path"] == name)
        source = manifest.parent / name
        if source.stat().st_size != entry["bytes"] or digest(source) != entry["sha256"]:
            raise ValueError(f"Installer does not match manifest: {source}")
        target = DESTINATION / name
        if target.exists():
            if digest(target) != entry["sha256"]:
                raise ValueError(f"Refusing to replace a different installer: {target}")
        else:
            shutil.copy2(source, target)
        if digest(target) != entry["sha256"]:
            raise ValueError(f"Copied installer does not match manifest: {target}")
        collected += 1
    print(f"Verified {collected} release DMGs in {DESTINATION}")


if __name__ == "__main__":
    main()
