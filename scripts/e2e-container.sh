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
packaging/install.sh / "$bin"

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

# Choosing download servers rewrites the mirror list and keeps the old one.
kura mirrors
grep -q "chosen by kura" /etc/pacman.d/mirrorlist
ls /etc/pacman.d/mirrorlist.kura-*

# The background check writes a status and installs nothing.
/usr/lib/kura/kura-helper check
test -s /var/lib/kura/status.json
cat /var/lib/kura/status.json

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

# Going back: install figlet, then return to the state before it.
printf 'y\n' | kura install figlet
pacman -Q figlet
printf 'y\n' | kura undo
if pacman -Q figlet 2>/dev/null; then
  echo "kura undo did not remove figlet" >&2
  exit 1
fi
grep -q '"cause":"kura:restore"' /var/lib/kura/history.jsonl

# Language: a German user gets the German dictionary and German OCR data.
printf 'j\n' | LANG=de_DE.UTF-8 kura install hunspell
pacman -Q hunspell-de
printf 'j\n' | LANG=de_DE.UTF-8 kura install tesseract
pacman -Q tesseract-data-deu
printf 'j\n' | LANG=de_DE.UTF-8 kura remove tesseract hunspell hunspell-de
if pacman -Q tesseract-data-deu; then exit 1; fi

kura security
grep -q '"checked"' /var/lib/kura/status.json

kura history
echo "e2e passed"
