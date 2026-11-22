# LinkUnbound

[Русский](README.md)

LinkUnbound opens links in the browser or profile you choose. Set it as your default browser in Windows, then create rules for sites, addresses, and the applications that open links.

[Download the installer for Windows x64](https://github.com/WeslyG/LinkUnbound/releases/latest)

## What's in this fork

- **Russian interface.** On first launch, LinkUnbound uses the system language when supported and falls back to English otherwise. Settings offer Russian, English, Spanish, and automatic language selection.
- **Fallback browser.** Links without a matching rule open directly in the selected browser. This disables the picker visibility setting and shows an explanation next to it.
- **Domain-list rules.** Each list can be attached to a single rule. A rule can match domains in the list or domains outside it.
- **Built-in and custom lists.** The lists for Russia and Ukraine can be selected individually. Settings show their sources and cache status. Add a custom list by pasting domains or entering an HTTPS URL.
- **Edit or remove lists.** Changing a URL or domain entries preserves linked rules. When removing a list, LinkUnbound shows the affected rules and removes them together with the list after confirmation.
- **Caching and refresh.** Enable domain-list rules, allow background refreshes, choose a refresh interval, or refresh the cache manually. Cached lists work offline.

## List format

Use UTF-8 text with one domain per line. Blank lines and `#` comments are allowed. A domain also matches its subdomains. A leading dot, as in .example.org, matches subdomains only. Schemes, paths, and wildcards such as `*.example.org` are unsupported. LinkUnbound validates entries when adding or refreshing a list.

The built-in lists come from [itdoginfo/allow-domains](https://github.com/itdoginfo/allow-domains). Settings identify the source of each list.

## Data and updates

LinkUnbound has neither telemetry nor a report generator. Settings, rules, and cached lists are stored locally. LinkUnbound contacts GitHub for app updates and reaches list sources when refreshing lists. It verifies the installer signature and version before applying an update.

## Releasing a new version

Push a `vX.Y.Z` tag to this fork. The [Windows release workflow](.github/workflows/windows-release.yml) uses the tag as the package version, builds and signs the Windows x64 installer, publishes the release, and updates the stable channel on the `manifest` branch. A manual run in Actions builds and verifies the signed installer without publishing.

Forked from [rgdevment/LinkUnbound](https://github.com/rgdevment/LinkUnbound). Licensed under [GPL-3.0](LICENSE).
