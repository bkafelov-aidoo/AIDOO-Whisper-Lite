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
