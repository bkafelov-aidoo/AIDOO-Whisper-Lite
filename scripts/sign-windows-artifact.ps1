param([Parameter(Mandatory, Position = 0)][string]$Path)
$ErrorActionPreference = 'Stop'
if (!$IsWindows) { throw 'Artifact Signing must run on Windows.' }
$file = Get-Item -LiteralPath $Path
if ($file.Extension -notin @('.exe', '.dll')) { throw 'Expected a Windows EXE or DLL.' }
. (Join-Path $PSScriptRoot 'windows-signature.ps1')
Import-Module ArtifactSigning -RequiredVersion 0.1.20 -ErrorAction Stop
# Use only the Azure CLI session created by azure/login with GitHub OIDC.
# Tauri invokes this hook for its application, NSIS plugins, uninstaller and installer.
$parameters = @{
    Endpoint = 'https://neu.codesigning.azure.net/'
    CodeSigningAccountName = 'aidooartifactsigning'
    CertificateProfileName = 'aidoo-whisper-lite'
    Files = $file.FullName
    FileDigest = 'SHA256'
    TimestampRfc3161 = 'http://timestamp.acs.microsoft.com'
    TimestampDigest = 'SHA256'
    ExcludeEnvironmentCredential = $true
    ExcludeWorkloadIdentityCredential = $true
    ExcludeManagedIdentityCredential = $true
    ExcludeSharedTokenCacheCredential = $true
    ExcludeVisualStudioCredential = $true
    ExcludeVisualStudioCodeCredential = $true
    ExcludeAzureCliCredential = $false
    ExcludeAzurePowerShellCredential = $true
    ExcludeAzureDeveloperCliCredential = $true
    ExcludeInteractiveBrowserCredential = $true
}
Invoke-ArtifactSigning @parameters
$verified = Assert-AidooWindowsSignature -Path $file.FullName
$applicationPath = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../src-tauri/target/x86_64-pc-windows-msvc/release/aidoo-whisper-lite.exe'))
if ($file.FullName -ieq $applicationPath) {
    # Tauri restores its unsigned working binary after NSIS packaging.
    # Keep the exact signed and patched payload that NSIS embeds for verification.
    Save-AidooSignedApplication -Path $file.FullName -Directory (Join-Path $PSScriptRoot '../release/windows-ci') | Out-Null
}
Write-Host "Verified $($verified.file): $($verified.publisher), $($verified.status), timestamp present."
