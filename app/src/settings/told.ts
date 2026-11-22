import type { Language } from "../i18n";
import news from "../news.json";
import russianNews from "../news.ru.json";

export type Told = { title: string; said: string };

const RU: Record<string, { headline: string; told: Told[] }> = russianNews;

export function toldFor(
  versions: string[],
  language: Language,
): { version: string; told: Told[] }[] {
  return versions.flatMap((version) => {
    if (language === "ru") {
      return RU[version] ? [{ version, told: RU[version].told }] : [];
    }
    const found = news.find((one) => one.version === version);
    return found ? [{ version, told: found[language] }] : [];
  });
}

export function headline(version: string | undefined, language: Language): string | undefined {
  if (language === "ru") return version ? RU[version]?.headline : undefined;
  return news.find((one) => one.version === version)?.headline[language];
}
