# Privacy

kura collects no telemetry and has no analytics or crash reporting service.

This is the complete list of servers kura contacts. A pull request that adds
a network request must add it here.

| Server | Why | When |
|---|---|---|
| Package mirrors configured in pacman | download databases and packages | update checks, installs |
| `archlinux.org/mirrors/status/json/` | rank mirrors | first start, monthly, on request |
| `archlinux.org/feeds/news/` | Arch news before updates | before each update |
| `aur.archlinux.org` | AUR search, metadata and build files | AUR enabled and in use |
| Upstream source hosts named in AUR build files | AUR builds | building an AUR package |
| `security.archlinux.org` | vulnerability data | daily |
| `archive.archlinux.org` | old package versions | going back to an earlier generation |
| `github.com/Serraniel/kura-bundles` (raw files) | bundles, flows, hardware rules | daily, or when browsing bundles |

Problem reports are never sent automatically. kura shows the report, removes
the user name, host name and home directory paths, and opens a prefilled
GitHub issue in the browser only when the user asks for it.

Local data: generations and settings are stored under `/var/lib/kura` and
`~/.config/kura`. Nothing leaves the computer except the requests above.
