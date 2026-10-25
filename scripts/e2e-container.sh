#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
#
# End-to-end test. Runs ONLY inside a throwaway Arch Linux container as root:
# it installs kura's files like the package does, starts D-Bus and polkit and
# makes real changes through kura-helper. Never run this on a real system.
set -euxo pipefail

if [ ! -f /.dockerenv ] && [ -z "${KURA_E2E_CONTAINER:-}" ]; then
  echo "refusing to run outside a container" >&2
  exit 1
fi

bin=${1:-target/debug}
install -Dm755 "$bin/kura-helper" /usr/lib/kura/kura-helper
install -Dm755 "$bin/kura" /usr/bin/kura
install -Dm644 packaging/org.kura.Helper1.service /usr/share/dbus-1/system-services/org.kura.Helper1.service
install -Dm644 packaging/org.kura.Helper1.conf /usr/share/dbus-1/system.d/org.kura.Helper1.conf
install -Dm644 packaging/org.kura.policy /usr/share/polkit-1/actions/org.kura.policy
install -Dm644 packaging/kura-history.hook /usr/share/libalpm/hooks/kura-history.hook

# Test only: root may use kura's actions without a password prompt.
install -Dm644 /dev/stdin /etc/polkit-1/rules.d/10-kura-e2e.rules <<'RULES'
polkit.addRule(function (action, subject) {
  if (action.id.indexOf("org.kura.") === 0 && subject.user === "root") {
    return polkit.Result.YES;
  }
});
RULES

mkdir -p /run/dbus
dbus-daemon --system --fork
/usr/lib/polkit-1/polkitd --no-debug &
for _ in $(seq 50); do
  busctl --system status org.freedesktop.PolicyKit1 >/dev/null 2>&1 && break
  sleep 0.2
done

printf 'y\n' | kura install figlet
pacman -Q figlet
grep -q '"cause":"kura:install"' /var/lib/kura/history.jsonl

printf 'y\n' | kura remove figlet
if pacman -Q figlet 2>/dev/null; then
  echo "figlet is still installed" >&2
  exit 1
fi
grep -q '"cause":"kura:remove"' /var/lib/kura/history.jsonl

# Saying no changes nothing.
printf 'n\n' | kura install figlet
if pacman -Q figlet 2>/dev/null; then
  echo "figlet was installed although the answer was no" >&2
  exit 1
fi

kura history
echo "e2e passed"
