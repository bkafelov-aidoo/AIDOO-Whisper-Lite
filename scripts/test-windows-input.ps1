$ErrorActionPreference = 'Stop'
$output = Join-Path $PSScriptRoot '../release/windows-ci'
New-Item -ItemType Directory -Force $output | Out-Null
function Run-NativeCheck([string]$Name) {
    cargo test --locked --release --target x86_64-pc-windows-msvc --manifest-path src-tauri/Cargo.toml $Name -- --ignored --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw "Native Windows check failed: $Name" }
}
Run-NativeCheck 'windows_credential_manager_round_trip'
Run-NativeCheck 'windows_global_f8_receives_press_and_release'
$result = Join-Path ([IO.Path]::GetTempPath()) "aidoo-input-$([Guid]::NewGuid()).json"
$ready = "$result.ready"
$fieldScript = Join-Path $PSScriptRoot 'windows-input-test-field.ps1'
$fieldProcess = Start-Process powershell.exe -ArgumentList @('-NoProfile', '-STA', '-File', "`"$fieldScript`"", '-ResultPath', "`"$result`"", '-ReadyPath', "`"$ready`"") -WindowStyle Hidden -PassThru
try {
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while (!(Test-Path $ready)) {
        if (Test-Path $result) {
            $failure = Get-Content -Raw $result | ConvertFrom-Json
            throw "The foreground test field failed: $($failure.error)"
        }
        if ($fieldProcess.HasExited -or [DateTime]::UtcNow -gt $deadline) { throw 'The foreground test field did not open.' }
        Start-Sleep -Milliseconds 200
    }
    Run-NativeCheck 'windows_paste_into_foreground_test_field'
    if (!$fieldProcess.WaitForExit(15000)) { throw 'The paste test field did not complete.' }
    $evidence = Get-Content -Raw $result | ConvertFrom-Json
    if ($evidence.error) { throw "The Windows test field failed: $($evidence.error)" }
    if ($evidence.text -ne 'Проверка на диктовка: 123.') { throw "Paste produced unexpected text: $($evidence.text)" }
    if ($evidence.keyboardCulture -ne 'bg-BG') { throw "Expected Bulgarian input; found $($evidence.keyboardCulture)" }
    @{ credentialManager = 'passed'; globalF8 = 'passed'; clipboardAndPaste = 'passed'; input = $evidence } |
        ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 (Join-Path $output 'native-input.json')
} finally {
    if (Test-Path $result) { Copy-Item $result (Join-Path $output 'input-field.json') -Force }
    if (!$fieldProcess.HasExited) { Stop-Process -Id $fieldProcess.Id -Force }
    Remove-Item $result, $ready -Force -ErrorAction SilentlyContinue
}
