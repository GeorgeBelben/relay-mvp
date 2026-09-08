#!/usr/bin/env bash
# Runs ON relay.local (via `ssh relay@relay.local 'bash -s' < this file`, from deploy.sh).
# Moves the binary deploy.sh just scp'd to /tmp into place. No sudo needed -- installs
# into the relay user's own ~/.local/bin rather than a root-owned system directory.
set -euo pipefail

mkdir -p "$HOME/.local/bin"
rm -f "$HOME/.local/bin/relay-cli"
mv /tmp/relay "$HOME/.local/bin/relay"
chmod +x "$HOME/.local/bin/relay"

echo "Installed to $HOME/.local/bin/relay"
"$HOME/.local/bin/relay" --help
