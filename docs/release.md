# Release steps

These steps make a release `vX.Y.Z`. The owner does them with their own
GitHub, crates.io, and npm accounts.

## 1. Set the version

On a new branch `chore/<issue>-release-X-Y-Z`, run:

```bash
scripts/bump-version.sh X.Y.Z
```

The script sets the version in `rust/Cargo.toml`, `rust/Cargo.lock`,
`.claude-plugin/plugin.json`, and `npm/package.json`.

## 2. Write the changelog

In `CHANGELOG.md`, change the heading `## Unreleased` to
`## X.Y.Z - YYYY-MM-DD`. Add a new empty `## Unreleased` section above it.
The release workflow uses the text of the `X.Y.Z` section as the release
notes, so the heading must have this format.

## 3. Check the packages

```bash
cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo publish --dry-run
cd ../npm && node --test && npm pack --dry-run
```

`npm pack --dry-run` must list only `LICENSE`, `README.md`,
`bin/swe-diagrams.js`, `lib/launcher.js`, and `package.json`.

## 4. Merge

Open a pull request, wait for the CI check, and merge it.

## 5. Tag

```bash
git switch main && git pull
git tag vX.Y.Z
git push origin vX.Y.Z
```

The tag starts the `release` workflow. Do not start it by hand.

## 6. Wait for the release workflow

Wait until the workflow is green. Then make sure that the GitHub release
`vX.Y.Z` has these files:

- `swe-diagrams-x86_64-unknown-linux-musl.tar.gz`
- `swe-diagrams-aarch64-unknown-linux-musl.tar.gz`
- `swe-diagrams-x86_64-apple-darwin.tar.gz`
- `swe-diagrams-aarch64-apple-darwin.tar.gz`
- `swe-diagrams-x86_64-pc-windows-msvc.zip`
- `SHA256SUMS`

The npm launcher downloads these files. Publish to npm only after they
are present.

## 7. Publish to crates.io

```bash
cd rust
cargo publish
```

## 8. Publish to npm

```bash
cd npm
npm publish
```

## 9. Check each install path

Do each check in a new temporary folder, with no old copy on the `PATH`.

```bash
# npm (the first run downloads the binary)
npx swe-diagrams@X.Y.Z --version

# crates.io
cargo install swe-diagrams --version X.Y.Z --root "$(mktemp -d)"

# install script
curl -fsSL https://raw.githubusercontent.com/Benny-Gil/swe-diagrams/main/install.sh | SWE_DIAGRAMS_BIN_DIR="$(mktemp -d)" SWE_DIAGRAMS_SKILL_DIR=none sh

# Claude Code plugin
/plugin install swe-diagrams --marketplace Benny-Gil/swe-diagrams
```

Each program must print `swe-diagrams X.Y.Z`. On Windows, do the npm check
in PowerShell too.
