export const DOMAIN_LISTS = {
  russia_inside: "domainListRussiaInside",
  russia_outside: "domainListRussiaOutside",
  ukraine: "domainListUkraine",
} as const;

export type DomainList = string;

export type ListStatus = {
  id: DomainList;
  name?: string | null;
  source_url?: string | null;
  rule_id: string | null;
  updated_at: number | null;
};

export function listLabel(id: string) {
  return Object.hasOwn(DOMAIN_LISTS, id)
    ? DOMAIN_LISTS[id as keyof typeof DOMAIN_LISTS]
    : undefined;
}

export const DOMAIN_LIST_DETAILS = {
  russia_inside: "domainListRussiaInsideDetail",
  russia_outside: "domainListRussiaOutsideDetail",
  ukraine: "domainListUkraineDetail",
} as const;

export function listDetail(id: string) {
  return Object.hasOwn(DOMAIN_LIST_DETAILS, id)
    ? DOMAIN_LIST_DETAILS[id as keyof typeof DOMAIN_LIST_DETAILS]
    : undefined;
}

export function domainListDate(seconds: number, language: string): string {
  return new Date(seconds * 1000).toLocaleString(language, {
    dateStyle: "short",
    timeStyle: "short",
  });
}
