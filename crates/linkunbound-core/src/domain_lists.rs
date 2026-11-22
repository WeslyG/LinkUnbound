use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::StoreError;
use crate::store::{guarded, save_atomically};

pub const MAX_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainList {
    RussiaInside,
    RussiaOutside,
    Ukraine,
}

impl DomainList {
    pub const ALL: [Self; 3] = [Self::RussiaInside, Self::RussiaOutside, Self::Ukraine];

    pub const fn id(self) -> &'static str {
        match self {
            Self::RussiaInside => "russia_inside",
            Self::RussiaOutside => "russia_outside",
            Self::Ukraine => "ukraine",
        }
    }

    pub const fn url(self) -> &'static str {
        match self {
            Self::RussiaInside => {
                "https://raw.githubusercontent.com/itdoginfo/allow-domains/main/Russia/inside-raw.lst"
            }
            Self::RussiaOutside => {
                "https://raw.githubusercontent.com/itdoginfo/allow-domains/main/Russia/outside-raw.lst"
            }
            Self::Ukraine => {
                "https://raw.githubusercontent.com/itdoginfo/allow-domains/main/Ukraine/inside-kvas.lst"
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListSource {
    pub id: String,
    pub name: Option<String>,
    pub url: Option<String>,
}

fn custom_path(dir: &Path) -> PathBuf {
    dir.join("domain-list-sources.json")
}

fn read_custom(path: &Path) -> Result<Vec<ListSource>, StoreError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(StoreError::Read {
                path: path.to_owned(),
                source,
            });
        }
    };
    serde_json::from_str(&raw).map_err(|source| StoreError::Read {
        path: path.to_owned(),
        source: source.into(),
    })
}

pub fn sources(dir: &Path) -> Vec<ListSource> {
    let custom = read_custom(&custom_path(dir)).unwrap_or_default();
    DomainList::ALL
        .into_iter()
        .map(|list| ListSource {
            id: list.id().to_owned(),
            name: None,
            url: Some(list.url().to_owned()),
        })
        .chain(custom.into_iter().filter(|list| custom_id(&list.id)))
        .collect()
}

fn custom_id(id: &str) -> bool {
    id.strip_prefix("custom-").is_some_and(|suffix| {
        !suffix.is_empty()
            && suffix.len() <= 64
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'-')
    })
}

pub fn save_source(
    dir: &Path,
    existing: Option<&str>,
    name: String,
    url: Option<String>,
    domains: &[String],
) -> Result<bool, StoreError> {
    let path = custom_path(dir);
    guarded(&path, || {
        let mut custom = read_custom(&path)?;
        let index = if let Some(id) = existing {
            let Some(index) = custom
                .iter()
                .position(|item| custom_id(id) && item.id == id)
            else {
                return Ok(false);
            };
            Some(index)
        } else {
            None
        };
        let id = if let Some(index) = index {
            custom[index].id.clone()
        } else {
            let epoch = crate::seconds_now().unwrap_or_default();
            (0..=u64::MAX)
                .map(|index| format!("custom-{epoch}-{index}"))
                .find(|id| {
                    !custom.iter().any(|list| &list.id == id) && !cache_path(dir, id).is_file()
                })
                .ok_or_else(|| StoreError::Write {
                    path: path.clone(),
                    source: std::io::Error::other("no available domain-list id"),
                })?
        };
        let old = index.and_then(|_| read(dir, &id));
        let source = ListSource {
            id: id.clone(),
            name: Some(name),
            url,
        };
        if let Some(index) = index {
            custom[index] = source;
        } else {
            custom.push(source);
        }
        let data = encode_sources(&path, &custom)?;
        save(dir, &id, domains)?;
        let written = save_atomically(&path, &data);
        if written.is_err() {
            if let Some(old) = old {
                let _ = save(dir, &id, &old.into_iter().collect::<Vec<_>>());
            } else {
                let _ = std::fs::remove_file(cache_path(dir, &id));
            }
        }
        written.map(|()| true)
    })
}

fn encode_sources(path: &Path, sources: &[ListSource]) -> Result<String, StoreError> {
    serde_json::to_string_pretty(sources).map_err(|cause| StoreError::Write {
        path: path.to_owned(),
        source: std::io::Error::other(cause),
    })
}

pub enum RemoveSource {
    Removed,
    RulesChanged,
    Unknown,
}

