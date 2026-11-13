# The diagram language

A diagram is a text file with the extension `.sd`. The harness reads the file
and writes an SVG file. The harness sets all positions. The author gives only
the content.

```
arch "Checkout"

user "Shopper" "browser" :actor
group api "API"
  web  "Web app" "next.js" @nextjs done
  cart "Cart service" "go" wip
db "Postgres" "orders db" :store @postgres done

user => web "browse"
web => cart "add item"
cart -> db "write" plan
```

## Lines

The file has one statement on each line. A `#` starts a comment.

| Line | Form |
|---|---|
| Header | `type "Title"`. This is the first line. The title is optional. |
| Node | `id "Label" "Second line" :kind @logo status` |
| Group | `group id "Label" @logo :vendor status`. The members are the lines with more indent. |
| Edge | `a -> b "label" status` |
| Direction | `dir right` or `dir down` |

Only the id is necessary on a node line. A node without a label shows its id.
The order of the kind, the logo, and the status is free.

An id is a word without spaces or quotes, for example `cart`, `src/api.ts`,
or `auth-v2`. The words `group` and `dir` are not ids.

Put spaces around an arrow. Write `a -> b`, not `a->b`.

## Types

| Type | Shows | Default direction |
|---|---|---|
| `arch` | Components, groups, and the connections between them | right |
| `seq` | Messages between participants, in time order | none |
| `plan` | Tasks, phases, and the order of the work | down |
| `deps` | Modules and packages, and what each one imports | down |
| `flow` | The steps and the decisions of a process | down |
| `state` | The states of a thing, and the events that change the state | right |
| `dataflow` | Where the data goes, and what changes it | right |
| `change` | The parts that a change adds, modifies, and removes | right |
| `timeline` | Events and milestones on one axis, in line order | right |
| `erd` | Tables, their fields, and the relations between them | right |
| `class` | Types, their members, and the relations between them | down |

In `seq`, each node is a participant and each edge is a message. The order of
the lines is the order in time. `a -> a` is a message to self. `seq` has no
groups.

In `plan`, each node is a task, each group is a phase, and `a -> b` means
that `a` comes before `b`. The title line shows the count of done tasks.

### deps

```
deps "Auth module imports"
group core "src/core"
  session "session.ts"
  token "token.ts"
api "src/api/login.ts" mod
jwt "jose" :ext
api -> session
session -> token
token -> jwt
token -> session del
```

Each node is a module or a package, and each group is a folder.
`a -> b` means that `a` imports `b`. `-->` is a type-only import or a dev
dependency. The arrows `=>` and `<->` are errors in `deps`.

| Kind | Shape |
|---|---|
| `:module` | A card. This is the default. |
| `:ext` | A dashed card, for a third-party package |

The harness finds the import cycles. Each edge in a cycle is red. The
summary line adds `cycles=<count>`, the count of the cycles. A cycle is a
set of two or more nodes that can each reach the others.

### flow

```
flow "Checkout"
start "Cart" :start
pay "Pay" :io
ok "Paid?" :decision
ship "Create shipment"
fail "Show error"
done "Done" :end
start -> pay -> ok
ok -> ship "yes"
ok -> fail "no"
fail -> pay
ship -> done
```

Each node is a step, and `a -> b` means that `b` comes after `a`. Use `=>`
for the main path and `-->` for a path that is not usual. The arrow `<->`
is an error in `flow`.

| Kind | Shape | Use |
|---|---|---|
| `:step` | A box. This is the default. | An action |
| `:start`, `:end` | A pill | The start or the end of the process |
| `:decision` | A diamond, with the label in it | A question with two or more answers |
| `:io` | A parallelogram | An input or an output |
| `:sub` | A box with a double line at each side | A process that has its own diagram |

Put the answer of a decision on each edge that leaves it, for example
`ok -> ship "yes"`. The diamond is as wide as its label needs.

### state

```
state "Order"
init :initial
draft "Draft"
paid "Paid" done
shipped "Shipped" wip
closed :final
init -> draft
draft -> paid "pay"
draft -> draft "edit"
paid -> shipped "ship [stock > 0]"
shipped -> closed
```

Each node is a state, and each edge is a transition. Write the label of a
transition as the event, an optional `[guard]`, and an optional `/ action`,
for example `"timer [tries < 3] / retry"`. The harness does not parse the
label. The arrows are `->` and `-->`.

| Kind | Shape |
|---|---|
| `:state` | A rounded card. This is the default. |
| `:initial` | A filled dot, with no label |
| `:final` | A dot in a ring, with no label |
| `:choice` | A small diamond, with no label |

