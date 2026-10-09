$ErrorActionPreference = 'Stop'
# Exercise the real signing hook with a temporary NSIS filename.
# Azure and Authenticode responses are fixtures; native verification remains in CI.
function Import-Module {
    param([string]$Name, [string]$RequiredVersion)
    if ($Name -ne 'ArtifactSigning' -or $RequiredVersion -ne '0.1.20') { throw 'Unexpected signing module.' }
}
function Invoke-ArtifactSigning {
    param([string]$Files)
    if ([System.IO.Path]::GetExtension($Files) -notin @('.exe', '.dll')) {
        throw 'Fixture signer requires an EXE or DLL filename.'
    }
    $env:AIDOO_HOOK_FIXTURE_SIGNED_PATH = $Files
    if ($env:AIDOO_HOOK_FIXTURE_FAIL -eq 'true') { throw 'FIXTURE_SIGNING_FAILURE' }
    $bytes = [System.IO.File]::ReadAllBytes($Files)
    [System.IO.File]::WriteAllBytes($Files, $bytes + [System.Text.Encoding]::ASCII.GetBytes('FIXTURE_ONLY_SIGNATURE'))
}
function Get-AuthenticodeSignature {
    param([string]$LiteralPath)
    $signed = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($LiteralPath)).EndsWith('FIXTURE_ONLY_SIGNATURE')
    $certificate = [pscustomobject]@{ Subject = 'CN=Aidoo LTD'; Issuer = 'Fixture'; Thumbprint = 'Fixture' }
    $certificate | Add-Member -MemberType ScriptMethod -Name GetNameInfo -Value { param($Type, $ForIssuer) 'Aidoo LTD' }
    [pscustomobject]@{
        Status = $(if ($signed) { 'Valid' } else { 'NotSigned' })
        SignerCertificate = $certificate
        TimeStamperCertificate = [pscustomobject]@{ Subject = 'Fixture timestamp'; Thumbprint = 'Fixture' }
    }
}
$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('aidoo-hook-fixture-' + [guid]::NewGuid())
$previousPlatform = $IsWindows
$previousSignedPath = $env:AIDOO_HOOK_FIXTURE_SIGNED_PATH
$previousFailure = $env:AIDOO_HOOK_FIXTURE_FAIL
try {
    # Override the platform only in this isolated test process to exercise the real hook on macOS.
    Set-Variable -Name IsWindows -Value $true -Force
    New-Item -ItemType Directory $fixtureRoot | Out-Null
    $pe = New-Object byte[] 256
    $pe[0] = 0x4d; $pe[1] = 0x5a
    [System.BitConverter]::GetBytes([uint32]128).CopyTo($pe, 60)
    $pe[128] = 0x50; $pe[129] = 0x45
    foreach ($name in @('application.exe', 'plugin.dll', 'nsu1234.tmp', 'extensionless-uninstaller')) {
        $path = Join-Path $fixtureRoot $name
        [System.IO.File]::WriteAllBytes($path, $pe)
        & (Join-Path $PSScriptRoot 'sign-windows-artifact.ps1') $path
        if (!(Get-AuthenticodeSignature $path).Status.Equals('Valid')) { throw "Hook left $name unsigned." }
        if ($name -notmatch '\.(exe|dll)$' -and (Test-Path $env:AIDOO_HOOK_FIXTURE_SIGNED_PATH)) {
            throw 'The temporary signing copy was not removed.'
        }
    }
    $invalid = Join-Path $fixtureRoot 'not-an-executable.tmp'
    Set-Content $invalid 'ordinary temporary text'
    $reason = $null
    try { & (Join-Path $PSScriptRoot 'sign-windows-artifact.ps1') $invalid }
    catch { $reason = $_.Exception.Message }
    if ($reason -notlike '*Expected a Windows PE executable*') { throw "Invalid payload was not rejected: $reason" }
    $failed = Join-Path $fixtureRoot 'failed-uninstaller.tmp'
    [System.IO.File]::WriteAllBytes($failed, $pe)
    $before = (Get-FileHash $failed).Hash
    $env:AIDOO_HOOK_FIXTURE_FAIL = 'true'
    $reason = $null
    try { & (Join-Path $PSScriptRoot 'sign-windows-artifact.ps1') $failed }
    catch { $reason = $_.Exception.Message }
    if ($reason -ne 'FIXTURE_SIGNING_FAILURE') { throw "Signing failure was not propagated: $reason" }
    if ((Get-FileHash $failed).Hash -ne $before) { throw 'Failed signing changed the NSIS input.' }
    if (Test-Path $env:AIDOO_HOOK_FIXTURE_SIGNED_PATH) { throw 'Failed signing left a temporary signing copy.' }
} finally {
    Set-Variable -Name IsWindows -Value $previousPlatform -Force
    $env:AIDOO_HOOK_FIXTURE_SIGNED_PATH = $previousSignedPath
    $env:AIDOO_HOOK_FIXTURE_FAIL = $previousFailure
    Remove-Item -LiteralPath $fixtureRoot -Recurse -Force
}
Write-Host 'Temporary NSIS uninstaller is signed; non-PE input and signing failures are rejected.'