pub fn remove_source(
    store: &crate::Store,
    id: &str,
    confirmed: &[String],
) -> Result<RemoveSource, StoreError> {
    let path = custom_path(store.dir());
    guarded(&path, || {
        let mut custom = read_custom(&path)?;
        let Some(index) = custom
            .iter()
            .position(|item| custom_id(id) && item.id == id)
        else {
            return Ok(RemoveSource::Unknown);
        };
        custom.remove(index);
        let data = encode_sources(&path, &custom)?;
        let mut removed = Vec::new();
        let mut rules_changed = false;
        store.edit_rules(|rules| {
            let linked: HashSet<_> = rules
                .rules
                .iter()
                .filter(|rule| rule.scope.list_id() == Some(id))
                .map(|rule| rule.id.as_str())
                .collect();
            if linked != confirmed.iter().map(String::as_str).collect() {
                rules_changed = true;
                return false;
            }
            for (index, rule) in rules.rules.iter().enumerate() {
                if rule.scope.list_id() == Some(id) {
                    removed.push((index, rule.clone()));
                }
            }
            rules.rules.retain(|rule| rule.scope.list_id() != Some(id));
            !removed.is_empty()
        })?;
        if rules_changed {
            return Ok(RemoveSource::RulesChanged);
        }
        if let Err(error) = save_atomically(&path, &data) {
            if !removed.is_empty() {
                store.edit_rules(|rules| {
                    for (index, rule) in removed {
                        rules.rules.insert(index.min(rules.rules.len()), rule);
                    }
                    true
                })?;
            }
            return Err(error);
        }
        let _ = std::fs::remove_file(cache_path(store.dir(), id));
        Ok(RemoveSource::Removed)
    })
}

pub fn save_download(
    dir: &Path,
    source: &ListSource,
    previous: Option<std::time::SystemTime>,
    domains: &[String],
) -> Result<(), StoreError> {
    guarded(&custom_path(dir), || {
        if updated_at(dir, &source.id) == previous
            && sources(dir)
                .iter()
                .any(|list| list.id == source.id && list.url == source.url)
        {
            save(dir, &source.id, domains)?;
        }
        Ok(())
    })
}

pub fn with_source(
    dir: &Path,
    id: &str,
    work: impl FnOnce() -> Result<bool, StoreError>,
) -> Result<Option<bool>, StoreError> {
    guarded(&custom_path(dir), || {
        if sources(dir).iter().any(|list| list.id == id) {
            work().map(Some)
        } else {
            Ok(None)
        }
    })
}

fn cache_path(dir: &Path, id: &str) -> PathBuf {
    dir.join("domain-lists").join(format!("{id}.txt"))
}

fn valid_id(id: &str) -> bool {
    custom_id(id) || DomainList::ALL.iter().any(|list| list.id() == id)
}

pub fn updated_at(dir: &Path, id: &str) -> Option<std::time::SystemTime> {
    if !valid_id(id) {
        return None;
    }
    cache_path(dir, id).metadata().ok()?.modified().ok()
}

fn due(dir: &Path, id: &str, interval: Duration) -> bool {
    updated_at(dir, id)
        .and_then(|time| time.elapsed().ok())
        .is_none_or(|age| age >= interval)
}

pub fn read(dir: &Path, id: &str) -> Option<HashSet<String>> {
    use std::io::Read;
    if !valid_id(id) {
        return None;
    }
    let mut text = String::new();
    std::fs::File::open(cache_path(dir, id))
        .ok()?
        .take(MAX_BYTES + 1)
        .read_to_string(&mut text)
        .ok()?;
    Some(parse(&text).ok()?.into_iter().collect())
}

pub fn save(dir: &Path, id: &str, domains: &[String]) -> Result<(), StoreError> {
    assert!(valid_id(id), "validated list identifier");
    let path = cache_path(dir, id);
    guarded(&path, || save_atomically(&path, &domains.join("\n")))
}

