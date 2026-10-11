# OpenAI file-transcription models

Included in version 1.0.8. The existing 1.0.7 installer is preserved. Settings contains a model selector; the default remains `gpt-transcribe`. Model selection takes effect only after Save Settings. There are no Economy/Maximum accuracy modes.

Prices were checked on 2026-10-11 against [OpenAI pricing](https://developers.openai.com/api/docs/pricing). USD/minute values are estimates, particularly for token-billed models; they are not an invoice or balance query.

| Model | Estimated USD/min | Billing | Vocabulary |
| --- | ---: | --- | --- |
| gpt-transcribe | 0.0045 | Duration | `keywords[]` |
| gpt-4o-mini-transcribe | 0.003 | Tokens | `prompt` |
| gpt-4o-transcribe | 0.006 | Tokens | `prompt` |
| gpt-4o-transcribe-diarize | 0.006 | Tokens | Unsupported |
| whisper-1 | 0.006 | Duration | Short `prompt` |

The two documented Mini snapshots (`2025-03-20` and `2025-12-15`) are also selectable at the Mini rate. Only models compatible with uploading a completed recording are included. Realtime models are excluded. Features and aliases come from the official [speech-to-text guide](https://developers.openai.com/api/docs/guides/speech-to-text) and model pages: [GPT-Transcribe](https://developers.openai.com/api/docs/models/gpt-transcribe), [Mini](https://developers.openai.com/api/docs/models/gpt-4o-mini-transcribe), [4o](https://developers.openai.com/api/docs/models/gpt-4o-transcribe), [Diarize](https://developers.openai.com/api/docs/models/gpt-4o-transcribe-diarize), [Whisper](https://developers.openai.com/api/docs/models/whisper-1).

## Language and vocabulary

Interface language uses Automatic/Bulgarian/English buttons. Dictation language uses a searchable combobox, filtering by localized name, English name or code. Options are limited to the selected model's curated language list. Automatic omits a language hint. The common language codes originate from [OpenAI Whisper's tokenizer](https://github.com/openai/whisper/blob/main/whisper/tokenizer.py); GPT-Transcribe also exposes documented Cantonese and regional Chinese hints. Quality varies by language. Choosing a model incompatible with the current hint resets the draft hint to Automatic before Save.

Personal vocabulary retains the existing limits: 50 terms, 80 Unicode characters per term, 2000 total, with whitespace trimming and case-insensitive deduplication. GPT-Transcribe receives `keywords[]`; 4o/Mini receive a comma-separated prompt. Whisper receives a conservative 200-byte prompt ending at a whole term. Diarize disables vocabulary use while retaining the user's terms. Hints never force a replacement in output. No separate text-cleanup model is called.

The native protocol sends `languages[]` for GPT-Transcribe, singular `language` for older models, and `chunking_strategy=auto` with `diarized_json` for Diarize. Diarize final text includes speaker labels from response segments. Other models use the completed JSON text. The existing completed FLAC upload and 25,000,000-byte boundary are retained. The app displays only the final result after Stop; there is no preliminary text, streaming audio or Realtime session. No automatic audio retry is added.

## History and recovery

Each new history entry stores an estimated USD cost, the per-minute rate, billing basis and pricing date used for that request. Later model/rate changes do not recalculate old entries. Legacy history without cost metadata displays no data. For token models, duration multiplied by the published estimated minute rate remains approximate.

Recovery records retain the model and rate used for an already completed request. Finish locally uses those details even after a Settings change, without another network request. Legacy completed recovery without model metadata records an unknown model/cost instead of inventing a charge.

## Key status

Settings distinguishes missing, unchecked, OpenAI-verified, successful transcription, invalid, restricted, rate-limited and unavailable checks. Background/manual checks use the free model-list endpoint. HTTP 200 verifies that OpenAI accepts the key; it does not prove transcription permission, sufficient balance or future availability. HTTP 403 means the list check is restricted, not proof that dictation is broken. Only a real successful transcription establishes Working. Status is held in memory; the key remains in Keychain and is excluded from settings and diagnostics.

See [catalog maintenance](MODEL-UPDATES.md) for background availability checks and safe selection.
