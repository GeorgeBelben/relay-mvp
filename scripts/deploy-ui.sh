#!/usr/bin/env bash
# Runs on the dev Mac. Triggers build-dev-ui.yml, waits for it, downloads the resulting
# relay-gui .deb + systemd unit + cursor theme, and hands them to install-ui-on-device.sh over
# SSH. Mirrors deploy.sh's pattern for relay-cli: the Mac (already gh-authenticated) does the
# downloading, the device never needs gh or a GitHub token.
#
# One-time device setup this depends on (not yet done on relay.local -- see
# relay-ui/scripts/relay-install-deb.sh's header comment): a root-owned copy of that script at
# /usr/local/sbin/relay-install-deb.sh, and a sudoers rule granting the `relay` user NOPASSWD
# execution of that exact path (sudoers rejects wildcards in command arguments, hence the
# fixed-path wrapper rather than a `dpkg -i *.deb` rule).
set -euo pipefail

REPO="GeorgeBelben/relay"
WORKFLOW="build-dev-ui.yml"
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
gh run download "$RUN_ID" --repo "$REPO" --name relay-gui --dir "$TMP_DIR"

DEB_FILE=$(find "$TMP_DIR" -name '*.deb' | head -1)
if [ -z "$DEB_FILE" ]; then
  echo "No .deb file found in artifact." >&2
  exit 1
fi

echo "Copying to $DEVICE..."
# Matches the paths relay-ui/scripts/relay-install-deb.sh (the existing root-owned wrapper, see this
# script's header) already expects -- /opt/relay/incoming must be relay-user-writable, a one-time
# provisioning step, not something this script creates.
scp "$DEB_FILE" "$DEVICE:/opt/relay/incoming/relay-gui.deb"
if [ -f "$TMP_DIR/relay.service" ]; then
  scp "$TMP_DIR/relay.service" "$DEVICE:/opt/relay/incoming/relay.service"
fi
if [ -d "$TMP_DIR/relay-blank" ]; then
  ssh "$DEVICE" 'rm -rf /opt/relay/incoming/cursor-theme && mkdir -p /opt/relay/incoming/cursor-theme'
  scp -r "$TMP_DIR/relay-blank" "$DEVICE:/opt/relay/incoming/cursor-theme/"
fi

echo "Installing on $DEVICE..."
ssh "$DEVICE" 'bash -s' < "$SCRIPT_DIR/install-ui-on-device.sh"

echo "Deploy complete."
