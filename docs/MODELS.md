# OpenAI transcription models

The product uses the stable OpenAI `gpt-transcribe` alias. A newer model ID can be selected only after a compatible curated recommendation and explicit user consent; the alias itself is not a discovery mechanism for unrelated successors.

The source candidate defaults to `gpt-transcribe` (about $0.0045/minute). The Economy/Maximum accuracy selector is removed from settings and onboarding. Existing settings, including former Economy settings, normalize to this default unless a curated successor was explicitly accepted while retaining language, vocabulary and other preferences. Existing history keeps its original model metadata.

Model availability and pricing must be checked before each release against [GPT-Transcribe](https://developers.openai.com/api/docs/models/gpt-transcribe) and [API pricing](https://developers.openai.com/api/docs/pricing). This source change is not a new installer or installed-runtime acceptance.

Explicit languages use `languages[]`; Automatic omits the language hint. See the [File transcription guide](https://developers.openai.com/api/docs/guides/speech-to-text#add-transcription-context).

The app uploads the completed recording as FLAC with an extension-bearing filename and `audio/flac` content type. The official [Create transcription API reference](https://developers.openai.com/api/reference/python/resources/audio/subresources/transcriptions/methods/create) lists FLAC as a supported transcription input. Keeping FLAC reduces transfer size while preserving lossless local audio.

OpenAI's [File transcription guide](https://developers.openai.com/api/docs/guides/speech-to-text) limits files to 25 MB. The app applies the conservative decimal boundary of 25,000,000 bytes before upload, keeps an oversized recording locally and disables Retry because sending the unchanged file again cannot succeed.

## Personal vocabulary

Settings → Dictation vocabulary accepts literal words or phrases, one per line. It keeps the first spelling, trims outer whitespace and removes empty lines and case-insensitive duplicates. Limits are 50 distinct terms, 80 Unicode characters per term and 2000 characters total. Invalid keywords and control characters are rejected before an API request. Older settings load with an empty list. Disabling vocabulary retains the list but sends no hints.

`gpt-transcribe` receives individual `keywords[]`. These are recognition hints, not mandatory output or automatic find-and-replace rules. No text correction model is called. Vocabulary is stored privately in settings and sent only with the explicit transcription request. Diagnostic settings expose its enabled state and term count, never its content.

The completed FLAC upload keeps the existing JSON response flow. The app waits for the full transcript before clipboard, paste, files and history. No preliminary text is displayed, no audio is transmitted while recording, and there is no Realtime session.

The native transcription client reuses its connection pool, sends credentials per request and preserves HTTPS-only, no-redirect and timeout policies. Per-minute pricing is an estimate; see [API pricing](https://developers.openai.com/api/docs/pricing#transcription-and-speech).

See [Model recommendations](MODEL-UPDATES.md) for background checks, consent, persistence and catalog maintenance. No text-cleanup model has been added.
