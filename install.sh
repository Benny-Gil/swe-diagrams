#!/bin/sh
# Installs the swe-diagrams binary and the agent skill.
#
#   curl -fsSL https://raw.githubusercontent.com/Benny-Gil/swe-diagrams/main/install.sh | sh
#
# Settings (environment variables):
#   SWE_DIAGRAMS_VERSION    a tag such as v0.2.0. The default is the latest release.
#   SWE_DIAGRAMS_BIN_DIR    where the binary goes. The default is ~/.local/bin.
#   SWE_DIAGRAMS_SKILL_DIR  where the skill goes. The default is ~/.claude/skills.
#                           Set it to "none" to skip the skill.
set -eu

REPO="Benny-Gil/swe-diagrams"
BIN_DIR="${SWE_DIAGRAMS_BIN_DIR:-$HOME/.local/bin}"
SKILL_DIR="${SWE_DIAGRAMS_SKILL_DIR:-$HOME/.claude/skills}"
VERSION="${SWE_DIAGRAMS_VERSION:-}"

say() { printf 'swe-diagrams: %s\n' "$1"; }
fail() { printf 'swe-diagrams: error: %s\n' "$1" >&2; exit 1; }

command -v curl >/dev/null 2>&1 || fail "curl is necessary"
command -v tar >/dev/null 2>&1 || fail "tar is necessary"

# The latest release, from the redirect of the releases/latest page.
if [ -z "$VERSION" ]; then
  VERSION=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest" | sed 's#.*/tag/##')
  case "$VERSION" in v*) ;; *) fail "cannot find the latest release" ;; esac
fi

case "$(uname -s)" in
  Linux) os=unknown-linux-musl ;;
  Darwin) os=apple-darwin ;;
  *) fail "no prebuilt binary for $(uname -s). Use: cargo install --git https://github.com/$REPO swe-diagrams" ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  arm64 | aarch64) arch=aarch64 ;;
  *) fail "no prebuilt binary for $(uname -m). Use: cargo install --git https://github.com/$REPO swe-diagrams" ;;
esac
target="$arch-$os"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

base="https://github.com/$REPO/releases/download/$VERSION"
say "downloading $VERSION for $target"
# No -S: a missing binary is a normal case, and the cargo path follows.
if curl -fsL "$base/swe-diagrams-$target.tar.gz" -o "$tmp/bin.tar.gz"; then
  # Check the file against the published checksum when a tool is present.
  if curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS"; then
    want=$(grep "swe-diagrams-$target.tar.gz" "$tmp/SHA256SUMS" | cut -d' ' -f1)
    if command -v sha256sum >/dev/null 2>&1; then
      got=$(sha256sum "$tmp/bin.tar.gz" | cut -d' ' -f1)
    else
      got=$(shasum -a 256 "$tmp/bin.tar.gz" | cut -d' ' -f1)
    fi
    [ "$want" = "$got" ] || fail "the checksum of the download is wrong"
  fi
  tar -xzf "$tmp/bin.tar.gz" -C "$tmp"
  mkdir -p "$BIN_DIR"
  mv "$tmp/swe-diagrams" "$BIN_DIR/swe-diagrams"
  chmod +x "$BIN_DIR/swe-diagrams"
elif command -v cargo >/dev/null 2>&1; then
  say "no prebuilt binary for $target; building it with cargo"
  cargo install --quiet --git "https://github.com/$REPO" --tag "$VERSION" --root "$tmp/cargo" swe-diagrams
  mkdir -p "$BIN_DIR"
  mv "$tmp/cargo/bin/swe-diagrams" "$BIN_DIR/swe-diagrams"
else
  fail "no prebuilt binary for $target, and cargo is not installed"
fi
say "installed $("$BIN_DIR/swe-diagrams" --version) in $BIN_DIR"

if [ "$SKILL_DIR" != "none" ]; then
  curl -fsSL "https://codeload.github.com/$REPO/tar.gz/refs/tags/$VERSION" -o "$tmp/src.tar.gz"
  tar -xzf "$tmp/src.tar.gz" -C "$tmp"
  mkdir -p "$SKILL_DIR"
  rm -rf "$SKILL_DIR/swe-diagrams"
  cp -R "$tmp"/swe-diagrams-*/skills/swe-diagrams "$SKILL_DIR/swe-diagrams"
  say "installed the skill in $SKILL_DIR/swe-diagrams"
fi

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) say "add $BIN_DIR to your PATH" ;;
esac
