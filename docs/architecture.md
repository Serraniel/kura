# Architecture

See [decisions.md](decisions.md) for the reasons behind this layout.

## Crates

| Crate | Kind | Purpose |
|---|---|---|
| `kura-core` | library | domain model: packages, transactions, plans, generations, sources, rules. No I/O against the real system. |
| `kura-alpm` | library | libalpm adapter: reads databases, prepares and commits transactions. |
| `kura-i18n` | library | Fluent loader and locale handling, shared by all binaries. |
| `kura-sbom` | library + CLI | CycloneDX SBOM from the local package database. |
| `kura-helper` | binary, root | D-Bus service with polkit checks. Runs plans. Nothing else. |
| `kura-agent` | binary, user | tray icon, notifications, udev, AUR builds. |
| `kura-cli` | binary `kura` | command line interface. |
| `kura-gui` | binary `kura-gui` | Qt/QML/Kirigami interface. |

`kura-core` holds the logic that tests cover at 90 %. Everything that touches
the real system goes through traits so tests can use fakes.

## How a change happens

```
user action
  -> plan     (kura-core: what should change; resolved via kura-alpm, L1 simulation)
  -> preview  (GUI or CLI shows the plan; user confirms)
  -> execute  (kura-helper over D-Bus, one polkit check per plan)
  -> record   (generation written by the pacman hook)
```

A plan is a plain data structure. It can be serialised, shown, tested and
replayed. The helper refuses anything that is not a valid plan.

Before it runs a plan, the helper derives it again from the system and stops
if the result differs from what the user approved. The user then sees the new
preview.

Besides package steps, a plan can carry actions (`kura_core::action`): fixed
operations such as enabling a systemd unit or creating the snapper
configuration. The helper runs only the actions it derived itself, never ones
it received, and starts fixed programs with fixed arguments.

## Sources

`Source` is a trait implemented by the pacman repositories and the AUR. Each
source can search, describe packages, list updates and contribute steps to a
plan. Flatpak and AppImage can be added later as further implementations.

## Data from kura-bundles

Bundles, flows, hardware rules, language-pack families, provider preferences
and repository profiles are fetched from the `kura-bundles` repository. The
index is signed. kura refuses unsigned or modified data.
