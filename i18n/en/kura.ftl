# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later

## Command line

cli-usage =
    Usage: kura <command>

    Commands:
      check      Look for updates and show what would change
      doctor     Show a summary of this system for problem reports
      history    Show saved states of this system
      mirrors    Choose fast download servers
      orphans    List packages that nothing needs any more
      sbom       Write a list of all installed software (CycloneDX)
      update     Install all updates (shows a preview first)
      install    Install an app: kura install <name>
      remove     Remove an app and what only it needed: kura remove <name>
      security   Show known security problems of installed software
      undo       Go back to an earlier state: kura undo [number]
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
change-blocked = The change cannot be prepared:
problem-unsatisfied = { $target } needs { $dependency }, which is no longer available.
problem-conflict = { $first } and { $second } cannot be installed together.
problem-arch = Built for a different processor type: { $packages }
problem-not-found = No package named { $name } was found.
problem-not-restorable = kura cannot bring back { $name } { $version }: it came from the AUR or was built on this computer, and the file is no longer in the package cache.
undo-title = Going back to state #{ $id } from { $when }:
undo-limits = Settings and personal files are not changed. The next update brings the newer versions back, so use this to get around a problem until a fix is released.
undo-nothing = This is already the current state.
undo-unknown = There is no saved state #{ $id }. kura history lists the saved states.
confirm-undo = Go back to this state? [y/N]
mirrors-running = Testing download servers near you…
mirrors-done = { $count } fast download servers are now used. The previous list was kept as { $backup }.
solver-found =
    { $count ->
        [one] kura found a solution: one old part that no app uses any more blocks the update. It is removed, everything else is updated.
       *[other] kura found a solution: { $count } old parts that no app uses any more block the update. They are removed, everything else is updated.
    }
solver-tested = kura tested this solution without changing anything.
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

## Applying changes

preview-title = This will change:
confirm-update = Install these updates? [y/N]
news-title = Arch Linux news about this update:
news-unavailable = The Arch Linux news could not be loaded. Please read https://archlinux.org/news/ before updating.
confirm-update-news = Have you read the news above, and do you want to install the updates? [y/N]
confirm-install = Install this? [y/N]
confirm-remove = Remove this? [y/N]
confirm-yes = y
apply-cancelled = Nothing was changed.
apply-refreshing = Refreshing the package lists…
apply-downloading = Downloading { $file }
apply-checking = Checking the packages…
apply-installing = Installing { $package } ({ $current }/{ $total })
apply-removing = Removing { $package } ({ $current }/{ $total })
apply-hook = Finishing: { $name }
apply-keyring = Updating the signing keys first…
apply-cleanup = Removing parts that are no longer needed…
apply-done = Done. The change is saved in the history.
apply-changed = Something changed since the preview. Please check the new preview.
apply-not-authorized = The change was not allowed. Either the password was not entered, or this account may not install software.
apply-locked = Another program is installing software right now. Please try again when it has finished.
apply-busy = kura is already making another change. Please wait until it has finished.
apply-failed = The change failed: { $reason }
error-helper = kura's system service is not available: { $reason }

## Tray

tray-uptodate = Everything is up to date
tray-updates =
    { $count ->
        [one] One update is ready
       *[other] { $count } updates are ready
    }
tray-attention = An update needs your attention
tray-unknown = kura has not looked for updates yet
tray-open-hint = Click to open kura.
notify-updates-body = Open kura to see what changes before installing.
notify-attention-body = Open kura to see what to do. Nothing has been changed.
notify-open = Open kura

## Security

security-one-app = { $app } has a known security problem
security-more-apps =
    { $count ->
        [one] { $app } and one other app have a known security problem
       *[other] { $app } and { $count } other apps have a known security problem
    }
security-system = The system has a known security problem
security-fix = An update fixes it. Please install it soon.
security-nofix = There is no fix yet. kura checks every day and tells you when there is one.
security-none = No known security problems.
security-running = Checking for known security problems…
security-unavailable = The list of known security problems could not be loaded.
security-detail = { $package } { $installed }: { $kind }
security-fixed-in = Fixed in { $version }
security-used-by = Used by: { $apps }
severity-low = low risk
severity-medium = medium risk
severity-high = high risk
severity-critical = critical risk

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
nav-updates = Updates
gui-review = Review and install
gui-check-now = Look for updates now
gui-install = Install
gui-install-news = I have read the news. Install
gui-read-news = Read the announcement
gui-go-back = Go back to this state
gui-cancel = Cancel
gui-updates-intro = This is everything that will change. Nothing happens until you press Install.
gui-tray-mono = Monochrome tray icon
gui-tray-mono-hint = Matches the other icons in the panel.
gui-last-check = Last checked { $when }.
nav-health = System health
nav-software = Software
health-cpu = Processor
health-gpu = Graphics
health-memory = Memory
health-disk = Storage
health-disk-used = { $used } of { $total } used
health-cache = Old update files: { $size }
health-kernels = Kernels
health-kernel-running = in use
health-suggestions = Suggestions
health-snapshots = Snapshots
snapshots-on = Taken before every update by { $tool }
snapshots-off = Not set up
snapshots-partial = Partly set up
snapshots-unsupported = Not possible on this file system
snapshots-missing = Missing:
snapshots-step-install = Install { $package }
snapshots-step-config = Create a snapper configuration for /
snapshots-step-enable = Turn on { $unit }
suggest-backup-kernel = Install a backup kernel
suggest-backup-kernel-text = If an update ever stops the computer from starting, the backup can be chosen in the start menu.
suggest-snapshots = Turn on automatic snapshots
suggest-snapshots-text = Your disk supports snapshots. Setting them up comes in a later version of kura.
suggest-mirrors = Choose faster download servers
suggest-mirrors-text = kura tests servers near you and uses the fastest.
suggest-cleanup =
    { $count ->
        [one] One package is no longer needed
       *[other] { $count } packages are no longer needed
    }
suggest-cleanup-text = They are removed with the next update.
search-placeholder = Search apps, for example “photo” or “music”
search-none = No apps found. Try a different word.
gui-installed = Installed
gui-remove = Remove
gui-install-title = Install { $name }
gui-remove-title = Remove { $name }
gui-setup = Set up
gui-details = Details
nav-history = History
nav-settings = Settings
overview-subtitle = A short overview of this computer.
overview-refresh = Refresh
