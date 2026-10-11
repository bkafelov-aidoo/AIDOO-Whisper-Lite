import { CircleCheck, CircleHelp, CircleX, LoaderCircle, RefreshCw } from "lucide-react";
import { translator } from "../i18n";
import { formatDate } from "../lib/presentation";
import type { ApiKeyStatus, AppLanguage } from "../types";
export function ApiKeyHealth({ hasKey, status, language, checking, disabled, onCheck }: {
  hasKey: boolean; status?: ApiKeyStatus; language: AppLanguage; checking: boolean; disabled: boolean; onCheck: () => void;
}) {
  const t = translator(language);
  const phase = !hasKey ? "missing" : checking ? "checking" : status?.phase || "unchecked";
  const good = phase === "verified" || phase === "working";
  const labels = { missing: "keyMissing", checking: "checking", unchecked: "keyUnchecked", verified: "keyVerified", working: "keyWorking", invalid: "keyInvalid", restricted: "keyRestricted", limited: "keyLimited", unavailable: "keyCheckUnavailable" } as const;
  const key = labels[phase as keyof typeof labels] ?? "keyUnchecked";
  return <div className="api-key-health"><div role="status" className={`api-key-status ${good ? "ready" : phase === "invalid" ? "invalid" : ""}`}>{checking ? <LoaderCircle className="spin" /> : good ? <CircleCheck /> : phase === "invalid" ? <CircleX /> : <CircleHelp />}<strong>{t(key)}</strong></div><button className="secondary-button" disabled={!hasKey || disabled || checking} onClick={onCheck}><RefreshCw />{t("checkKey")}</button><p>{t(good ? "keyVerifiedHelp" : phase === "invalid" ? "keyInvalidHelp" : phase === "restricted" ? "keyRestrictedHelp" : phase === "limited" ? "keyLimitedHelp" : phase === "unavailable" ? "keyUnavailableHelp" : "keyCheckHelp")}</p>{status?.checkedAt && <small>{t("lastChecked")}: {formatDate(status.checkedAt, language)}</small>}{status?.lastSuccessAt && <small>{t("lastSuccessfulTranscription")}: {formatDate(status.lastSuccessAt, language)}</small>}</div>;
}
