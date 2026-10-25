# Contributing

Thanks for helping. Please read [docs/decisions.md](docs/decisions.md) before
working on a larger change; it explains what kura does and does not do.

## Before you start

- For anything bigger than a small fix, open or comment on an issue first.
- Questions and ideas: GitHub Discussions.

## Pull requests

- The title follows [Conventional Commits](https://www.conventionalcommits.org/):
  `feat: show download size in the preview`, `fix(i18n): ...`, `docs: ...`.
  Pull requests are squash-merged, so the title becomes the commit message and
  decides the next version number.
- Sign off every commit (`git commit -s`). This confirms the
  [Developer Certificate of Origin](https://developercertificate.org/): you
  wrote the change or have the right to submit it under the project's licence.
- New files need SPDX headers. Run `reuse lint` to check.
- Tests must pass. Core crates must stay at 90 % line coverage or above.
- Changed user-visible text needs updates in every language file. See
  [docs/i18n.md](docs/i18n.md).

## Licence

By contributing you agree that your contribution is published under the
licence of the files you change: GPL-3.0-or-later for code, CC-BY-SA-4.0 for
documentation. Modified files must keep their copyright and licence notices.

## Translations

Translations are welcome through Weblate (link follows once set up) or as a
pull request. Read the glossary in [docs/i18n.md](docs/i18n.md) first.
