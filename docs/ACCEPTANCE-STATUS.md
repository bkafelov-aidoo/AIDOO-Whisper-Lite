# macOS 1.0.5 acceptance status

Status date: 15 September 2026

## Current audited release candidate

- Source: parent commit `be02bdec48f33078c44709ab473860add57474df` (`Show model accuracy levels`), exported to the dedicated repository as `9e5dfd154e2d5c4d6cbc3711e3528ec4e146dcd8`.
- Package: `release/1.0.5/AIDOO Whisper Lite_1.0.5_aarch64.dmg`.
- SHA-256: `6d022df0acaec4dd5f8d56aa969334a483ecb1e1c6e31bb7d1b34a90fa96ef34`.
- Architecture: Apple Silicon (`arm64`); minimum macOS version: 13; bundle ID: `app.aidoo.whisper-lite`.
- Signing identity: `Developer ID Application: Aidoo Ltd. OOD (4KKVT2TUUA)`.
- Apple notarization tickets are stapled to the application and DMG. Gatekeeper accepts both as `Notarized Developer ID`.
- The audited application was installed from this exact DMG and launched from `/Applications/AIDOO Whisper Lite.app`.

## Automated evidence

- `npm run release:mac` completed successfully from a clean tree at the exact local `lite-v1.0.5` source tag.
- TypeScript checking, all 89 representative English error-localization cases, release configuration, Apple Silicon dependency boundary and all three website-page checks pass.
- The production frontend build passes.
- All 51 native unit tests pass and Rust Clippy passes for all Apple Silicon release targets with warnings denied.
- `npm audit --omit=dev` and the current RustSec database scan report no production vulnerabilities.
- GitHub Actions run [34988829806](https://github.com/bkafelov-aidoo/AIDOO-Whisper-Lite/actions/runs/34988829806) validates dedicated-repository commit `edd83570ad1a788610dae7d1b1744c9d5cfe3246`; local release checks, tests, strict Clippy, signing and notarization also pass for the same source.
- Recovery safety tests prove that audio is marked non-retryable before an ambiguous OpenAI request, completed text can only take the local-finalization path, cleanup failure leaves a Delete-only item, missing/corrupt recovery metadata fails safe, and no already charged audio can re-enter the network path.
- A full local HTTP upload-path test proves that a timeout after request transmission, an HTTP 5xx response and an unreadable HTTP 2xx response are all non-retryable. It uses a test-only local client; production still allows only the two audited OpenAI HTTPS destinations and rejects redirects.
- History file deletion is journaled and crash-safe; the automated suite covers rollback before the history commit, cleanup after the commit and the narrow interruption between the JSON commit and journal phase update.
- The 25 MB upload boundary is tested on both sides, every FLAC/TXT save combination is tested with private file permissions, and a stale five-minute watchdog is proven unable to stop a later recording.
- Release configuration requires the overlay to be transparent, non-focusable, always on top and visible on all macOS Spaces. A native state-policy test keeps it visible for starting, recording, transcribing, completion and error, and hides it only after returning to idle.
- Local-finalization failure tests prove that a completed FLAC is retained if the TXT write fails and that an uncommitted diagnostic ZIP is removed without deleting a committed file.
- The release audit verifies the checksum, architecture, minimum OS, bundle ID, entitlements, exact AIDOO Developer ID and Team ID, notarization tickets, Gatekeeper acceptance, absence of an updater, required website documents and matching product/website icons.
- The signed bundle includes English and Bulgarian macOS Microphone permission explanations and current third-party notices.

## Behavioral evidence on the development Apple Silicon Mac

- Existing settings and the Keychain API key survive replacement of an older build and installation from the final DMG. The API key is displayed only as saved and is absent from persisted settings and diagnostics.
- The exact notarized 1.0.4 app was installed from the audited DMG. Its Settings header displays version 1.0.4 and a circled right-arrow on “Продължи настройката”, with the previous AI star removed and the existing alignment preserved.
- The exact notarized 1.0.5 app was installed from the audited DMG. Its model cards use the same four-position level scale: Economy has one active bar and Maximum accuracy has four active bars.
- Onboarding was completed in English, including the explicit microphone test, Accessibility state, default Right Option shortcut, model descriptions/prices and independent FLAC/TXT/history storage explanations.
- A real Bulgarian microphone dictation completed through the Economy model. The result reached history and the clipboard, and its FLAC/TXT files were saved with a twelve-character identifier and `0600` permissions.
- The overlay appears while listening, shows waveform and elapsed recording time and remains above the active interface without an opaque window background. The exact notarized app kept it visible for more than one minute while the main window was hidden.
- After recording stops, the overlay remains visible until the app can accept a new recording. It names the current operation and shows progress for preparation, compression, upload, OpenAI transcription and local finalization.
- The recording overlay includes a visible bilingual Stop button. On the exact installed 1.0.2 build, clicking it from the non-focusable overlay moved immediately to `Транскрибирам…` / `OpenAI транскрибира`, displayed progress, completed through the Economy model and returned the application to `Готов за диктовка` with a new local history item. In 1.0.3 the same Stop action is also visible and clickable while the microphone is starting; pointer input is disabled again as soon as processing begins.
- The main window and menu bar use the same busy/error/ready state model. During recording the main controls are disabled and the Stop and transcribe action remains available. The macOS application and tray Quit actions are disabled for the full native operation and re-enabled only after it ends.
- The exact notarized app installed from the final DMG successfully retranscribed existing 47-second FLAC audio with the Economy model. With the main window hidden, the overlay visibly showed `Транскрибирам…` and `OpenAI транскрибира` with progress, and the app then returned to `Готов за диктовка` with a new history item.
- The exact notarized 1.0.3 app was installed from the audited DMG and completed a 46-second Bulgarian dictation through the Economy model using the physical MacBook microphone. Stop moved the native operation through transcription to `Текстът е готов`, created the normal history/FLAC/TXT result and re-enabled all controls. The original automatic-microphone and fallback preferences were restored afterward.
- Launch at Login was enabled and disabled through the exact signed 1.0.3 interface. Enabling it created the expected macOS LaunchAgent for the installed executable; disabling it removed that item and restored the original preference.
- Both history deletion choices were exercised through the signed 1.0.3 confirmation dialog using release-test entries. History-only deletion preserved both linked files; entry-and-files deletion removed both linked files, committed the eight-entry history and left no deletion journal.
- The same installed app successfully retranscribed a three-second FLAC with the Maximum accuracy model (`gpt-transcribe`). The result is labelled `Maximum accuracy` in history; the user's Economy preference was restored afterward.
- Newly generated release-test FLAC/TXT files use `0600`; legacy six-character files remain readable without being rewritten.
- The signed UI was exercised with all four FLAC/TXT combinations and with history both disabled and enabled. It created only the requested artifacts, kept history unchanged when disabled, retained its ten-item cap and stored paired files with the same identifier.
- A custom output folder selected through the signed UI received both FLAC and TXT files with `0600` permissions, and the original default folder preference was restored afterward.
- Pressing Escape while shortcut capture was active in the signed UI cancelled capture and re-enabled all settings. The saved Right Option shortcut remained unchanged across restart.
- A diagnostics package created by the installed build uses `0600`, contains only `diagnostics.log`, redacted `settings.json` and `system.txt`, and contains no OpenAI key, known transcription text or user home path.
- Earlier interactive checks also covered settings, local persistence, recovery after a failed transcription, clipboard copy and the close/reopen lifecycle.

## Candidate — movable recording overlay, 2026-10-10

The user approved top-right placement, dragging and persistent position on 2026-10-10. This is a source candidate based on `8f255310601e76882641d53840e84877174d640a`, developed on `codex/lite-movable-recording-overlay`; it is not an installed-app acceptance or a promoted working baseline.

- With no saved position, the overlay opens at the top right of the cursor's active screen, inside its usable area. Drag the card to choose another position; Stop remains a separate action. Every visible state accepts dragging while the native window remains non-focusable.
- The user position is stored as logical offsets in private `overlay-position.json`, independently of dictation settings. Native move events from automatic positioning do not replace it. Writes are coalesced while dragging, and quitting flushes the last move immediately.
- Status, progress and height changes retain the visible window's screen and position, including manually chosen points at the screen edge. A position is clamped only when needed to keep the window inside the usable area. A new recording or app restart restores the saved position on the active display, accounting for DPI and clamping for a smaller display.
- Automated evidence: all 60 Rust tests pass, including nine placement/persistence tests and the visible-state pointer policy. Strict Clippy, TypeScript, source-layout, localization, release-configuration, macOS-dependency, website, production frontend build and diff checks pass. The full tests require loopback access for the existing local HTTP failure-contract test.
- Visual/interaction evidence: a local browser preview loaded the actual production frontend with simulated native IPC. Recording, starting, transcribing, done and error copy rendered; dragging was requested from the card in recording/done, and Stop requested transcription without requesting a drag. The rendered error card had `scrollWidth == clientWidth == 504`. This verifies the frontend, not actual macOS window dragging, keyboard-focus retention or physical dictation. The design detector reported two pre-existing width-transition warnings in unchanged progress-bar styles; this scoped change preserves those animations.
- The existing signed/notarized ARM DMG remains unchanged at `release/1.0.5/AIDOO Whisper Lite_1.0.5_aarch64.dmg`, SHA-256 `d260dbb408d2385ec09d4913c6d2d833a0c87b77829c8bacf32e9063579e38c6`. It does not contain this candidate. No new installer or installed application replacement was performed.

Installed acceptance after a separately requested test installer: confirm initial top-right placement; drag during recording/processing; verify that Stop works without moving the window and that the destination app keeps keyboard focus; confirm status changes preserve position; quit immediately after dragging and restart; check a second monitor/DPI and a smaller work area. Only an explicit user statement that this exact candidate works promotes it.

## Candidate — personal vocabulary, 2026-10-11

Authorized by the user's “НАправи го”, then narrowed by the explicit instruction to preserve the existing completed-text flow and use the vocabulary only as transcription context. The candidate is on `codex/lite-vocabulary-and-latency`, based on the preserved overlay commit `bbb386b`. It is source verification, not installed-app acceptance or a promoted working baseline.

- Settings includes an editable vocabulary with an on/off switch, one literal term per line, 50-term/80-character/2000-character limits, inline bilingual errors, private persistence and case-insensitive deduplication that retains the first spelling. Older settings preserve empty vocabulary behavior. The unresolved word “локторация” was not silently changed or inserted; the UI example is “кофердам”.
- Enabled terms enter the existing Economy request as a prompt or Maximum accuracy request as keywords. GPT-Transcribe uses `languages[]`; Economy uses `language`. Disabled or empty vocabulary sends no hints. Neither model nor its rate was changed; no correction model was added. Economy's prompt contributes text input to the same request, so exact token billing need not be identical to an empty dictionary.
- The completed-JSON response and final-text flow are preserved. There is no streaming response request, preliminary text, new overlay resize behavior or change to recording, Recovery, history, clipboard or paste. The native transcription client retains a connection pool with credentials per request; HTTPS, redirects, timeouts and retry policy are preserved. The earlier uncommitted preview candidate is archived only under ignored development evidence.
- Verification after removing preliminary text: all 65 native tests passed, strict Clippy passed with warnings denied, formatting and diff checks passed, and TypeScript/source layout/90 English-error fixtures/release configuration/macOS dependency boundary/website/production frontend build passed. A request-level regression proves dictionary fields for both models, disabled vocabulary, completed JSON with no `stream` field and four requests using one TCP connection; pre-request invalid-vocabulary rejection, old settings and diagnostic privacy are covered. Existing dictionary UI evidence used the production frontend with simulated native IPC: Bulgarian/English labels/errors, invalid-input Save blocking, dictionary retention while disabled and the minimum 900×620 layout. The dictionary screenshot is `release/development-evidence/20261011-vocabulary-latency/vocabulary-settings-bg.png`; this is frontend evidence, not a real microphone/Keychain/OpenAI test.
- Existing production ARM DMG was independently rehashed unchanged: `d260dbb408d2385ec09d4913c6d2d833a0c87b77829c8bacf32e9063579e38c6`. No new installer, installed replacement, upload, Windows port or baseline promotion was performed. Every newly built/delivered DMG must use a new application version per `RELEASE.md`.

Installed acceptance after a separately requested installer: save a real short dental vocabulary and restart; compare recognition with vocabulary enabled/disabled for each model; check numbers and terminology, final-text delivery and paste exactly once. Live OpenAI latency, billing and real dental accuracy have not been measured by this candidate.

## Required before public launch

- Complete the first-run and critical dictation path from [ACCEPTANCE.md](ACCEPTANCE.md) on a second Apple Silicon Mac or a clean macOS account, including Bulgarian/English system-language behavior and fresh Microphone/Accessibility permission prompts.
- Complete the remaining physical-input cases: save and use a modified shortcut, five-minute automatic stop, sleep/wake synchronization and microphone disconnect with fallback enabled and disabled.
- Maximum accuracy, both history deletion choices, Launch at Login, all FLAC/TXT combinations and a custom output folder have been verified on signed builds; crash-safe history deletion is also covered by native tests.
- Run the remaining forced-termination cases around each OpenAI request path on the signed build. The over-25-MB boundary, ambiguous timeout/5xx/unreadable-2xx outcomes, local finalization, recovery metadata corruption and diagnostic ZIP cleanup have automated coverage.
- Verify representative Bulgarian and English microphone/OpenAI errors, Accessibility-denied clipboard fallback and VoiceOver announcements.
- Configure the six Apple signing/notarization secrets in the private GitHub repository before relying on its release workflow. Local Keychain notarization is already validated.
- Upload the exact audited 1.0.5 DMG and matching checksum to the AIDOO website together with the privacy, support and release-notes pages.
