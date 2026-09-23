# Security

RepoTower reads local source files and builds a static dependency model. It does not execute inspected source, build scripts, compilers or package managers. The native UI and analysis library run in one Rust process; there is no Electron renderer, webview, local web server, telemetry or repository upload.

Application-managed data stays beside the executable. Normal launches expose no automation interface. The explicitly requested `--automation-stdio` diagnostic mode accepts input only from the launching process through inherited stdin; it does not open a network listener or execute shell commands.

Report ordinary bugs in [Issues](https://github.com/cabal312512/RepoTower/issues). For a sensitive vulnerability, use GitHub's private **Report a vulnerability** option if available, or request a private contact channel without posting exploit details, credentials or private source.

Security fixes target the latest release. Include the version, OS, a minimal non-sensitive reproduction and observed impact. Do not include an entire private repository or runtime profile.

Use this repository's releases and compare downloads against `SHA256SUMS.txt`. Checksums detect damaged or mismatched downloads; they are not a signing certificate. Windows binaries are unsigned; macOS builds are ad-hoc signed and not notarized.