/// UTF-8, one domain per line, with optional hash comments. Zero denotes
/// a file-level error (empty or too large); otherwise the error is the line number.
pub fn parse(raw: &str) -> Result<Vec<String>, usize> {
    if raw.len() as u64 > MAX_BYTES {
        return Err(0);
    }
    let mut domains = Vec::new();
    for (index, line) in raw.trim_start_matches('\u{feff}').lines().enumerate() {
        let token = line.split('#').next().unwrap_or_default().trim();
        if token.is_empty() {
            continue;
        }
        let (suffix, domain) = token
            .strip_prefix('.')
            .map_or(("", token), |rest| (".", rest));
        let domain = domain.strip_suffix('.').unwrap_or(domain);
        if domain
            .chars()
            .any(|ch| ch.is_ascii() && !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-')
        {
            return Err(index + 1);
        }
        let Ok(url::Host::Domain(ascii)) = url::Host::parse(domain) else {
            return Err(index + 1);
        };
        if (suffix.is_empty() && !ascii.contains('.'))
            || ascii.len() > 253
            || ascii.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
        {
            return Err(index + 1);
        }
        domains.push(format!("{suffix}{ascii}"));
    }
    domains.sort_unstable();
    domains.dedup();
    if domains.is_empty()
        || domains.iter().map(|name| name.len() + 1).sum::<usize>() > MAX_BYTES as usize
    {
        return Err(0);
    }
    Ok(domains)
}

#[must_use]
pub fn matches(domains: &HashSet<String>, host: &str) -> bool {
    let mut tail = host.trim_end_matches('.');
    loop {
        if domains.contains(tail) {
            return true;
        }
        let Some(dot) = tail.find('.') else {
            return false;
        };
        if domains.contains(&tail[dot..]) {
            return true;
        }
        tail = &tail[dot + 1..];
    }
}

pub fn due_for(store: &crate::Store) -> Vec<ListSource> {
    let prefs = store.prefs();
    if !prefs.domain_lists_enabled || !prefs.domain_lists_auto_refresh {
        return Vec::new();
    }
    let interval = prefs.domain_list_refresh_interval();
    let Ok(rules) = store.rules() else {
        return Vec::new();
    };
    sources(store.dir())
        .into_iter()
        .filter(|list| {
            list.url.is_some()
                && due(store.dir(), &list.id, interval)
                && rules
                    .rules
                    .iter()
                    .any(|rule| rule.scope.list_id() == Some(list.id.as_str()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hosts_and_explicit_suffixes_without_partial_label_matches() {
        let domains = parse(".ua\nEXAMPLE.COM.\nпример.рф\n# comment\n")
            .unwrap()
            .into_iter()
            .collect();
        assert!(matches(&domains, "sub.example.com."));
        assert!(matches(&domains, "test.ua"));
        assert!(matches(&domains, "xn--e1afmkfd.xn--p1ai"));
        assert!(!matches(&domains, "notexample.com"));
        assert!(!matches(&domains, "example.com.other"));
        assert_eq!(parse("example.com\n<html>"), Err(2));
        assert_eq!(parse("# empty"), Err(0));
        for invalid in [
            "https://example.org",
            "192.0.2.1",
            "*.example.org",
            "example.org:443",
            "exam%70le.org",
        ] {
            assert_eq!(parse(invalid), Err(1));
        }
        let suffix = parse(".example.net").unwrap().into_iter().collect();
        assert!(matches(&suffix, "inside.example.net"));
        assert!(!matches(&suffix, "example.net"));
    }

    #[test]
    fn a_saved_custom_list_has_a_safe_id_and_a_valid_cache() {
        let dir = std::env::temp_dir().join(format!("linkunbound-custom-{}", std::process::id()));
        let domains = parse("EXAMPLE.ORG\n# ignored\nexample.org").unwrap();
        assert!(save_source(&dir, None, "My list".to_owned(), None, &domains).unwrap());
        let items = sources(&dir);
        let custom = items
            .iter()
            .find(|item| item.name.as_deref() == Some("My list"))
            .unwrap();
        assert_eq!(
            read(&dir, &custom.id),
            Some(HashSet::from(["example.org".to_owned()]))
        );
        assert!(read(&dir, "../preferences").is_none());
        let store = crate::Store::at(dir.clone());
        store
            .edit_rules(|rules| {
                rules.upsert(crate::Rule {
                    id: String::new(),
                    scope: crate::Scope::NotDomainList(custom.id.clone()),
                    source_app: None,
                    target: crate::Target {
                        browser_id: "fox".to_owned(),
                        profile_id: None,
                    },
                    private: false,
                });
                true
            })
            .unwrap();
        assert!(matches!(
            remove_source(&store, &custom.id, &[]).unwrap(),
            RemoveSource::RulesChanged
        ));
        let replacement = parse("changed.example").unwrap();
        assert!(
            save_source(
                &dir,
                Some(&custom.id),
                "New name".to_owned(),
                Some("https://example.org/list.txt".to_owned()),
                &replacement
            )
            .unwrap()
        );
        save_download(&dir, custom, updated_at(&dir, &custom.id), &domains).unwrap();
        assert_eq!(
            read(&dir, &custom.id),
            Some(HashSet::from(["changed.example".to_owned()]))
        );
        assert_eq!(
            store.rules().unwrap().rules[0].scope.list_id(),
            Some(custom.id.as_str())
        );
        let id = format!("not_domain_list:{}", custom.id);
        assert!(matches!(
            remove_source(&store, &custom.id, &[id]).unwrap(),
            RemoveSource::Removed
        ));
        assert!(store.rules().unwrap().rules.is_empty());
        assert!(read(&dir, &custom.id).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
