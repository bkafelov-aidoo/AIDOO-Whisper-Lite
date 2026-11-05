$ErrorActionPreference = 'Stop'
if (!$IsWindows -or $env:GITHUB_ACTIONS -ne 'true') { throw 'Disposable Windows CI only.' }
New-Item -ItemType Directory -Force 'release/shortcut-repro' | Out-Null
$executable = (Resolve-Path 'src-tauri/target/x86_64-pc-windows-msvc/debug/aidoo-whisper-lite.exe').Path
$previous = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9222'
try { $app = Start-Process $executable -PassThru }
finally { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previous }
try {
  $trials = @()
  for ($trial = 1; $trial -le 3; $trial++) {
    node (Join-Path $PSScriptRoot 'shortcut-repro.mjs')
    if ($LASTEXITCODE -ne 0) { throw "Shortcut regression failed on trial $trial." }
    $trials += Get-Content -Raw 'release/shortcut-repro/results.json' | ConvertFrom-Json
  }
  @{ trials = $trials; noMicrophoneOrApi = $true } | ConvertTo-Json -Depth 10 | Set-Content -Encoding utf8 'release/shortcut-repro/results.json'
} finally { if (!$app.HasExited) { Stop-Process -Id $app.Id -Force } }
