use super::{
    Edit, Errand, Store, added, describe, duplicated_in, edited, errand_in, far_along, free_id,
    hidden_in, keep_in, ordered, readable, rescanned, spoken, without,
};
use crate::editing::seen;
use linkunbound_core::{Locale, Preferences, Profile, Rule, Scope, Target};

/// The resident's errands are told apart from a launch that wants the window, and from each
/// other; a stray word in the arguments is not one.
#[test]
fn an_errand_is_read_off_the_arguments_and_nothing_else_is() {
    let args = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(
        errand_in(&args(&["settings.exe", "--look"])),
        Some(Errand::Look)
    );
    assert_eq!(
        errand_in(&args(&["settings.exe", "--update"])),
        Some(Errand::Update)
    );
    assert_eq!(errand_in(&args(&["settings.exe"])), None);
    assert_eq!(errand_in(&args(&["settings.exe", "--register"])), None);
    assert_eq!(
        errand_in(&args(&["settings.exe", "update"])),
        None,
        "not a flag"
    );
}

fn with_profiles(id: &str, profiles: &[&str]) -> linkunbound_core::Browser {
    linkunbound_core::Browser {
        profiles: profiles
            .iter()
            .map(|p| Profile {
                id: (*p).to_owned(),
                name: (*p).to_owned(),
                args: vec![],
            })
            .collect(),
        ..detected(id)
    }
}

fn aimed(browser: &str, profile: Option<&str>) -> Target {
    Target {
        browser_id: browser.to_owned(),
        profile_id: profile.map(str::to_owned),
    }
}

/// The form writes the same rule the picker would for the same choice, so the two ways in
/// cannot disagree about what a kind means.
#[test]
fn a_rule_from_the_form_covers_what_its_kind_says() {
    use super::rule_from;
    let all = vec![with_profiles("chrome", &["Default"]), detected("firefox")];

    let url = rule_from(
        "url",
        " https://docs.google.com/a?b=1 ",
        aimed("firefox", None),
        false,
        &all,
    )
    .expect("a url");
    assert_eq!(
        url.scope,
        Scope::Url("https://docs.google.com/a?b=1".to_owned())
    );
    assert!(url.source_app.is_none());

    let fallback = rule_from("any", "", aimed("chrome", Some("Default")), false, &all)
        .expect("an unmatched-link rule needs no address");
    assert_eq!(fallback.scope, Scope::Any);
    assert!(fallback.source_app.is_none());
    let mut rules = linkunbound_core::RuleSet::default();
    rules.upsert(fallback);
    rules.upsert(url);
    assert_eq!(
        rules
            .resolve_from("https://docs.google.com/a?b=1", "docs.google.com", None)
            .expect("exact URL rule takes priority")
            .target,
        aimed("firefox", None)
    );
    assert_eq!(
        rules
            .resolve_from("https://example.org/", "example.org", None)
            .expect("fallback handles an unmatched URL")
            .target,
        aimed("chrome", Some("Default"))
    );

    let host = rule_from(
        "host",
        "https://docs.google.com/x",
        aimed("firefox", None),
        true,
        &all,
    )
    .expect("a host, read off a url");
    assert_eq!(host.scope, Scope::Host("docs.google.com".to_owned()));
    assert!(host.private);

    let site = rule_from(
        "site",
        "Docs.Google.com",
        aimed("chrome", Some("Default")),
        false,
        &all,
    )
    .expect("a site, read off a bare host");
    assert_eq!(site.scope, Scope::Site("google.com".to_owned()));

    let app = rule_from("app", "Slack.EXE", aimed("firefox", None), false, &all).expect("an app");
    assert_eq!(app.scope, Scope::Any);
    assert_eq!(
        app.source_app.as_deref(),
        Some("slack"),
        "as the picker records an origin: lowercased, without the executable's suffix"
    );

    let odd = rule_from(
        "host",
        "User@Docs.Google.com:8443/x.",
        aimed("firefox", None),
        false,
        &all,
    )
    .expect("a host, read the way a link's host is read");
    assert_eq!(odd.scope, Scope::Host("docs.google.com".to_owned()));
}

