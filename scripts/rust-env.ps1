# Dot-source this file. All settings apply only to this PowerShell process.
$RepoTowerRoot = Split-Path -Parent $PSScriptRoot
$env:CARGO_HOME = Join-Path $RepoTowerRoot '.cache\cargo'
$env:RUSTUP_HOME = Join-Path $RepoTowerRoot '.tools\rustup'
$env:CARGO_TARGET_DIR = Join-Path $RepoTowerRoot 'target'
$env:TEMP = Join-Path $RepoTowerRoot '.tmp'
$env:TMP = $env:TEMP
$env:CC = Join-Path $RepoTowerRoot '.tools\w64devkit\bin\gcc.exe'
$env:CXX = Join-Path $RepoTowerRoot '.tools\w64devkit\bin\g++.exe'
$env:AR = Join-Path $RepoTowerRoot '.tools\w64devkit\bin\ar.exe'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = $env:CC
# Rust's bundled unwind/system libraries match its standard library. Modern GCC
# distributions can omit the separately named libgcc_eh archive Rust expects.
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = '-C link-self-contained=yes'
$env:PATH = "$(Join-Path $RepoTowerRoot '.tools\rust\bin');$(Join-Path $RepoTowerRoot '.tools\w64devkit\bin');$env:PATH"
New-Item -ItemType Directory -Force $env:CARGO_HOME,$env:TEMP | Out-Null
