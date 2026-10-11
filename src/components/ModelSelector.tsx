import { AudioLines } from "lucide-react";
import { translator } from "../i18n";
import { formatUsd } from "../lib/presentation";
import type { AppLanguage, AppSettings, CatalogModel } from "../types";
import { SettingsSection } from "./SettingsControls";

export function ModelSelector({ settings, models, language, disabled, loading, onChange }: {
  settings: AppSettings; models: CatalogModel[]; language: AppLanguage; disabled: boolean;
  loading: boolean; onChange: (settings: AppSettings) => void;
}) {
  const t = translator(language);
  const selected = models.find((m) => m.id === settings.model) ?? settings.modelDetails;
  const choices = selected && !models.some((m) => m.id === selected.id) ? [...models, selected] : models;
  return <SettingsSection icon={<AudioLines />} title={t("transcriptionModel")}>
    <label className="language-select"><span>{t("chooseModel")}</span><select value={settings.model} disabled={disabled} onChange={(event) => {
      const model = choices.find((m) => m.id === event.target.value);
      if (model) onChange({ ...settings, model: model.id, modelDetails: model, language: model.languages.includes(settings.language) ? settings.language : "auto" });
    }}>{choices.map((m) => <option key={m.id} value={m.id} disabled={m.available === false && m.id !== settings.model}>{m.id} · ≈ {formatUsd(m.estimatedUsdPerMinute, language)}{t("perMinute")}{m.isNew ? ` · ${t("newModel")}` : ""}{m.available === false ? ` · ${t("modelUnavailable")}` : ""}</option>)}</select></label>
    {selected && <div className="model-details" aria-live="polite"><p>{language === "bg" ? selected.benefitBg : selected.benefitEn}</p><dl><div><dt>{t("estimatedPrice")}</dt><dd>≈ {formatUsd(selected.estimatedUsdPerMinute, language)}{t("perMinute")}</dd></div><div><dt>{t("billingMethod")}</dt><dd>{t(selected.billing === "tokens" ? "billingTokens" : "billingDuration")}</dd></div><div><dt>{t("vocabularyTitle")}</dt><dd>{t(selected.protocol === "diarize-v1" ? "vocabularyUnsupported" : selected.protocol === "gpt-transcribe-v1" ? "vocabularyKeywords" : "vocabularyPrompt")}</dd></div><div><dt>{t("modelAccess")}</dt><dd>{loading ? t("checking") : t(selected.available === true ? "modelAvailable" : selected.available === false ? "modelUnavailable" : "modelAccessUnknown")}</dd></div></dl><small>{t("modelPriceEstimate")} {t("priceChecked")}: {selected.pricingCheckedAt}.</small></div>}
    <p className="section-help">{t("modelSelectionHelp")}</p>
  </SettingsSection>;
}
