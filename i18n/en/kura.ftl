# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later

## Command line

cli-usage =
    Usage: kura <command>

    Commands:
      doctor     Show a summary of this system for problem reports
      orphans    List packages that nothing needs any more
      sbom       Write a list of all installed software (CycloneDX)
      version    Show the version of kura
cli-unknown-command = Unknown command: { $command }
cli-missing-command = No command given.

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