`a -> a` is a self-transition. Only `state` and `seq` allow it. The loop is
on the top side of the card, with the label above it. When the layers go
down, the loop is on the right side. Two self-transitions on one state
share one loop, and the labels join with a comma.

### dataflow

```
dataflow "Event pipeline"
app "Web app" :source
q "events" :queue
etl "Normalize"
wh "Warehouse" :store
dash "Dashboards" :sink
app -> q "click events"
q -> etl "json"
etl -> wh "rows"
wh -> dash "sql"
```

Each node is a place where the data is, or a step that changes the data.
`a -> b` means that data moves from `a` to `b`. Write the data that moves
as the label. The label is a pill on the line. When two pills touch, they
go beside their lines. The arrows are `->` and `-->`.

| Kind | Shape |
|---|---|
| `:transform` | A box. This is the default. |
| `:source` | A box with a thick left side |
| `:sink` | A box with a thick right side |
| `:store` | A cylinder |
| `:queue` | A box with two bars at the right end |

### change

```
change "Replace the session cache"
api "API" @hono mod
cache "Redis" :store @redis del
kv "Workers KV" :store @cloudflare add
db "Postgres" :store @postgres
api -> cache del
api -> kv add
api -> db
```

The language of `change` is the language of `arch`: the same kinds, logos,
groups, and arrows. Give `add`, `mod`, or `del` to each part that the change
touches. A node, a group, or an edge with no status is quiet: it has a
lower opacity, so the changed parts stand out.

The title line counts the changed nodes and groups, for example
`change / +1 ~1 -1`. The summary line adds
`added=1 changed=1 removed=1`. The counts do not include the edges.
## timeline

```
timeline "Rate limit roll-out"
group design "Design" done
  spec "Spec" "Nov 3" done
  algo "Choose algorithm" "Nov 5" done
group build "Build"
  mw "Middleware" "Nov 12" wip
  ship "Ship to 1%" "Nov 20" :milestone plan
  ga "GA" "Dec 1" :milestone plan
```

In `timeline`, each node is an item on one horizontal axis. The order of the
lines is the order on the axis, from left to right.

- The second string is the date or a note. The harness does not parse it.
- Kinds: `:event` (default) is a dot on the axis. `:milestone` is a diamond.
- The labels go above and below the axis in turn. A wide label makes more
  space between its item and the neighbour items.
- A group is a phase. It is a band under the axis, from its first item to
  its last item. A phase cannot hold a phase.
- The axis is solid up to the last item with `done` or `wip`, and dashed
  after it. An item with `wip` gets a "now" mark.
- The title line shows the count of done items when an item has `done`,
  `wip`, or `plan`.
- `timeline` has no edges. An edge line is an error.

## Edges

| Arrow | Meaning |
|---|---|
| `=>` | A solid line in the accent colour. Use it for the main flow. |
| `->` | A solid line. In `seq`, a call. |
| `-->` | A dashed line. In `seq`, a reply. |
| `<->` | A solid line with an arrowhead at each end. |

A chain makes one edge for each arrow: `a -> b -> c`. A label or a status
after a chain applies to each edge in the chain.

An edge can name an id that has no node line. The harness then makes a node
with that id as the label, in the group of the edge line. The summary line
lists these ids after `implicit=`, so an id with a typing error is visible.

An edge end can be a group. An edge cannot join a group and its own member.

## Status

A status shows the state of the work on a node, a group, or an edge.

| Word | Meaning |
|---|---|
| `done` | The work is complete. |
| `wip` | The work is in progress. |
| `plan` | The work has not started. |
| `add` | The change adds this part. |
| `mod` | The change modifies this part. |
| `del` | The change removes this part. |

Use `done`, `wip`, and `plan` to show progress. Use `add`, `mod`, and `del`
to show the effect of a change. A part without a status is neutral. The
diagram gets a legend for the statuses that it uses.

The terminal text shows a status as a symbol after the label: `✓` done,
`◐` wip, `○` plan, `+` add, `~` mod, and `−` del.

## Kinds

In `arch`, a node is a tile with an icon, and the label is below the tile.
A kind sets the icon of a node that has no logo.

| Kind | Icon | Use |
|---|---|---|
| `:service` | Server. This is the default. | A program or a module |
| `:store` | Cylinder | A database, a cache, a bucket |
| `:queue` | Row of messages | A queue, a topic, a stream |
| `:actor` | Person | A person or a client |
| `:ext` | Globe, with a dashed tile | A system that the team does not own |

In `plan` and `seq`, a node is a card with the label in it. `:actor` gives a
round card and `:ext` gives a dashed card.

A group can have the kind `:vendor`. It gives the group a warm colour. Use it
for the platform of a vendor, for example a cloud account.

## Logos

