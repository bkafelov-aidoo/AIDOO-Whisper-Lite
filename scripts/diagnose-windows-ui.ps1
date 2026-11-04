$ErrorActionPreference = 'Stop'
$source = @(Get-ChildItem -Recurse 'release/ui-input' -Filter '*-setup.exe')
if ($source.Count -ne 1) { throw 'Expected the single original test installer.' }
$expectedHash = 'be3cd5053087a6a71c826e9865353e237395da4f3b059f3f580db105712314d8'
if ((Get-FileHash $source[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedHash) {
    throw 'Diagnostic input differs from the previously tested artifact.'
}
$destination = 'src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis'
New-Item -ItemType Directory -Force $destination | Out-Null
Copy-Item $source[0].FullName $destination
$results = @()
$env:AIDOO_UI_DIAGNOSTICS = '1'
foreach ($attempt in 1..3) {
    $started = [DateTime]::UtcNow
    $outcome = 'passed'
    $message = $null
    try { & (Join-Path $PSScriptRoot 'test-windows-installer.ps1') }
    catch { $outcome = 'failed'; $message = $_.Exception.Message }
    $results += @{ attempt = $attempt; outcome = $outcome; message = $message; seconds = ([DateTime]::UtcNow - $started).TotalSeconds }
    $output = "release/ui-diagnostics/attempt-$attempt"
    New-Item -ItemType Directory -Force $output | Out-Null
    foreach ($name in @('endpoint-wait.json', 'installer-verification.json')) {
        $observation = Join-Path 'release/windows-ci' $name
        if (Test-Path $observation) { Copy-Item $observation $output }
    }
    # Remove only disposable observations; the input installer remains unchanged.
    Get-ChildItem 'release/windows-ci' -File | Remove-Item
    $uninstallDir = Join-Path $env:LOCALAPPDATA 'AIDOO Whisper Lite'
    $uninstall = Join-Path $uninstallDir 'uninstall.exe'
    if (Test-Path $uninstall) { Start-Process $uninstall -ArgumentList @('/S', "_?=$uninstallDir") -Wait }
}
$results | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 'release/ui-diagnostics/results.json'
$results | Format-Table
if (@($results | Where-Object outcome -eq 'failed').Count) { throw 'Installed WebView2 failure reproduced; inspect diagnostic observations.' }
