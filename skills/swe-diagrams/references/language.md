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
| Detail | `detail id type "Title"` at the left margin. The body is the lines with more indent. See [Levels](#levels). |

Only the id is necessary on a node line. A node without a label shows its id.
The order of the kind, the logo, and the status is free.

An id is a word without spaces or quotes, for example `cart`, `src/api.ts`,
or `auth-v2`. The words `group` and `dir` are not ids. The word `detail` is
not an id at the left margin.

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
| `journey` | The pages of an app, and the user actions between them | right |

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
as the label. The label is a pill on the line. When a pill touches another
pill or line, it goes beside its line. The arrows are `->` and `-->`.

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

### journey

```
journey "Sign up"
home "Home" "/"
signup "Sign up" "/signup"
login "Sign in" "/login" :modal
docs "Docs" "docs.example.com" :external
home => signup "click Start"
home -> login "click Sign in"
signup --> docs "read terms"
```

Each node is a page, and each edge is a user action. Write the action as
the label, for example `click Buy` or `submit form`. The second string is
the route. It shows in mono in the bar of the page. Use `=>` for the main
path. The arrows are `=>`, `->`, and `-->`.

| Kind | Shape |
|---|---|
| `:page` | A browser frame with three dots and the route. This is the default. |
| `:modal` | A smaller frame with a close mark |
| `:external` | A browser frame with a dashed border, for a page that the app does not own |
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

When the layers go down, a label is at one side of its line. The label
takes the side with no other line, or moves along its line to a clear
place. A group grows to hold the labels of its edges.

## Stories

A story shows motion: a request that goes from part to part, or data that
moves through a pipeline. A story is a list of steps. Each step moves a
token along one edge of the diagram.

```
arch "Checkout"
user "Shopper" :actor
web "Web app"
gw "API gateway"
cart "Cart service"
db "Postgres" :store

user => web => gw => cart => db

story "Add an item"
  user -> web "click Add"
  web -> gw "POST /cart"
  gw -> cart & cart -> db
  db --> cart "row"
  cart --> gw "200 OK"
```

- `story "Title"` starts a story. Each line with more indent is a step.
  A line without more indent ends the story.
- A step names an edge in either direction. `db --> cart` moves the token
  from `db` to `cart` on the edge `cart => db`. A step between two nodes
  with no edge is an error.
- `->` gives a solid token. `-->` gives a hollow token, for a reply or an
  event.
- The string is the caption. The token carries it when it is not the
  same text as the edge label.
- `&` joins steps that run at the same time: `a -> b & a -> c`.
- When two nodes have more than one edge, as in `seq`, a step takes the
  next edge after the edge of the step before it.
- A diagram can have many stories. Each title must be unique.
- `timeline` has no stories, because it has no edges.

With no story, each type has a default story. When the source has
stories, the default story comes after them in the list.

| Type | Default story |
|---|---|
| `seq` | "Messages": each message, one at a time. |
| `flow` | "Main path": the `=>` edges. Without them, a walk from the `:start` node. |
| `state` | "Walk": a walk from the `:initial` state. It takes each transition one time. The event is the caption. |
| `dataflow` | "Stream": tokens move along each edge without stop. |
| `plan`, `timeline` | "Progress": the items change from planned to in progress to done, in line order, up to their status. A `done` item goes to done. A `wip` item stops at in progress. The other items do not change. The done count in the kicker follows each frame and ends at the real count. With no `done` or `wip` item, there is no story. |
| the other types | "Main path": the `=>` edges, in line order. With no `=>` edge, there is no story. |

The time of a step comes from the length of its edge: 120 px each
second, with 400 ms at minimum and 1600 ms at maximum. The steps of one
line take the time of the longest step. A story stops for 1000 ms at the
end, and then it starts again. A progress step takes 800 ms. `--speed`
divides all times.

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

## Cloud vendor icons

In `arch` and `change`, `@aws/<name>`, `@gcp/<name>`, and `@azure/<name>`
put an icon from the official package of the vendor on a node:

```
group cloud "AWS Cloud" :aws
  group reg "us-east-1" :region
    group vpc "VPC" :vpc
      group pub "Public subnet" :public
        alb "ALB" @aws/alb
      group priv "Private subnet" :private
        app "App" @aws/ecs
        db "Orders" @aws/rds
    bucket "Assets" @aws/s3
```

The first use of a vendor downloads its package from the vendor page with
`curl` and keeps it in `<icon dir>/vendor/`, with an index of the names.
The next runs do not use the network. The packages are AWS Architecture
Icons, the Google Cloud core product and category icons, and Azure Public
Service Icons. The credit line under the canvas names the package. To get
a new release of a package, delete its files in `<icon dir>/vendor/`.

- The name is the file name of the icon. The match ignores case and the
  characters `-`, `_`, and space. The vendor prefix (`Amazon`, `AWS`,
  `Cloud`, `Azure`) and a plural `s` are optional: `@aws/simple-queue-service`,
  `@aws/ec2`, `@azure/virtual-network`.
- Short names work: `s3` `ec2` `rds` `vpc` `alb` `nlb` `iam` `sqs` `sns`
  `ecs` `eks` `lambda` for AWS, `gke` `gcs` `bq` `gce` `run` `sql` for
  Google Cloud, and `aks` `vm` `vnet` `cosmos` `blob` `sql` for Azure.
- Google gives an icon only to its core products. Another product gets the
  icon of its category, as the Google guide says: `@gcp/pubsub` gives the
  Integration Services icon.
- `swe-diagrams icons aws s3` prints the names that match, with the best
  match first. Without a filter, it prints all names.
- When the package has no icon, the node keeps the icon of its kind, and
  the summary line lists `noicon=aws/<name>`.

These group kinds give the vendor look: `:aws`, `:gcp`, and `:azure` for
the cloud, and inside them `:region`, `:az`, `:vpc`, `:vnet`, `:subnet`,
`:public`, `:private`, and `:rg`. The look applies only inside a vendor
group.

- In `:aws`, an icon has no tile, because the icon has its own square.
  Each group has square corners and an outline only, with its icon in the
  top left corner. A region and an availability zone are dashed teal, a
  VPC is purple, a public subnet is green, and a private subnet is teal.
- In `:gcp` and `:azure`, the nodes keep their tiles. The cloud group has
  the vendor blue. A group inside it has a plain outline.

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
mark at the start: `+` (public), `-` (private), `#` (protected), or `~`
(package). In a class member line, a `#` at the start is the protected
mark. In all other places, a `#` starts a comment.

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

## Levels

A node can have a detail level: a diagram of its parts. The top level shows
the large parts. The detail level of a node shows what is inside it. A
diagram has four levels at most: the top level and three detail levels.

```
arch "Checkout"
gw "API gateway"
cart "Cart service" "go"
pay "Payments" >payments.sd
db "Postgres" :store

gw => cart "add item"
gw -> pay
cart => db

detail cart "Cart service modules"
  handler "HTTP handler" "handler.go" done
  svc "Cart logic" "cart.go" wip
  ^gw => handler
  handler => svc
  svc => ^db "SQL"

  detail svc class "Cart types"
    Cart
      + add(item) void
```

- `detail <id>` starts the detail level of node `id`. The line is at the
  left margin of its level. The body is the lines under it with more
  indent, and it uses the normal grammar.
- The optional string is the title of the detail level. Without it, the
  title is the label of the node.
- The detail level has the type of its parent. A type word after the id
  changes it: `detail svc class "Cart types"`.
- A `detail` line in a body starts a level below that body.
- `>path.sd` on a node line names a file that holds the detail level. The
  path is relative to the source file. The file is a normal diagram file,
  and it can have its own levels. A cycle of files is an error.
- In a body, `^name` is node `name` of the parent level. Use it only in an
  edge, and only for a node with an edge to the zoomed node.

The harness adds the context of the parent level to a detail level:

- Each node of the parent level with an edge to the zoomed node shows as a
  quiet tile. The nodes with an edge in go at the start, and the nodes with
  an edge out go at the end.
- The parts of the detail level are in a frame, with the label of the
  zoomed node.
- A parent edge that no `^name` edge uses goes from the context tile to the
  frame. A `^name` edge with no label takes the label of the parent edge.

A node with a detail level gets a zoom mark at its bottom right corner. A
node with no status takes a status from the parts of its detail level:

| Parts of the detail level | Status of the node |
|---|---|
| `add`, `mod`, or `del` on a part | `mod` |
| All parts `done` | `done` |
| All parts with a status are `plan` | `plan` |
| A part `wip`, or some parts `done` and some not | `wip` |
| No part with a status | none |

A status on the node line wins over this rule.

## Command

```
swe-diagrams render diagram.sd -o diagram.svg
swe-diagrams render - < diagram.sd > diagram.svg
swe-diagrams render diagram.sd -o diagram.svg --icons ./icons --offline
swe-diagrams icons aws load-balancer
swe-diagrams render diagram.sd -o diagram.html
swe-diagrams render diagram.sd -o diagram.png --theme dark --scale 3
swe-diagrams render diagram.sd -o diagram.svg --animate --story "Add an item"
swe-diagrams render diagram.sd -o diagram.gif --fps 15 --speed 1.5
swe-diagrams render diagram.sd -o diagram.mp4
```

The options:

| Option | Effect |
|---|---|
| `-o <file>` | Writes the output to the file. Without `-o`, the output goes to standard output. |
| `--format <F>` | Sets the format: `svg`, `html`, `png`, `txt`, `gif`, `mp4`, or `webm`. |
| `--theme <T>` | Sets the palette: `auto`, `light`, or `dark`. |
| `--scale <N>` | Sets the pixel scale of a PNG, a GIF, or a video, from 0.1 to 16. The default is 2 for a PNG and 1 for a GIF or a video. |
| `--animate` | Adds motion to an SVG: the first story, or the story of `--story`. |
| `--story <title>` | Selects the story of an animated SVG, a GIF, or a video. |
| `--fps <N>` | Sets the frames each second of a GIF or a video, from 1 to 50. The default is 15. |
| `--speed <X>` | Divides the times of a story, from 0.1 to 10. The default is 1. |
| `--level <path>` | Writes the detail level at the path, for example `cart` or `cart/svc`. For HTML, the page opens at that level. |
| `--icons <dir>` | Looks for logos in the directory first. |
| `--offline` | Does not use the network for logos. |

## Output formats

Without `--format`, the extension of `-o` gives the format. An unknown
extension gives SVG. Without `-o`, the format is SVG.

| Format | Output | Theme default |
|---|---|---|
| `svg` | One SVG file. It works in a README, a pull request, and a browser. | `auto` |
| `html` | One page with the SVG, a stylesheet, and a script. It loads nothing from the network. | `auto` |
| `png` | A PNG image. It needs `-o`. | `light` |
| `gif` | A GIF that loops one time through the story. It needs `-o`. | `light` |
| `mp4`, `webm` | A video of the story, made with `ffmpeg`. It needs `-o`, and `ffmpeg` on the PATH. | `light` |
| `txt` | Not ready. The command gives an error. | |

An SVG with `--animate` moves without a script, with SMIL motion on the
edge paths. It works where an SVG works as an image, for example in a
GitHub README. With `prefers-reduced-motion`, the viewer sees the still
diagram. Without `--animate`, the SVG has no motion.

A GIF or a video of a diagram with no story is one still frame. The
summary then says `story=none`.

The themes:

- `auto` keeps the light palette and the dark palette. The viewer picks
  one with its colour setting.
- `light` removes the dark palette.
- `dark` always uses the dark palette.

The HTML page also follows the `data-theme` attribute (`light` or `dark`)
on its root element. The attribute wins over the system setting. In the
page:

- Drag to pan. Use the wheel, a pinch, or the `+` and `-` keys to zoom.
  The `0` key and the "fit" button show all of the diagram.
- Hover over a node: its edges and its neighbours stay bright, and the
  rest dims.
- Click a status in the legend: only the parts with that status stay
  bright. Click it again to clear it.
- The arrow keys step through the diagram: the messages of a `seq`, and
  the nodes of the other types in line order. `Escape` clears the step.
- A diagram with a story gets a player: a list of the stories, play and
  pause, step back, step forward, and the speed. The active edge is
  bright, the other edges are quiet, and the node at the end of a step
  pulses. A caption panel shows the step number, the caption, and the
  edge label. The space key plays and pauses. In a page with a player,
  the left and right arrow keys step through the story.
- A click on a node with a zoom mark opens its detail level. `Tab` moves
  to such a node, and `Enter` opens it. The view zooms into the node, and
  the detail level grows from the node box. With `prefers-reduced-motion`,
  the view changes with no motion.
- The breadcrumb at the top left shows the path of levels. A click on a
  part of it goes up to that level. `Escape` goes up one level.
- The hash of the address holds the path, for example `#cart/svc`. A link
  with a hash opens that level. The back button goes up.

The PNG uses the bundled fonts Geist and JetBrains Mono, and no system
font, so each machine gives the same picture.

In the SVG and the HTML, each part is in a group with data attributes:
`data-k` (`node`, `group`, `edge`, or `msg`), `data-id` and `data-s` (the
status word) for a node or a group, and `data-i`, `data-from`, and
`data-to` for an edge or a message. The line of an edge has the class
`rt`. A node with a detail level has `data-z`. In a detail level, a context
node has the id `^name` and the class `cx`, and the frame group has the id
`^`.

The data of the page (`#sd-data`) has a `levels` list when the diagram has
more than one level. Each entry has the `path`, the `title`, the `type`,
the `order` of steps, and the `svg` text. The `svg` of the top level is
`null`, because the page holds it. The data holds the stories in
`stories`, with the times at speed 1.

## Summary and errors

On success, the harness prints one summary line:

```
ok arch nodes=4 edges=3 diagram.svg
```

An animated output adds the story, and a GIF or a video adds the frame
count:

```
ok arch nodes=5 edges=4 frames=66 story="Add an item" diagram.gif
```

The counts are the counts of the top level. A diagram with detail levels
adds the count of levels and the ids of the nodes with a detail level:
`levels=3 details=cart,svc`. An error in a linked file names the link line
and the file: `top.sd:4: in cart.sd:7: unknown word 'x'`.

On a diagram error, the harness prints one line for each error and exits
with code 2:

```
diagram.sd:7: unknown word 'todo'. Statuses: done wip plan add mod del
```
