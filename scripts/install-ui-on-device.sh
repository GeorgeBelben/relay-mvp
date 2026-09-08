#!/usr/bin/env bash
# Runs ON relay.local (via `ssh relay@relay.local 'bash -s' < this file`, from deploy-ui.sh).
# Just invokes the existing root-owned wrapper -- see relay-ui/scripts/relay-install-deb.sh's own header
# comment for why installing the .deb needs a fixed-path sudoers rule rather than a wildcard one.
set -euo pipefail

sudo /usr/local/sbin/relay-install-deb.sh
echo "Installed. relay.service status:"
systemctl is-active relay.service || true
