use crate::browser::LaunchRefused;
use crate::launch::LaunchError;
use crate::prefs::Locale;

macro_rules! catalogue {
    ($($key:ident: $es:literal | $en:literal | $ru:literal,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct Strings {
            $(pub $key: &'static str,)*
        }

        const SPANISH: Strings = Strings { $($key: $es,)* };
        const ENGLISH: Strings = Strings { $($key: $en,)* };
        const RUSSIAN: Strings = Strings { $($key: $ru,)* };

        impl Strings {
            #[must_use]
            pub fn pairs(&self) -> Vec<(&'static str, &'static str)> {
                vec![$((stringify!($key), self.$key),)*]
            }
        }
    };
}

catalogue! {
    tray_settings: "Ajustes" | "Settings" | "Настройки",
    tray_quit: "Salir" | "Exit" | "Выход",
    picker_from: "Desde {}" | "From {}" | "Из {}",
    picker_remember: "Recordar esta elección" | "Remember this choice" | "Запомнить этот выбор",
    picker_private: "Privada" | "Private" | "Приватно",
    picker_copy: "Copiar la dirección" | "Copy the address" | "Копировать адрес",
    picker_close: "Cerrar" | "Close" | "Закрыть",
    picker_ask: "¿Dónde abro este enlace?" | "Where should this link open?" | "Где открыть эту ссылку?",
    picker_ask_private: "¿Dónde lo abro en privado?" | "Where should it open privately?" | "Где открыть ссылку приватно?",
    picker_private_line: "Se abrirá en una ventana privada" | "It will open in a private window" | "Ссылка откроется в приватном окне",
    picker_remember_short: "Recordar" | "Remember" | "Запомнить",
    picker_motto: "Cada enlace, donde tú decides." | "Every link, where you decide." | "Каждая ссылка открывается там, где вы хотите.",
    picker_preview: "Vista previa: elegir no abre nada" | "Preview: choosing opens nothing" | "Предпросмотр: выбор ничего не открывает",
    picker_add: "Añadir navegador" | "Add a browser" | "Добавить браузер",
    picker_add_more: "o un perfil" | "or a profile" | "или профиль",
    browser_copy_suffix: "{} (copia)" | "{} (copy)" | "{} (копия)",
    reach_once: "Solo esta vez" | "Just this time" | "Только сейчас",
    reach_url: "Esta URL" | "This URL" | "Этот URL",
    reach_subdomain: "Subdominio" | "Subdomain" | "Поддомен",
    reach_site: "Todo el sitio" | "The whole site" | "Весь сайт",
    reach_from_app: "Desde {}" | "From {}" | "Из {}",
    reach_more: "Más" | "More" | "Ещё",
    remember_url: "Recordar esta URL" | "Remember this URL" | "Запомнить этот URL",
    remember_subdomain: "Recordar el subdominio" | "Remember the subdomain" | "Запомнить поддомен",
    remember_site: "Recordar todo el sitio" | "Remember the whole site" | "Запомнить весь сайт",
    remember_from_app: "Recordar desde {}" | "Remember from {}" | "Запомнить выбор для {}",
    update_ready: "Versión {} lista" | "Version {} ready" | "Версия {} готова",
    update_how: "Se descarga, se instala y LinkUnbound se reinicia solo" | "It downloads, installs and LinkUnbound restarts on its own" | "После установки LinkUnbound перезапустится автоматически",
    update_store: "La instala Microsoft Store desde Ajustes" | "Microsoft Store installs it, from Settings" | "Microsoft Store установит обновление через настройки",
    update_store_coming: "Microsoft Store la está trayendo; Ajustes dice cómo va" | "Microsoft Store is bringing it; Settings says how it goes" | "Microsoft Store загружает обновление. Ход загрузки виден в настройках",
    update_store_landed: "Ya está en este equipo; entra al salir de LinkUnbound" | "Already on this computer; it takes over once LinkUnbound exits" | "Обновление готово и установится после выхода из LinkUnbound",
    update_open_settings: "Desde Ajustes, que explica cómo" | "From Settings, which says how" | "Откройте настройки, чтобы узнать подробности",
    update_take: "Actualizar" | "Update" | "Обновить",
    update_open: "Abrir" | "Open" | "Открыть",
    update_starting: "Preparando la actualización…" | "Getting the update ready…" | "Подготовка обновления…",
    update_getting: "Descargando la {}…" | "Downloading {}…" | "Загрузка версии {}…",
    update_installing: "Instalando la {}…" | "Installing {}…" | "Установка версии {}…",
    update_restart: "LinkUnbound se reinicia en un momento" | "LinkUnbound restarts in a moment" | "LinkUnbound скоро перезапустится",
    update_failed: "No se pudo actualizar" | "The update did not go through" | "Не удалось обновить приложение",
    update_again: "Reintentar" | "Try again" | "Повторить",
    update_later: "Ahora no" | "Not now" | "Не сейчас",
    wrapper_unresolved: "Enlace envuelto: el destino real se conocerá al abrirlo" | "Wrapped link: the real destination is known once it opens" | "Ссылка обёрнута: конечный адрес станет известен при открытии",
    no_browsers: "Ningún navegador que ofrecer. Añade uno en Ajustes." | "No browser to offer. Add one in Settings." | "Нет доступных браузеров. Добавьте браузер в настройках.",
    notice_opened: "Abierto en {}" | "Opened in {}" | "Открыто в {}",
    notice_by_rule: "Una regla decidió por {}" | "A rule decided for {}" | "Браузер {} выбран по правилу",
    notice_undo: "Deshacer" | "Undo" | "Отменить",
    notice_rule_failed: "La regla no pudo abrir {}" | "The rule could not open {}" | "По правилу не удалось открыть ссылку в {}",
    notice_rules_unreadable: "No se pudieron leer tus reglas" | "Your rules could not be read" | "Не удалось прочитать правила",
    notice_pick_instead: "Elige un navegador esta vez" | "Pick a browser this time" | "Выберите браузер для этой ссылки",
    fail_not_remembered: "No se pudo guardar la regla" | "The rule could not be saved" | "Не удалось сохранить правило",
    fail_unknown_browser: "Ese navegador ya no está configurado" | "That browser is no longer configured" | "Этот браузер больше не настроен",
    fail_profile_gone: "El perfil {} ya no existe" | "The profile {} is no longer there" | "Профиль {} больше не существует",
    fail_private_impossible: "Este navegador no puede abrir una ventana privada" | "This browser cannot open a private window" | "Этот браузер не поддерживает приватные окна",
    fail_spawn: "El navegador no arrancó" | "The browser would not start" | "Не удалось запустить браузер",
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Spanish,
    English,
    Russian,
}

impl Language {
    #[must_use]
    pub const fn strings(self) -> Strings {
        match self {
            Self::Spanish => SPANISH,
            Self::English => ENGLISH,
            Self::Russian => RUSSIAN,
        }
    }

    #[must_use]
    pub fn of_tag(tag: Option<&str>) -> Self {
        match tag.and_then(|tag| tag.trim().split(['-', '_', '.', '@']).next()) {
            Some(language) if language.eq_ignore_ascii_case("es") => Self::Spanish,
            Some(language) if language.eq_ignore_ascii_case("ru") => Self::Russian,
            _ => Self::English,
        }
    }

    #[must_use]
    pub fn chosen(locale: Locale) -> Self {
        match locale {
            Locale::Spanish => Self::Spanish,
            Locale::English => Self::English,
            Locale::Russian => Self::Russian,
            Locale::System => Self::of_tag(sys_locale::get_locale().as_deref()),
        }
    }
}

impl Strings {
    #[must_use]
    pub fn fill(template: &str, value: &str) -> String {
        template.replace("{}", value)
    }

    /// The `Display` of these stays English for the log; the picker cannot.
    #[must_use]
    pub fn on_failure(&self, failure: &LaunchError) -> String {
        match failure {
            LaunchError::UnknownBrowser(_) => self.fail_unknown_browser.to_owned(),
            LaunchError::Refused(LaunchRefused::ProfileGone(profile)) => {
                Self::fill(self.fail_profile_gone, profile)
            }
            LaunchError::Refused(LaunchRefused::PrivateImpossible) => {
                self.fail_private_impossible.to_owned()
            }
            LaunchError::Spawn(_) => self.fail_spawn.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ENGLISH, Language, SPANISH, Strings};
    use crate::browser::LaunchRefused;
    use crate::launch::LaunchError;
    use crate::prefs::Locale;

    #[test]
    fn an_explicit_choice_is_obeyed_and_the_rest_is_asked_of_the_system() {
        assert_eq!(Language::chosen(Locale::Spanish), Language::Spanish);
        assert_eq!(Language::chosen(Locale::English), Language::English);
        assert_eq!(Language::chosen(Locale::Russian), Language::Russian);
    }

    #[test]
    fn only_a_spanish_system_gets_spanish() {
        assert_eq!(Language::of_tag(Some("es-CL")), Language::Spanish);
        assert_eq!(Language::of_tag(Some("es")), Language::Spanish);
        assert_eq!(Language::of_tag(Some("en-US")), Language::English);
        assert_eq!(Language::of_tag(Some("ru-RU")), Language::Russian);
        assert_eq!(Language::of_tag(Some("RU_RU.UTF-8")), Language::Russian);
        assert_eq!(Language::of_tag(Some("eso")), Language::English);
        assert_eq!(Language::of_tag(Some("pt-BR")), Language::English);
        assert_eq!(Language::of_tag(None), Language::English);
    }

    #[test]
    fn a_placeholder_present_in_one_language_is_present_in_both() {
        for ((key, spanish), (_, english)) in SPANISH.pairs().into_iter().zip(ENGLISH.pairs()) {
            assert_eq!(
                spanish.contains("{}"),
                english.contains("{}"),
                "{key} disagrees about its placeholder"
            );
        }
    }

    /// Words that are the same in both languages, and would read as an
    /// oversight rather than as a translation.
    const SHARED: [&str; 2] = ["picker_private", "tray_settings"];

    /// A string left as the Spanish one ships green otherwise: it is not blank,
    /// it keeps its placeholders, and no test compares the two languages.
    #[test]
    fn an_english_string_is_not_just_the_spanish_one() {
        for ((key, spanish), (_, english)) in SPANISH.pairs().into_iter().zip(ENGLISH.pairs()) {
            if SHARED.contains(&key) {
                continue;
            }
            assert_ne!(
                spanish, english,
                "{key} reads the same in both languages: untranslated, or a shared word to list"
            );
        }
    }

    /// The reaches sit side by side in one row, so a pair swapped in one
    /// language alone points the user at the wrong scope without looking odd.
    #[test]
    fn each_reach_says_the_same_thing_in_both_languages() {
        for (spanish, english, widening) in [
            (SPANISH.reach_url, ENGLISH.reach_url, "url"),
            (
                SPANISH.reach_subdomain,
                ENGLISH.reach_subdomain,
                "subdomain",
            ),
            (SPANISH.reach_site, ENGLISH.reach_site, "site"),
        ] {
            let es = spanish.to_lowercase();
            let en = english.to_lowercase();
            let agreed = match widening {
                "url" => es.contains("url") && en.contains("url"),
                "subdomain" => es.contains("subdominio") && en.contains("subdomain"),
                _ => es.contains("sitio") && en.contains("site"),
            };
            assert!(
                agreed,
                "the {widening} reach disagrees: {spanish} / {english}"
            );
        }
    }

    #[test]
    fn nothing_ships_blank_or_untranslated() {
        for ((key, spanish), (_, english)) in SPANISH.pairs().into_iter().zip(ENGLISH.pairs()) {
            assert!(!spanish.trim().is_empty(), "{key} is blank in Spanish");
            assert!(!english.trim().is_empty(), "{key} is blank in English");
        }
    }

    #[test]
    fn a_failure_is_reported_in_the_users_language_and_keeps_the_detail() {
        let gone = LaunchError::Refused(LaunchRefused::ProfileGone("Trabajo".to_owned()));
        assert_eq!(SPANISH.on_failure(&gone), "El perfil Trabajo ya no existe");
        assert_eq!(
            ENGLISH.on_failure(&gone),
            "The profile Trabajo is no longer there"
        );
    }

    #[test]
    fn filling_a_template_puts_the_value_where_the_marker_was() {
        assert_eq!(
            Strings::fill(SPANISH.notice_opened, "Chrome"),
            "Abierto en Chrome"
        );
        assert_eq!(
            Strings::fill(ENGLISH.notice_by_rule, "github.com"),
            "A rule decided for github.com"
        );
    }
}