#[test]
fn an_app_typed_by_name_is_the_one_a_rule_already_names() {
    use super::named_alike;
    let saved = |app: &str| Rule {
        id: String::new(),
        scope: Scope::Any,
        source_app: Some(app.to_owned()),
        target: aimed("chrome", None),
        private: false,
    };
    let typed = saved("slack");
    assert!(named_alike(&[saved("Slack")], &typed));
    assert!(!named_alike(&[saved("teams")], &typed));
    #[cfg(target_os = "macos")]
    assert!(named_alike(&[saved("com.apple.Safari")], &saved("safari")));
}

/// The picker saves a rule for `intranet` or `localhost`; the form has to accept what the
/// picker writes, or the same rule can be made one way and not the other.
#[test]
fn a_single_name_is_a_host_in_the_form_as_in_the_picker() {
    use super::rule_from;
    let all = vec![with_profiles("chrome", &["Default"])];
    for (said, host) in [
        ("localhost", "localhost"),
        ("intranet", "intranet"),
        ("https://intranet/", "intranet"),
        ("localhost:3000", "localhost"),
        ("https://intranet:8080/", "intranet"),
        ("intranet.", "intranet"),
        ("ana@intranet", "intranet"),
    ] {
        let rule = rule_from("host", said, aimed("chrome", None), false, &all).expect(said);
        assert_eq!(rule.scope, Scope::Host(host.to_owned()), "{said}");
    }
    let site = rule_from("site", "bbc.co.uk", aimed("chrome", None), false, &all).expect("a site");
    assert_eq!(site.scope, Scope::Site("bbc.co.uk".to_owned()));
}

/// Every refusal names what to fix; none of them writes a rule that would match nothing or
/// open nowhere.
#[test]
fn a_rule_from_the_form_is_refused_by_name() {
    use super::rule_from;
    let all = vec![with_profiles("chrome", &["Default"])];
    let chrome = || aimed("chrome", None);

    assert_eq!(
        rule_from("url", "   ", chrome(), false, &all),
        Err("ruleValueEmpty")
    );
    assert_eq!(
        rule_from("url", "not a url", chrome(), false, &all),
        Err("ruleValueNotUrl")
    );
    assert_eq!(
        rule_from("host", "just words", chrome(), false, &all),
        Err("ruleValueNotHost")
    );
    assert_eq!(
        rule_from("host", "*.example.com", chrome(), false, &all),
        Err("ruleValueNotHost")
    );
    assert_eq!(
        rule_from("site", "co.uk", chrome(), false, &all),
        Err("ruleValueIsSuffix")
    );
    for said in [
        "github.com/login?return_to=https://gitlab.com/",
        "example.com/redirect/https://foo",
    ] {
        let rule = rule_from("host", said, chrome(), false, &all).expect(said);
        assert_ne!(rule.scope, Scope::Host("gitlab.com".to_owned()), "{said}");
        assert_ne!(rule.scope, Scope::Host("foo".to_owned()), "{said}");
    }
    for said in [r"a\b.com", r"intranet\reports"] {
        assert_eq!(
            rule_from("host", said, chrome(), false, &all),
            Err("ruleValueNotHost"),
            "{said}"
        );
    }
    for said in ["np", "github", "intranet"] {
        assert_eq!(
            rule_from("site", said, chrome(), false, &all),
            Err("ruleValueNotSite"),
            "{said}"
        );
    }
    assert_eq!(
        rule_from("site", "https://github.io/", chrome(), false, &all),
        Err("ruleValueIsSuffix")
    );
    assert_eq!(
        rule_from("site", "a/b.com", chrome(), false, &all),
        Err("ruleValueNotHost")
    );
    assert_eq!(
        rule_from("when", "x.com", chrome(), false, &all),
        Err("ruleKindUnknown")
    );
    assert_eq!(
        rule_from("site", "x.com", aimed("edge", None), false, &all),
        Err("browserGone")
    );
    assert_eq!(
        rule_from("site", "x.com", aimed("chrome", Some("Work")), false, &all),
        Err("browserGone"),
        "a profile the browser does not have is a destination that is gone"
    );
}

