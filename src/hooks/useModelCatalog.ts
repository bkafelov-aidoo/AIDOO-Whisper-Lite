import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import bundled from "../../catalog/transcription-models.json";
import type { CatalogModel, CatalogReport } from "../types";

export function useModelCatalog(hasApiKey: boolean, checkedAt?: string | null) {
  const [report, setReport] = useState<CatalogReport>({ models: bundled.models.map((m) => ({ ...m, available: null, isNew: false })) as CatalogModel[], catalogUpdated: false });
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    let disposed = false;
    if (!hasApiKey) { setReport({ models: bundled.models.map((m) => ({ ...m, available: null, isNew: false })) as CatalogModel[], catalogUpdated: false }); return; }
    setLoading(true);
    void invoke<CatalogReport>("get_model_catalog").then((next) => { if (!disposed) setReport(next); }).catch(() => undefined).finally(() => { if (!disposed) setLoading(false); });
    return () => { disposed = true; };
  }, [hasApiKey, checkedAt]);
  return { ...report, loading };
}
