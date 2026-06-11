#!/usr/bin/env bash
# release.sh — the one true way to cut a release.
# Usage: ./scripts/release.sh 0.1.5
# Bumps Cargo.toml (+ lock), commits, tags vX.Y.Z. Pushing stays a human act:
#   git push origin main vX.Y.Z
set -euo pipefail

VERSION="${1:?usage: release.sh X.Y.Z (no leading v)}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "version must be X.Y.Z" >&2; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "working tree not clean" >&2; exit 1; }
git rev-parse -q --verify "refs/tags/v$VERSION" >/dev/null && { echo "tag v$VERSION already exists" >&2; exit 1; }

sed -i.bak "0,/^version = \".*\"/s//version = \"$VERSION\"/" Cargo.toml && rm Cargo.toml.bak
cargo check --quiet   # refresh Cargo.lock
git add Cargo.toml Cargo.lock
git commit -m "chore(release): v$VERSION"
git tag "v$VERSION"
echo "release v$VERSION ready — push with: git push origin HEAD v$VERSION"
