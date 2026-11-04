export type ShortcutBinding = { kind: "key"; code: string; modifiers: string[] };

export interface AppSettings {
  onboardingComplete: boolean;
  uiLanguage: "auto" | "bg" | "en";
  language: string;
  model: string;
  modelDetails?: CatalogModel | null;
  vocabularyEnabled: boolean;
  vocabularyTerms: string[];
  autoPaste: boolean;
  saveAudio: boolean;
  saveText: boolean;
  historyEnabled: boolean;
  outputDirectory: string | null;
  launchAtLogin: boolean;
  microphoneName: string | null;
  automaticMicrophoneFallback: boolean;
  dictationShortcut: ShortcutBinding;
}

export interface TranscriptEntry {
  id: string;
  text: string;
  createdAt: string;
  durationSeconds: number;
  model: string;
  language: string;
  audioPath: string | null;
  textPath: string | null;
  cost?: { estimatedUsd: number; usdPerMinute: number; pricingCheckedAt: string; billing: string } | null;
}

export interface FailedRecording {
  path: string;
  createdAt: string;
  durationSeconds: number;
  error: string;
  retryable: boolean;
  completedText?: string | null;
}

export interface MicrophoneProbe {
  deviceName: string;
  preferredName: string | null;
  usedFallback: boolean;
  peakLevel: number;
  heardAudio: boolean;
}

export interface RecordingProgress {
  percent: number;
  stage: string;
  determinate: boolean;
}

export interface RecordingSnapshot {
  state: "idle" | "starting" | "recording" | "transcribing" | "done" | "error";
  progress: RecordingProgress;
  elapsedSeconds: number;
  error: string | null;
}

export interface BootstrapState {
  settings: AppSettings;
  history: TranscriptEntry[];
  failedRecording: FailedRecording | null;
  microphones: string[];
  hasApiKey: boolean;
  apiKeyStatus: ApiKeyStatus;
  accessibilityGranted: boolean;
  appVersion: string;
  defaultOutputDirectory: string;
  recording: RecordingSnapshot;
}

export interface OverlayBootstrapState {
  uiLanguage: "auto" | "bg" | "en";
  recording: RecordingSnapshot;
}

export interface TranscriptionCompleted {
  entry: TranscriptEntry | null;
  text: string;
  pasteSucceeded: boolean;
  pasteError: string | null;
}

export type AppLanguage = "bg" | "en";

export interface CatalogModel {
  id: string;
  rank: number;
  protocol: string;
  estimatedUsdPerMinute: number;
  billing: string;
  pricingCheckedAt: string;
  languages: string[];
  benefitBg: string;
  benefitEn: string;
  source: string;
}
export interface CatalogModelOption extends CatalogModel {
  available?: boolean | null;
  isNew?: boolean;
}
export interface CatalogReport { models: CatalogModelOption[]; catalogUpdated: boolean; }
export interface ApiKeyStatus { phase: string; checkedAt: string | null; lastSuccessAt: string | null; }
