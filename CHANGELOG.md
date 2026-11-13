# Changelog

This file lists the changes in each version. The project uses
[semantic versioning](https://semver.org).

## Unreleased

### Added

- Easy install: steps for an agent (`docs/install.md`), a Claude Code
  plugin, an install script, and prebuilt binaries for each release.
- A CI check for each pull request.
- Output formats: `--format svg|html|png`, or the extension of `-o`.
- HTML output: one self-contained page with pan, zoom, a fit button,
  hover highlights, a status filter in the legend, and arrow-key steps.
  The page follows `prefers-color-scheme` and the `data-theme` attribute.
- PNG output with resvg and bundled fonts (Geist, JetBrains Mono).
  `--scale` sets the pixel scale.
- `--theme auto|light|dark` sets the palette of each format.
- Each part of the SVG is in a group with `data-k` and other data
  attributes, so a script can find it.

### Changed

- A class member line that starts with `#` is a protected member, not
  a comment.
- In a layout that goes down, an edge label takes the free side of its
  line, or moves along the line, so it does not sit on another line. A
  node gets a wider port for a label with no free place. A group grows
  to hold the labels of its edges.
- Text widths come from the bundled Geist faces, with a margin for
  fallback faces, so a long title or label does not clip.

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
