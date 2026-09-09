#!/usr/bin/env bash
# One-time setup on relay.local so deploy-ui.sh can install the GUI's .deb without needing gh or
# a GitHub token on the device. Run this ON the device (e.g. `ssh relay@relay.local`, copy this
# file over, then `sudo bash provision-ui-install.sh`) -- it needs an interactive sudo password,
# so it's not something automated from the Mac side.
#
# What this sets up:
# 0. The `relay` system user itself, if it doesn't already exist (a fresh device won't have one --
#    relay.service runs as User=relay on /dev/tty1 via PAM, and this script's own steps below chown
#    to it). video/input/render/audio group membership matches what cage needs for direct DRM/input
#    access on a device with no display manager doing seat assignment for it.
# 1. cage itself (/usr/bin/cage) -- relay.service's ExecStart runs `cage -- relay-gui`, but the
#    relay .deb only depends on relay-gui's own GTK/webview libraries, not the compositor it runs
#    under. This was apparently installed by hand on relay.local at some point and never captured
#    anywhere -- confirmed missing (and the service crash-looping on 203/EXEC because of it) when
#    provisioning artemis.local from scratch.
# 2. /opt/relay/incoming, owned by the `relay` user -- deploy-ui.sh scp's the .deb/service/cursor
#    theme here without needing root itself.
# 3. /usr/local/sbin/relay-install-deb.sh, a root-owned copy of relay-ui/scripts/relay-install-deb.sh
#    (already in this repo) -- does the actual dpkg -i + apt --fix-broken install.
# 4. A sudoers rule letting `relay` run that exact script, no password, no wildcard (sudoers
#    rejects wildcards in command arguments -- see relay-install-deb.sh's own header comment for
#    why this is a fixed-path wrapper rather than a `dpkg -i *.deb` rule).
#
# Safe to re-run: every step is guarded so this is a no-op on a device that's already provisioned.
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "Run this with sudo." >&2
  exit 1
fi

if ! id relay >/dev/null 2>&1; then
  useradd -m -s /bin/bash -G video,input,render,audio relay
  echo "Created relay user (groups: video,input,render,audio) -- verify this matches relay.local if it's reachable."
fi

# The Mac's own key, so deploy.sh/deploy-ui.sh can `ssh relay@<device>` passwordlessly, matching
# however relay.local's `relay` user was already set up for the same key.
RELAY_HOME=$(getent passwd relay | cut -d: -f6)
mkdir -p "$RELAY_HOME/.ssh"
touch "$RELAY_HOME/.ssh/authorized_keys"
if ! grep -qF "george.belben@outlook.com" "$RELAY_HOME/.ssh/authorized_keys"; then
  echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIO9eoEs5wqBeLYwC3tSkcIJDECLSC5OUfiRYtZqRFRYK george.belben@outlook.com" >> "$RELAY_HOME/.ssh/authorized_keys"
fi
chmod 700 "$RELAY_HOME/.ssh"
chmod 600 "$RELAY_HOME/.ssh/authorized_keys"
chown -R relay:relay "$RELAY_HOME/.ssh"

if ! command -v cage >/dev/null 2>&1; then
  apt-get update
  apt-get install -y cage
fi

mkdir -p /opt/relay/incoming
chown relay:relay /opt/relay/incoming

# Embedded verbatim (not read from a repo checkout) so this one file is self-contained -- only
# this script needs to be copied to the device, not the whole repo. Keep in sync with
# relay-ui/scripts/relay-install-deb.sh if that ever changes.
cat > /usr/local/sbin/relay-install-deb.sh <<'INSTALLER'
#!/usr/bin/env bash
# Installed on the device at /usr/local/sbin/relay-install-deb.sh, owned by root.
# Wrapping dpkg -i in a fixed-path script (rather than a sudoers wildcard rule)
# avoids the "wildcards are not allowed in command arguments" sudoers restriction.
set -euo pipefail

DEB=$(find /opt/relay/incoming -maxdepth 1 -name '*.deb' | head -1)
if [ -z "$DEB" ]; then
  echo "No .deb found in /opt/relay/incoming" >&2
  exit 1
fi

# `apt install ./file.deb` doesn't reliably resolve local-file dependencies on this
# system (apt reports them as "not going to be installed" despite valid candidates).
# dpkg -i followed by apt --fix-broken install is the standard robust recipe instead.
dpkg -i "$DEB" || true
apt-get install -f -y

CURSOR_THEME=/opt/relay/incoming/cursor-theme/relay-blank
if [ -d "$CURSOR_THEME" ]; then
  rm -rf /usr/share/icons/relay-blank
  cp -r "$CURSOR_THEME" /usr/share/icons/relay-blank
fi

SERVICE=/opt/relay/incoming/relay.service
if [ -f "$SERVICE" ]; then
  cp "$SERVICE" /etc/systemd/system/relay.service
  systemctl daemon-reload
  systemctl enable relay.service
  systemctl restart relay.service
fi
INSTALLER
chown root:root /usr/local/sbin/relay-install-deb.sh
chmod 755 /usr/local/sbin/relay-install-deb.sh

cat > /etc/sudoers.d/relay-install-deb <<'EOF'
relay ALL=(root) NOPASSWD: /usr/local/sbin/relay-install-deb.sh
EOF
chmod 440 /etc/sudoers.d/relay-install-deb
visudo -c

echo "Done. relay can now run: sudo /usr/local/sbin/relay-install-deb.sh"
