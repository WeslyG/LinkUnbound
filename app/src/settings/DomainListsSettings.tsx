import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useRef, useState } from "react";
import { useSpoken, useWords } from "../i18n";
import { saidPlainly } from "../refusal";
import Confirm from "./Confirm";
import {
  type DomainList,
  domainListDate,
  type ListStatus,
  listDetail,
  listLabel,
} from "./domainLists";
import { Card, Line, Section, Switch } from "./parts";

export type DomainListPreferences = {
  domain_lists_enabled: boolean;
  domain_lists_auto_refresh: boolean;
  domain_lists_refresh_hours: number;
};

const INTERVALS = [6, 12, 24, 48, 168];
const SOURCE_REPOSITORY = "https://github.com/itdoginfo/allow-domains";

type LinkedRule = {
  id: string;
  kind: string;
  covers: string;
  browser: string;
  profile: string | null;
};

export default function DomainListsSettings({
  prefs,
  apply,
  onConnectList,
}: {
  prefs: DomainListPreferences;
  apply: (patch: Partial<DomainListPreferences>) => void;
  onConnectList?: (list: DomainList) => void;
}) {
  const t = useWords();
  const language = useSpoken();
  const [statuses, setStatuses] = useState<ListStatus[]>([]);
  const [unreadable, setUnreadable] = useState(false);
  const [refreshing, setRefreshing] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [preview, setPreview] = useState<{ id: DomainList; hosts: string[] } | null>(null);
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState("");
  const [adding, setAdding] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<{ list: ListStatus; rules: LinkedRule[] } | null>(null);
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("");
  const [sourceType, setSourceType] = useState("text");
  const [sourceUrl, setSourceUrl] = useState("");
  const [sourceText, setSourceText] = useState("");
  const nameInput = useRef<HTMLInputElement>(null);
  const enabled = prefs.domain_lists_enabled;
  const autoRefresh = prefs.domain_lists_auto_refresh;
  const hours = prefs.domain_lists_refresh_hours;

  const read = useCallback(
    () =>
      invoke<ListStatus[]>("domain_lists_status")
        .then((next) => {
          setStatuses(next);
          setUnreadable(false);
        })
        .catch(() => setUnreadable(true)),
    [],
  );

  useEffect(() => {
    void read();
    const timer = setInterval(() => void read(), 15000);
    return () => clearInterval(timer);
  }, [read]);

  useEffect(() => {
    if (adding) nameInput.current?.focus();
  }, [adding]);

  function refresh(ruleId: string) {
    setRefreshing(ruleId);
    setProblem(null);
    void invoke("rules_refresh_list", { id: ruleId })
      .then(async () => {
        await read();
        setPreview(null);
      })
      .catch((error: unknown) => setProblem(saidPlainly(language, error)))
      .finally(() => setRefreshing(null));
  }

  function viewHosts(id: DomainList) {
    setProblem(null);
    setQuery("");
    if (preview?.id === id) {
      setPreview(null);
      return;
    }
    setLoading(true);
    void invoke<string[]>("domain_list_hosts", { list: id })
      .then((hosts) => setPreview({ id, hosts }))
      .catch((error: unknown) => setProblem(saidPlainly(language, error)))
      .finally(() => setLoading(false));
  }

  function add() {
    setEditing(null);
    setName("");
    setSourceType("text");
    setSourceUrl("");
    setSourceText("");
    setProblem(null);
    setAdding(true);
  }

  function edit(list: ListStatus) {
    setProblem(null);
    setLoading(true);
    void invoke<string[]>("domain_list_hosts", { list: list.id })
      .then((hosts) => {
        setEditing(list.id);
        setName(list.name ?? "");
        setSourceType(list.source_url ? "url" : "text");
        setSourceUrl(list.source_url ?? "");
        setSourceText(hosts.join("\n"));
        setAdding(true);
      })
      .catch((error: unknown) => setProblem(saidPlainly(language, error)))
      .finally(() => setLoading(false));
  }

  function confirmRemoval(list: ListStatus) {
    setProblem(null);
    setLoading(true);
    void invoke<LinkedRule[]>("rules_list")
      .then((rules) =>
        setDeleting({
          list,
          rules: rules.filter(
            (rule) =>
              rule.covers === list.id && ["domain_list", "not_domain_list"].includes(rule.kind),
          ),
        }),
      )
      .catch((error: unknown) => setProblem(saidPlainly(language, error)))
      .finally(() => setLoading(false));
  }

  function remove(id: string, ruleIds: string[]) {
    setDeleting(null);
    setProblem(null);
    setSaving(true);
    void invoke<ListStatus[]>("domain_list_remove", { id, ruleIds })
      .then((lists) => {
        setStatuses(lists);
        if (preview?.id === id) setPreview(null);
      })
      .catch((error: unknown) => setProblem(saidPlainly(language, error)))
      .finally(() => setSaving(false));
  }

  const filtered = preview?.hosts.filter((host) => host.includes(query.trim().toLowerCase()));
  const action =
    "text-[11.5px] text-[#2F62D8] hover:underline disabled:opacity-50 dark:text-[#6E9BFF]";
  const field =
    "w-full rounded-md border border-black/[0.12] bg-transparent px-2 py-1.5 text-[12px] dark:border-white/[0.12]";
  const openSource = (url: string) => {
    setProblem(null);
    void openUrl(url).catch((error: unknown) => setProblem(saidPlainly(language, error)));
  };

  return (
    <Section title={t("domainListsSettings")}>
      <Card>
        <Line title={t("domainListsOrigin")} note={t("domainListsOriginNote")}>
          <button type="button" className={action} onClick={() => openSource(SOURCE_REPOSITORY)}>
            itdoginfo/allow-domains
          </button>
        </Line>
        <Line title={t("domainListCustom")} note={t("domainListFormat")}>
          <button
            type="button"
            className={action}
            aria-expanded={adding}
            disabled={saving || loading}
            onClick={() => {
              if (adding) {
                setAdding(false);
                setProblem(null);
              } else {
                add();
              }
            }}
          >
            {t(adding ? "cancel" : "domainListAdd")}
          </button>
        </Line>
        {adding && (
          <form
            className="space-y-2 px-3.5 pb-3"
            onSubmit={(event) => {
              event.preventDefault();
              setSaving(true);
              setProblem(null);
              void invoke<ListStatus[]>("domain_list_add", {
                ...(editing ? { id: editing } : {}),
                name,
                url: sourceType === "url" ? sourceUrl : null,
                text: sourceType === "text" ? sourceText : null,
              })
                .then((lists) => {
                  setStatuses(lists);
                  setAdding(false);
                  setPreview(null);
                })
                .catch((error: unknown) => setProblem(saidPlainly(language, error)))
                .finally(() => setSaving(false));
            }}
          >
            {editing && <p className="text-[12px] font-semibold">{t("domainListEditing")}</p>}
            <input
              ref={nameInput}
              required
              maxLength={80}
              aria-label={t("domainListName")}
              placeholder={t("domainListName")}
              value={name}
              onChange={(event) => setName(event.currentTarget.value)}
              className={field}
            />
            <select
              aria-label={t("domainListSourceType")}
              value={sourceType}
              onChange={(event) => {
                setSourceType(event.currentTarget.value);
              }}
              className={field}
            >
              <option value="text">{t("domainListText")}</option>
              <option value="url">{t("domainListUrl")}</option>
            </select>
            {sourceType === "url" ? (
              <input
                type="url"
                required
                aria-label={t("domainListUrl")}
                placeholder="https://example.org/domains.txt"
                value={sourceUrl}
                onChange={(event) => setSourceUrl(event.currentTarget.value)}
                className={field}
              />
            ) : (
              <textarea
                required
                rows={5}
                aria-label={t("domainListText")}
                placeholder={"# Example\nexample.org\n.example.net"}
                value={sourceText}
                onChange={(event) => setSourceText(event.currentTarget.value)}
                className={field}
              />
            )}
            <button type="submit" disabled={saving} className={action}>
              {t(saving ? "domainListValidating" : "save")}
            </button>
            {problem && (
              <p role="alert" className="text-[11px] text-[#C0362F] dark:text-[#FF8A85]">
                {problem}
              </p>
            )}
          </form>
        )}
      </Card>
      <Card>
        <Line title={t("domainListsEnabled")} note={t("domainListsEnabledNote")}>
          <Switch
            on={enabled}
            label={t("domainListsEnabled")}
            onChange={(domain_lists_enabled) => apply({ domain_lists_enabled })}
          />
        </Line>
        <Line title={t("domainListsAutoRefresh")} note={t("domainListsAutoRefreshNote")}>
          <Switch
            on={autoRefresh}
            disabled={!enabled}
            label={t("domainListsAutoRefresh")}
            onChange={(domain_lists_auto_refresh) => apply({ domain_lists_auto_refresh })}
          />
        </Line>
        <Line title={t("domainListsInterval")} note={t("domainListsIntervalNote")}>
          <select
            aria-label={t("domainListsInterval")}
            disabled={!enabled}
            value={hours}
            onChange={(event) =>
              apply({ domain_lists_refresh_hours: Number(event.currentTarget.value) })
            }
            className="rounded-md border border-black/[0.12] bg-white px-2.5 py-1.5 text-[12px] disabled:opacity-50 dark:border-white/[0.12] dark:bg-[#1B1D23]"
          >
            {[...new Set([...INTERVALS, hours])]
              .sort((a, b) => a - b)
              .map((interval) => (
                <option key={interval} value={interval}>
                  {t("domainListsHours", String(interval))}
                </option>
              ))}
          </select>
        </Line>
      </Card>
      {deleting && (
        <Confirm
          title={t("domainListDeleteTitle", deleting.list.name ?? deleting.list.id)}
          body={t(deleting.rules.length ? "domainListDeleteWithRules" : "domainListDeleteBody")}
          go={t(deleting.rules.length ? "domainListDeleteAll" : "deleteGo")}
          onConfirm={() =>
            remove(
              deleting.list.id,
              deleting.rules.map((rule) => rule.id),
            )
          }
          onCancel={() => setDeleting(null)}
        >
          {deleting.rules.length > 0 && (
            <ul className="mt-2 list-disc space-y-1 pl-5 text-[12px]">
              {deleting.rules.map((rule) => (
                <li key={rule.id}>
                  {t(rule.kind === "not_domain_list" ? "kindNotDomainList" : "kindDomainList")}
                  {" → "}
                  {rule.browser}
                  {rule.profile && ` · ${rule.profile}`}
                </li>
              ))}
            </ul>
          )}
        </Confirm>
      )}
      {statuses.length > 0 && (
        <Card>
          <p className="px-3.5 py-3 text-[11.5px] text-neutral-600 dark:text-[#A1A7B3]">
            {t("domainListsIndependent")}
          </p>
          {statuses.map((list) => {
            const { id, name, source_url, rule_id, updated_at } = list;
            const title = name ?? t(listLabel(id) ?? "kindDomainList");
            const detail = listDetail(id);
            const age = updated_at === null ? null : Date.now() - updated_at * 1000;
            const date =
              updated_at === null
                ? t("domainListMissing")
                : t("domainListUpdated", domainListDate(updated_at, language));
            const state =
              rule_id === null
                ? t("domainListNoRule")
                : !enabled
                  ? t("domainListRulePaused")
                  : !source_url && age !== null
                    ? t("domainListText")
                    : age === null || age < 0 || age >= hours * 3600000
                      ? t("domainListDue")
                      : t("domainListFresh");
            return (
              <div key={id} className="border-t border-black/[0.08] dark:border-white/[0.08]">
                <Line title={title} note={detail ? t(detail) : (source_url ?? t("domainListText"))}>
                  <div className="flex shrink-0 flex-wrap justify-end gap-2">
                    {updated_at !== null && (
                      <button
                        type="button"
                        disabled={loading}
                        aria-expanded={preview?.id === id}
                        className={action}
                        onClick={() => viewHosts(id)}
                      >
                        {t("domainListViewHosts")}
                      </button>
                    )}
                    {source_url && (
                      <button
                        type="button"
                        className={action}
                        onClick={() => openSource(source_url)}
                      >
                        {t("domainListViewSource")}
                      </button>
                    )}
                    {rule_id !== null && source_url && (
                      <button
                        type="button"
                        disabled={refreshing !== null}
                        aria-label={t("domainListRefreshNamed", title)}
                        onClick={() => refresh(rule_id)}
                        className={action}
                      >
                        {t(refreshing === rule_id ? "domainListRefreshing" : "domainListRefresh")}
                      </button>
                    )}
                    {onConnectList && (
                      <button type="button" className={action} onClick={() => onConnectList(id)}>
                        {t(rule_id === null ? "domainListConnect" : "ruleAdd")}
                      </button>
                    )}
                    {name !== null && name !== undefined && (
                      <>
                        <button
                          type="button"
                          className={action}
                          disabled={adding || saving || loading}
                          onClick={() => edit(list)}
                        >
                          {t("domainListEdit")}
                        </button>
                        <button
                          type="button"
                          className={action}
                          disabled={saving || adding || loading}
                          aria-label={t("domainListDeleteAction", title)}
                          onClick={() => confirmRemoval(list)}
                        >
                          {t("deleteGo")}
                        </button>
                      </>
                    )}
                  </div>
                </Line>
                <p className="px-3.5 pb-3 text-[11px] text-neutral-500 dark:text-[#8B92A1]">
                  {state} · {date}
                </p>
                {preview?.id === id && filtered && (
                  <div className="space-y-2 px-3.5 pb-3">
                    <input
                      type="search"
                      aria-label={t("domainListSearch")}
                      placeholder={t("domainListSearch")}
                      value={query}
                      onChange={(event) => setQuery(event.currentTarget.value)}
                      className="w-full rounded-md border border-black/[0.12] bg-transparent px-2 py-1.5 text-[12px] dark:border-white/[0.12]"
                    />
                    <p className="text-[11px] text-neutral-500 dark:text-[#8B92A1]">
                      {t(
                        "domainListHostCount",
                        String(filtered.length),
                        String(preview.hosts.length),
                      )}
                    </p>
                    {filtered.length > 0 && (
                      <pre className="max-h-56 overflow-auto rounded-md bg-black/[0.04] p-2.5 text-[11px] dark:bg-white/[0.06]">
                        {filtered.join("\n")}
                      </pre>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </Card>
      )}
      {(unreadable || (problem && !adding)) && (
        <p role="alert" className="text-[11px] text-[#C0362F] dark:text-[#FF8A85]">
          {problem ?? t("domainListsStatusFailed")}
        </p>
      )}
    </Section>
  );
}
