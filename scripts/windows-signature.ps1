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
