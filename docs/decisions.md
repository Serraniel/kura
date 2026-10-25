# Decisions

This file records what was decided and why. It is the reference for
contributors and AI agents. Change a decision only through a pull request
that updates this file and says what changed.

Last full review: 2026-10-25.

## Audience and goal

kura is a graphical front end for package management on Arch Linux and
Arch-based distributions. It is written for people with little or no Linux
experience, for example people coming from Windows or older people.

Consequences:

- Plain language. No unexplained jargon. Technical detail is available, but
  behind a "Details" control.
- Every change is shown as a preview before it happens.
- Every warning comes with something the user can do, or a clear statement
  that nothing needs to be done.
- kura picks the lowest-risk option automatically wherever it can.

## Scope

| Topic | Decision |
|---|---|
| Distributions | Arch, EndeavourOS and CachyOS are supported. Garuda: best effort. Manjaro: planned later, low priority. Artix and other non-systemd distributions: not supported. |
| Desktops | KDE Plasma is the reference. Other desktops must work but get less testing. Without a tray (GNOME without extension), kura falls back to notifications. |
| Package sources | pacman repositories and the AUR. Sources are pluggable so Flatpak and AppImage can be added much later. |
| Own package manager | No. kura uses libalpm (the library behind pacman). It never parses pacman's text output, because that output is translated and changes between versions. |

## Technology

- Rust for all logic.
- Qt 6 / QML with Kirigami for the interface, bridged with `cxx-qt`. Kirigami
  follows the desktop's colours, icons, fonts and dark mode.
- libalpm through the `alpm` crate.
- AUR access through `raur` and `aur-depends`. kura builds AUR packages
  itself. yay or paru are not required.
- Localisation with Project Fluent.

## Processes and privileges

| Process | Runs as | Job |
|---|---|---|
| `kura` | user | the window |
| `kura-agent` | user, autostarted | tray icon, notifications, udev hardware events, AUR builds |
| `kura-helper` | root, D-Bus activated | the only process that changes the system; authorised through polkit |
| `kura-check.timer` | root | periodic update and vulnerability checks |

No root process runs permanently. The GUI never runs as root.

Password: polkit with "keep" authorisation. kura groups the steps of one flow
into a single helper call so the user enters the password once per flow.

## Updates

- Default: download in the background, then ask. One click shows the preview.
- Check interval is configurable. Default: weekly. Security checks: daily.
- Security issues always produce a warning. Automatic installation of
  security updates is an opt-in setting.
- Never do a partial upgrade (sync without upgrading everything). Background
  checks use a separate copy of the sync databases, like `checkupdates`.
- Read Arch news before every update. If a news item concerns a package in
  the update, stop and show it, together with a flow if one exists.
