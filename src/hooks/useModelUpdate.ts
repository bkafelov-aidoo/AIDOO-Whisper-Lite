import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, ModelProposal } from "../types";

export function useModelUpdate(hasApiKey: boolean, onSelected: (settings: AppSettings) => void) {
  const [proposal, setProposal] = useState<ModelProposal | null>(null);
  const [saving, setSaving] = useState(false);
  const inFlight = useRef(false);
  useEffect(() => {
    if (!hasApiKey) return;
    let disposed = false;
    // Native code deduplicates mounts, key changes and concurrent calls.
    void invoke<ModelProposal | null>("check_model_update").then((next) => {
      if (!disposed) setProposal(next);
    }).catch(() => undefined); // Offline checks never interrupt dictation.
    return () => { disposed = true; };
  }, [hasApiKey]);

  const accept = async () => {
    if (!proposal || inFlight.current) return false;
    inFlight.current = true;
    setSaving(true);
    try {
      const settings = await invoke<AppSettings>("accept_model_update", { modelId: proposal.candidate.id });
      onSelected(settings);
      setProposal(null);
      return true;
    } finally {
      inFlight.current = false;
      setSaving(false);
    }
  };
  return { proposal, saving, accept, dismiss: () => { if (!inFlight.current) setProposal(null); } };
}
