# Design: animate how data and calls move

Issue: #3. Status: plan. Nothing in this document is built yet.

## Goal

A diagram can show motion: a request that goes from service to service,
data that flows through a pipeline, a user who moves from page to page, or a
plan that moves from step to step. The motion shows the order, which a
still picture shows badly.

## The model: a story

A story is a list of steps in order. Each step moves a token along one edge
of the diagram. A diagram can have many stories, and the viewer chooses
one.

```
arch "Checkout"
user "Shopper" :actor
web "Web app" @nextjs
gw "API gateway"
cart "Cart service"
db "Postgres" :store @postgres

user => web => gw => cart => db

story "Add an item"
  user -> web "click Add"
  web -> gw "POST /cart"
  gw -> cart
  cart -> db "INSERT"
  db --> cart "row"
  cart --> web "200 OK"
```

- `story "Title"` starts a story. The indented lines are its steps.
- A step names an edge of the diagram in either direction. `db --> cart`
  moves the token from `db` to `cart` along the edge `cart -> db`. A step
  between two nodes with no edge is an error.
- The string of a step is its caption. The viewer shows the caption, and
  the token can carry it as a small label.
- `->` gives a solid token. `-->` gives a hollow token, for a reply or an
  event.
- `a -> b & a -> c` on one line runs two steps at the same time.
- With no story, the edges with `=>` make a default story, in the order of
  the lines.

## Defaults by type

| Type | Default motion |
|---|---|
| `seq` | The messages are a story already. They show one at a time. |
| `flow` | A token walks the main path. A story chooses the branch at a decision. |
| `dataflow` | Tokens stream along each edge without stop, to show a flow, not one event. |
| `state` | A token moves from state to state. The caption is the event. |
| `plan`, `timeline` | The tasks change from planned to in progress to done, in order. |
| `arch`, `change` | The default story from `=>`. |

## A new type for page navigation: `journey`

The request "how a user moves through pages" needs pages as nodes. A
`journey` node is a page card: a small browser frame with the page title
and the route. An edge is a user action, for example `click Buy` or
`submit form`. A story is one session of a user. This type is a proposal.
It can come after the eight types of issue #1.

```
journey "Sign up"
home "Home" "/"
signup "Sign up" "/signup"
verify "Check your email" "/verify"
app "Dashboard" "/app"

home -> signup "click Start"
signup -> verify "submit"
verify -> app "open link"

story "New user"
  home -> signup
  signup -> verify
  verify -> app
```

## Output

### Animated SVG

`--animate` writes an SVG with motion that needs no script. Each step is an
SVG `animateMotion` element on the path of its edge, and the next step
starts when the step before it ends. The SVG loops the default story.

- The motion works where an SVG works as an image, for example in a GitHub
  README and in a pull request. Scripts do not run there, but SVG motion
  does.
- With `prefers-reduced-motion`, the tokens do not show, and the SVG is a
  still diagram.
- Without `--animate`, the SVG has no motion. The default stays a still
  picture.

### HTML player

The HTML output (see `zoom.md`) adds a player:

- Play, pause, step back, step forward, and a speed control.
- A list of the stories.
- The active edge is bright, and the other edges are quiet. The active node
  gets a short pulse.
- A caption panel shows the step number, the caption, and the edge label.
- The arrow keys step. The space key plays and pauses.
- A step can go into a detail level (see `zoom.md`): `cart/svc -> cart/repo`.
  The view zooms into `cart`, plays the steps inside, and zooms out.

## Timing

The time of a step comes from the length of its path: 120 px each second,
with 400 ms at minimum and 1600 ms at maximum. All times are whole
milliseconds, so the output stays deterministic. `--speed` scales the times.

## Work order

1. The language: `story` blocks, the step rules, parse errors, the default
   story.
2. The animated SVG for `arch`, `change`, `flow`, `dataflow`, and `state`.
3. The animated SVG for `seq`, `plan`, and `timeline`.
4. The HTML player. It needs the HTML output from `zoom.md`, step 1.
5. Steps that go into detail levels. It needs the zoom feature.
6. The `journey` type.

## Open questions

- A GIF or a video file for chat apps that do not show SVG: this needs a
  browser to record. The plan is an optional command that uses a local
  Chrome when one is present.
- Tokens that carry a data label can hide other labels on a dense diagram.
  The plan is to show the label only on the active step.
