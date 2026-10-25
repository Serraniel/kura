# Security policy

## Reporting a problem

Please do not open a public issue for security problems.

Report them privately through GitHub:
<https://github.com/Serraniel/kura/security/advisories/new>

Include what you found, how to reproduce it, and which version you used.
You will get an answer within 7 days. Once a fix is available, the problem is
published as a GitHub security advisory, with credit to you unless you prefer
otherwise.

## Supported versions

Only the latest release receives security fixes.

## What counts

Anything that lets someone gain privileges through `kura-helper`, run code
through bundle or flow data, bypass the signature check of `kura-bundles`, or
leak data that [docs/privacy.md](docs/privacy.md) says stays local.

Security problems in packages that kura installs belong to their upstream
projects or to Arch Linux.
