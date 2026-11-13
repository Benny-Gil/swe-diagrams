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

   If the command is not found, tell the user to install the harness:
   `cargo install --git https://github.com/Benny-Gil/swe-diagrams swe-diagrams`.
4. Read the summary line, for example `ok plan nodes=6 edges=5 plan.svg`.
   An `implicit=` list names ids that only an edge uses. An id there that
   you did not intend is a typing error. A `noicon=` list names logos that
   no source has; use another name or remove the `@name`. On an error line
   (`file:line: message`), correct that line and render again.
5. Give the user the path of the SVG file, and one sentence on what it shows.

Keep the files in `docs/diagrams/` when the user wants to keep them.
Otherwise use a temporary directory.

## Choose the type

| Question | Type |
|---|---|
| What are the parts, and how do they connect? What does this change touch? | `arch` |
| In what order do the parts talk for one request? | `seq` |
| What are the steps of the work, and where is the work now? | `plan` |
| What happens in which order, and where are we now? | `timeline` |

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
- `dir right` or `dir down` changes the direction of `arch` and `plan`.

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
- For a change: `add`, `mod`, and `del` on the parts that the change adds,
  modifies, and removes.
