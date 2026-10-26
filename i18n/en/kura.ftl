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
      search     Search apps in the repositories and the AUR: kura search <word>
      update     Install all updates (shows a preview first)
      install    Install an app: kura install <name>
      remove     Remove an app and what only it needed: kura remove <name>
      report     Show a problem report and, if you want, open it on GitHub: kura report [what happened]
      security   Show known security problems of installed software
      settings   Show or change settings: kura settings aur on|off, kura settings aur-wait <days>
      snapshots  Show snapshot status; kura snapshots setup adds what is missing
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
history-auto-security = (security fixes, installed automatically)
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
apply-building = Building { $name } ({ $current }/{ $total })
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
notify-auto-security-title = Security updates were installed
notify-auto-security-body = { $count ->
    [one] They fix a known security problem.
   *[other] They fix { $count } known security problems.
} You can see every change in kura's history and go back there.

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
gui-aur = Software from the AUR
gui-aur-hint = The AUR has software that other users share. It is not checked by Arch Linux. kura builds it on this computer in a separate, restricted environment.
gui-aur-wait-hint = New AUR versions wait this long before kura installs them, so problems can show up first. Installing an app by hand always takes the newest version.
gui-aur-after = { $count ->
    [one] After this, one update from the AUR is ready. Review again to build it.
   *[other] After this, { $count } updates from the AUR are ready. Review again to build them.
}
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
snapshots-setup-hint = To add what is missing: kura snapshots setup
snapshots-nothing = Nothing to set up.
preview-actions = Settings that change:
confirm-snapshots = Set up snapshots? [y/N]
suggest-backup-kernel = Install a backup kernel
suggest-backup-kernel-text = If an update ever stops the computer from starting, the backup can be chosen in the start menu.
suggest-snapshots = Turn on automatic snapshots
suggest-snapshots-text = A snapshot saves the state of the system before every update, so you can go back if something breaks. kura keeps what is already set up and only adds what is missing.
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
search-repos = From the repositories:
search-aur = From the AUR (shared by the community, built on this computer):
search-aur-failed = The AUR could not be searched: { $reason }
aur-votes = { $count ->
    [one] { $count } vote
   *[other] { $count } votes
}
aur-new = new
aur-orphaned = no maintainer
aur-out-of-date = marked out of date
aur-mixed = Some of these come from the AUR. Install those separately: kura install { $names }
aur-build-list = Built on this computer from the AUR:
aur-notice = AUR packages are shared by other users and are not checked by Arch Linux. kura builds them in a separate, restricted environment.
confirm-aur = Build and install this? [y/N]
aur-off = { $names } is not in the repositories, and the AUR is turned off. To turn it on: kura settings aur on
aur-check-failed = AUR updates could not be checked: { $reason }
check-aur = From the AUR: { $ready } ready, { $waiting } still waiting
aur-updates-title = Updates from the AUR:
aur-waiting = { $count ->
    [one] One AUR update waits
   *[other] { $count } AUR updates wait
} until { $count ->
    [one] it is
   *[other] they are
} { $days } days old, so problems in new versions can show up first: { $names }
confirm-aur-update = Build and install these updates? [y/N]
settings-aur = AUR
settings-aur-wait = Wait before AUR updates
settings-on = on
settings-off = off
settings-days = { $days ->
    [one] { $days } day
   *[other] { $days } days
}
settings-saved = Saved.
settings-auto-security = Install security fixes automatically
gui-auto-security-hint = When an update fixes a known security problem, kura installs it without asking, but only if the test run found no problem and no Arch news concerns it. You get a message afterwards and can go back in History.
settings-usage = Usage: kura settings [aur on|off] [aur-wait <0–14>] [auto-security on|off]
report-intro = This report contains no user name, computer name or home folder. Please read it before you share it:
confirm-report = Open it as a prefilled problem report on GitHub in your browser? [y/N]
report-opened = If no browser opened, copy this address: { $url }
gui-report = Report this problem
gui-report-open = Open on GitHub
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
