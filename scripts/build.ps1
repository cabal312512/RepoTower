param(
    [switch]$SkipTests,
    [switch]$SkipPackage
)
$ErrorActionPreference = 'Stop'
$RepoTowerRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $RepoTowerRoot
try {
    . (Join-Path $PSScriptRoot 'rust-env.ps1')
    $env:npm_config_cache = Join-Path $RepoTowerRoot '.cache\npm'
    $env:npm_config_audit = 'false'
    $env:npm_config_fund = 'false'
    $env:ELECTRON_CACHE = Join-Path $RepoTowerRoot '.cache\electron'
    $env:electron_config_cache = $env:ELECTRON_CACHE
    $env:ELECTRON_BUILDER_CACHE = Join-Path $RepoTowerRoot '.cache\electron-builder'

    $NodeCommand = Get-Command node.exe -ErrorAction SilentlyContinue
    $LocalNode = Join-Path $RepoTowerRoot '.tools\node\node.exe'
    if (Test-Path -LiteralPath $LocalNode) { $NodeExecutable = $LocalNode }
    elseif ($NodeCommand) { $NodeExecutable = $NodeCommand.Source }
    else { throw 'Node.js 24 or newer is required to build. Put its portable Windows archive in .tools\node; the portable release needs no Node installation.' }
    $NpmCli = Join-Path (Split-Path -Parent $NodeExecutable) 'node_modules\npm\bin\npm-cli.js'
    if (-not (Test-Path -LiteralPath $NpmCli)) { throw 'The selected Node.js distribution does not contain npm.' }
    $env:PATH = "$(Split-Path -Parent $NodeExecutable);$env:PATH"

    if (-not (Test-Path -LiteralPath (Join-Path $RepoTowerRoot '.tools\rust\.portable-version'))) {
        & (Join-Path $PSScriptRoot 'setup-rust.ps1')
    }
    if (-not (Test-Path -LiteralPath (Join-Path $RepoTowerRoot 'node_modules\vite\package.json'))) {
        if (Test-Path -LiteralPath (Join-Path $RepoTowerRoot 'package-lock.json')) { & $NodeExecutable $NpmCli ci }
        else { & $NodeExecutable $NpmCli install }
        if ($LASTEXITCODE -ne 0) { throw 'JavaScript dependency installation failed.' }
    }
    if (-not (Test-Path -LiteralPath (Join-Path $RepoTowerRoot 'node_modules\electron\dist\electron.exe'))) {
        & $NodeExecutable (Join-Path $RepoTowerRoot 'node_modules\electron\install.js')
        if ($LASTEXITCODE -ne 0) { throw 'Electron portable runtime download failed.' }
    }
    if (-not $SkipTests) {
        & cargo.exe test --workspace --locked
        if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed.' }
        & $NodeExecutable $NpmCli test
        if ($LASTEXITCODE -ne 0) { throw 'Frontend tests failed.' }
    }
    & cargo.exe build --workspace --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed.' }
    & $NodeExecutable $NpmCli run build
    if ($LASTEXITCODE -ne 0) { throw 'Frontend build failed.' }
    if (-not $SkipPackage) {
        & $NodeExecutable (Join-Path $PSScriptRoot 'package.mjs')
        if ($LASTEXITCODE -ne 0) { throw 'Portable packaging failed.' }
    }
    Write-Host 'RepoTower build completed. Open the portable executable in release.'
} finally {
    Pop-Location
}
