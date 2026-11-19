#!/usr/bin/env bash
# Usage: scripts/release.sh <version>   (e.g. 0.2.0)
# Bumps versions, runs cargo check, prints (does not run) the git commands.
set -euo pipefail

version="${1:-}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  echo "usage: $0 <semver>  (e.g. 0.2.0)" >&2
  exit 1
fi

cd "$(dirname "$0")/.."

# Replace the first `version` entry of each file (top-level key / [package] version).
export NEW_VERSION="$version"
for f in src-tauri/tauri.conf.json ui/package.json; do
  perl -0pi -e 's/("version"\s*:\s*")[^"]*(")/$1$ENV{NEW_VERSION}$2/' "$f"
done
for f in src-tauri/Cargo.toml crates/shark-core/Cargo.toml; do
  perl -0pi -e 's/^(version\s*=\s*")[^"]*(")/$1$ENV{NEW_VERSION}$2/m' "$f"
done

cargo check --workspace

cat <<MSG

Version bumped to $version. Review with git diff, then run:

  git add -A
  git commit -m "chore: release v$version"
  git tag v$version
  git push origin HEAD v$version
MSG
