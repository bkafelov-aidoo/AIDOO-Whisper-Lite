$ErrorActionPreference = 'Stop'
if (!$IsWindows -or $env:GITHUB_ACTIONS -ne 'true') { throw 'Disposable Windows CI only.' }
New-Item -ItemType Directory -Force 'release/shortcut-repro' | Out-Null
$installers = @(Get-ChildItem -Recurse 'release/input' -Filter '*_x64-setup.exe')
if ($installers.Count -ne 1) { throw 'Expected exact signed installer.' }
if ((Get-FileHash $installers[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant() -ne '5fc7aa95d5fb0b45725837de5c948de8c95c58eac145b885035d36c0aeea1657') { throw 'Input hash differs.' }
$installed = Start-Process $installers[0].FullName -ArgumentList '/S' -Wait -PassThru
if ($installed.ExitCode -ne 0) { throw 'Install failed.' }
$dir = Join-Path $env:LOCALAPPDATA 'AIDOO Whisper Lite'
$previous = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9222'
try { $app = Start-Process (Join-Path $dir 'aidoo-whisper-lite.exe') -PassThru }
finally { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previous }
try {
  node (Join-Path $PSScriptRoot 'shortcut-repro.mjs')
  if ($LASTEXITCODE -ne 0) { throw 'Shortcut regression reproduced.' }
} finally {
  if (!$app.HasExited) { Stop-Process -Id $app.Id -Force }
  $uninstall = Join-Path $dir 'uninstall.exe'
  if (Test-Path $uninstall) { Start-Process $uninstall -ArgumentList @('/S', "_?=$dir") -Wait }
}
