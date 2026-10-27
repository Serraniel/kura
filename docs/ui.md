# Interface

How kura's window is organised. Based on the clickable demo reviewed on
2026-10-25 (linked in issue #15). Wording examples are English; every text
goes through the translation files.

## Window

- The window keeps the size the user gives it. Navigating never resizes it;
  pages scroll inside the window.
- Left: navigation (Kirigami global drawer). Right: the page.
- Colours, fonts and icons come from the desktop theme. kura sets none,
  except its own logo, which is built in so it shows even without the
  installed icon.
- Bottom, opt-in (F4 or "Show details" in Settings): a read-only panel like
  Dolphin's terminal panel. Each step kura takes has a time in front. Below
  it, indented, comes the raw output: every program the helper runs as
  `$ command`, everything it prints, and what libalpm, install scripts and
  hooks print. AUR builds mark their lines with the package base, because
  several build at once. The panel takes room from the page; the window
  does not grow. The choice is remembered.

## Pages

| Page | Purpose |
|---|---|
| Overview | One status at the top with one main action. Below: suggestions and the last three changes. |
| Updates | The preview: apps, system parts, community apps (AUR) waiting for their cooling period, cleanup. Arch news check result. Install button. |
| Software | Search, ready-made sets (bundles), installed apps with Remove. App pages show the source; AUR apps show trust signals and the cooling period. |
| History | Saved states of the system. Going back shows a preview and its limits. |
| System health | Computer, storage, security, hardware and drivers, kernels, suggestions. Export of the software list (SBOM). |
| Settings | Update interval, automatic security updates, snapshots, download servers, AUR and its waiting time, software sources (CachyOS, Chaotic-AUR), tray icon. Language follows the desktop. |

## Status model

The overview and the tray show exactly one of these, most important first:

1. Security problem: red, "Kdenlive has a security problem".
2. Update needs a step: Arch news or a fixable problem; kura shows the solution.
3. Updates paused: nothing could be solved safely; nothing was changed; offer a problem report.
4. Updates ready.
5. Everything is up to date.

## Flows

- **First start:** language taken from the desktop, mirrors ranked, system
  detected, snapshots (recommended on btrfs) and AUR switched on or off.
- **Every change:** preview, then progress with named steps, then a result
  page that links to History.
- **AUR installs:** one progress row per package, running at the same time.
  Packages that wait for another one say which.
- **Problem report:** shown in full before anything is sent; opens a
  prefilled GitHub issue or copies the text.

## Words

User-facing text says "app", "system part", "saved state", "community apps
(AUR)". Package names appear only in details, in monospace. See the glossary
in [i18n.md](i18n.md).
