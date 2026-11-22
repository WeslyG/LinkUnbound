import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { type Key, useSpoken, useWords } from "../i18n";
import { saidPlainly } from "../refusal";
import Confirm from "./Confirm";
import {
  type DomainList,
  domainListDate,
  type ListStatus,
  listDetail,
  listLabel,
} from "./domainLists";

/// One entry per place a rule can open: a browser with profiles is one per
/// profile, the same way the picker offers them.
type Destination = {
  browserId: string;
  profileId: string | null;
  label: string;
  opensPrivately: boolean;
};

type BrowserView = {
  id: string;
  name: string;
  profile_names: [string, string][];
  private: boolean;
  hidden: boolean;
};

type RuleView = {
  id: string;
  browser_id: string;
  profile_id: string | null;
  kind: "any" | "url" | "host" | "site" | "domain_list" | "not_domain_list";
  covers: string;
  browser: string;
  profile: string | null;
  icon: string | null;
  private: boolean;
  source_app: string | null;
  resolved: boolean;
  list_updated_at?: number | null;
  list_name?: string | null;
};

const KIND: Record<RuleView["kind"], Key> = {
  any: "kindAny",
  url: "kindUrl",
  host: "kindHost",
  site: "kindSite",
  domain_list: "kindDomainList",
  not_domain_list: "kindNotDomainList",
};

/// A `<select>` carries one string and a destination is a pair. NUL separates
/// them because no browser id or profile id can contain one.
const SPLIT = "\0";

function named(browserId: string, profileId: string | null): string {
  return `${browserId}${SPLIT}${profileId ?? ""}`;
}

type NewKind = RuleView["kind"] | "app";

const NEW_KINDS: { id: NewKind; label: Key; hint: Key }[] = [
  { id: "url", label: "ruleNewKindUrl", hint: "ruleNewValueUrl" },
  { id: "host", label: "ruleNewKindHost", hint: "ruleNewValueHost" },
  { id: "site", label: "ruleNewKindSite", hint: "ruleNewValueSite" },
  { id: "app", label: "ruleNewKindApp", hint: "ruleNewValueApp" },
  { id: "domain_list", label: "kindDomainList", hint: "domainListSource" },
  { id: "not_domain_list", label: "kindNotDomainList", hint: "domainListSource" },
  { id: "any", label: "ruleNewKindAny", hint: "ruleNewKindAny" },
];

function isFallback(rule: RuleView) {
  return rule.kind === "any" && rule.source_app === null;
}

function isList(kind: NewKind) {
  return kind === "domain_list" || kind === "not_domain_list";
}

