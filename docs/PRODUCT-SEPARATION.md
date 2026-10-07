# Whisper Lite source separation

Separated on 2026-10-07 following the user's explicit naming: Lite is the small dictation app; Speak is the AI assistant.

- Parent source: `24b12a8f30a9b33005656e14377fb5ecbc67da28` on `main`.
- Standalone subtree: `353579e88059e305943c9c09d3ae2be13e5bdaf3`.
- Product version: 1.0.5, including the modular source refactor before wake-word and AI assistant work.
- Repository: https://github.com/bkafelov-aidoo/AIDOO-Whisper-Lite (existing public visibility retained).
- Local checkout: `products/Whisper Lite` under the original parent folder.

The existing remote main is retained as an ancestor of the separation commit. Old branches/tags remain historical evidence; the current main tree contains dictation only. The split changes repository organization and documentation, not application code or runtime configuration. Existing release acceptance documents remain historical and do not claim a newly built installer.

The original parent checkout and its nested folder remain intact. Old installers/build caches are preserved there rather than included in this repository. No new DMG or application/AI session was started for the split.

Migration checks: application/build files are unchanged from the standalone dictation subtree. TypeScript, source layout (45 files), localization (89 cases), release configuration, macOS dependency boundary, website validation, production frontend build, Rust formatting and diff checks pass in this independent folder. This is separation/source evidence, not a new installed-app test.
