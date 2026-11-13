# Design: eight more diagram types

Issue: #1. This document is the contract for the parser, the layout, and the
renderer. The language reference (`skills/swe-diagrams/references/language.md`)
gets the user-facing part of it.

## Rules for all types

- All types use the same line grammar: a header, node lines, `group` lines,
  edge lines, and `dir`. All types use the six statuses.
- All types use the same look: the graph-paper canvas, the kicker and the
  title, the legend, the mono labels, and the status badges.
- A type has its own kinds. A kind from another type is an error, and the
  error message lists the kinds of the current type.
- A type has its own arrows. An arrow from another type is an error, and the
  error message lists the arrows of the current type.
- The output stays deterministic: integer math only, stable sorts.
- Each type gets at least two fixtures in `spec/fixtures/` (`<type>-*.sd`),
  and each new error gets a fixture in `spec/errors/`.

## Summary of the types

| Type | Question | Node | Default dir | Layout |
|---|---|---|---|---|
| `deps` | What depends on what? | card | down | layered graph |
| `flow` | What are the steps and the decisions? | shape by kind | down | layered graph |
| `state` | What states can a thing be in? | rounded card | right | layered graph |
| `dataflow` | Where does the data go? | card | right | layered graph |
| `change` | What does this change touch? | tile, like `arch` | right | layered graph |
| `erd` | What are the tables and the relations? | record | right | layered graph |
| `class` | What are the types and their relations? | record | down | layered graph |
| `timeline` | What happens in which order? | marker on an axis | right | axis |

## deps

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

- `a -> b` means "a imports b" or "a depends on b". `-->` is a type-only or
  a dev dependency.
- Kinds: `:module` (default), `:ext` (a third-party package, dashed card).
- The harness finds the cycles. It draws each edge in a cycle in the
  removed colour, and adds `cycles=<count>` to the summary line. The count
  is the count of strongly connected components with more than one node.

## flow

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

- Kinds: `:step` (default, box), `:start` and `:end` (pill), `:decision`
  (diamond), `:io` (parallelogram), `:sub` (box with a double line at each
  side, a sub-process).
- A decision has the label inside the diamond. The diamond is at least as
  wide as its label.

## state

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

- Kinds: `:state` (default, rounded card), `:initial` (a filled dot, no
  label), `:final` (a dot in a ring, no label), `:choice` (a small diamond).
- `a -> a` is a self-transition. This is the only type other than `seq`
  that allows it. The harness draws a loop on the top side of the card and
  puts the label above the loop. A self-transition does not take part in
  the layering.
- An edge label is the event, an optional `[guard]`, and an optional
  `/ action`. The harness does not parse the label.

## dataflow

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

- Kinds: `:transform` (default, box), `:source` (box with a thick left
  side), `:sink` (box with a thick right side), `:store` (cylinder),
  `:queue` (box with two bars).
- An edge label is the data that moves. The harness draws it as a pill on
  the line.

## change

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

- The language of `change` is the language of `arch`: the same kinds,
  logos, groups, and arrows.
- A part with no status is quiet: its tile, its text, and its edges have a
  lower opacity. The parts with `add`, `mod`, and `del` stand out.
- The kicker counts the parts: `change / +2 ~1 -1`. The summary line adds
  `added=2 changed=1 removed=1`.

## erd

```
erd "Orders"
users "User"
  id uuid pk
  email text uniq
orders "Order" add
  id uuid pk
  user_id uuid fk
  total numeric
items "Line item"
  order_id uuid fk
  sku text
users 1--* orders "places"
orders 1--+ items
```

- A node is an entity: a header with the label, and one row for each member
  line. A member line is an indented line under the node line. Its first
  word is the field name, the second word is the type, and the next words
  are flags: `pk`, `fk`, `uniq`, `null`. A member line can have a status at
  the end.
- The record shows the name at the left, the type at the right in mono,
  and a small `PK` or `FK` mark. A key row has bold text.
- Arrows: `<left>--<right>`, with each end one of `1` (exactly one), `?`
  (zero or one), `*` (zero or many), `+` (one or many). Example:
  `users 1--* orders`. The harness draws crow's-foot marks at each end.
- `group` is a schema. The group box is as for `arch`.

## class

```
class "Storage"
store "Store" :interface
  + get(key) Value
  + put(key, value)
mem "MemoryStore"
  - map HashMap
  + get(key) Value
disk "DiskStore" add
  - path PathBuf
cache "Cache"
mem ..|> store
disk ..|> store
cache *-- store "backend"
```

- A node is a class: a header, then a section for fields and a section for
  methods. A member line with `(` is a method. Other member lines are
  fields. The harness keeps the text of a member line as it is, so a
  visibility mark (`+ - # ~`) shows at the start.
- Kinds: `:class` (default), `:interface`, `:abstract`, `:enum`. The kind
  shows as `«interface»` above the name. An abstract name is italic.
- Arrows:

  | Arrow | Meaning | Mark |
  |---|---|---|
  | `a --|> b` | a extends b | hollow triangle at b |
  | `a ..|> b` | a implements b | hollow triangle at b, dashed line |
  | `a *-- b` | a is made of b | filled diamond at a |
  | `a o-- b` | a has b | hollow diamond at a |
  | `a -> b` | a uses b | arrow at b |
  | `a ..> b` | a depends on b | open arrow at b, dashed line |

- For `--|>` and `..|>`, the harness ranks b above a, so a parent is above
  its children.

## timeline

```
timeline "Rate limit roll-out"
group design "Design"
  spec "Spec" "Nov 3" done
  algo "Choose algorithm" "Nov 5" done
group build "Build"
  mw "Middleware" "Nov 12" wip
  ship "Ship to 1%" "Nov 20" :milestone plan
  ga "GA" "Dec 1" :milestone plan
```

- The items are in the order of the lines, from left to right on one axis.
  The second string is the date or a note. The harness does not parse it.
- Kinds: `:event` (default, a dot on the axis), `:milestone` (a diamond on
  the axis).
- The labels go above and below the axis in turn, so neighbours do not
  touch. The space between two items is the space that the wider label
  needs.
- A group is a phase: a band under the axis from its first item to its
  last item, with the label in the band. Groups do not nest in `timeline`.
- The axis is solid up to the last item with `done` or `wip`, and dashed
  after it. A `wip` item gets a "now" mark.
- `timeline` has no edges. An edge line is an error.

## Work order

1. Parser and model for all eight types (types, kinds, arrows, member lines,
   self-transitions). The new types draw with the current renderers.
2. `deps`, `flow`, `state`, `dataflow`, `change`: shapes and rules on the
   current graph layout.
3. `erd` and `class`: record nodes, end marks.
4. `timeline`: the axis layout.

Steps 3 and 4 start after step 1 is on the branch.
