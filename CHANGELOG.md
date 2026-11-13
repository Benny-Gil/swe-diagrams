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
- Stories: `story "Title"` blocks with steps along the edges, `->` and
  `-->` tokens, captions, and `&` for steps at the same time. Each type
  has a default story.
- `--animate` writes an SVG with SMIL motion and no script.
  `prefers-reduced-motion` shows the still diagram.
- GIF output (`-o x.gif`) and video output (`-o x.mp4`, `-o x.webm`,
  with `ffmpeg`). `--fps`, `--speed`, and `--story` set the motion.
- A story player in the HTML page: play, pause, steps, speed, a story
  list, a caption panel, and keys.
- A progress story of a `plan` or a `timeline` moves each item up to its
  status, and the done count of the kicker follows each frame.
- Zoom levels: `detail <id> [type] ["Title"]` blocks, `>file.sd` links,
  and `^name` for a node of the parent level. A detail level shows the
  context of the parent level, and a node with a detail level gets a zoom
  mark. `--level cart/svc` writes one level. The HTML page opens a level
  with a click or `Enter`, and has a breadcrumb, `Escape`, and a hash path.
  A node with no status takes a status from its detail level. The summary
  line adds `levels=` and `details=`.
- Terminal text output (`text_out::to_text`) for all eleven types. It
  draws with Unicode box drawing, shows status symbols, and can add ANSI
  colours. A drawing that is too wide becomes an outline.
- `--format txt` (or `-o x.txt`) writes the terminal text. `--color
  auto|always|never` sets the ANSI colour, and `--width N` sets the
  columns. `--level` works for the text.
- `--tour` writes a GIF or a video that zooms into each detail level,
  depth first, and back out. The summary line adds `tour=` and `frames=`.

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
