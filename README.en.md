<div align="center">
  <img src="app/src-tauri/icons/icon.png" width="96" height="96" alt="LinkUnbound icon">
  <h1>LinkUnbound</h1>
  <p><strong>Open every link in the browser you choose.</strong></p>
  <p><a href="https://github.com/WeslyG/LinkUnbound/releases/latest"><img src="https://img.shields.io/github/v/release/WeslyG/LinkUnbound?style=flat-square&amp;label=Download&amp;color=0078D4" alt="Download the latest release"></a> <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0-lightgrey?style=flat-square" alt="GPL-3.0"></a></p>
  <p><a href="README.md">Русский</a> · <a href="CHANGELOG.md">Changelog</a></p>
  <img src="resources/assets/store/screenshots/en/01-picker.png" width="92%" alt="The browser and profile picker next to the pointer">
</div>

## Why use LinkUnbound?

If you browse for work in one browser and use another for personal sites, links from email, chat, and documents still go to the same system browser. LinkUnbound becomes your default browser and applies your rules first. For example, it can send links from Teams to your work profile in Chrome and everything else to Firefox.

When no rule matches, LinkUnbound can show a picker next to your pointer or open the link immediately in a **fallback browser**. Activating the fallback disables the picker visibility setting and explains why.

## Getting started

1. Download the installer from the [latest release](https://github.com/WeslyG/LinkUnbound/releases/latest). On Windows, choose the `_x64-setup.exe` file. If macOS builds are available, choose the `.dmg` marked `aarch64` for Apple Silicon or `x86_64` for Intel.
2. Set LinkUnbound as your default browser in system settings.
3. In LinkUnbound settings, create a rule for a site, address, or app, then choose a browser or profile. Set a fallback browser if you want links that match no rule to open automatically.

This fork adds a **Russian interface**. On first launch LinkUnbound uses the system language when supported, with English as the default for other languages. The settings dropdown offers Russian, English, Spanish, and automatic selection.

<p align="center"><img src="resources/assets/store/screenshots/en/03-rules.png" width="85%" alt="Settings for link routing rules"></p>

## Route several sites with one rule

A **domain list** groups site names so you can route many sites with one rule. You can put your work email and documentation sites in a list and open all of them in a work profile. A rule can also match domains **outside** the list. For example, links outside your work list can open in a personal browser.

Under **Domain lists**, select individual lists for Russia and Ukraine from [itdoginfo/allow-domains](https://github.com/itdoginfo/allow-domains) or add your own. Settings show each list's source and cache status. You can enable list rules and background refreshes separately, choose an interval, or refresh the cache manually. Cached copies work offline.

### Add a custom list

Paste site names or provide the HTTPS address of a text file. Use UTF-8 with one site name per line. Lines beginning with `#` are comments.

```text
# Work email and documentation
mail.example.org
docs.example.org
```

The entry `docs.example.org` also matches `team.docs.example.org`. With a leading dot, `.docs.example.org` matches only subdomains. Enter the site name without `https://`, a path, or a wildcard such as `*.docs.example.org`. LinkUnbound validates entries when adding or refreshing a list.

Changing the address or contents of a list preserves its linked rules. When removing a list, LinkUnbound shows which rules use it and lets you remove them together after confirmation.

## Data and updates

Settings, rules, and the domain-list cache stay on your computer. LinkUnbound has neither telemetry nor a report generator. It contacts GitHub for app updates and reaches the configured list sources when refreshing them. It verifies the installer signature and version before applying an update.

## Releasing a new version

Add a non-empty `## [X.Y.Z] - YYYY-MM-DD` section to [CHANGELOG.md](CHANGELOG.md) and push the `vX.Y.Z` tag. [GitHub Actions](.github/workflows/windows-release.yml) builds Windows x64 and both macOS variants, signs the files with the Tauri key, publishes the installers, and copies that changelog section into the release description. It then updates the `manifest` branch. A manual run checks the artifacts without publishing.

macOS self-updates require an Apple-notarized package. With no Apple notarization credentials, the release includes DMG images for manual installation.

---

Forked from [rgdevment/LinkUnbound](https://github.com/rgdevment/LinkUnbound) · [GPL-3.0 license](LICENSE) · [Third-party notices](THIRD-PARTY-BUNDLED.md)
