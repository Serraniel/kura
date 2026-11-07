#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
#
# End-to-end test of trial runs, through the real helper service. Runs ONLY
# inside a throwaway Arch Linux container that boots systemd and has btrfs as
# its root file system. Never run this on a real system.
set -euxo pipefail

if [ ! -f /.dockerenv ] && [ -z "${KURA_E2E_CONTAINER:-}" ]; then
  echo "refusing to run outside a container" >&2
  exit 1
fi

for _ in $(seq 60); do
  state=$(systemctl is-system-running || true)
  [ "$state" = running ] || [ "$state" = degraded ] && break
  sleep 1
done

test "$(findmnt -no FSTYPE /)" = btrfs

pacman -Syu --noconfirm --needed rust clang lld git polkit btrfs-progs
cd /src
cargo build --locked -p kura-cli -p kura-helper
packaging/install.sh / target/debug
systemctl daemon-reload
systemctl reload dbus

# Test only: root may use kura's actions without a password prompt.
install -Dm644 /dev/stdin /etc/polkit-1/rules.d/10-kura-e2e.rules <<'RULES'
polkit.addRule(function (action, subject) {
  if (action.id.indexOf("org.kura.") === 0 && subject.user === "root") {
    return polkit.Result.YES;
  }
});
RULES

# Make the installed figlet look older, so an update is waiting.
pacman -S --noconfirm figlet
db=$(echo /var/lib/pacman/local/figlet-*)
version=$(sed -n '/^%VERSION%$/{n;p}' "$db/desc")
sed -i "/^%VERSION%$/{n;s/.*/0.1-1/}" "$db/desc"
mv "$db" /var/lib/pacman/local/figlet-0.1-1

since=$(date '+%F %T')
printf 'y\ny\n' | kura update 2>&1 | tee /tmp/kura-update.log

journal() { journalctl -u kura-helper --since "$since" --no-pager; }
journal
# The trial really ran, and it went well.
journal | grep -q "kura: trial run passed"
if journal | grep -q "kura: no trial run"; then exit 1; fi
# nspawn's two messages that looked like errors are gone.
if journal | grep -Eq "refusing to link journals|in a future version of systemd-nspawn"; then
  exit 1
fi
# The real update ran after it, and the snapshot is gone.
test "$(pacman -Q figlet)" = "figlet $version"
test ! -e /var/lib/kura/trial

echo "e2e-trial passed"