/// A copy that lands at the end of a long list reads as a browser somebody else added, and
/// the person who just pressed «duplicate» goes looking for it.
#[test]
fn a_copy_lands_next_to_what_it_was_copied_from() {
    let all = vec![detected("a"), detected("b"), detected("c")];

    let done = duplicated_in(all.clone(), "b", "{} (copy)").expect("b is there");
    assert_eq!(
        done.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "custom-1", "c"]
    );

    let done = duplicated_in(all.clone(), "c", "{} (copy)").expect("c is there");
    assert_eq!(done.last().expect("a copy").id, "custom-1");
    assert_eq!(done.len(), 4);

    assert_eq!(
        duplicated_in(all, "nothing-like-it", "{} (copy)").unwrap_err(),
        "browserGone"
    );
}

/// The bar reads out of this, and an installer whose size the server never declared is the
/// ordinary case rather than a fault.
#[test]
fn the_bar_reads_what_has_arrived_against_what_was_promised() {
    assert_eq!(far_along(0, Some(1_000)), 0);
    assert_eq!(far_along(500, Some(1_000)), 50);
    assert_eq!(far_along(1_000, Some(1_000)), 100);
    assert_eq!(
        far_along(400, None),
        0,
        "a size nobody declared is nought, not everything"
    );
    assert_eq!(
        far_along(400, Some(0)),
        100,
        "a size declared as zero neither divides by it nor overflows the bar"
    );
    assert_eq!(
        far_along(1_500, Some(1_000)),
        100,
        "more than was promised is still a full bar"
    );
}
/// The form hands over whatever was typed, and an empty box arrives as an empty string rather
/// than as nothing. Kept as it comes, the browser claims a private window it cannot open and
/// an icon that is not there.
#[test]
fn a_box_left_empty_is_nothing_rather_than_an_empty_value() {
    let all = added(
        Vec::new(),
        Edit {
            name: "Roto".to_owned(),
            exe: r"C:\Apps\roto.exe".to_owned(),
            args: String::new(),
            private_flag: Some(String::new()),
            icon_path: Some(String::new()),
            keys: [(String::new(), " ".to_owned())].into(),
        },
        |_| true,
    )
    .expect("the path is there");

    assert_eq!(all[0].private_flag, None);
    assert_eq!(all[0].icon_path, None);
    assert!(
        all[0].keys.is_empty(),
        "an empty key box leaves the row to the defaults"
    );
    assert!(all[0].custom, "one somebody added is theirs to edit");
    assert!(!all[0].hidden);
}

/// A browser whose path names nothing is a row that fails only when somebody clicks it, by
/// which time they are looking at a link that went nowhere.
#[test]
fn a_path_with_no_program_at_it_is_refused_before_it_is_saved() {
    let said = added(
        Vec::new(),
        Edit {
            name: "Roto".to_owned(),
            exe: r"C:\nope.exe".to_owned(),
            args: String::new(),
            private_flag: None,
            icon_path: None,
            keys: Default::default(),
        },
        |_| false,
    );

    assert_eq!(said.unwrap_err(), "browserNoProgram");
}

/// What is typed is a command line, so the quoted parts stay together.
#[test]
fn the_arguments_are_read_as_a_command_line() {
    let all = added(
        Vec::new(),
        Edit {
            name: "Chrome".to_owned(),
            exe: r"C:\Apps\chrome.exe".to_owned(),
            args: r#"--new-window --user-data-dir="C:\My Data""#.to_owned(),
            private_flag: Some("--incognito".to_owned()),
            icon_path: None,
            keys: Default::default(),
        },
        |_| true,
    )
    .expect("the path is there");

    assert_eq!(
        all[0].extra_args,
        vec![
            "--new-window".to_owned(),
            r"--user-data-dir=C:\My Data".to_owned()
        ]
    );
    assert_eq!(all[0].private_flag.as_deref(), Some("--incognito"));
}

