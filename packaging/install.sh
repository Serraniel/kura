#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: CC0-1.0
#
# Installs kura's files. Used by the PKGBUILD (DESTDIR=$pkgdir) and by the
# end-to-end test, so both install exactly the same files.
# Usage: install.sh <destdir> <directory with built binaries>
set -euo pipefail

dest=${1:?destination}
bin=${2:?binary directory}
here=$(dirname "$0")

install -Dm755 "$bin/kura" "$dest/usr/bin/kura"
if [ -f "$bin/kura-gui" ]; then
  install -Dm755 "$bin/kura-gui" "$dest/usr/bin/kura-gui"
fi
install -Dm755 "$bin/kura-helper" "$dest/usr/lib/kura/kura-helper"
if [ -f "$bin/kura-agent" ]; then
  install -Dm755 "$bin/kura-agent" "$dest/usr/lib/kura/kura-agent"
fi

install -Dm644 "$here/org.kura.Helper1.service" "$dest/usr/share/dbus-1/system-services/org.kura.Helper1.service"
install -Dm644 "$here/org.kura.Helper1.conf" "$dest/usr/share/dbus-1/system.d/org.kura.Helper1.conf"
install -Dm644 "$here/org.kura.policy" "$dest/usr/share/polkit-1/actions/org.kura.policy"
install -Dm644 "$here/kura-helper.service" "$dest/usr/lib/systemd/system/kura-helper.service"
install -Dm644 "$here/kura-check.service" "$dest/usr/lib/systemd/system/kura-check.service"
install -Dm644 "$here/kura-check.timer" "$dest/usr/lib/systemd/system/kura-check.timer"
# Timers are enabled by the package, like man-db and shadow do.
install -dm755 "$dest/usr/lib/systemd/system/timers.target.wants"
ln -sf ../kura-check.timer "$dest/usr/lib/systemd/system/timers.target.wants/kura-check.timer"
install -Dm644 "$here/kura-history.hook" "$dest/usr/share/libalpm/hooks/kura-history.hook"
install -Dm644 "$here/org.kura.app.desktop" "$dest/usr/share/applications/org.kura.app.desktop"
install -Dm644 "$here/org.kura.agent.desktop" "$dest/etc/xdg/autostart/org.kura.agent.desktop"
install -Dm644 "$here/icons/kura.svg" "$dest/usr/share/icons/hicolor/scalable/apps/kura.svg"
install -Dm644 "$here/icons/kura-symbolic.svg" "$dest/usr/share/icons/hicolor/symbolic/apps/kura-symbolic.svg"
