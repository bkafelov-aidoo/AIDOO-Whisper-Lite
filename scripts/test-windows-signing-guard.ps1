param([string]$UnsignedArtifact)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-signature.ps1')
Get-ChildItem $PSScriptRoot -Filter '*.ps1' | ForEach-Object {
    $tokens = $null
    $parseErrors = $null
    [System.Management.Automation.Language.Parser]::ParseFile($_.FullName, [ref]$tokens, [ref]$parseErrors) | Out-Null
    if ($parseErrors.Count) { throw "PowerShell syntax errors in $($_.Name): $parseErrors" }
}
function Assert-RejectedSignature([string]$Path, [string]$ExpectedReason) {
    $rejected = $false
    try { Assert-AidooWindowsSignature -Path $Path | Out-Null }
    catch {
        if ($_.Exception.Message -notlike "*$ExpectedReason*") { throw }
        $rejected = $true
    }
    if (!$rejected) { throw "Signing guard accepted an invalid publisher or unsigned file: $Path" }
}
# A trusted Microsoft binary still must not be accepted as an Aidoo release.
Assert-RejectedSignature (Join-Path $env:WINDIR 'System32/WindowsPowerShell/v1.0/powershell.exe') 'Unexpected signing publisher'
if ($UnsignedArtifact) {
    Assert-RejectedSignature $UnsignedArtifact 'Invalid Authenticode signature'
}
Write-Host 'Windows signing syntax and signature rejection checks passed.'
