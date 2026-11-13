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
