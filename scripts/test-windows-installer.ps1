$ErrorActionPreference = 'Stop'
$output = Join-Path $PSScriptRoot '../release/windows-ci'
New-Item -ItemType Directory -Force $output | Out-Null
$installers = @(Get-ChildItem 'src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe')
if ($installers.Count -ne 1) { throw 'Expected exactly one Windows installer.' }
$installer = $installers[0]
$install = Start-Process -FilePath $installer.FullName -ArgumentList '/S' -Wait -PassThru
if ($install.ExitCode -ne 0) { throw "Installer exit code: $($install.ExitCode)" }
$installDir = Join-Path $env:LOCALAPPDATA 'AIDOO Whisper Lite'
$exe = Join-Path $installDir 'aidoo-whisper-lite.exe'
if (!(Test-Path $exe)) { throw "The installed application is missing: $exe" }
$notices = Join-Path $installDir 'THIRD_PARTY_NOTICES.txt'
if (!(Test-Path $notices)) { throw 'The required third-party notices are missing.' }
$app = Start-Process -FilePath $exe -PassThru
try {
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        Start-Sleep -Milliseconds 500
        $app.Refresh()
        if ($app.HasExited) { throw "The installed application exited: $($app.ExitCode)" }
        if ([DateTime]::UtcNow -gt $deadline) { throw 'The installed main window did not appear.' }
    } while ($app.MainWindowTitle -ne 'AIDOO Whisper Lite')
    Start-Sleep -Seconds 4
    Add-Type -AssemblyName System.Drawing
    Add-Type -AssemblyName System.Windows.Forms
    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $bitmap.Save((Join-Path $output 'installed-app.png'), [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    # The first-run settings file contains no API key and no customer data.
    $settings = Join-Path $env:LOCALAPPDATA 'AIDOO/Whisper Lite/settings.json'
    if (Test-Path $settings) {
        $saved = Get-Content -Raw $settings | ConvertFrom-Json
        if ($saved.dictationShortcut.code -ne 'f8') { throw 'Installed Windows shortcut is not F8.' }
        if ($saved.PSObject.Properties.Name -contains 'apiKey') { throw 'An API key appeared in settings.' }
    }
} finally {
    if (!$app.HasExited) { Stop-Process -Id $app.Id -Force }
}
$dataDir = Join-Path $env:LOCALAPPDATA 'AIDOO/Whisper Lite'
New-Item -ItemType Directory -Force $dataDir | Out-Null
$marker = Join-Path $dataDir 'ci-user-data-retention.txt'
Set-Content -Encoding utf8 $marker 'disposable CI retention marker'
$uninstaller = Join-Path $installDir 'uninstall.exe'
if (!(Test-Path $uninstaller)) { throw 'The uninstaller is missing.' }
$uninstall = Start-Process -FilePath $uninstaller -ArgumentList @('/S', "_?=$installDir") -Wait -PassThru
if ($uninstall.ExitCode -ne 0) { throw "Uninstaller exit code: $($uninstall.ExitCode)" }
if (Test-Path $exe) { throw 'Uninstall left the application executable installed.' }
if (!(Test-Path $marker)) { throw 'Uninstall removed user data.' }
Remove-Item $marker -Force
$sha = (Get-FileHash -Algorithm SHA256 $installer.FullName).Hash.ToLowerInvariant()
$signature = Get-AuthenticodeSignature $installer.FullName
@{
    version = '1.0.5'; target = 'x86_64-pc-windows-msvc'; sourceCommit = $env:GITHUB_SHA
    installer = $installer.Name; sha256 = $sha; signatureStatus = "$($signature.Status)"
    install = 'passed'; launch = 'passed'; uninstall = 'passed'; userDataRetention = 'passed'
    microphoneAndLiveDictation = 'requires physical Windows acceptance'
} | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $output 'installer-verification.json')
"$sha  $($installer.Name)" | Set-Content -Encoding ascii (Join-Path $output 'SHA256SUMS.txt')
