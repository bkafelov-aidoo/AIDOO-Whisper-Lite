$ErrorActionPreference = 'Stop'
if (!$IsWindows -or $env:GITHUB_ACTIONS -ne 'true') { throw 'Disposable Windows CI only.' }
New-Item -ItemType Directory -Force 'release/shortcut-repro' | Out-Null
$executable = (Resolve-Path 'src-tauri/target/x86_64-pc-windows-msvc/debug/aidoo-whisper-lite.exe').Path
$previous = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$previousProbe = $env:AIDOO_SHORTCUT_CI_PROBE
$env:AIDOO_SHORTCUT_CI_PROBE = 'true'
$trace = Join-Path ([IO.Path]::GetTempPath()) 'aidoo-shortcut-ci-stderr.txt'
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9222'
try { $app = Start-Process $executable -PassThru -RedirectStandardError $trace }
finally { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previous; $env:AIDOO_SHORTCUT_CI_PROBE = $previousProbe }
try {
  node (Join-Path $PSScriptRoot 'shortcut-repro.mjs')
  if ($LASTEXITCODE -ne 0) { throw 'Shortcut regression failed.' }
} finally {
  if (!$app.HasExited) { Stop-Process -Id $app.Id -Force }
  $raw = if (Test-Path $trace) { Get-Content -Raw $trace } else { '' }
  $result = Get-Content -Raw 'release/shortcut-repro/results.json' | ConvertFrom-Json -AsHashtable
  $result.listenerTrace = @{
    called = $raw.Contains('[DEBUG-shortcut-a12] install-called')
    installed = $raw.Contains('[DEBUG-shortcut-a12] installed')
    pumpEnded = $raw.Contains('[DEBUG-shortcut-a12] pump-ended')
    rawF8 = $raw.Contains('[DEBUG-shortcut-a12] raw f8 true')
    processedF8 = $raw.Contains('[DEBUG-shortcut-a12] processed f8 true')
    rawF9 = $raw.Contains('[DEBUG-shortcut-a12] raw f9 true')
    processedF9 = $raw.Contains('[DEBUG-shortcut-a12] processed f9 true')
    panic = $raw.Contains('panicked')
    installFailed = $raw.Contains('Could not install Windows keyboard hook')
  }
  $json = $result | ConvertTo-Json -Depth 8
  $json | Set-Content -Encoding utf8 'release/shortcut-repro/results.json'
  Write-Output $json
  Remove-Item $trace -Force -ErrorAction SilentlyContinue
}
