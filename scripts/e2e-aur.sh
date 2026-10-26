#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
#
# End-to-end test of AUR builds. Runs ONLY inside a throwaway Arch Linux
# container that boots systemd, because builds run in systemd's sandbox.
# Never run this on a real system.
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

pacman -Syu --noconfirm --needed rust clang lld git polkit
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

# A package without dependencies.
printf 'y\n' | kura install pfetch
pacman -Qi pfetch | grep -q "Explicitly installed"

# A package that needs git to build.
printf 'y\n' | kura install neofetch
pacman -Q neofetch

# Nothing from the build sandbox stays readable to other users.
test "$(stat -c %a /var/lib/private)" = 700

# Settings: the AUR can be turned off, and installs then stop with a hint.
kura settings aur off
kura settings | grep -q "AUR: off"
if kura install yay-bin; then exit 1; fi
kura settings aur on
kura settings aur-wait 0
kura settings | grep -q "0 days"

# An AUR update: make the installed pfetch look older, then update.
db=$(echo /var/lib/pacman/local/pfetch-*)
version=$(sed -n '/^%VERSION%$/{n;p}' "$db/desc")
sed -i "/^%VERSION%$/{n;s/.*/0.1-1/}" "$db/desc"
mv "$db" /var/lib/pacman/local/pfetch-0.1-1
kura check | grep -q "From the AUR: 1 ready"
printf 'y\ny\n' | kura update
test "$(pacman -Q pfetch)" = "pfetch $version"
pacman -Qi pfetch | grep -q "Explicitly installed"
