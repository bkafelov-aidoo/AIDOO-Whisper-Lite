# Подписване на Windows версията

Azure профилът `aidoo-whisper-lite` е създаден на 8 октомври 2026 г. в съществуващия Artifact Signing акаунт `aidooartifactsigning`, North Europe. Типът е Public Trust, издателят е `Aidoo LTD`. Профилът е активен. Това само по себе си не означава, че даден инсталатор е подписан.

## Настроена връзка

На 8 октомври 2026 г. собственикът създаде отделната Microsoft Entra регистрация `aidoo-whisper-lite-github-signing`. Федеративната връзка с GitHub е създадена и регистрацията получи роля **Artifact Signing Certificate Profile Signer** само върху профила `aidoo-whisper-lite`. Правото е проверено върху точния ресурс на профила. Не са предоставени администраторски права или достъп до други Azure ресурси.

GitHub използва временна самоличност, без парола или постоянен клиентски ключ. Настроеният достъп е:

- Repository: `bkafelov-aidoo/AIDOO-Whisper-Lite`.
- Environment: `windows-signing`.
- Разрешен клон: `codex/whisper-lite-windows`.
- Required reviewer: `bkafelov-aidoo`; ръчно одобрение преди всяко подписване. Администраторският bypass е изключен и запазен през GitHub интерфейса на 8 октомври 2026 г.
- Federated issuer: `https://token.actions.githubusercontent.com`.
- Federated subject: `repo:bkafelov-aidoo@223467801/AIDOO-Whisper-Lite@1371171708:environment:windows-signing`. Репото използва неизменяеми GitHub идентификатори; проверено е чрез неговите OIDC настройки.
- Audience: `api://AzureADTokenExchange`.
- Само `AZURE_CLIENT_ID` и `AZURE_TENANT_ID` като защитени GitHub environment secrets; тези идентификатори не са пароли. Постоянен клиентски секрет не се създава.

Собственикът одобри тази връзка и правото за подписване. Регистрацията в Microsoft Entra, включително приемането на [Microsoft identity platform Terms of Use](https://learn.microsoft.com/en-us/legal/microsoft-identity-platform/terms-of-use), е завършена от него. Azure достъпът и GitHub защитата са настроени. Подписан инсталатор все още не е издаден; първото изпълнение предстои след ръчно одобрение.

## Издаване на подписан инсталатор

1. Отворете GitHub → Actions → **Build and test AIDOO Whisper Lite for Windows** → Run workflow.
2. Изберете `codex/whisper-lite-windows` и включете **signed**.
3. Одобрете чакащото изпълнение в **windows-signing**. Проверете клона и конкретната версия на кода преди одобрение.
4. След успешно изпълнение изтеглете **AIDOO-Whisper-Lite-Windows-x64-signed**.

Подписващото изпълнение компилира на чист Windows runner и не възстановява изпълними файлове от build cache. Преди Cargo тестовете Tauri подготвя Windows настройките и интерфейса с `--no-bundle`, без да създава инсталатор. Това предотвратява конфликт с macOS настройките при първа компилация. Проверките завършват преди входа в Azure. Официалният Microsoft модул `ArtifactSigning` 0.1.20 подписва през Tauri hook. Приложението се подписва преди опаковането; същият hook подписва NSIS помощните DLL файлове, деинсталатора и крайния инсталатор. Използват се SHA-256 и Microsoft RFC3161 timestamp.

Проверката изисква валиден Authenticode подпис, издател `Aidoo LTD` и timestamp. Тя инсталира точния инсталатор, проверява подписи на инсталираното приложение и деинсталатора, сравнява инсталираното приложение с подписания build, стартира интерфейса и деинсталира със запазени потребителски данни. Артефакт с име **signed** се качва само ако всички стъпки са успешни. В него има `signature-verification.json`, `installer-verification.json` и SHA-256.

Обикновените push проверки продължават да създават неподписана тестова версия. Подписването не публикува GitHub release, не променя macOS версията и не приема реалната диктовка за проверена. Физическият тест с микрофон и личен OpenAI ключ остава необходим преди раздаване на клиенти.

Подписът показва проверения издател. Не обещава премахване на всяко предупреждение на SmartScreen, което оценява и репутацията на файла.

## Официални източници

- [Microsoft: Artifact Signing и роли](https://learn.microsoft.com/en-us/azure/artifact-signing/concept-resources-roles).
- [Microsoft: OIDC за Artifact Signing в GitHub](https://github.com/Azure/artifact-signing-action/blob/main/docs/OIDC.md).
- [Microsoft: ArtifactSigning 0.1.20](https://www.powershellgallery.com/packages/ArtifactSigning/0.1.20).
- [Tauri: Windows signing hook](https://v2.tauri.app/distribute/sign/windows/).