In `arch`, `@name` puts the logo of a product on the tile of a node, or
before the label of a group: `db "Postgres" @postgres`.

The harness looks for `name.svg` or `name.png` in the icon directory. If
the file is not there, the harness gets the logo from these public sources,
in this order, and keeps it in the icon directory: svgl, devicon,
gilbarbara/logos. The first run with a new logo uses the network. The next
runs do not.

- The name is the name that the sources use, in lower case: `nextjs`,
  `postgresql`, `redis`, `githubactions`. Some short names work too, for
  example `postgres`, `k8s`, `node`, `ts`.
- When no source has the logo, the node keeps the icon of its kind, and the
  summary line lists the name after `noicon=`.
- To use your own logo, put `name.svg` in the icon directory.
- The icon directory is `--icons <dir>`, or `$SWE_DIAGRAMS_ICONS`, or
  `~/.cache/swe-diagrams/icons`.
- `--offline` or `SWE_DIAGRAMS_OFFLINE=1` stops all network use.
- The harness does not change the colours of a logo. A white logo gets a
  dark tile. The logos are the trademarks of their owners.

## Records: erd and class

In `erd` and `class`, a node is a record: a header with the label, and one
row for each member line. A member line is a line under a node line, with
more indent. The record gets its height from its rows. A member line can
have a status at the end. The row then gets the colour of that status.

### erd

```
erd "Orders"
users "User"
  id uuid pk
  email text uniq
orders "Order" add
  id uuid pk
  user_id uuid fk
  note text null
users 1--* orders "places"
```

A member line is a field: `name type flags status`. The flags are `pk`,
`fk`, `uniq`, and `null`. The row shows a `PK`, `FK`, or `UQ` mark, the
name, and the type. A key field has bold text. A field with `null` shows
its type with a `?`, for example `text?`.

An arrow is `<left>--<right>`. Each end is one of these marks:

| End | Meaning | Mark at that end |
|---|---|---|
| `1` | Exactly one | Two bars |
| `?` | Zero or one | A bar and a ring |
| `*` | Zero or many | A crow's foot and a ring |
| `+` | One or many | A crow's foot and a bar |

`users 1--* orders` means: one user has zero or more orders. A group is a
schema.

### class

```
class "Storage"
store "Store" :interface
  + get(key) Value
mem "MemoryStore"
  - map HashMap
  + get(key) Value
mem ..|> store
```

A member line with `(` is a method. The other member lines are fields. The
record shows the fields, then the methods, each in its own section. The
harness keeps the text of a member line as it is, so write a visibility
mark at the start: `+`, `-`, or `~`. A `#` starts a comment, so put a
protected member in quotes: `"# origin Point"`.

The kinds are `:class` (the default), `:interface`, `:abstract`, and
`:enum`. The kind shows above the name, for example `«interface»`. An
abstract name is italic.

| Arrow | Meaning | Mark |
|---|---|---|
| `a --\|> b` | a extends b | Hollow triangle at b |
| `a ..\|> b` | a implements b | Hollow triangle at b, dashed line |
| `a *-- b` | a is made of b | Filled diamond at a |
| `a o-- b` | a has b | Hollow diamond at a |
| `a -> b` | a uses b | Arrowhead at b |
| `a ..> b` | a depends on b | Open arrowhead at b, dashed line |

For `--|>` and `..|>`, the harness puts b above a, so a parent is above its
children.

## Command

```
swe-diagrams render diagram.sd -o diagram.svg
swe-diagrams render - < diagram.sd > diagram.svg
swe-diagrams render diagram.sd -o diagram.svg --icons ./icons --offline
```

On success, the harness prints one summary line:

```
ok arch nodes=4 edges=3 diagram.svg
```

On a diagram error, the harness prints one line for each error and exits
with code 2:

```
diagram.sd:7: unknown word 'todo'. Statuses: done wip plan add mod del
```

## Terminal text

The library function `text_out::to_text` draws a diagram for a terminal.
It uses Unicode box drawing, and it supports all eleven types.

- A node is a box. A card has round corners. A planned node has a dashed
  border.
- An edge is a line with an arrowhead. A dashed edge has a dashed line.
- A record (`erd`, `class`) shows its rows. An `erd` line shows the end
  marks `1`, `?`, `*`, and `+`.
- A `seq` diagram shows lifelines and the messages in order.
- A `timeline` shows one axis, with the labels above and below it.
- A label that is too long for its box gets an ellipsis.
- With colour, the status symbol and the box get ANSI colours.

When the drawing is wider than the limit, the text is an outline: the
groups as headings, the nodes with their status symbols, and the edges as
`a → b  label` lines. The first line of the outline gives the reason.
