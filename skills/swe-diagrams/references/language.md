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
