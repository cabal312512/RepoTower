# Downloads and portable data

Download binaries from [GitHub Releases](https://github.com/cabal312512/RepoTower/releases). Choose the archive for your OS and CPU. Keep the extracted folder together. Source archives require a build and do not contain bundled tools or caches.

## Windows

The portable ZIP contains `RepoTower.exe` and its runtime. The application keeps its writable data in `runtime-data` beside that executable.

The single EXE is a self-extracting launcher. Put it in a writable folder, then open it. It checks its embedded archive and unpacks the runtime into `RepoTower-data/app-<content-hash>` beside itself. Preferences and Chromium data live in `RepoTower-data/runtime-data`. Later launches verify and reuse the cache; missing or corrupted cached files are rebuilt. It does not install anything or fall back to the system temporary folder. First launch needs extra time and disk space for the unpacked runtime.

To move the single EXE with your preferences, move its `RepoTower-data` folder too. Close every copy before deleting the cache or replacing files. The folder can be removed to reset all settings. Downloaded binaries are unsigned, so Windows may display a reputation warning.

## macOS

Choose `arm64` for Apple Silicon or `x64` for Intel. Extract into a writable location and open `RepoTower.app`. Keep the app in its extracted directory so adjacent `runtime-data` can be written. The app is not signed or notarized; macOS may require approval for this specific app in **System Settings → Privacy & Security**. Do not disable system-wide security protections.

## Linux

Extract the `.tar.gz` and run `./RepoTower`. Use a writable folder and retain executable permissions. Builds use Ubuntu 22.04 with native GTK 3, NSS, ALSA and GBM libraries available. A minimal server environment needs these desktop libraries and a display server; it is not a supported GUI runtime out of the box.

Linux creates a private, temporary `/tmp/repotower-*` alias to the adjacent temporary directory so Chromium's Unix socket stays within the OS pathname limit. Actual temporary data remains beside the app. The alias is removed on normal exit; an abrupt termination may leave an empty alias directory for the OS to clean up.

## Verify a download

Compare your result with the matching filename in `SHA256SUMS.txt`:

```powershell
Get-FileHash .\RepoTower-0.5.0-windows-x64.exe -Algorithm SHA256
```

```sh
shasum -a 256 RepoTower-0.5.0-macos-arm64.zip
sha256sum RepoTower-0.5.0-linux-x64.tar.gz
```

Checksums are integrity checks, not signatures. The release workflow builds each platform natively and publishes only after all required jobs succeed. A failed or cancelled Actions run does not create a completed release.

Application-controlled caches and settings stay beside the portable app. Operating-system records such as recent-file history, security scans and crash bookkeeping are controlled by the OS.
