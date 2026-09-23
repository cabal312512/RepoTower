@echo off
setlocal
cd /d "%~dp0"
set "TEMP=%~dp0.tmp"
set "TMP=%~dp0.tmp"
if not exist "%TEMP%" mkdir "%TEMP%"
if exist "release\RepoTower-0.5.0-win32-x64\RepoTower.exe" (
    start "" "release\RepoTower-0.5.0-win32-x64\RepoTower.exe" %*
    exit /b 0
)
if exist "node_modules\electron\dist\electron.exe" if exist "dist\index.html" if exist "target\release\repotower-core.exe" (
    start "" "node_modules\electron\dist\electron.exe" "." %*
    exit /b 0
)
echo The portable application has not been built yet. Starting the local build.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build.ps1"
if errorlevel 1 (
    echo Build failed. Please read the message above.
    pause
    exit /b 1
)
start "" "release\RepoTower-0.5.0-win32-x64\RepoTower.exe" %*
