# swe-diagrams

Fast diagrams of software, written by agents in a compact language.

```bash
npx swe-diagrams render app.sd -o app.svg
```

This package is a small launcher. On the first run, it downloads the
release binary for your platform from
[GitHub releases](https://github.com/Benny-Gil/swe-diagrams/releases),
checks it against `SHA256SUMS`, and keeps it in
`~/.cache/swe-diagrams/bin/<version>/` (`%LOCALAPPDATA%\swe-diagrams\Cache`
on Windows). The package has no dependencies and no install script.

Supported platforms: Linux (x64, arm64), macOS (x64, arm64), and Windows
(x64). On other platforms, use `cargo install swe-diagrams`.

Settings (environment variables):

- `XDG_CACHE_HOME` changes the cache folder on Linux and macOS.
- `SWE_DIAGRAMS_DOWNLOAD_BASE` changes the download URL. The launcher
  gets `<base>/v<version>/<asset>`. The default base is
  `https://github.com/Benny-Gil/swe-diagrams/releases/download`.

The diagram language and the agent skill are in the
[repository](https://github.com/Benny-Gil/swe-diagrams).