/// A directory is not a program, and the form accepts one as readily as a path.
#[test]
fn only_a_file_counts_as_a_program_to_run() {
    let dir = std::env::temp_dir().join("lu-readable");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a place to write");
    let file = dir.join("browser.exe");
    std::fs::write(&file, b"not really a program").expect("written");

    assert!(readable(&file.to_string_lossy()));
    assert!(
        !readable(&dir.to_string_lossy()),
        "a directory is not a program"
    );
    assert!(!readable(&dir.join("gone.exe").to_string_lossy()));

    let bundle = dir.join("Browser.app");
    std::fs::create_dir_all(bundle.join("Contents")).expect("a bundle");
    assert!(
        !readable(&bundle.to_string_lossy()),
        "a folder called .app with nothing inside is not an application"
    );
    std::fs::write(bundle.join("Contents").join("Info.plist"), b"<plist/>").expect("a plist");
    assert_eq!(
        readable(&bundle.to_string_lossy()),
        cfg!(target_os = "macos"),
        "a bundle is a program on a Mac and a folder anywhere else"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

fn detected(id: &str) -> linkunbound_core::Browser {
    linkunbound_core::Browser {
        id: id.to_owned(),
        name: "Google Chrome".to_owned(),
        exe: "chrome.exe".to_owned(),
        profiles: vec![Profile {
            id: "Default".to_owned(),
            name: "Personal".to_owned(),
            args: Vec::new(),
        }],
        extra_args: Vec::new(),
        private_flag: Some("--incognito".to_owned()),
        icon_path: None,
        custom: false,
        hidden: false,
        keys: Default::default(),
    }
}

fn rule_for(browser: &str, profile: Option<&str>) -> Rule {
    Rule {
        id: "r1".to_owned(),
        scope: Scope::Site("github.com".to_owned()),
        source_app: None,
        target: Target {
            browser_id: browser.to_owned(),
            profile_id: profile.map(str::to_owned),
        },
        private: false,
    }
}

#[test]
fn a_rule_pointing_at_a_browser_that_is_gone_is_not_called_resolved() {
    let installed = [detected("chrome")];

    let live = describe(&rule_for("chrome", Some("Default")), &installed);
    assert!(live.resolved);
    assert_eq!(live.browser, "Google Chrome");
    assert_eq!(live.profile.as_deref(), Some("Personal"));

    let gone = describe(&rule_for("brave", None), &installed);
    assert!(
        !gone.resolved,
        "an uninstalled browser must not read as live"
    );
    assert_eq!(gone.browser, "brave", "and it still names what it was");
}

#[test]
fn a_rule_carries_the_destination_it_was_saved_with() {
    let view = describe(&rule_for("chrome", Some("Default")), &[detected("chrome")]);
    assert_eq!(view.browser_id, "chrome");
    assert_eq!(view.profile_id.as_deref(), Some("Default"));
}

#[test]
fn a_detected_browser_is_not_reported_as_one_the_user_added() {
    let view = seen(&detected("chrome"), Default::default());
    assert!(!view.custom);
    assert_eq!(view.profiles, 1);
    assert_eq!(
        view.profile_names,
        vec![("Default".to_owned(), "Personal".to_owned())],
        "named, so a rule can be pointed at one profile"
    );
    assert!(view.private, "it declares a private switch");

    let mine = seen(&custom("custom-1"), Default::default());
    assert!(mine.custom);
    assert!(!mine.private);
}

/// `Locale::System` must mean the same in both windows, so it is decided
/// here rather than in each webview.
#[test]
fn the_language_handed_to_the_screen_obeys_the_explicit_choice() {
    let spanish = Preferences {
        locale: Locale::Spanish,
        ..Preferences::default()
    };
    let english = Preferences {
        locale: Locale::English,
        ..Preferences::default()
    };
    assert_eq!(spoken(&spanish), "es");
    assert_eq!(spoken(&english), "en");
}

pub(crate) fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("linkunbound-cmd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create");
    dir
}

#[test]
fn preference_patches_keep_other_choices_and_distinguish_a_cleared_shortcut() {
    let dir = scratch("preferences-patch");
    let store = Store::at(&dir);
    store
        .edit_prefs(|kept| {
            kept.welcomed = true;
            kept.shortcut = Some("Alt+F9".to_owned());
            true
        })
        .unwrap();

    let patch = |json| serde_json::from_str(json).unwrap();
    assert!(
        !super::write_preferences(&store, patch(r#"{"domain_lists_refresh_hours":1}"#)).unwrap()
    );
    assert!(
        super::write_preferences(&store, patch(r#"{"domain_lists_auto_refresh":true}"#)).unwrap()
    );
    assert_eq!(store.prefs().shortcut.as_deref(), Some("Alt+F9"));

    assert!(!super::write_preferences(&store, patch(r#"{"shortcut":null}"#)).unwrap());
    let saved = store.prefs();
    assert!(saved.domain_lists_auto_refresh);
    assert_eq!(saved.domain_lists_refresh_hours, 6);
    assert!(saved.shortcut.is_none());
    assert!(saved.welcomed);
}

/// `SCHEMA_VERSION` exists so a newer file is recognised rather than parsed.
/// Writing over it anyway erases the fields this version cannot read.
#[test]
fn a_file_this_version_cannot_read_is_not_written_over() {
    let dir = scratch("unreadable");
    let store = Store::at(&dir);
    std::fs::write(
        dir.join("browsers.json"),
        r#"{"schema_version": 99, "browsers": []}"#,
    )
    .expect("should write");

    let before = std::fs::read(dir.join("browsers.json")).expect("should read");
    assert!(keep_in(&store, vec![custom("custom-1")]).is_err());
    assert_eq!(
        std::fs::read(dir.join("browsers.json")).expect("should read"),
        before,
        "the file must be left exactly as it was"
    );
}

/// Forgetting every detected entry would take the name, arguments, icon and
/// place the person had given each one. Only what was hidden comes back.
#[test]
fn a_rescan_shows_what_was_hidden_and_keeps_everything_else_the_person_set() {
    let mut chrome = detected("chrome");
    chrome.name = "Work".to_owned();
    chrome.extra_args = vec!["--new-window".to_owned()];
    chrome.hidden = true;
    let saved = vec![
        detected("firefox"),
        chrome,
        custom("custom-1"),
        detected("gone"),
    ];

    let found = rescanned(
        vec![detected("chrome"), detected("firefox"), detected("brave")],
        &saved,
    );

    let ids: Vec<&str> = found.browsers.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(
        ids,
        ["firefox", "chrome", "custom-1", "brave"],
        "order is kept"
    );
    let chrome = &found.browsers[1];
    assert_eq!(chrome.name, "Work");
    assert_eq!(chrome.extra_args, ["--new-window"]);
    assert!(!chrome.hidden);
    assert_eq!((found.added, found.removed), (1, 1));

    let first = rescanned(vec![detected("chrome"), detected("firefox")], &[]);
    assert_eq!(
        (first.added, first.removed),
        (0, 0),
        "nothing saved, nothing new"
    );
    let only_mine = rescanned(vec![detected("chrome")], &[custom("custom-1")]);
    assert_eq!(
        only_mine.added, 0,
        "only my own browsers saved, nothing detected"
    );

    let mut typed = detected("chrome");
    typed.private_flag = Some("--private".to_owned());
    let fixed = rescanned(vec![detected("chrome")], &[typed]);
    assert_eq!(
        fixed.browsers[0].private_flag.as_deref(),
        Some("--incognito")
    );

    let mut unknown = detected("comet");
    unknown.private_flag = None;
    let mut mine = detected("comet");
    mine.private_flag = Some("--incognito".to_owned());
    let kept = rescanned(vec![unknown], &[mine]);
    assert_eq!(
        kept.browsers[0].private_flag.as_deref(),
        Some("--incognito"),
        "what detection does not know stays as the person typed it"
    );
}

fn edit_of(name: &str, exe: &str) -> Edit {
    Edit {
        name: name.to_owned(),
        exe: exe.to_owned(),
        args: "--new-window".to_owned(),
        private_flag: Some(String::new()),
        icon_path: None,
        keys: Default::default(),
    }
}

/// Detection owns the path of what it found: a rescan would put it back, so
/// letting it be typed over promises an edit that does not survive.
#[test]
fn only_a_browser_the_user_added_may_move_its_path() {
    let all = vec![detected("chrome"), custom("custom-1")];

    let touched = edited(
        all.clone(),
        "chrome",
        edit_of("Trabajo", "moved.exe"),
        |_| true,
    )
    .expect("a detected browser may still be renamed");
    assert_eq!(touched[0].name, "Trabajo");
    assert_eq!(touched[0].exe, "chrome.exe", "but not moved");
    assert_eq!(
        touched[0].private_flag.as_deref(),
        Some("--incognito"),
        "nor its private flag: detection knows how it opens privately, and the form does \
         not offer the field"
    );

    let odd = vec![linkunbound_core::Browser {
        exe: "C:/Orion/orion.exe".to_owned(),
        private_flag: None,
        ..detected("orion")
    }];
    let told = edited(odd, "orion", edit_of("Orion", "moved.exe"), |_| true)
        .expect("a detected browser of a family detection does not know");
    assert_eq!(
        told[0].private_flag, None,
        "the empty switch is honoured, since the flag was the person's to say"
    );

    let mine = edited(all, "custom-1", edit_of("Mío", "other.exe"), |_| true)
        .expect("one the user added may move");
    assert_eq!(mine[1].exe, "other.exe");
    assert_eq!(
        mine[1].private_flag, None,
        "an empty switch on a browser the user added clears it rather than saving a blank"
    );
}

#[test]
fn an_edit_pointing_nowhere_is_refused_before_it_is_saved() {
    let all = vec![custom("custom-1")];
    assert!(
        edited(
            all.clone(),
            "custom-1",
            edit_of("Mío", "ghost.exe"),
            |_| false
        )
        .is_err()
    );
    assert!(edited(all, "gone", edit_of("Mío", "x.exe"), |_| true).is_err());
}

/// Removing a detected browser would last until the next scan, so the screen
/// offers hiding instead and the command refuses.
#[test]
fn a_detected_browser_cannot_be_removed_only_hidden() {
    let all = vec![detected("chrome"), custom("custom-1")];

    assert!(without(all.clone(), "chrome").is_err());

    let hidden = hidden_in(all.clone(), "chrome", true).expect("hiding is allowed");
    assert!(hidden[0].hidden);
    let shown = hidden_in(hidden, "chrome", false).expect("and so is showing it again");
    assert!(!shown[0].hidden, "the switch has to work both ways");

    let gone = without(all, "custom-1").expect("one the user added may go");
    assert_eq!(gone.len(), 1);
    assert_eq!(gone[0].id, "chrome");
}

#[test]
fn hiding_something_that_is_not_there_says_so() {
    assert!(hidden_in(vec![detected("chrome")], "brave", true).is_err());
}

/// The order decides which browser sits under the first key of the picker.
/// Anything the screen did not know about keeps its place at the end.
#[test]
fn reordering_keeps_what_the_caller_never_named() {
    let all = vec![detected("chrome"), custom("custom-1"), detected("firefox")];
    let asked = vec!["custom-1".to_owned(), "chrome".to_owned()];

    let moved = ordered(all, &asked);
    let ids: Vec<&str> = moved.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(
        ids,
        ["custom-1", "chrome", "firefox"],
        "the unnamed one survives at the end rather than falling off"
    );
}

fn custom(id: &str) -> linkunbound_core::Browser {
    linkunbound_core::Browser {
        id: id.to_owned(),
        name: id.to_owned(),
        exe: "x.exe".to_owned(),
        profiles: Vec::new(),
        extra_args: Vec::new(),
        private_flag: None,
        icon_path: None,
        custom: true,
        hidden: false,
        keys: Default::default(),
    }
}

/// Reusing a number a live entry holds would make an edit rename the other.
#[test]
fn a_new_entry_never_takes_an_id_another_one_already_holds() {
    let mut all = vec![custom("custom-1"), custom("custom-3")];
    assert_eq!(free_id(&all), "custom-2");
    all.push(custom("custom-2"));
    assert_eq!(free_id(&all), "custom-4");
}

#[test]
fn the_first_added_browser_starts_the_numbering() {
    assert_eq!(free_id(&[]), "custom-1");
}
