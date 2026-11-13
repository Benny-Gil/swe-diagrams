# Changelog

This file lists the changes in each version. The project uses
[semantic versioning](https://semver.org).

## Unreleased

### Added

- Easy install: steps for an agent (`docs/install.md`), a Claude Code
  plugin, an install script, and prebuilt binaries for each release.
- A CI check for each pull request.
- The npm package `swe-diagrams`: a launcher with no dependencies. The
  first run downloads the release binary and checks its SHA-256 sum.
  `npx swe-diagrams` works on Linux, macOS, and Windows.
- The crate `swe-diagrams` is ready for crates.io, so
  `cargo install swe-diagrams` works after the first `cargo publish`.
- `scripts/bump-version.sh` sets the version in each file that holds it.
- Release steps in `docs/release.md`.

### Changed

- The README shows pictures and install steps first. The diagram language
  is in the language reference only.

## 0.1.0 - 2026-11-13

The first version.

### Added

- The diagram language (`.sd`): nodes, groups, edges, chains, kinds, and
  six statuses (`done`, `wip`, `plan`, `add`, `mod`, `del`).
- Three diagram types: `arch`, `seq`, and `plan`.
- Automatic layout: layers, nested groups, right-angle edges.
- SVG output with a light theme and a dark theme.
- Product logos with `@name`, from svgl, devicon, and gilbarbara/logos,
  with a local cache, `--icons`, and `--offline`.
- The `swe-diagrams` agent skill.
