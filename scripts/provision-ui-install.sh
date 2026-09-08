#!/usr/bin/env bash
# One-time setup on relay.local so deploy-ui.sh can install the GUI's .deb without needing gh or
# a GitHub token on the device. Run this ON the device (e.g. `ssh relay@relay.local`, copy this
# file over, then `sudo bash provision-ui-install.sh`) -- it needs an interactive sudo password,
# so it's not something automated from the Mac side.
#
# What this sets up:
# 1. /opt/relay/incoming, owned by the `relay` user -- deploy-ui.sh scp's the .deb/service/cursor
#    theme here without needing root itself.
# 2. /usr/local/sbin/relay-install-deb.sh, a root-owned copy of relay-ui/scripts/relay-install-deb.sh
#    (already in this repo) -- does the actual dpkg -i + apt --fix-broken install.
# 3. A sudoers rule letting `relay` run that exact script, no password, no wildcard (sudoers
#    rejects wildcards in command arguments -- see relay-install-deb.sh's own header comment for
#    why this is a fixed-path wrapper rather than a `dpkg -i *.deb` rule).
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "Run this with sudo." >&2
  exit 1
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
