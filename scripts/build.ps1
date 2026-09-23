param([switch]$SkipTests,[switch]$SkipPackage)
$ErrorActionPreference = 'Stop'
$RepoTowerRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $RepoTowerRoot
try {
    . (Join-Path $PSScriptRoot 'rust-env.ps1')
    $env:npm_config_cache = Join-Path $RepoTowerRoot '.cache\npm'
    $LocalNode = Join-Path $RepoTowerRoot '.tools\node\node.exe'
    $NodeCommand = Get-Command node.exe -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $LocalNode) { $NodeExecutable = $LocalNode }
    elseif ($NodeCommand) { $NodeExecutable = $NodeCommand.Source }
    else { throw 'Node.js 24+ is needed only for building. Extract a portable copy into .tools\node.' }
    $env:PATH = "$(Split-Path -Parent $NodeExecutable);$env:PATH"
    if (-not (Test-Path -LiteralPath (Join-Path $RepoTowerRoot '.tools\rust\.portable-version'))) {
        & (Join-Path $PSScriptRoot 'setup-rust.ps1')
    }
    if (-not (Test-Path -LiteralPath (Join-Path $RepoTowerRoot 'node_modules\resedit\package.json'))) {
        $NpmCli = Join-Path (Split-Path -Parent $NodeExecutable) 'node_modules\npm\bin\npm-cli.js'
        & $NodeExecutable $NpmCli ci
        if ($LASTEXITCODE -ne 0) { throw 'Packaging dependency installation failed.' }
    }
    $BuildArguments = @((Join-Path $PSScriptRoot 'build-native.mjs'))
    if ($SkipTests) { $BuildArguments += '--skip-tests' }
    if ($SkipPackage) { $BuildArguments += '--skip-package' }
    & $NodeExecutable @BuildArguments
    if ($LASTEXITCODE -ne 0) { throw 'Native build failed.' }
    Write-Host 'RepoTower native build completed. Open the executable in release.'
} finally { Pop-Location }
