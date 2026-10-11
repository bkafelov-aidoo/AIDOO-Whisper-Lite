import { useEffect, useId, useRef, useState } from "react";
import { Check, ChevronDown } from "lucide-react";
import languages from "../../catalog/languages.json";
import { translator } from "../i18n";
import type { AppLanguage, AppSettings } from "../types";

export function LanguageCombobox({ settings, language, disabled = false, onChange }: {
  settings: AppSettings; language: AppLanguage; disabled?: boolean; onChange: (settings: AppSettings) => void;
}) {
  const t = translator(language), id = useId(), root = useRef<HTMLDivElement>(null);
  const [opened, setOpened] = useState(false), [query, setQuery] = useState(""), [active, setActive] = useState(0);
  const names = new Intl.DisplayNames([language], { type: "language" });
  const supported = settings.modelDetails?.languages;
  const list = [{ code: "auto", nameEn: t("autoLanguage") }, ...languages, { code: "yue", nameEn: "Cantonese" }, { code: "zh-cn", nameEn: "Chinese (China)" }, { code: "zh-tw", nameEn: "Chinese (Taiwan)" }, { code: "zh-hk", nameEn: "Chinese (Hong Kong)" }].filter((l) => !supported || supported.includes(l.code)).map((l) => {
    let name = l.code === "auto" ? t("autoLanguage") : names.of(l.code === "jw" ? "jv" : l.code) ?? l.nameEn;
    name = name.charAt(0).toLocaleUpperCase(language) + name.slice(1);
    return { ...l, name };
  }).sort((a,b) => a.code === "auto" ? -1 : b.code === "auto" ? 1 : a.name.localeCompare(b.name, language));
  const filtered = list.filter((l) => `${l.name} ${l.nameEn} ${l.code}`.toLocaleLowerCase(language).includes(query.toLocaleLowerCase(language).trim()));
  const selected = list.find((l) => l.code === settings.language)?.name ?? settings.language;
  const choose = (code: string) => { onChange({ ...settings, language: code }); setOpened(false); setQuery(""); };
  useEffect(() => {
    const close = (event: PointerEvent) => { if (!root.current?.contains(event.target as Node)) { setOpened(false); setQuery(""); } };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, []);
  useEffect(() => { if (disabled) { setOpened(false); setQuery(""); } }, [disabled]);
  useEffect(() => { if (opened) root.current?.querySelector(`[data-option-index="${active}"]`)?.scrollIntoView({ block: "nearest" }); }, [active, opened]);
  return <div className="language-combobox" ref={root} onBlur={(event) => { if (!event.currentTarget.contains(event.relatedTarget)) { setOpened(false); setQuery(""); } }}>
    <label htmlFor={`${id}-input`}>{t("modelLanguage")}</label><div className="language-combobox-input"><input id={`${id}-input`} role="combobox" aria-expanded={opened} aria-controls={`${id}-list`} aria-autocomplete="list" aria-activedescendant={opened && filtered[active] ? `${id}-${active}` : undefined} autoComplete="off" disabled={disabled} placeholder={t("languageSearch")} value={opened ? query : selected} onFocus={() => { setOpened(true); setActive(Math.max(0,list.findIndex((l) => l.code === settings.language))); }} onClick={() => setOpened(true)} onChange={(event) => { setQuery(event.target.value); setActive(0); setOpened(true); }} onKeyDown={(event) => {
      if (event.key === "Escape") { setOpened(false); setQuery(""); }
      else if (["ArrowDown", "ArrowUp"].includes(event.key)) { event.preventDefault(); setOpened(true); setActive((n) => Math.max(0, Math.min(filtered.length - 1, n + (event.key === "ArrowDown" ? 1 : -1)))); }
      else if (event.key === "Enter" && opened && filtered[active]) { event.preventDefault(); choose(filtered[active].code); }
    }} /><ChevronDown aria-hidden="true" /></div>
    {opened && <ul id={`${id}-list`} role="listbox" aria-label={t("modelLanguage")} className="language-options">{filtered.map((l,index) => <li key={l.code} id={`${id}-${index}`} data-option-index={index} role="option" aria-selected={l.code === settings.language} className={active === index ? "active" : ""} onPointerDown={(event) => event.preventDefault()} onClick={() => choose(l.code)} onMouseEnter={() => setActive(index)}><span>{l.name}<small>{l.code}</small></span>{l.code === settings.language && <Check aria-hidden="true" />}</li>)}{!filtered.length && <li role="presentation">{t("noLanguagesFound")}</li>}</ul>}
    <small className="section-help">{t("languageAccuracyHelp")}</small>
  </div>;
}
