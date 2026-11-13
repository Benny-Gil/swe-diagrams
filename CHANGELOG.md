# Changelog

This file lists the changes in each version. The project uses
[semantic versioning](https://semver.org).

## Unreleased

### Added

- Easy install: steps for an agent (`docs/install.md`), a Claude Code
  plugin, an install script, and prebuilt binaries for each release.
- A CI check for each pull request.
- Cloud vendor icons in `arch` and `change`: `@aws/<name>`, `@gcp/<name>`,
  and `@azure/<name>`. The first use downloads the official package of the
  vendor into the icon directory. Short names such as `s3`, `gke`, and `aks`
  work.
- Vendor groups: `:aws`, `:gcp`, and `:azure`, with `:region`, `:az`,
  `:vpc`, `:vnet`, `:subnet`, `:public`, `:private`, and `:rg` inside. An
  AWS group follows the AWS guide.
- `swe-diagrams icons <aws|gcp|azure> [filter]` lists the icon names.

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
