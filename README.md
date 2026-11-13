# swe-diagrams

Diagrams of software that an agent can write in seconds.

An agent writes a short text file. The harness sets all positions and writes
an SVG file. Each part of the diagram can carry a status, so one picture
shows what the agent has built, what it builds now, and what it plans.
An architecture node can show the logo of its product: `db "Postgres"
@postgres`.

```
arch "Checkout service"

user "Shopper" "browser" :actor
group edge "Edge"
  web "Web app" "next.js" done
  gw "API gateway" "rest" done
group core "Core services" :vendor
  cart "Cart service" "go" wip
  pay "Payments" "worker" plan
  group data "Data"
    db "Postgres" "orders db" :store done
    cache "Redis" "session cache" :store wip
    q "Order events" "topic" :queue plan
stripe "Stripe" "card processor" :ext

user => web "browse"
web => gw "REST"
gw => cart "add item"
gw -> pay "charge" plan
cart => db "write"
cart -> cache "read"
cart --> q "publish" plan
pay -> stripe "capture" plan
q --> pay plan
```

![Architecture diagram of a checkout service](spec/golden/arch-checkout.svg)

A plan and a sequence use the same language:

![Plan graph for a rate limit feature](spec/golden/plan-ratelimit.svg)

![Sequence diagram of a login](spec/golden/seq-login.svg)

## Why

- **Few tokens.** A diagram is 10 to 30 short lines. The agent does not
  write coordinates, styles, or markup.
- **No check loop.** The layout is deterministic. The agent reads a
  one-line summary. It does not render an image and look at it.
- **Status is part of the language.** `done`, `wip`, and `plan` show
  progress. `add`, `mod`, and `del` show the effect of a change.
- **One file out.** The SVG has no script and no external file. It has a
  light theme and a dark theme. It works in a README and in a pull request.
- **Fast.** One diagram takes about 3 milliseconds.

## Status

This is an early version. The harness draws three diagram types: `arch`,
`seq`, and `plan`, and writes SVG. The language reference is
[skills/swe-diagrams/references/language.md](skills/swe-diagrams/references/language.md).

## Install

```bash
cargo install --git https://github.com/Benny-Gil/swe-diagrams swe-diagrams
swe-diagrams render diagram.sd -o diagram.svg
```

In a clone of the repo, use `cargo install --path rust`.

The harness is one binary with no dependencies. It uses `curl` to get a
logo the first time a diagram names that logo, and keeps the file in
`~/.cache/swe-diagrams/icons`. With `--offline`, it does not use the network.

The folder `skills/swe-diagrams/` is an agent skill. Copy it into the skills
folder of your agent.

## Logos

The harness gets logos from svgl, devicon, and gilbarbara/logos. It does not
include logo files, and it does not change the colours of a logo. The logos
are the trademarks of their owners. A diagram that uses logos names the
sources below the canvas.

## Develop

| Folder | Content |
|---|---|
| `skills/swe-diagrams/` | The skill: instructions and the language reference |
| `rust/` | The harness. `cargo test`, `cargo build --release` |
| `spec/fixtures/` | Diagram sources |
| `spec/golden/` | The SVG and the summary line that each source must give |
| `spec/errors/` | Sources with errors, and the messages that they must give |
| `spec/icons/` | Logos for the tests. They are not real product logos. |

To change the output, change the harness, run
`UPDATE_GOLDEN=1 cargo test --test golden` in `rust/`, and look at the new
SVG files in `spec/golden/` before you keep them.

## Credits

The look of the diagrams comes from
[stack-diagram](https://github.com/caru-ini/stack-diagram) by caru-ini: the
graph-paper canvas, the logo tiles with the label below, the tinted group
boxes, the colour values, and the rules for edges and labels. The order of
the logo sources (svgl, devicon, gilbarbara/logos) comes from that project
too. stack-diagram has the MIT License, Copyright (c) 2026 caru-ini.

swe-diagrams does not include code from that project. The language, the
automatic layout, the status model, and the harness are new.

## Versions

The project uses [semantic versioning](https://semver.org). The version is
in `rust/Cargo.toml`, and each release has a git tag such as `v0.1.0`.
[CHANGELOG.md](CHANGELOG.md) lists the changes. Before 1.0, a minor version
can change the language or the SVG output.

## License

MIT. See [LICENSE](LICENSE).
