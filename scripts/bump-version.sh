#!/bin/sh
# Sets the version in each file that holds it.
#
#   scripts/bump-version.sh 0.3.0
set -eu

v="${1:-}"
echo "$v" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || {
  echo "usage: scripts/bump-version.sh X.Y.Z" >&2
  exit 2
}
cd "$(dirname "$0")/.."

# Write to a temp file and move it, because BSD sed and GNU sed differ on -i.
edit() {
  file="$1"
  shift
  awk -v v="$v" "$@" "$file" > "$file.tmp"
  mv "$file.tmp" "$file"
}

# Only the first version line in Cargo.toml is the package version.
edit rust/Cargo.toml '!done && /^version = "/ { $0 = "version = \"" v "\""; done = 1 } { print }'
# In Cargo.lock, the version line follows the name line of the package.
edit rust/Cargo.lock 'pkg && /^version = "/ { $0 = "version = \"" v "\"" } { pkg = ($0 == "name = \"swe-diagrams\"") ; print }'
for f in .claude-plugin/plugin.json npm/package.json; do
  edit "$f" '!done && /^  "version": "/ { sub(/"version": "[^"]*"/, "\"version\": \"" v "\""); done = 1 } { print }'
done

# Each file must now hold the new version.
grep -q "^version = \"$v\"" rust/Cargo.toml
grep -A1 '^name = "swe-diagrams"$' rust/Cargo.lock | grep -q "^version = \"$v\""
grep -q "^  \"version\": \"$v\"" .claude-plugin/plugin.json
grep -q "^  \"version\": \"$v\"" npm/package.json
echo "version $v in rust/Cargo.toml, rust/Cargo.lock, .claude-plugin/plugin.json, npm/package.json"
