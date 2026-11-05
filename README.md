# AIDOO Whisper Lite

A focused voice typing app for Windows x64. This separate Windows branch retains the dictation-only product. See [Windows setup](docs/WINDOWS.md). Hold F8 or your configured keyboard shortcut, speak, and release it to send the recording to the selected OpenAI transcription model. The result stays in the clipboard and can be pasted automatically into the active application.

Version 1.0.13 for Windows adds automatic settings persistence with confirmation, shortcut capture for Right Ctrl, and working global shortcuts while the app window is focused. Earlier installers remain preserved.

## Product behavior

- Six-step Bulgarian/English onboarding for API key, permissions, shortcut, model, language and local storage.
- OpenAI API key is stored in Windows Credential Manager, with an explicit verification/working status in Settings. The app never stores it in settings, logs or diagnostics.
- Transcription defaults to `gpt-transcribe` (about $0.0045/minute). Settings lets users compare compatible file-transcription models, features and estimated prices; see [models](docs/MODELS.md).
- Personal vocabulary: add dental words or phrases in Settings, one per line, and enable them for transcription.
- Transcription reuses its connection pool; the existing flow shows only the completed text.
- Automatic language detection or a searchable explicit language hint, filtered by model support; interface language uses buttons.
- Optional FLAC, TXT and 10-item local history, with independent controls and estimated per-transcription costs.
- Automatic paste with a clear clipboard fallback message.
- A static AIDOO system tray icon and a movable recording/status overlay initially placed at the top right.
- Failed audio survives restarts until the user retries or deletes it.
- If OpenAI has already returned text, recovery finishes clipboard and local files without sending or charging the audio again.
- Windows uses a per-user NSIS EXE installer. The macOS product remains separate.
- No analytics. Diagnostics are created locally only when the user requests them.

## Requirements

- Windows 10/11 x64 (Intel/AMD)
- Node.js 22, Python 3.11+, the pinned Rust 1.93.1 toolchain and MSVC C++ Build Tools with Windows SDK
- An OpenAI Platform account with API billing enabled

## Development and Windows release

Use Windows with Node.js 22, Python 3.11+, Rust 1.93.1, MSVC C++ Build Tools and Windows SDK:

```powershell
npm ci
npm run check
npm run bundle:windows
```

Signed release builds run in GitHub Actions on the Windows branch through the existing `windows-signing` approval environment. The application, temporary NSIS uninstaller and final installer must all pass Authenticode checks, then the exact installer is installed, launched and uninstalled on the disposable runner. See [Windows signing](docs/WINDOWS-SIGNING.md). No shared OpenAI key is bundled.

## Source layout

- `src/App.tsx` — application shell and native event coordination.
- `src/pages/` — complete product screens; `src/components/` contains reusable interface sections, `src/hooks/` owns React lifecycles, and `src/lib/` contains framework-independent helpers.
- `src-tauri/src/lib.rs` — Tauri composition root and process lifecycle.
- `src-tauri/src/app_ui.rs`, `dictation.rs`, `recovery.rs` and `commands.rs` — native presentation, recording orchestration, fail-safe completion and the command interface.
- `src-tauri/src/audio.rs`, `shortcuts.rs`, `storage.rs` and `transcription.rs` — focused adapters for devices, operating-system input, persistence and OpenAI.
- `website/` — ready-to-publish privacy, support and release pages.
- `docs/` — architecture, model source, release and manual acceptance checklist.

`npm run check:structure` prevents source files from growing beyond 1,000 lines. Split at a cohesive module seam before extending a file past that limit.

## Support

Use Settings → Diagnostics to create a local ZIP that excludes the API key, transcript content and audio. Review it before sharing, then email `support@aidoo.bg` and attach the ZIP only if you choose to. The app never uploads diagnostics automatically.

Windows private data is under `%LOCALAPPDATA%\AIDOO\Whisper Lite`. Each client supplies a personal OpenAI API key. Signed builds use the approval-gated Azure profile described in [Windows signing](docs/WINDOWS-SIGNING.md). Real microphone/OpenAI acceptance remains pending.
