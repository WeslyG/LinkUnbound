use std::io::Read;
use std::time::Duration;

use linkunbound_core::{
    Store,
    domain_lists::{self, ListSource, MAX_BYTES},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ListStatus {
    id: String,
    name: Option<String>,
    source_url: Option<String>,
    rule_id: Option<String>,
    updated_at: Option<u64>,
}

#[tauri::command]
pub fn domain_lists_status() -> Result<Vec<ListStatus>, String> {
    let store = super::store();
    let rules = store.rules().map_err(|err| err.to_string())?;
    Ok(domain_lists::sources(store.dir())
        .into_iter()
        .map(|list| ListStatus {
            updated_at: updated_at(store.dir(), &list.id),
            rule_id: rules
                .rules
                .iter()
                .find(|rule| rule.scope.list_id() == Some(list.id.as_str()))
                .map(|rule| rule.id.clone()),
            id: list.id,
            name: list.name,
            source_url: list.url,
        })
        .collect())
}

#[tauri::command]
pub fn domain_list_hosts(list: String) -> Vec<String> {
    let mut hosts: Vec<_> = domain_lists::read(super::store().dir(), &list)
        .unwrap_or_default()
        .into_iter()
        .collect();
    hosts.sort_unstable();
    hosts
}

pub fn updated_at(dir: &std::path::Path, list: &str) -> Option<u64> {
    domain_lists::updated_at(dir, list)?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|age| age.as_secs())
}

fn parsed(raw: &str) -> Result<Vec<String>, String> {
    domain_lists::parse(raw).map_err(|line| match line {
        0 => "ruleListEmptyOrLarge".to_owned(),
        line => format!("ruleListInvalidLine: {line}"),
    })
}

fn source_url(value: &str) -> Result<url::Url, String> {
    let url = url::Url::parse(value.trim()).map_err(|_| "ruleListInvalidUrl")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("ruleListInvalidUrl".to_owned());
    }
    Ok(url)
}

fn download(url: url::Url) -> Result<Vec<String>, String> {
    let failed = || "ruleListDownloadFailed".to_owned();
    let client = crate::http::client()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| failed())?;
    let response = client.get(url).send().map_err(|_| failed())?;
    if !response.status().is_success() {
        return Err(failed());
    }
    let mut raw = String::new();
    response
        .take(MAX_BYTES + 1)
        .read_to_string(&mut raw)
        .map_err(|_| failed())?;
    parsed(&raw)
}

#[tauri::command]
pub async fn domain_list_add(
    id: Option<String>,
    name: String,
    url: Option<String>,
    text: Option<String>,
) -> Result<Vec<ListStatus>, String> {
    let name = name.trim().to_owned();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("ruleListInvalidName".to_owned());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let (source, domains) = match (url, text) {
            (Some(url), None) => {
                let url = source_url(&url)?;
                (Some(url.to_string()), download(url)?)
            }
            (None, Some(text)) => (None, parsed(&text)?),
            _ => return Err("ruleListInvalidUrl".to_owned()),
        };
        if !domain_lists::save_source(super::store().dir(), id.as_deref(), name, source, &domains)
            .map_err(|err| err.to_string())?
        {
            return Err("ruleListUnknown".to_owned());
        }
        domain_lists_status()
    })
    .await
    .map_err(|_| "ruleListDownloadFailed".to_owned())?
}

#[tauri::command]
pub fn domain_list_remove(id: String, rule_ids: Vec<String>) -> Result<Vec<ListStatus>, String> {
    match domain_lists::remove_source(&super::store(), &id, &rule_ids)
        .map_err(|err| err.to_string())?
    {
        domain_lists::RemoveSource::Removed => domain_lists_status(),
        domain_lists::RemoveSource::RulesChanged => Err("ruleListLinksChanged".to_owned()),
        domain_lists::RemoveSource::Unknown => Err("ruleListUnknown".to_owned()),
    }
}

fn current_source(store: &Store, id: &str) -> Option<ListSource> {
    domain_lists::sources(store.dir())
        .into_iter()
        .find(|list| list.id == id)
}

fn refresh(
    store: &Store,
    source: &ListSource,
    scheduled: bool,
    fetch: impl FnOnce(url::Url) -> Result<Vec<String>, String>,
) -> Result<(), String> {
    domain_lists::try_refresh(store.dir(), source, || {
        let current = if scheduled {
            let Some(due) = domain_lists::due_list(store, &source.id) else {
                return Ok(());
            };
            due
        } else {
            current_source(store, &source.id).ok_or("ruleListUnknown")?
        };
        let Some(url) = current.url.as_deref() else {
            return Ok(());
        };
        let previous = domain_lists::updated_at(store.dir(), &current.id);
        let domains = fetch(source_url(url)?)?;
        domain_lists::save_download(store.dir(), &current, previous, &domains)
            .map_err(|err| err.to_string())
    })
    .map_err(|err| err.to_string())?
    .ok_or_else(|| "ruleListRefreshBusy".to_owned())?
}

