---
name: swe-diagrams
description: Draw software diagrams fast from a compact text language - architecture and component diagrams with product logos, sequence diagrams, and plan or task graphs - as SVG, with a status on each part (done, in progress, planned, added, changed, removed). Use to show what an agent has built, is building, or will build - with a plan before multi-step work, when the user asks for a diagram or asks how something works or what a change touches, and with the summary at the end of a change that spans three or more components.
---

# swe-diagrams

Write a short `.sd` file. The harness sets all positions and writes the SVG.
Do not plan coordinates. Do not open the image to check the layout: the
layout is deterministic, and the summary line reports what the harness drew.

## Steps

1. Choose one question for the diagram, and the type that answers it.
2. Write the `.sd` file.
3. Render it:

   ```bash
   swe-diagrams render plan.sd -o plan.svg
   ```

   If the command is not found, install it with the steps in
   https://github.com/Benny-Gil/swe-diagrams/blob/main/docs/install.md
   (ask the user first). If `npx` is present, also give the user this
   choice: `npx swe-diagrams` downloads the program on the first run and
   works in place of `swe-diagrams`.
4. Read the summary line, for example `ok plan nodes=6 edges=5 plan.svg`.
   An `implicit=` list names ids that only an edge uses. An id there that
   you did not intend is a typing error. A `noicon=` list names logos that
   no source has; use another name or remove the `@name`. On an error line
   (`file:line: message`), correct that line and render again.
5. Give the user the path of the output file, and one sentence on what it
   shows.

## Choose the format

The extension of `-o` gives the format.

- `.svg`: for a README, a pull request, or a doc. It has a light and a
  dark theme. This is the default.
- `.html`: for a big diagram that the user explores, or for a Claude
  artifact. The page pans, zooms, and highlights a node and its
  neighbours. It loads nothing from the network.
- `.png`: for a chat, a slide, or an issue that does not show SVG. Add
  `--theme dark` for a dark picture, and `--scale N` for more pixels.
- `.svg` with `--animate`: motion in a README or a pull request. Tokens
  move along the edges of the first story. It needs no script.
- `.gif`: motion for a chat that does not show SVG. `.mp4` and `.webm`
  need `ffmpeg`.

To show the order of calls or the path of data, add a story: a
`story "Title"` line, then one indented step for each edge, for example
`web -> api "POST /cart"`. `-->` is a reply. `a -> b & a -> c` runs two
steps at the same time. Without a story, the `=>` edges make one. The
HTML page has a player for the stories.

Keep the files in `docs/diagrams/` when the user wants to keep them.
Otherwise use a temporary directory.

## Choose the type

| Question | Type |
|---|---|
| What are the parts, and how do they connect? | `arch` |
| In what order do the parts talk for one request? | `seq` |
| What are the steps of the work, and where is the work now? | `plan` |
| What imports what? Is there an import cycle? | `deps` |
| What are the steps and the decisions of a process? | `flow` |
| What states can a thing be in, and what changes the state? | `state` |
| Where does the data go, and what changes it? | `dataflow` |
| What does this change touch? | `change` |
| What happens in which order, and where are we now? | `timeline` |
| What are the tables and the relations between them? | `erd` |
| What are the types and their relations? | `class` |
| How does a user move through the pages of the app? | `journey` |

## The language

```
arch "Checkout"                      # type, then an optional title

user "Shopper" "browser" :actor      # id "Label" "Second line" :kind @logo status
group api "API"                      # members have more indent
  web  "Web app" "next.js" @nextjs done
  cart "Cart service" "go" wip
db "Postgres" "orders db" :store @postgres done

user => web "browse"                 # => is the main flow, in the accent colour
web => cart -> db                    # a chain makes one edge for each arrow
cart --> queue "publish" plan        # -> is solid, --> is dashed, <-> has two arrowheads
```

- Statuses: `done` `wip` `plan` show progress. `add` `mod` `del` show the
  effect of a change.
- Kinds: `:service` (default) `:store` `:queue` `:actor` `:ext`. A kind
  sets the icon of an `arch` node. A group can be `:vendor`.
- Logos (`arch` only): `@name` puts a product logo on the tile, for example
  `@nextjs`, `@postgres`, `@redis`, `@stripe`. The first use of a logo gets
  it from the network. A node without a logo keeps the icon of its kind.
- An edge can use an id that has no node line. The label is then the id.
- `seq`: nodes are participants, edges are messages in time order, `-->` is
  a reply, `a -> a` is a message to self. No groups.
- `plan`: nodes are tasks, groups are phases, `a -> b` means `a` comes first.
- `erd` and `class`: a node is a record. Each indented line under a node
  line is a row: `id uuid pk` in `erd`, `+ get(key) Value` in `class`.
  `erd` arrows are `a 1--* b`, with each end one of `1 ? * +`. `class`
  arrows are `--|>` `..|>` `*--` `o--` `->` `..>`.
- `dir right` or `dir down` changes the direction of `arch` and `plan`.
- Levels: `detail cart "Title"` at the left margin, with an indented body,
  is the diagram of the parts of node `cart`. In the body, `^gw` is node
  `gw` of the parent level. `cart >cart.sd` takes the detail level from a
  file. A type word changes the type: `detail svc class`. Use `-o x.html`
  so the reader can click into a node; `--level cart` writes one level.

For the full rules, read [references/language.md](references/language.md).

## Content rules

- Draw from evidence. Read the code or the plan first. Do not add a part
  that you did not see.
- Use 12 nodes or fewer. Split a larger subject into two diagrams.
- Use the names that the code uses. Put a short role, a file path, or a
  technology on the second line of a node.
- Draw the main flow with `=>`, as one path from the start to the end. Use
  `->` for the other connections and `-->` for async or deploy-time links.
- Add `@name` only for a real product. Do not put a logo on your own
  modules.
- Label an edge only when the label adds information: a verb, a protocol,
  or the data that moves.
- Give a status only to the parts that the work touches. Leave the rest
  neutral, so the work is easy to see.
- For a plan: `done` for work that is complete, `wip` for the current step,
  `plan` for the rest. Render the plan again when the status changes.
- For a change: use `change`, with `add`, `mod`, and `del` on the parts
  that the change adds, modifies, and removes.
- For levels: draw the top level first, with 12 nodes or fewer. Add a
  detail level only for the parts that the user asks about or that the
  change touches, not for each node. Use `^name` for each parent edge that
  reaches the zoomed node, so the reader sees where the request enters.
  Leave the status off a node with a detail level: the harness takes it
  from the parts.
