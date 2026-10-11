import { FolderOpen } from "lucide-react";
import { translator } from "../i18n";
import { type AppLanguage, type AppSettings } from "../types";

export function SettingsSection({ icon, title, children }: { icon: React.ReactNode; title: string; children: React.ReactNode }) {
  return <section className="settings-section"><header><div>{icon}</div><h2>{title}</h2></header><div className="settings-body">{children}</div></section>;
}
export function SettingRow({ title, detail, children }: { title: string; detail?: string; children: React.ReactNode }) {
  return <div className="setting-row"><div><strong>{title}</strong>{detail && <span>{detail}</span>}</div>{children}</div>;
}
export function Toggle({ label, checked, disabled = false, onChange }: { label: string; checked: boolean; disabled?: boolean; onChange: (value: boolean) => void }) {
  return <button type="button" role="switch" aria-label={label} aria-checked={checked} disabled={disabled} className={`toggle ${checked ? "on" : ""}`} onClick={() => onChange(!checked)}><span /></button>;
}
export function LanguagePicker({ settings, language, disabled = false, onChange }: { settings: AppSettings; language: AppLanguage; disabled?: boolean; onChange: (settings: AppSettings) => void }) {
  const t = translator(language);
  return <label className="language-select"><span>{t("modelLanguage")}</span><select value={settings.language} disabled={disabled} onChange={(event) => onChange({ ...settings, language: event.target.value })}><option value="auto">{t("autoLanguage")}</option><option value="bg">{t("bulgarian")}</option><option value="en">{t("english")}</option><option value="de">Deutsch</option><option value="es">Español</option><option value="fr">Français</option><option value="it">Italiano</option></select></label>;
}
export function StorageControls({ settings, language, outputPath, disabled = false, onChange, onChooseFolder }: { settings: AppSettings; language: AppLanguage; outputPath: string; disabled?: boolean; onChange: (settings: AppSettings) => void; onChooseFolder: () => void }) {
  const t = translator(language);
  return <div className="storage-controls"><SettingRow title={t("saveFlac")} detail={t("saveFlacHelp")}><Toggle label={t("saveFlac")} checked={settings.saveAudio} disabled={disabled} onChange={(saveAudio) => onChange({ ...settings, saveAudio })} /></SettingRow><SettingRow title={t("saveTxt")} detail={t("saveTxtHelp")}><Toggle label={t("saveTxt")} checked={settings.saveText} disabled={disabled} onChange={(saveText) => onChange({ ...settings, saveText })} /></SettingRow><SettingRow title={t("saveHistory")} detail={t("saveHistoryHelp")}><Toggle label={t("saveHistory")} checked={settings.historyEnabled} disabled={disabled} onChange={(historyEnabled) => onChange({ ...settings, historyEnabled })} /></SettingRow><div className="folder-picker"><div><strong>{t("folder")}</strong><span title={outputPath}>{outputPath}</span></div><button className="secondary-button" disabled={disabled} onClick={onChooseFolder}><FolderOpen />{t("chooseFolder")}</button></div></div>;
}
