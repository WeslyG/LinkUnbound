use std::io::Read;
use std::time::Duration;

use linkunbound_core::domain_lists::{self, ListSource, MAX_BYTES, due_for};
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

fn refresh(list: &ListSource, store: &linkunbound_core::Store) -> Result<(), String> {
    let Some(url) = list.url.as_deref() else {
        return Ok(());
    };
    let previous = domain_lists::updated_at(store.dir(), &list.id);
    let domains = download(source_url(url)?)?;
    domain_lists::save_download(store.dir(), list, previous, &domains)
        .map_err(|err| err.to_string())
}

pub fn refresh_due(store: &linkunbound_core::Store) {
    for list in due_for(store) {
        if !due_for(store).iter().any(|due| due.id == list.id) {
            continue;
        }
        let _ = refresh(&list, store);
    }
}

pub async fn refresh_now(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = super::store();
        let list = domain_lists::sources(store.dir())
            .into_iter()
            .find(|list| list.id == id)
            .ok_or("ruleListUnknown")?;
        refresh(&list, &store)
    })
    .await
    .map_err(|_| "ruleListDownloadFailed".to_owned())?
}
