# Design: zoom into a node

Issue: #2. Status: plan. Nothing in this document is built yet.

## Goal

A diagram can have levels. The top level shows the large parts, for
example the services of a system. A click on a service shows the parts of
that service, for example its modules. A click on a module can show its
classes. The user moves down and up through the levels in one HTML file or
one Claude artifact.

The agent writes all levels in the same language. The harness makes one
file that holds all levels.

## The language

### A detail block

A `detail` line at the left margin starts the diagram of one node. The
indented lines under it are the body of that diagram. The body uses the
normal grammar: nodes, groups, edges, and `dir`.

```
arch "Checkout"
user "Shopper" :actor
gw "API gateway"
cart "Cart service" "go" wip
db "Postgres" :store @postgres

user => gw => cart => db

detail cart "Cart service"
  handler "HTTP handler" "handler.go" done
  svc "Cart logic" "cart.go" wip
  repo "Repository" "repo.go"
  ^gw => handler "add item"
  handler => svc => repo
  repo => ^db "SQL"
```

- `detail <id>` names a node of the parent level. The optional string is
  the title of the detail level.
- The detail level has the type of the parent level. A type word after the
  id changes it: `detail svc class "Cart types"`. Then the body uses the
  grammar of that type.
- A detail block can hold `detail` blocks for its own nodes, with more
  indent. The limit is four levels.

### A detail file

A node can name a file that holds its detail level:

```
cart "Cart service" >cart.sd
```

The path is relative to the source file. The file is a normal diagram file.
The harness reads it when it renders the parent. A cycle of files is an
error.

### Context at the border

A detail level shows the nodes of the parent level that connect to the
zoomed node. These nodes are the context. In the body, `^name` names a node
of the parent level:

- `^gw => handler` means "the parent edge from `gw` comes in at `handler`".
- If the body does not name a context node, the harness draws that parent
  edge to the border of the detail canvas.

The harness draws context nodes as quiet tiles at the left and right sides
of the canvas, with the edge labels of the parent level.

## Output

### SVG

An SVG file shows one level, the top level by default. A node that has a
detail level gets a small zoom mark in the corner of its tile.
`--level cart` renders the detail level of `cart` as its own SVG.

### HTML and Claude artifacts

`swe-diagrams render checkout.sd -o checkout.html` writes one
self-contained HTML file. It holds the SVG of each level and a small script
(about 5 KB). It loads nothing from the network, so it works as a Claude
artifact without changes.

- A click on a node with a zoom mark zooms the view into the tile. Then the
  detail level grows from the tile to the full canvas. The motion takes
  about 350 ms.
- A breadcrumb at the top shows the path: `Checkout › Cart service`. A
  click on a part of the breadcrumb goes up to that level. The Escape key
  goes up one level.
- The address hash holds the path (`#cart/svc`), so a link can open one
  level, and the browser back button goes up.
- The keyboard can do all of it: Tab moves to a node with a zoom mark, and
  Enter zooms.
- With `prefers-reduced-motion`, the view changes without motion.
- The HTML follows the theme of the viewer. It reads
  `prefers-color-scheme` and the `data-theme` attribute that the Claude
  artifact viewer sets on the root element.

## Status on several levels

A node with no status takes a status from its detail level:

| Detail level | Status of the node |
|---|---|
| All parts `done` | `done` |
| A part `wip`, or some `done` and some not | `wip` |
| All parts `plan` | `plan` |
| Parts with `add`, `mod`, or `del` | `mod` |

A status that the agent writes on the node wins over this rule.

## The summary line

The summary line adds `levels=<count of levels>` and the ids of the nodes
that have a detail level: `details=cart,svc`.

## Skill guidance

- Draw the top level first, with 12 nodes or fewer.
- Add a detail level for the parts that the user asks about, or that the
  change touches. Do not add a detail level to each node.
- Use `^name` for each edge of the parent level that reaches the zoomed
  node, so the reader sees where the request enters.

## Work order

1. HTML output: one level, with the viewer script (pan, zoom with the
   wheel, hover, status filter, themes). This is also the base of the
   animation feature (#3).
2. The language: `detail` blocks, `^name`, detail files, parse errors.
3. The layout and the SVG of a detail level, with the context nodes.
4. The HTML drill-down: the motion, the breadcrumb, the hash, the keys.
5. The status rule over levels, the summary line, and the skill text.

## Open questions

- Should a detail level of an `arch` node keep the logo of the parent node
  at the top left of its canvas? The plan is yes.
- Should the SVG output have an option that draws all levels on one large
  canvas, with each detail level inside its parent node? This is a
  different layout. The plan is to wait for user demand.