fn refresh_due_with(store: &Store, fetch: impl Fn(url::Url) -> Result<Vec<String>, String>) {
    for list in domain_lists::due_for(store) {
        let _ = refresh(store, &list, true, &fetch);
    }
}

pub fn refresh_due(store: &Store) {
    refresh_due_with(store, download);
}

pub async fn refresh_now(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = super::store();
        let list = current_source(&store, &id).ok_or("ruleListUnknown")?;
        refresh(&store, &list, false, download)
    })
    .await
    .map_err(|_| "ruleListDownloadFailed".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkunbound_core::{Rule, Scope, Target};
    use std::cell::Cell;

    fn linked_list(store: &Store, url: String) -> ListSource {
        let domains = ["old.example".to_owned()];
        assert!(
            domain_lists::save_source(store.dir(), None, "Remote".to_owned(), Some(url), &domains)
                .unwrap()
        );
        let list = domain_lists::sources(store.dir())
            .into_iter()
            .find(|list| list.name.as_deref() == Some("Remote"))
            .unwrap();
        store
            .edit_rules(|rules| {
                rules.upsert(Rule {
                    id: String::new(),
                    scope: Scope::DomainList(list.id.clone()),
                    source_app: None,
                    target: Target {
                        browser_id: "firefox".to_owned(),
                        profile_id: None,
                    },
                    private: false,
                });
                true
            })
            .unwrap();
        list
    }

    #[test]
    fn background_refresh_checks_permission_age_and_overlapping_downloads() {
        let store = Store::at(crate::tests::scratch("domain-list-scheduled"));
        let url = "https://list.example/domains.txt";
        let list = linked_list(&store, url.to_owned());
        std::fs::File::options()
            .write(true)
            .open(
                store
                    .dir()
                    .join("domain-lists")
                    .join(format!("{}.txt", list.id)),
            )
            .unwrap()
            .set_modified(std::time::SystemTime::now() - Duration::from_secs(48 * 3600))
            .unwrap();

        refresh_due_with(&store, |_| panic!("downloads require permission"));
        store
            .edit_prefs(|prefs| {
                prefs.domain_lists_auto_refresh = true;
                true
            })
            .unwrap();

        let requests = Cell::new(0);
        refresh_due_with(&store, |asked| {
            assert_eq!(asked.as_str(), url);
            requests.set(requests.get() + 1);
            refresh_due_with(&store, |_| panic!("a second request for the same list"));
            assert_eq!(
                refresh(&store, &list, false, |_| panic!(
                    "another request for the same list"
                )),
                Err("ruleListRefreshBusy".to_owned())
            );
            Ok(vec!["new.example".to_owned()])
        });
        assert_eq!(requests.get(), 1);
        assert_eq!(
            domain_lists::read(store.dir(), &list.id),
            Some(["new.example".to_owned()].into())
        );
        refresh_due_with(&store, |_| panic!("recent caches are not due"));
    }

    #[test]
    fn a_failed_https_refresh_keeps_the_last_working_cache() {
        use std::io::Write;
        use std::net::{Ipv4Addr, TcpListener};
        use std::time::Instant;

        let store = Store::at(crate::tests::scratch("domain-list-https"));
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("https://{}/domains.txt", listener.local_addr().unwrap());
        let list = linked_list(&store, url);
        let previous = domain_lists::updated_at(store.dir(), &list.id);
        let listener_thread = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut record = [0];
                        stream.read_exact(&mut record).unwrap();
                        assert_eq!(record[0], 22, "expected a TLS handshake");
                        stream
                            .write_all(b"HTTP/1.1 503 Unavailable\r\n\r\n")
                            .unwrap();
                        break;
                    }
                    Err(why)
                        if why.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(why) => panic!("the HTTPS client did not reach the test server: {why}"),
                }
            }
        });
        let result = refresh(&store, &list, false, download);
        listener_thread.join().unwrap();
        assert_eq!(result, Err("ruleListDownloadFailed".to_owned()));
        assert_eq!(domain_lists::updated_at(store.dir(), &list.id), previous);
        assert_eq!(
            domain_lists::read(store.dir(), &list.id),
            Some(["old.example".to_owned()].into())
        );
    }
}
