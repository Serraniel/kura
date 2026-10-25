# Instructions for AI agents

These rules apply to every AI agent working in this repository. They do not
replace [docs/decisions.md](docs/decisions.md); read that first. If a task
conflicts with a decision, stop and ask instead of working around it.

## Who kura is for

People with little Linux experience. Every user-facing change must be
understandable without knowing what a dependency, a repository or a terminal
is. Technical detail goes behind "Details".

## Hard rules

1. Never run a partial upgrade. No `-Sy` without `-u`. Background checks use a
   separate database copy.
2. Never parse pacman's text output. Use libalpm through `kura-alpm`.
3. Only `kura-helper` changes the system, and only by executing a plan. Do not
   add other privileged code paths.
4. Every change to the system is shown as a preview before it runs.
5. No arbitrary shell commands from data files. Bundles and flows use the
   declarative actions defined in `kura-bundles`.
6. No telemetry. A new network request must be added to
   [docs/privacy.md](docs/privacy.md) in the same pull request.
7. No user-visible text in code. Use Fluent message IDs (see below).

## Text and translations

- When you add or change a message in `i18n/en/kura.ftl`, add or update the
  same message in `de`, `fr` and `ja`.
- Follow [docs/i18n.md](docs/i18n.md), including the glossary. Keep English
  terms that are standard in the target language ("USB" stays "USB").
- German must be correct and natural. French and Japanese are marked
  "review wanted"; list changed FR/JA message IDs in the pull request
  description so a native speaker can check them.

## Code

- Rust, edition and toolchain as in `rust-toolchain.toml`.
- `cargo fmt`, `cargo clippy -- -D warnings` and `cargo test` must pass.
- Core crates (`kura-core`, `kura-sbom`, `kura-i18n`) keep at least 90 % line
  coverage. Put logic in core crates behind traits so it can be tested
  without root or a real pacman database.
- Every new source file starts with SPDX headers:

  ```
  // SPDX-FileCopyrightText: 2026 The kura contributors
  // SPDX-License-Identifier: GPL-3.0-or-later
  ```

  Documentation uses `CC-BY-SA-4.0`. `reuse lint` must pass.

## Commits and pull requests

- Conventional Commits. The pull request title becomes the squash commit and
  drives versioning: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`,
  `test:`, `ci:`, `build:`, `perf:`. Use `!` for breaking changes.
- Every commit carries `Signed-off-by:` (`git commit -s`).
- One topic per pull request. Link the issue it closes.
- Do not edit `CHANGELOG.md` by hand; release-please writes it.

## Documentation style

Write like a person explaining something to a colleague.

- Short sentences. Concrete statements. Say what something does.
- No marketing words ("seamless", "powerful", "robust", "effortless",
  "leverage", "cutting-edge").
- No emoji in documentation or commit messages.
- No filler sections. If there is nothing to say, leave the section out.
- Do not restate what the code already makes obvious.
