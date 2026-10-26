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

# A repository profile comes and goes, leaving pacman.conf as it was.
# Real installs create pacman's local signing key; the Docker image has none.
pacman-key --init
cp /etc/pacman.conf /tmp/pacman.conf.before
printf 'y\n' | kura profiles add chaotic-aur
pacman-conf --repo-list | grep -qx chaotic-aur
pacman -Q chaotic-keyring chaotic-mirrorlist
pacman-key --list-keys EF925EA60F33D0CB85C44AD13056513887B78AEB
printf 'y\n' | kura profiles remove chaotic-aur
cmp /etc/pacman.conf /tmp/pacman.conf.before
if pacman -Q chaotic-keyring; then exit 1; fi
if pacman-key --list-keys EF925EA60F33D0CB85C44AD13056513887B78AEB; then exit 1; fi

# CachyOS: add it, switch every package, go back, and nothing is left.
if grep -qw avx2 /proc/cpuinfo; then
  cp /etc/pacman.conf /tmp/pacman.conf.before-cachyos
  printf 'y\nn\n' | kura profiles add cachyos
  pacman-conf --repo-list | grep -q '^cachyos-core-v[34]$'
  # A crash leaves a core dump with a backtrace in the log.
  pacman -S --noconfirm --needed gdb
  echo /tmp/core.%p >/proc/sys/kernel/core_pattern || true
  ulimit -c unlimited
  if ! printf 'y\n' | kura profiles switch cachyos; then
    pacman-conf Architecture
    /lib/ld-linux-x86-64.so.2 --help | grep -i "x86-64-v" || true
    pacman -Syuw --noconfirm 2>&1 | grep -iv "downloading\|^ " | head -20 || true
    for core in /tmp/core.*; do
      gdb -batch -ex 'thread apply all bt' "$(command -v kura)" "$core" || true
    done
    exit 1
  fi
  test "$(pacman -Qi | grep -c '^Architecture *: x86_64_v[34]')" -gt 0
  printf 'y\ny\n' | kura profiles remove cachyos
  cmp /etc/pacman.conf /tmp/pacman.conf.before-cachyos
  test "$(pacman -Qi | grep -c '^Architecture *: x86_64_v[34]')" -eq 0
  if pacman -Q cachyos-keyring; then exit 1; fi
fi

# Kernels: one can be added; the only kernel is never removed.
printf 'y\n' | kura kernels add linux-lts
pacman -Q linux-lts
if kura kernels remove linux-lts; then exit 1; fi
pacman -Q linux-lts
# kura remembers the kernel it added, to offer removing the old one later.
grep -q '"added":"linux-lts"' /var/lib/kura/kernel-added.json

# Search reaches the AUR.
kura search paru | grep -q "From the AUR"

# Snapshots: the root of this container is not btrfs, so nothing is offered.
kura snapshots | grep -q "Not possible"
kura snapshots setup | grep -q "Nothing to set up"

# A snapper configuration next to an existing .snapshots mount, on a btrfs
# image laid out like a typical install (@ and @snapshots).
pacman -S --noconfirm --needed btrfs-progs snapper
truncate -s 512M /tmp/kura-btrfs.img
mkfs.btrfs -q /tmp/kura-btrfs.img
dev=$(losetup -f --show /tmp/kura-btrfs.img)
mkdir -p /mnt/kura-top /mnt/kura-root
mount "$dev" /mnt/kura-top
btrfs subvolume create /mnt/kura-top/@
btrfs subvolume create /mnt/kura-top/@snapshots
mount -o subvol=@ "$dev" /mnt/kura-root
mkdir /mnt/kura-root/.snapshots
mount -o subvol=@snapshots "$dev" /mnt/kura-root/.snapshots
KURA_TEST_BTRFS=/mnt/kura-root cargo test --locked -p kura-helper --test snapper -- --ignored
snapper --no-dbus -c kuratest create -d e2e
btrfs subvolume list /mnt/kura-top | grep -q "@snapshots/1/snapshot"
