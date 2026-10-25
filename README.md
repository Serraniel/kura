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

Not yet available. Once the first version is released, kura will be
available from the AUR, from its own package repository, and as a package
file on the releases page.

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
