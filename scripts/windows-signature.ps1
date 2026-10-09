function Invoke-AidooWindowsArtifactSigning {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][scriptblock]$Sign)
    $file = Get-Item -LiteralPath $Path -ErrorAction Stop
    # NSIS passes a temporary uninstaller filename to !uninstfinalize.
    # Validate the PE content instead of assuming its extension is .exe.
    $reader = [System.IO.BinaryReader]::new([System.IO.File]::OpenRead($file.FullName))
    try {
        if ($reader.BaseStream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5a4d) {
            throw 'Expected a Windows PE executable.'
        }
        $reader.BaseStream.Position = 60
        $peOffset = $reader.ReadUInt32()
        if ($peOffset -lt 64 -or $peOffset -gt ($reader.BaseStream.Length - 24)) {
            throw 'Expected a Windows PE executable.'
        }
        $reader.BaseStream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) { throw 'Expected a Windows PE executable.' }
    } finally { $reader.Dispose() }
    $stagingDir = $null
    $signingPath = $file.FullName
    try {
        if ($file.Extension -notin @('.exe', '.dll')) {
            # Give the signing tools an EXE path, then return the verified bytes to NSIS.
            $stagingDir = Join-Path ([System.IO.Path]::GetTempPath()) ('aidoo-nsis-signing-' + [guid]::NewGuid())
            New-Item -ItemType Directory $stagingDir | Out-Null
            $signingPath = Join-Path $stagingDir 'uninstaller.exe'
            Copy-Item -LiteralPath $file.FullName -Destination $signingPath
        }
        & $Sign $signingPath | Out-Null
        $verified = Assert-AidooWindowsSignature -Path $signingPath
        if ($stagingDir) {
            Copy-Item -LiteralPath $signingPath -Destination $file.FullName -Force
            if ((Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -ne $verified.sha256) {
                throw 'Signed uninstaller changed while returning it to NSIS.'
            }
        }
        $verified
    } finally {
        if ($stagingDir) { Remove-Item -LiteralPath $stagingDir -Recurse -Force }
    }
}

function Assert-AidooWindowsSignature {
    param([Parameter(Mandatory)][string]$Path)
    $file = Get-Item -LiteralPath $Path -ErrorAction Stop
    $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName -ErrorAction Stop
    if ($signature.Status -ne 'Valid') {
        throw "Invalid Authenticode signature on $($file.Name): $($signature.Status)"
    }
    $publisher = $signature.SignerCertificate.GetNameInfo(
        [System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName, $false)
    if ($publisher -cne 'Aidoo LTD') {
        throw "Unexpected signing publisher on $($file.Name): $publisher"
    }
    if ($null -eq $signature.TimeStamperCertificate) {
        throw "Missing trusted timestamp on $($file.Name)."
    }
    [ordered]@{
        file = $file.Name
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        status = "$($signature.Status)"
        publisher = $publisher
        subject = $signature.SignerCertificate.Subject
        issuer = $signature.SignerCertificate.Issuer
        certificateThumbprint = $signature.SignerCertificate.Thumbprint
        timestampSubject = $signature.TimeStamperCertificate.Subject
        timestampThumbprint = $signature.TimeStamperCertificate.Thumbprint
    }
}

function Save-AidooSignedApplication {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$Directory)
    $signed = Assert-AidooWindowsSignature -Path $Path
    $snapshotDir = Join-Path $Directory 'signed-application'
    New-Item -ItemType Directory -Force $snapshotDir | Out-Null
    $snapshotPath = Join-Path $snapshotDir 'aidoo-whisper-lite.exe'
    Copy-Item -LiteralPath $Path -Destination $snapshotPath -Force
    $snapshot = Assert-AidooWindowsSignature -Path $snapshotPath
    if ($snapshot.sha256 -ne $signed.sha256) { throw 'Signed application changed while saving the bundle snapshot.' }
    $snapshot | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $Directory 'app-signature.json')
    $snapshot
}

function Assert-AidooSignedApplicationSnapshot {
    param([Parameter(Mandatory)][string]$Directory)
    $record = Get-Content -Raw (Join-Path $Directory 'app-signature.json') | ConvertFrom-Json
    $snapshot = Assert-AidooWindowsSignature -Path (Join-Path $Directory 'signed-application/aidoo-whisper-lite.exe')
    foreach ($field in @('sha256', 'status', 'publisher', 'certificateThumbprint', 'timestampThumbprint')) {
        if ($snapshot[$field] -cne $record.$field) { throw 'Signed application snapshot differs from its signing evidence.' }
    }
    $snapshot
}
