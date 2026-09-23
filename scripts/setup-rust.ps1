# Portable Windows toolchain: no installers, registry entries or persistent PATH changes.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'rust-env.ps1')
$RustVersion = '1.90.0'
$W64Version = '2.10.0'
$Downloads = Join-Path $RepoTowerRoot '.cache\downloads'
$Unpack = Join-Path $RepoTowerRoot '.tmp\rust-components'
$RustDestination = Join-Path $RepoTowerRoot '.tools\rust'
New-Item -ItemType Directory -Force $Downloads,$Unpack,$RustDestination | Out-Null

function Get-LocalDownload([string]$Url, [string]$Destination) {
    if (-not (Test-Path -LiteralPath $Destination)) {
        & curl.exe -fL --retry 3 --silent --show-error $Url -o "$Destination.partial"
        if ($LASTEXITCODE -ne 0) { throw "Download failed: $Url" }
        Move-Item -LiteralPath "$Destination.partial" -Destination $Destination -Force
    }
}

if (-not (Test-Path -LiteralPath (Join-Path $RustDestination '.portable-version')) -or -not (Test-Path -LiteralPath (Join-Path $RustDestination 'bin\rustfmt.exe'))) {
    foreach ($Component in @('rustc', 'rust-std', 'cargo', 'rust-mingw', 'rustfmt')) {
        $ArchiveName = "$Component-$RustVersion-x86_64-pc-windows-gnu.tar.xz"
        $Archive = Join-Path $Downloads $ArchiveName
        $Url = "https://static.rust-lang.org/dist/$ArchiveName"
        Get-LocalDownload $Url $Archive
        Get-LocalDownload "$Url.sha256" "$Archive.sha256"
        $Expected = ((Get-Content -LiteralPath "$Archive.sha256" -Raw).Trim() -split '\s+')[0]
        if ((Get-FileHash -Algorithm SHA256 -LiteralPath $Archive).Hash -ne $Expected) {
            throw "SHA256 mismatch for $ArchiveName. Remove this local archive and retry."
        }
        & tar.exe -xf $Archive -C $Unpack
        if ($LASTEXITCODE -ne 0) { throw "Could not extract $ArchiveName" }
        $Extracted = Join-Path $Unpack "$Component-$RustVersion-x86_64-pc-windows-gnu"
        foreach ($Entry in (Get-Content -LiteralPath (Join-Path $Extracted 'components'))) {
            $Source = Join-Path $Extracted $Entry
            Get-ChildItem -LiteralPath $Source -Force | Where-Object { $_.Name -ne 'manifest.in' } | Copy-Item -Destination $RustDestination -Recurse -Force
        }
    }
    Set-Content -LiteralPath (Join-Path $RustDestination '.portable-version') -Value $RustVersion
}

if (-not (Test-Path -LiteralPath $env:CC)) {
    $ArchiveName = "w64devkit-x64-$W64Version.7z.exe"
    $Archive = Join-Path $Downloads $ArchiveName
    Get-LocalDownload "https://github.com/skeeto/w64devkit/releases/download/v$W64Version/$ArchiveName" $Archive
    $W64Sha256 = '18d0a4c71a166f8401ab6305781bec5882b40b5e06ba9807c61cb5f3b3c6325e'
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $Archive).Hash -ne $W64Sha256) {
        throw 'SHA256 mismatch for w64devkit. Remove the local archive and retry.'
    }
    # This executable is a 7-Zip self-extractor. It only extracts into the explicit destination.
    $Extractor = Start-Process -FilePath $Archive -ArgumentList @('-y', ('"-o' + (Join-Path $RepoTowerRoot '.tools') + '"')) -WindowStyle Hidden -Wait -PassThru
    if ($Extractor.ExitCode -ne 0) { throw 'Could not extract w64devkit.' }
}

& rustc.exe --version
& cargo.exe --version
& $env:CC --version | Select-Object -First 1
Write-Host 'Portable Rust is ready. Run: . .\scripts\rust-env.ps1'