- Update `archlinux-keyring` (and the distribution's keyring) first.
- Mirrors are ranked by kura itself on first start, monthly, and on request.
  Distribution-specific mirror lists are respected. The previous list is kept.

## Problem solver

Three levels, tried in this order:

1. Built-in rules for classes of problems (for example: a package removed
   from the repositories holds an old version of a library; remove the chain
   if no explicitly installed package needs it).
2. Flows from the `kura-bundles` repository for known cases that need
   specific steps or a decision by the user.
3. If nothing matches: a calm explanation and an optional problem report
   (prefilled GitHub issue, anonymised, shown to the user before it is sent).

Choosing between alternatives (providers, language packs) is automatic:

- Language packs follow the system language and its fallback list. kura maps
  locales to all common code forms (`de`, `de-DE`, `de_DE`, `deu`, `ger`).
  The naming scheme of each package family is data in `kura-bundles`.
- Other providers: already installed, then exact name match, then official
  repositories before AUR, then version constraints, then a preference from
  `kura-bundles`.

Simulation:

| Level | What | When |
|---|---|---|
| L1 | libalpm transaction prepared but not committed | always |
| L2 | update applied in a throwaway copy (btrfs snapshot or overlay, systemd-nspawn) to check builds, DKMS and hooks | automatically on btrfs when needed; opt-in elsewhere |
| L3 | does the program work | not automatable; generations and snapper are the safety net |

Repair strategies, lowest risk first: rebuild an AUR package against the new
libraries (built in a clean container before the system is touched); hold
back a package temporarily (never for glibc, systemd, pacman, keyrings, or a
kernel without its modules; only if no other package depends on it); remove
packages no longer needed. kura picks the strategy. Inside an update the user
has confirmed, repairs run without another question. Every repair is recorded
as a generation.

## Generations

A generation is the list of installed packages (name, version, repository,
explicit or dependency) after a transaction. A pacman hook records one for
every transaction, including those done with pacman directly.

Going back installs the old versions from the package cache or the Arch Linux
Archive in one transaction. Limits shown to the user: configuration and data
formats are not reverted; AUR packages are rebuilt from older sources; the
next update brings the new versions back, so going back is a temporary fix.

## AUR

- On by default. Users can turn it off.
- Cooling period: AUR updates enter the normal update only after 3 days
  (configurable 0–14). The period restarts when the maintainer changes.
  Security updates and explicit user requests skip it.
- Trust signals are shown (votes, age, maintainer, out-of-date flag).
- Builds run in parallel. kura resolves the full dependency graph first,
  fetches all sources at once, installs all needed build dependencies from
  the repositories in one transaction, and builds independent packages at
  the same time. Packages that depend on other AUR packages wait only for
  those. Parallelism is limited by CPU and memory.

## Security and SBOM

- The package database is turned into a CycloneDX SBOM by `kura-sbom`
  (library and small CLI in this repository), using `pkg:alpm/...` package URLs.
- Data source for user-facing warnings: the Arch Security Tracker. Sources
  that do not map to Arch package names cleanly (OSV, NVD) appear only in the
  details panel, to avoid false alarms.
- Warnings name the application the user knows (found by walking reverse
  dependencies up to explicitly installed packages, with names and icons from
  AppStream). System libraries are reported as "System". The vulnerable
  package is shown in the details.

## Bundles, flows and hardware

All curated data lives in the `kura-bundles` repository: app bundles, flows,
hardware rules, language-pack families, provider preferences, repository
profiles. The index is signed and kura verifies it.

- Configuration is declarative only (enable a service, add a user to a group,
  write a file from a template). No arbitrary scripts.
- Every action has a system or user scope, can be previewed, and can be undone.
- Hardware detection reads sysfs and listens to udev in `kura-agent`.

## Snapper

kura detects an existing snapper setup and only adds what is missing
(configuration, `snap-pac`, boot menu entries). It does not create duplicate
snapshots. Booting into a snapshot is a later step.

## Kernels

Install new kernels next to existing ones, check that DKMS modules build, set
the default, offer to remove the old one only after a successful boot.
Suggest `linux-lts` as a fallback. First supported: GRUB and systemd-boot,
mkinitcpio and dracut. Limine, rEFInd and UKI later.

## Repository profiles

CachyOS repositories and Chaotic-AUR can be added and removed. Each profile
has a tested way back to plain Arch. kura may suggest a profile when the
hardware fits (for example a CPU supporting x86-64-v3), always with an honest
statement of the benefit and a trust warning.

## Command line

`kura` has a CLI with verbs from the first release: `update`, `install`,
`remove`, `undo`, `history`, `doctor`, `bundle`, `sbom`. A pacman-compatible
mode (`kura -Syu`) comes later.

## Languages

English is the source language. German, French and Japanese ship from the
start. French and Japanese are marked "review wanted" until a native speaker
has checked them. Community translations go through Hosted Weblate. See
[i18n.md](i18n.md).

## Privacy

No telemetry. Every network endpoint kura contacts is listed in
[privacy.md](privacy.md).

## Project

| Topic | Decision |
|---|---|
| Licence | Code: GPL-3.0-or-later. Bundles, flows, documentation: CC-BY-SA-4.0. REUSE-compliant. |
| Contributions | Developer Certificate of Origin (`Signed-off-by`). No CLA. |
| Hosting | GitHub, personal account `Serraniel`, public. May move to an organisation later. |
| Name | "kura" (蔵, storehouse). Trademark check pending before the first public release. |
| Versioning | Semantic Versioning, Conventional Commits in PR titles, squash merge, release-please. |
| Edge builds | `kura-git` in the AUR. No separate edge channel. |
| Distribution | AUR (`kura`, `kura-bin`, `kura-git`, updated automatically on release), a signed pacman repository, and a package file attached to each release. Later: ask EndeavourOS and CachyOS to include kura. |
| Tests | Line coverage of at least 90 % for core crates, enforced in CI. No third-party coverage service. |
| CRA | SBOM for every release, SECURITY.md with a disclosure process, signed release artefacts, build provenance. |

## Milestones

1. Foundation
2. v0.1 Safe updates
3. Security and SBOM
4. Problem solver
5. AUR
6. Bundles and hardware
7. Snapper
8. Kernels
9. Repository profiles

Later: Manjaro, Flatpak and AppImage, pacman-compatible CLI, inclusion in
distribution repositories.
