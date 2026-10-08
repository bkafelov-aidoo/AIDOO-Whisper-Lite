$ErrorActionPreference = 'Stop'
# Replay Tauri's sign -> package -> restore sequence against the real installer check.
# Signature responses are fixtures here; native Authenticode checks run separately in CI.
function Get-AuthenticodeSignature {
    param([string]$LiteralPath)
    $signed = [System.IO.File]::ReadAllText($LiteralPath).StartsWith('SIGNED:')
    $certificate = [pscustomobject]@{ Subject = 'CN=Aidoo LTD'; Issuer = 'Fixture'; Thumbprint = 'Fixture' }
    $certificate | Add-Member -MemberType ScriptMethod -Name GetNameInfo -Value {
        param($Type, $ForIssuer)
        'Aidoo LTD'
    }
    [pscustomobject]@{
        Status = $(if ($signed) { 'Valid' } else { 'NotSigned' })
        SignerCertificate = $certificate
        TimeStamperCertificate = [pscustomobject]@{ Subject = 'Fixture timestamp'; Thumbprint = 'Fixture' }
    }
}
function Start-Process {
    param([string]$FilePath, [string[]]$ArgumentList, [switch]$Wait, [switch]$PassThru)
    if (!$Wait) { throw 'FIXTURE_REACHED_VERIFIED_LAUNCH' }
    if (!(Split-Path $FilePath -Leaf).EndsWith('-setup.exe')) { throw 'Unexpected fixture process.' }
    $installDir = Join-Path $env:LOCALAPPDATA 'AIDOO Whisper Lite'
    New-Item -ItemType Directory -Force $installDir | Out-Null
    Copy-Item (Join-Path $env:AIDOO_SIGNATURE_FIXTURE_ROOT 'release/windows-ci/signed-application/aidoo-whisper-lite.exe') `
        (Join-Path $installDir 'aidoo-whisper-lite.exe')
    if ($env:AIDOO_SIGNATURE_FIXTURE_TAMPER -eq 'true') {
        Add-Content (Join-Path $installDir 'aidoo-whisper-lite.exe') 'changed installed payload'
    }
    Set-Content (Join-Path $installDir 'THIRD_PARTY_NOTICES.txt') 'fixture notices'
    [pscustomobject]@{ ExitCode = 0 }
}
function Test-BundledApplication([bool]$TamperInstalled, [string]$ExpectedResult, [bool]$TamperRecord = $false) {
    $script:fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('aidoo-signature-fixture-' + [guid]::NewGuid())
    $previousLocation = Get-Location
    $previousLocalAppData = $env:LOCALAPPDATA
    $previousFixtureRoot = $env:AIDOO_SIGNATURE_FIXTURE_ROOT
    $previousFixtureTamper = $env:AIDOO_SIGNATURE_FIXTURE_TAMPER
    $env:AIDOO_SIGNATURE_FIXTURE_ROOT = $script:fixtureRoot
    $env:AIDOO_SIGNATURE_FIXTURE_TAMPER = $TamperInstalled.ToString().ToLowerInvariant()
    try {
        foreach ($folder in @('scripts', 'docs', 'release/windows-ci/signed-application',
            'src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis', 'local-app-data')) {
            New-Item -ItemType Directory -Force (Join-Path $script:fixtureRoot $folder) | Out-Null
        }
        foreach ($name in @('windows-signature.ps1', 'test-windows-installer.ps1')) {
            Copy-Item (Join-Path $PSScriptRoot $name) (Join-Path $script:fixtureRoot "scripts/$name")
        }
        foreach ($name in @('WINDOWS.md', 'WINDOWS-SIGNING.md', 'OPENAI-API-KEY-GUIDE.md')) {
            Set-Content (Join-Path $script:fixtureRoot "docs/$name") 'fixture guide'
        }
        $workingExe = Join-Path $script:fixtureRoot 'src-tauri/target/x86_64-pc-windows-msvc/release/aidoo-whisper-lite.exe'
        Set-Content $workingExe 'SIGNED:exact NSIS application payload'
        . (Join-Path $script:fixtureRoot 'scripts/windows-signature.ps1')
        Save-AidooSignedApplication -Path $workingExe -Directory (Join-Path $script:fixtureRoot 'release/windows-ci') | Out-Null
        if ($TamperRecord) {
            $recordPath = Join-Path $script:fixtureRoot 'release/windows-ci/app-signature.json'
            $record = Get-Content -Raw $recordPath | ConvertFrom-Json
            $record.sha256 = '0' * 64
            $record | ConvertTo-Json | Set-Content $recordPath
        }
        # Tauri restores this working file after embedding the signed payload in NSIS.
        Set-Content $workingExe 'UNSIGNED:restored working binary'
        Set-Content (Join-Path $script:fixtureRoot 'src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/fixture-setup.exe') `
            'SIGNED:installer containing the signed payload'
        $env:LOCALAPPDATA = Join-Path $script:fixtureRoot 'local-app-data'
        Set-Location $script:fixtureRoot
        $result = $null
        try { & (Join-Path $script:fixtureRoot 'scripts/test-windows-installer.ps1') -RequireSignature }
        catch { $result = $_.Exception.Message; $failureLocation = $_.ScriptStackTrace }
        if ($result -ne $ExpectedResult) {
            throw "Bundled application regression: expected '$ExpectedResult', received '$result'. $failureLocation"
        }
    } finally {
        Set-Location $previousLocation
        $env:LOCALAPPDATA = $previousLocalAppData
        $env:AIDOO_SIGNATURE_FIXTURE_ROOT = $previousFixtureRoot
        $env:AIDOO_SIGNATURE_FIXTURE_TAMPER = $previousFixtureTamper
        Remove-Item -LiteralPath $script:fixtureRoot -Recurse -Force
    }
}
Test-BundledApplication $false 'FIXTURE_REACHED_VERIFIED_LAUNCH'
Test-BundledApplication $true 'The installed application differs from the signed bundle snapshot.'
Test-BundledApplication $false 'Signed application snapshot differs from its signing evidence.' $true
Write-Host 'Signed payload survives restoration; changed installed payload and inconsistent signing evidence are rejected.'
