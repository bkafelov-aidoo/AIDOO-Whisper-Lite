# OpenAI transcription models

The product uses stable OpenAI model aliases so users receive the current compatible model behind each alias without an application update.

| Product choice | API model | Displayed estimate | Purpose |
| --- | --- | --- | --- |
| Economy | `gpt-4o-mini-transcribe` | about $0.003/minute | Lowest-cost everyday dictation |
| Maximum accuracy | `gpt-transcribe` | about $0.0045/minute | High-accuracy file transcription |

The model IDs and price copy were checked on 11 October 2026 against the official [GPT-4o Mini Transcribe](https://developers.openai.com/api/docs/models/gpt-4o-mini-transcribe), [GPT-Transcribe](https://developers.openai.com/api/docs/models/gpt-transcribe) and [API pricing](https://developers.openai.com/api/docs/pricing) pages. Pricing is deliberately labelled approximate in the interface. Recheck these pages before every release because OpenAI can change availability and pricing independently from this application.

The app sends an explicit ISO-639-1 language hint only when selected: `language` for Economy, and `languages[]` for Maximum accuracy. In Automatic mode it omits both fields. The current [File transcription guide](https://developers.openai.com/api/docs/guides/speech-to-text#add-transcription-context) uses `languages` instead of `language` for `gpt-transcribe`; they must not be sent together. An explicit language hint can improve accuracy and latency.

The app uploads the completed recording as FLAC with an extension-bearing filename and `audio/flac` content type. The official [Create transcription API reference](https://developers.openai.com/api/reference/python/resources/audio/subresources/transcriptions/methods/create) lists FLAC as a supported transcription input. Keeping FLAC reduces transfer size while preserving lossless local audio.

OpenAI's [File transcription guide](https://developers.openai.com/api/docs/guides/speech-to-text) limits files to 25 MB. The app applies the conservative decimal boundary of 25,000,000 bytes before upload, keeps an oversized recording locally and disables Retry because sending the unchanged file again cannot succeed.

## Personal vocabulary

Settings → Dictation vocabulary accepts literal words or phrases, one per line. It keeps the first spelling, trims outer whitespace and removes empty lines and case-insensitive duplicates. Limits are 50 distinct terms, 80 Unicode characters per term and 2000 characters total. Invalid keywords and control characters are rejected before an API request. Older settings load with an empty list. Disabling vocabulary retains the list but sends no hints.

Economy receives the list in `prompt`; Maximum accuracy receives individual `keywords[]`. These are recognition hints, not mandatory output or automatic find-and-replace rules. No text correction model is called. Vocabulary is stored privately in settings and sent only with the explicit transcription request. Diagnostic settings expose its enabled state and term count, never its content.

The completed FLAC upload keeps the existing JSON response flow. The app waits for the full transcript before clipboard, paste, files and history. No preliminary text is displayed, no audio is transmitted while recording, and there is no Realtime session.

The native transcription client reuses its connection pool, sends credentials per request and preserves HTTPS-only, no-redirect and timeout policies. The models and rates remain unchanged. Economy's vocabulary prompt is additional text input, so exact billing can depend on vocabulary length as well as audio and transcript tokens; the displayed per-minute rate remains an estimate. See [API pricing](https://developers.openai.com/api/docs/pricing#transcription-and-speech).