/// The same rule the picker writes for the same choice, asked for in a form: what it covers,
/// where it opens, and whether privately. A refusal stays under the form, with the form.
function NewRule({
  choices,
  initialList,
  lists,
  onSaved,
  onCancel,
}: {
  choices: Destination[];
  initialList?: DomainList | null;
  lists: ListStatus[];
  onSaved: (rules: RuleView[]) => void;
  onCancel: () => void;
}) {
  const t = useWords();
  const language = useSpoken();
  const [kind, setKind] = useState<NewKind>(initialList ? "domain_list" : "site");
  const [value, setValue] = useState("");
  const [selected, setDomainList] = useState<DomainList | null>(initialList ?? null);
  const domainList = selected ?? lists[0]?.id ?? "";
  const detail = listDetail(domainList);
  const [where, setWhere] = useState(() =>
    choices[0] ? named(choices[0].browserId, choices[0].profileId) : "",
  );
  const [isPrivate, setPrivate] = useState(false);
  const [saving, setSaving] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const hint = NEW_KINDS.find((k) => k.id === kind)?.hint ?? "ruleNewValueSite";
  // A browser with no private window cannot be asked for one: the box is greyed rather than the
  // rule refused after the fact.
  const opensPrivately =
    choices.find((c) => named(c.browserId, c.profileId) === where)?.opensPrivately ?? true;

  // The catalogue is read off the registry and may land after the form opened: the select shows
  // its first entry either way, so the choice has to follow.
  useEffect(() => {
    if (where === "" && choices[0]) setWhere(named(choices[0].browserId, choices[0].profileId));
  }, [choices, where]);

  // The form opens on a press that asked for it, so the caret goes where the typing goes.
  const value_box = useRef<HTMLInputElement>(null);
  useEffect(() => value_box.current?.focus(), []);

  const field =
    "rounded-md border border-black/[0.12] bg-transparent px-2.5 py-1.5 text-[12px] focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-[#2F62D8] dark:border-white/[0.12] dark:focus-visible:outline-[#6E9BFF]";

  return (
    <form
      className="flex flex-col gap-3 rounded-lg border border-black/[0.08] bg-black/[0.015] p-3.5 dark:border-white/[0.08] dark:bg-white/[0.02]"
      onSubmit={(e) => {
        e.preventDefault();
        const [browserId, profileId] = where.split(SPLIT);
        setSaving(true);
        setRefusal(null);
        void invoke<RuleView[]>("rules_add", {
          kind,
          value: isList(kind) ? domainList : value,
          browserId,
          profileId: profileId || null,
          private: isPrivate && opensPrivately,
        })
          .then(onSaved)
          .catch((err: unknown) => setRefusal(saidPlainly(language, err)))
          .finally(() => setSaving(false));
      }}
    >
      <p className="text-[12.5px] font-semibold">{t("ruleNewTitle")}</p>
      <div className="flex flex-wrap gap-2">
        <label className="flex min-w-0 items-center gap-2 text-[12px]">
          <span className="text-neutral-500 dark:text-[#8B92A1]">{t("ruleNewKind")}</span>
          <select
            id="rule-new-kind"
            aria-label={t("ruleNewKind")}
            value={kind}
            onChange={(e) => setKind(e.target.value as NewKind)}
            className={field}
          >
            {NEW_KINDS.map((k) => (
              <option key={k.id} value={k.id}>
                {t(k.label)}
              </option>
            ))}
          </select>
        </label>
        {isList(kind) && (
          <select
            aria-label={t("domainListSource")}
            value={domainList}
            onChange={(event) => setDomainList(event.currentTarget.value as DomainList)}
            className={field}
          >
            {lists.map(({ id, name }) => (
              <option key={id} value={id}>
                {name ?? t(listLabel(id) ?? "kindDomainList")}
              </option>
            ))}
          </select>
        )}
        {kind !== "any" && !isList(kind) && (
          <input
            id="rule-new-value"
            aria-label={t(hint)}
            value={value}
            placeholder={t(hint)}
            onChange={(e) => setValue(e.target.value)}
            ref={value_box}
            className={`${field} min-w-[220px] flex-1`}
          />
        )}
      </div>
      {isList(kind) && (
        <div className="text-[11px] text-neutral-500 dark:text-[#8B92A1]">
          {detail && <p>{t(detail)}</p>}
          {kind === "not_domain_list" && <p>{t("domainListNegatedNote")}</p>}
          <p>{t("domainListNote")}</p>
        </div>
      )}
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <label className="flex items-center gap-2 text-[12px]">
          <span className="text-neutral-500 dark:text-[#8B92A1]">{t("ruleNewBrowser")}</span>
          <select
            id="rule-new-browser"
            aria-label={t("ruleNewBrowser")}
            value={where}
            onChange={(e) => setWhere(e.target.value)}
            className={field}
          >
            {choices.map((choice) => (
              <option
                key={named(choice.browserId, choice.profileId)}
                value={named(choice.browserId, choice.profileId)}
              >
                {choice.label}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-2 text-[12px]">
          <input
            id="rule-new-private"
            type="checkbox"
            checked={isPrivate && opensPrivately}
            disabled={!opensPrivately}
            onChange={(e) => setPrivate(e.target.checked)}
          />
          {t("ruleNewPrivate")}
        </label>
      </div>
      {refusal && (
        <p
          role="alert"
          className="rounded-md bg-[#C0362F]/10 px-3 py-2 text-[11.5px] text-[#C0362F] dark:bg-[#FF8A85]/10 dark:text-[#FF8A85]"
        >
          {refusal}
        </p>
      )}
      {choices.length === 0 && (
        <p className="text-[11.5px] text-neutral-500 dark:text-[#8B92A1]">{t("browsersNone")}</p>
      )}
      <div className="flex gap-2">
        <button
          type="submit"
          disabled={saving || choices.length === 0 || (isList(kind) && !domainList)}
          className="rounded-md bg-[#2F62D8] px-3 py-1.5 text-[11.5px] font-medium text-white disabled:opacity-50 dark:bg-[#6E9BFF] dark:text-[#12141B]"
        >
          {t("save")}
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="rounded-md border border-black/[0.12] px-3 py-1.5 text-[11.5px] dark:border-white/[0.12]"
        >
          {t("cancel")}
        </button>
      </div>
    </form>
  );
}

export default function Rules({ initialList }: { initialList?: DomainList | null }) {
  const t = useWords();
  const covers = (rule: RuleView) => {
    if (rule.kind === "any") return rule.source_app ?? t("ruleNewKindAny");
    if (isList(rule.kind)) {
      const label = listLabel(rule.covers);
      return rule.list_name ?? (label ? t(label) : rule.covers);
    }
    return rule.covers;
  };
  const language = useSpoken();
  const [rules, setRules] = useState<RuleView[] | null>(null);
  const [lists, setLists] = useState<ListStatus[]>([]);
  const [choices, setChoices] = useState<Destination[]>([]);
  const [asking, setAsking] = useState<RuleView | null>(null);
  const [adding, setAdding] = useState(!!initialList);
  const [selectedList, setSelectedList] = useState(initialList);
  const [problem, setProblem] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState<string | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);

  function refreshList(id: string) {
    setRefreshing(id);
    setRefreshError(null);
    void invoke<RuleView[]>("rules_refresh_list", { id })
      .then(setRules)
      .catch((error: unknown) => setRefreshError(saidPlainly(language, error)))
      .finally(() => setRefreshing(null));
  }

  const load = useCallback(
    (command: string, args: Record<string, unknown> = {}) => {
      void invoke<RuleView[]>(command, args)
        .then((next) => {
          setRules(next);
          setProblem(null);
        })
        .catch((e: unknown) => setProblem(saidPlainly(language, e)));
    },
    [language],
  );

  useEffect(() => load("rules_list"), [load]);
  useEffect(() => {
    void invoke<ListStatus[]>("domain_lists_status")
      .then(setLists)
      .catch(() => {});
  }, []);

  // The 1.x screen let a rule be pointed elsewhere in place. Deleting and
  // remaking it would send it to the end of the order, where a broader rule
  // may already answer first.
  useEffect(() => {
    void invoke<BrowserView[]>("browsers_list")
      .then((all) =>
        setChoices(
          all
            .filter((b) => !b.hidden)
            .flatMap<Destination>((b) =>
              b.profile_names.length > 0
                ? b.profile_names.map(([id, name]) => ({
                    browserId: b.id,
                    profileId: id,
                    label: `${b.name} · ${name}`,
                    opensPrivately: b.private,
                  }))
                : [{ browserId: b.id, profileId: null, label: b.name, opensPrivately: b.private }],
            ),
        ),
      )
      .catch(() => setChoices([]));
  }, []);

  if (problem) {
    return (
      <p className="rounded-md bg-[#C0362F]/10 px-3 py-2 text-[11.5px] text-[#C0362F] dark:bg-[#FF8A85]/10 dark:text-[#FF8A85]">
        {problem}
      </p>
    );
  }

  if (rules === null) return null;

  const addButton = (
    <button
      type="button"
      onClick={() => setAdding(true)}
      className="rounded-md bg-[#2F62D8] px-3 py-1.5 text-[11.5px] font-medium text-white dark:bg-[#6E9BFF] dark:text-[#12141B]"
    >
      {t("ruleAdd")}
    </button>
  );
  const form = adding && (
    <NewRule
      choices={choices}
      initialList={selectedList}
      lists={lists}
      onSaved={(next) => {
        setRules(next);
        setAdding(false);
        setSelectedList(null);
      }}
      onCancel={() => {
        setAdding(false);
        setSelectedList(null);
      }}
    />
  );

  const priority = (
    <div className="space-y-1 rounded-lg border border-black/[0.08] px-3 py-2.5 text-[11.5px] text-neutral-600 dark:border-white/[0.08] dark:text-[#A5ADBC]">
      <h2 className="font-semibold">{t("rulesPriorityTitle")}</h2>
      <p>{t("rulesPrioritySource")}</p>
      <p>{t("rulesPriorityScope")}</p>
      <p>{t("rulesPriorityOrder")}</p>
    </div>
  );

  if (rules.length === 0) {
    return (
      <div className="flex flex-col gap-3">
        {form}
        {!adding && (
          <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed border-black/[0.12] px-4 py-7 text-center dark:border-white/[0.12]">
            <div>
              <p className="text-[12.5px] font-semibold">{t("rulesEmptyTitle")}</p>
              <p className="mt-1 text-[12px] text-neutral-500 dark:text-[#8B92A1]">
                {t("rulesEmptyBody")}
              </p>
            </div>
            {addButton}
          </div>
        )}
        {priority}
      </div>
    );
  }

  const displayedRules = [...rules].sort((a, b) => Number(isFallback(a)) - Number(isFallback(b)));

  return (
    <section className="flex flex-col gap-2">
      {form}
      {asking && (
        <Confirm
          title={t("ruleDeleteTitle", covers(asking))}
          body={t("ruleDeleteBody")}
          go={t("deleteGo")}
          onConfirm={() => {
            load("rules_remove", { id: asking.id });
            setAsking(null);
          }}
          onCancel={() => setAsking(null)}
        />
      )}
      <div className="flex items-center justify-between">
        <h2 className="text-[11px] font-semibold tracking-[0.09em] text-neutral-400 uppercase dark:text-[#646B7C]">
          {rules.length === 1 ? t("rulesOne") : t("rulesMany", String(rules.length))}
        </h2>
        {!adding && addButton}
      </div>

      {refreshError && (
        <p role="alert" className="text-[11px] text-[#C0362F] dark:text-[#FF8A85]">
          {refreshError}
        </p>
      )}
      {priority}
      <ul className="overflow-hidden rounded-lg border border-black/[0.08] dark:border-white/[0.08]">
        {displayedRules.map((rule) => (
          <li
            key={rule.id}
            className="group flex items-center gap-3 border-black/[0.08] bg-black/[0.015] px-3 py-2.5 not-first:border-t hover:bg-black/[0.04] dark:border-white/[0.08] dark:bg-white/[0.02] dark:hover:bg-white/[0.05]"
          >
            <span
              className={`w-[70px] shrink-0 rounded px-1.5 py-0.5 text-center text-[10px] tracking-[0.03em] uppercase ${
                rule.source_app
                  ? "bg-[#A85B14]/[0.14] text-[#A85B14] dark:bg-[#E9A05C]/[0.16] dark:text-[#E9A05C]"
                  : rule.kind === "url"
                    ? "bg-[#2F62D8]/[0.11] text-[#2F62D8] dark:bg-[#6E9BFF]/[0.16] dark:text-[#6E9BFF]"
                    : "bg-black/[0.055] text-neutral-500 dark:bg-white/[0.07] dark:text-[#8B92A1]"
              }`}
            >
              {rule.source_app ? t("kindFrom") : t(KIND[rule.kind])}
            </span>

            <div className="min-w-0 flex-1 text-[12.5px]">
              <div className="truncate" title={covers(rule)}>
                {covers(rule)}
              </div>
              {isList(rule.kind) && (
                <div className="flex flex-wrap gap-x-2 text-[11px] text-neutral-500 dark:text-[#8B92A1]">
                  <span>
                    {rule.list_updated_at
                      ? t("domainListUpdated", domainListDate(rule.list_updated_at, language))
                      : t("domainListMissing")}
                  </span>
                  {lists.some((list) => list.id === rule.covers && list.source_url) && (
                    <button
                      type="button"
                      className="underline disabled:opacity-60"
                      disabled={refreshing !== null}
                      onClick={() => refreshList(rule.id)}
                    >
                      {t(refreshing === rule.id ? "domainListRefreshing" : "domainListRefresh")}
                    </button>
                  )}
                </div>
              )}
            </div>
            <span className="shrink-0 text-[11px] text-neutral-400 dark:text-[#646B7C]">→</span>

            <span className="flex shrink-0 items-center gap-1.5 text-[12px]">
              {rule.icon ? (
                <img src={rule.icon} alt="" className="h-4 w-4" />
              ) : (
                <span className="h-4 w-4 rounded bg-black/10 dark:bg-white/10" />
              )}
              {choices.length > 0 ? (
                <select
                  aria-label={t("ruleBrowser", covers(rule))}
                  value={named(rule.browser_id, rule.profile_id)}
                  onChange={(e) => {
                    const [browserId, profileId] = e.target.value.split(SPLIT);
                    load("rules_retarget", {
                      id: rule.id,
                      browserId,
                      profileId: profileId || null,
                    });
                  }}
                  className={`cursor-pointer rounded border-0 bg-transparent py-0 pr-5 pl-0 text-[12px] focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-[#2F62D8] dark:focus-visible:outline-[#6E9BFF] ${
                    rule.resolved ? "" : "line-through opacity-60"
                  }`}
                >
                  {!choices.some(
                    (c) =>
                      named(c.browserId, c.profileId) === named(rule.browser_id, rule.profile_id),
                  ) && (
                    <option value={named(rule.browser_id, rule.profile_id)}>{rule.browser}</option>
                  )}
                  {choices.map((choice) => (
                    <option
                      key={named(choice.browserId, choice.profileId)}
                      value={named(choice.browserId, choice.profileId)}
                    >
                      {choice.label}
                    </option>
                  ))}
                </select>
              ) : (
                <span className={rule.resolved ? "" : "line-through opacity-60"}>
                  {rule.browser}
                </span>
              )}
              {rule.profile && (
                <span className="rounded bg-black/[0.055] px-1.5 py-px text-[10.5px] text-neutral-500 dark:bg-white/[0.07] dark:text-[#8B92A1]">
                  {rule.profile}
                </span>
              )}
            </span>

            {rule.private && (
              <span className="shrink-0 text-[10.5px] text-[#2F62D8] dark:text-[#6E9BFF]">
                {t("rulePrivate")}
              </span>
            )}

            <span className="flex shrink-0 gap-px opacity-0 transition group-hover:opacity-100 focus-within:opacity-100">
              <button
                type="button"
                aria-label={t("ruleRemove", covers(rule))}
                onClick={() => setAsking(rule)}
                className="grid h-6 w-6 place-items-center rounded text-neutral-500 hover:bg-[#C0362F]/10 hover:text-[#C0362F] dark:text-[#8B92A1] dark:hover:bg-[#FF8A85]/10 dark:hover:text-[#FF8A85]"
              >
                ✕
              </button>
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
