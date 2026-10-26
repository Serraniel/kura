# kura

kura (蔵, "storehouse") installs, updates and removes software on Arch Linux,
EndeavourOS and CachyOS. It is made for people who are new to Linux.

> **Status:** early development. kura is not usable yet. Do not install it on
> a computer you depend on.

## What kura does

- Shows every change before it happens, like a classic installer.
- Lets you go back to an earlier state of your installed software.
- Solves common update problems on its own and explains the rest in plain
  language.
- Warns you when an app you use has a known security problem, by the app's
  name, not the name of some library inside it.
- Installs software from the Arch repositories and the AUR without extra tools.
- Removes software that nothing needs any more.
- Suggests drivers for your hardware.
- Offers ready-made sets of apps and settings, for example for gaming.
- Follows your desktop's colours and theme.
- Speaks English, German, French and Japanese.

kura uses pacman underneath. It does not replace it, and you can keep using
pacman alongside kura.

## Install

Not yet released. Developers can build the `kura-git` package from
[packaging/aur/kura-git](packaging/aur/kura-git/PKGBUILD) on a test machine.

## What works today

| Command | What it does |
|---|---|
| `kura check` | Shows which updates are available and what would change. Changes nothing. |
| `kura update` | Shows the preview and Arch news that concern it, asks, then installs everything. AUR updates follow once they are a few days old (3 by default), so problems in new versions can show up first. |
| `kura install <name>` / `kura remove <name>` | The same for single apps. Apps only the AUR has are built on this computer in a restricted environment, several at once. |
| `kura search <word>` | Finds apps in the repositories and the AUR. AUR results show votes and warnings. |
| `kura history` | Lists the saved states of the system. |
| `kura undo [number]` | Goes back to an earlier state. |
| `kura mirrors` | Picks fast, up-to-date download servers. |
| `kura security` | Lists known security problems, named by the apps that use the affected part. |
| `kura orphans` | Lists packages nothing needs any more. |
| `kura sbom` | Writes a list of all installed software (CycloneDX). |
| `kura doctor` | A short system summary for problem reports. |
| `kura report [what happened]` | Shows a problem report without user name, computer name or home folder, and opens it as a prefilled GitHub issue if you want. kura sends nothing itself. |
| `kura settings` | Shows or changes settings for all users: `aur on`/`off`, `aur-wait <days>`. |
| `kura snapshots [setup]` | Shows whether snapshots are taken before updates; `setup` adds what is missing. |

The window (`kura-gui`) shows the same status, update preview, history and
settings. It also lets you search for apps and install or remove them, warns
about security problems and has a system health page: hardware, disk space,
kernels, snapshots and suggestions. A tray icon (`kura-agent`) tells you when updates are ready or
something needs attention. A daily background check downloads updates
without installing them.

## Privacy

kura sends no usage data anywhere. [docs/privacy.md](docs/privacy.md) lists
every server it contacts.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Translations are welcome; see
[docs/i18n.md](docs/i18n.md).

Bundles, fixes for known problems and hardware rules live in a separate
repository: [kura-bundles](https://github.com/Serraniel/kura-bundles).

## Security

Please report security problems privately. See [SECURITY.md](SECURITY.md).

## License

kura is free software under the GNU General Public License, version 3 or
later. Documentation is under CC-BY-SA-4.0. Each file states its licence; the
full texts are in [LICENSES](LICENSES).

kura comes with no warranty, to the extent permitted by law. See the licence
for details.
