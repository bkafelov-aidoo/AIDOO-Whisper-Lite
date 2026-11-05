import { useRef, useSyncExternalStore } from "react";
import { SettingsAutosave } from "../lib/settings-autosave";
import type { AppSettings } from "../types";
export function useSettingsAutosave(commit: (settings: AppSettings) => Promise<AppSettings>, report: (error?: unknown) => void) {
  const handlers = useRef({ commit, report });
  handlers.current = { commit, report };
  const ref = useRef<SettingsAutosave | null>(null);
  if (!ref.current) ref.current = new SettingsAutosave(settings => handlers.current.commit(settings), error => handlers.current.report(error));
  const store = ref.current;
  useSyncExternalStore(store.subscribe, store.snapshot);
  return store;
}
