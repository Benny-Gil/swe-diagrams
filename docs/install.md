# Install swe-diagrams (steps for an agent)

These steps install the `swe-diagrams` program and the `swe-diagrams` skill.
Do the steps in order. Tell the user the result of each step.

## 1. Check for the program

Run `swe-diagrams --version`. If it prints a version, go to step 3.

## 2. Install the program

On macOS or Linux, run:

```bash
curl -fsSL https://raw.githubusercontent.com/Benny-Gil/swe-diagrams/main/install.sh | SWE_DIAGRAMS_SKILL_DIR=none sh
```

The script puts the program in `~/.local/bin`. If that folder is not on the
`PATH`, tell the user to add it, and use the full path for the next steps.

On Windows, or if the script fails, run:

```bash
cargo install --git https://github.com/Benny-Gil/swe-diagrams swe-diagrams
```

If `cargo` is not installed, stop and tell the user to install Rust from
https://rustup.rs.

## 3. Install the skill

Find the skills folder of the agent that you are:

| Agent | Skills folder |
|---|---|
| Claude Code | `~/.claude/skills` |
| Other agents | The folder that your own documentation names |

If you do not know the folder, ask the user.

Copy the folder `skills/swe-diagrams` from the repository into the skills
folder, so that the result is `<skills folder>/swe-diagrams/SKILL.md`. One
way:

```bash
tmp=$(mktemp -d)
curl -fsSL https://codeload.github.com/Benny-Gil/swe-diagrams/tar.gz/refs/heads/main | tar -xz -C "$tmp"
mkdir -p ~/.claude/skills
cp -R "$tmp"/swe-diagrams-main/skills/swe-diagrams ~/.claude/skills/
```

In Claude Code, the user can instead run
`/plugin install swe-diagrams --marketplace Benny-Gil/swe-diagrams`.

## 4. Draw a test diagram

Write this text to a file `test.sd` in a temporary folder:

```
arch "Test"
user "User" :actor
api "API" done
db "Database" :store wip
user => api "request"
api -> db "query"
```

Run `swe-diagrams render test.sd -o test.svg --offline`. The output must
start with `ok arch nodes=3 edges=2`. Give the user the path of
`test.svg`.

## 5. Finish

Tell the user that the skill is active in new sessions, and give two example
requests: "Draw the architecture of this repo" and "Make a plan diagram for
this change".
