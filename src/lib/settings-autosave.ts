import type { AppSettings } from "../types";
import { settingsForCommand } from "./settings-contract";
import { vocabularyIssue } from "../components/VocabularyEditor";

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
export type SettingsEdit = AppSettings | ((current: AppSettings) => AppSettings);

// One writer lives at the app root, so navigation cannot discard queued changes.
export class SettingsAutosave {
  draft: AppSettings | null = null;
  saved: AppSettings | null = null;
  error: unknown = null;
  revision = 0;
  private listeners = new Set<() => void>();
  private running: Promise<void> | null = null;
  private version = 0;
  constructor(private commit: (settings: AppSettings) => Promise<AppSettings>, private report: (error?: unknown) => void) {}
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  snapshot = () => this.version;
  private emit() { this.version++; this.listeners.forEach(listener => listener()); }
  receive(settings: AppSettings, revision: number) {
    if (this.draft && (revision !== this.revision || this.running || this.error)) return;
    const terms = this.draft?.vocabularyTerms;
    this.saved = settings;
    this.draft = terms && same(normalizeTerms(terms), settings.vocabularyTerms) ? { ...settings, vocabularyTerms: terms } : settings;
    this.emit();
  }
  change(edit: SettingsEdit, base: AppSettings) {
    if (!this.draft) return;
    const next = typeof edit === "function" ? edit(this.draft) : edit;
    const comparison = typeof edit === "function" ? this.draft : base;
    const patch = Object.fromEntries(Object.entries(next).filter(([key, value]) => !same(value, comparison[key as keyof AppSettings])));
    if (!Object.keys(patch).length) return;
    this.draft = { ...this.draft, ...patch };
    this.revision++;
    this.error = null;
    this.emit();
    void this.flush().catch(() => undefined);
  }
  private desired() {
    const draft = this.draft!;
    return settingsForCommand({ ...draft, vocabularyTerms: vocabularyIssue(draft.vocabularyTerms) ? this.saved!.vocabularyTerms : normalizeTerms(draft.vocabularyTerms) });
  }
  flush = (): Promise<void> => {
    if (this.running) return this.running;
    // Set the shared promise before invoking commit (which may synchronously notify React).
    this.running = Promise.resolve().then(async () => {
      let changed = false;
      while (this.draft && this.saved && !same(this.desired(), settingsForCommand(this.saved))) {
        const sent = this.desired();
        const accepted = await this.commit(sent);
        const patch = Object.fromEntries(Object.entries(accepted).filter(([key]) => key !== "vocabularyTerms" && same(this.draft![key as keyof AppSettings], sent[key as keyof AppSettings])));
        this.saved = accepted;
        this.draft = { ...this.draft!, ...patch };
        this.error = null;
        changed = true;
        this.emit();
      }
      if (changed) this.report();
    }).catch(reason => { this.error = reason; this.emit(); this.report(reason); throw reason; }).finally(() => { this.running = null; });
    return this.running;
  };
}
function normalizeTerms(terms: string[]) {
  const seen = new Set<string>();
  return terms.map(term => term.trim()).filter(term => { const key = term.toLowerCase(); if (!term || seen.has(key)) return false; seen.add(key); return true; });
}
