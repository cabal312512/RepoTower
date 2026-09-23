# Security

RepoTower reads local source files and builds a static dependency model. It does not execute analyzed source, build scripts or package managers. The renderer uses a sandboxed Electron context with a narrow preload API. Repository contents are not uploaded.

Report ordinary bugs in [Issues](https://github.com/cabal312512/RepoTower/issues). For a sensitive vulnerability, use GitHub's private **Report a vulnerability** option if it is available, or open an issue requesting a private contact channel without posting exploit details, credentials or private source code.

Security fixes target the latest release. Include the version, operating system, a minimal non-sensitive reproduction and the impact you observed. Do not include your entire private repository or runtime profile.

Use releases from this repository and compare downloads against `SHA256SUMS.txt`. Checksums detect damaged or mismatched downloads; they are not a code-signing certificate. Current desktop binaries are unsigned, and macOS builds are not notarized.
