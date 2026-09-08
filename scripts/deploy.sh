#!/usr/bin/env bash
# Runs on the dev Mac. Triggers the build-dev.yml workflow, waits for it, downloads
# the resulting relay binary, and hands it to install-on-device.sh over SSH.
#
# relay.local is x86_64 Ubuntu with no Rust toolchain, no `gh`, and no passwordless
# sudo -- building on GitHub's ubuntu-latest runner sidesteps cross-compiling from
# this (arm64) Mac, and copying the binary over avoids needing any of that on-device.
set -euo pipefail

REPO="GeorgeBelben/relay"
WORKFLOW="build-dev.yml"
DEVICE="relay@relay.local"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BRANCH=$(git rev-parse --abbrev-ref HEAD)

echo "Triggering $WORKFLOW on branch $BRANCH..."
TRIGGER_OUTPUT=$(gh workflow run "$WORKFLOW" --repo "$REPO" --ref "$BRANCH" 2>&1)
echo "$TRIGGER_OUTPUT"
RUN_ID=$(echo "$TRIGGER_OUTPUT" | grep -oE '/runs/[0-9]+' | grep -oE '[0-9]+' | tail -1)

if [ -z "$RUN_ID" ]; then
  echo "Could not parse run ID from trigger output; falling back to polling..."
  for _ in $(seq 1 15); do
    RUN_ID=$(gh run list --repo "$REPO" --workflow "$WORKFLOW" --branch "$BRANCH" --limit 1 --json databaseId --jq '.[0].databaseId' 2>/dev/null || true)
    if [ -n "$RUN_ID" ]; then
      break
    fi
    sleep 2
  done
fi

if [ -z "$RUN_ID" ]; then
  echo "Timed out waiting for the workflow run to appear." >&2
  exit 1
fi

echo "Watching run $RUN_ID..."
gh run watch "$RUN_ID" --repo "$REPO" --exit-status

echo "Build succeeded. Downloading artifact..."
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
gh run download "$RUN_ID" --repo "$REPO" --name relay --dir "$TMP_DIR"

BIN="$TMP_DIR/relay"
chmod +x "$BIN"

echo "Copying to $DEVICE..."
scp "$BIN" "$DEVICE:/tmp/relay"

echo "Installing on $DEVICE..."
ssh "$DEVICE" 'bash -s' < "$SCRIPT_DIR/install-on-device.sh"

echo "Deploy complete."
