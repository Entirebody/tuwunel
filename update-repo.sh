#!/usr/bin/env bash

set -euo pipefail

eecho () {
    >&2 echo "$@"
}

if [[ ! -x ./make_pub.sh ]]; then
    eecho "make_pub.sh is missing or not executable"
    exit 1
fi
MAKE_PUB_SCRIPT="$(cat ./make_pub.sh)"
if [[ -z "$MAKE_PUB_SCRIPT" ]]; then
    eecho "make_pub.sh is empty"
    exit 1
fi

MIRROR='git@github.com:matrix-construct/tuwunel.git'
if [[ "$(2>/dev/null git remote get-url mirror || true)" != "$MIRROR" ]]; then
    eecho "git remote mirror is not '$MIRROR'"
    eecho "run 'git remote add mirror $MIRROR' to fix"
    exit 1
fi

ORIGIN="$(2>/dev/null git remote get-url origin || true)"
if [[ -z "$ORIGIN" ]]; then
    eecho "git remote origin is not set"
    eecho "run 'git remote add origin <url>' to fix"
    exit 1
fi

if [[ "$ORIGIN" = "$MIRROR" ]]; then
    eecho "git remote origin is the same as mirror"
    eecho "set origin to a fork of the repo"
    exit 1
fi

git checkout main

echo 'Fetching tags in mirror'
git fetch -t mirror

LATEST_TAG="$(git ls-remote --tags mirror | cut -f2 | sort -n | tail -n1)"
echo "Latest tag in mirror: $LATEST_TAG"

if git merge-base --is-ancestor "$LATEST_TAG" HEAD; then
    echo "HEAD is already ahead of $LATEST_TAG"
else
    echo "Resetting to $LATEST_TAG"
    git reset --hard "$LATEST_TAG"
fi

echo "Running make_pub.sh"
/usr/bin/env bash -c "$MAKE_PUB_SCRIPT"

if ! git add . && git diff --quiet && git diff --cached --quiet; then
    echo "No changes to commit"
    exit 0
else
    NEW_TAG="${LATEST_TAG#"refs/tags/"}_pub"
    echo "Committing changes and tagging as $NEW_TAG"
    git commit -m "!PUB $NEW_TAG"
    git tag -m "${NEW_TAG}" "$NEW_TAG"
fi
