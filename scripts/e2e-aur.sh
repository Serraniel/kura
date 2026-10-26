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
