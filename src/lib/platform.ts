export const isWindows = import.meta.env.VITE_APP_PLATFORM === "windows";

export function credentialHelp(language: "bg" | "en") {
  if (!isWindows) return null;
  return language === "bg"
    ? "Ключът се пази в Windows Credential Manager. Не се записва в настройките, логовете или диагностиката."
    : "Your key is stored in Windows Credential Manager, never in settings, logs or diagnostics.";
}

export function microphonePrivacyHelp(language: "bg" | "en") {
  return language === "bg"
    ? "В Windows включете достъпа до микрофона за настолни приложения, след което тествайте избрания микрофон."
    : "Enable microphone access for desktop apps in Windows, then test your selected microphone.";
}
