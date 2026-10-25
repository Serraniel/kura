# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later

## Command line

cli-usage =
    Usage: kura <command>

    Commands:
      check      Look for updates and show what would change
      doctor     Show a summary of this system for problem reports
      history    Show saved states of this system
      orphans    List packages that nothing needs any more
      sbom       Write a list of all installed software (CycloneDX)
      version    Show the version of kura
cli-unknown-command = Unknown command: { $command }
cli-missing-command = No command given.

## check

check-running = Looking for updates…
check-none = Everything is up to date.
check-found =
    { $count ->
        [one] One update is available.
       *[other] { $count } updates are available.
    }
check-group-apps = Apps ({ $count })
check-group-system = System parts ({ $count })
check-group-replaced = Replaced during the update ({ $count })
check-group-cleanup = No longer needed ({ $count })
check-sizes = Download: { $download } · Disk space: { $delta }
check-preview-only = Nothing was changed. Installing updates comes with a later version of kura.
check-blocked = The update cannot be prepared:
problem-unsatisfied = { $target } needs { $dependency }, which is no longer available.
problem-conflict = { $first } and { $second } cannot be installed together.
problem-arch = Built for a different processor type: { $packages }
problem-not-found = No package named { $name } was found.
error-check = Could not look for updates: { $reason }

## history

history-title = Saved states of this system
history-empty = Nothing has been saved yet. kura saves every change from now on.
history-today = today
history-yesterday = yesterday
history-days-ago =
    { $days ->
        [one] one day ago
       *[other] { $days } days ago
    }
history-installed = { $count } installed
history-upgraded = { $count } updated
history-downgraded = { $count } went back to an older version
history-removed = { $count } removed
history-start = Starting point: { $count } packages
history-outside = (changed outside kura)
list-separator = {", "}

## doctor

doctor-title = System summary
doctor-distro = Distribution
doctor-packages = Installed packages
doctor-explicit = Installed on purpose
doctor-foreign = From the AUR or built locally
doctor-orphans = No longer needed
doctor-language = Language

## orphans

orphans-none = Nothing to clean up. Every installed package is still needed.
orphans-found =
    { $count ->
        [one] One package is no longer needed:
       *[other] { $count } packages are no longer needed:
    }
orphans-hint = kura will offer to remove them after the next update.

## sbom

sbom-written = Software list written to { $path }.

## Errors

error-database = Could not read the package database: { $reason }
error-write = Could not write { $path }: { $reason }

## Window

nav-overview = Overview
nav-history = History
nav-settings = Settings
overview-subtitle = A short overview of this computer.
overview-refresh = Refresh
prototype-note = This is an early prototype. It only reads information and changes nothing.
