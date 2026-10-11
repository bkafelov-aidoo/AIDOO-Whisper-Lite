import { BookOpen } from "lucide-react";
import { translator } from "../i18n";
import { type AppLanguage, type AppSettings } from "../types";
import { SettingsSection, SettingRow, Toggle } from "./SettingsControls";

export function vocabularyIssue(terms: string[]) {
  const seen = new Set<string>();
  const unique = terms.map((term) => term.trim()).filter((term) => {
    const key = term.toLowerCase();
    if (!term || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  if (unique.some((term) => /[<>\p{Cc}]/u.test(term))) return "vocabularyInvalid";
  if (unique.length > 50) return "vocabularyTooMany";
  if (unique.some((term) => Array.from(term).length > 80)) return "vocabularyTermTooLong";
  if (unique.reduce((count, term) => count + Array.from(term).length, 0) > 2000) return "vocabularyTooLong";
  return null;
}

export function VocabularyEditor({ settings, language, disabled, onChange }: {
  settings: AppSettings;
  language: AppLanguage;
  disabled: boolean;
  onChange: (settings: AppSettings) => void;
}) {
  const t = translator(language);
  const unsupported = settings.modelDetails?.protocol === "diarize-v1";
  const issue = vocabularyIssue(settings.vocabularyTerms);
  return <SettingsSection icon={<BookOpen />} title={t("vocabularyTitle")}>
    <SettingRow title={t("vocabularyEnabled")} detail={t(unsupported ? "vocabularyDiarizeHelp" : "vocabularyHelp")}>
      <Toggle label={t("vocabularyEnabled")} checked={settings.vocabularyEnabled && !unsupported} disabled={disabled || unsupported} onChange={(vocabularyEnabled) => onChange({ ...settings, vocabularyEnabled })} />
    </SettingRow>
    <div className="vocabulary-editor">
      <label htmlFor="dictation-vocabulary">{t("vocabularyWords")}</label>
      <textarea id="dictation-vocabulary" value={settings.vocabularyTerms.join("\n")} rows={5} disabled={disabled}
        spellCheck={false} autoCapitalize="off" autoCorrect="off" placeholder={t("vocabularyPlaceholder")}
        aria-invalid={Boolean(issue)} aria-describedby="vocabulary-help vocabulary-error"
        onChange={(event) => onChange({ ...settings, vocabularyTerms: event.target.value.split("\n") })} />
      <p id="vocabulary-help">{t("vocabularyLimits")}</p>
      <p id="vocabulary-error" className="field-error" role="status">{issue ? t(issue) : ""}</p>
    </div>
  </SettingsSection>;
}
