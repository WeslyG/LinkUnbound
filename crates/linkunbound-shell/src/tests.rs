use super::{
    Arrival, FRONT_SETTLES_WITHIN, INPUT_SETTLES_WITHIN, Listed, Shown, UI, Ui, WAITING_ROOM,
    claims_the_window, host, link_from, physical, present, show_queued, still_settling, too_soon,
    wants_no_window, with_icons,
};
use linkunbound_core::{Language, normalise};
use linkunbound_shell::{Notice, Picker, Reaches};
use slint::ComponentHandle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[cfg(target_os = "macos")]
mod launch_services {
    use super::super::{Reaction, host, reaction_to};
    use linkunbound_mac::Event;

    #[test]
    fn a_link_and_a_web_document_are_opened_and_anything_else_is_not() {
        assert_eq!(
            reaction_to(Event::Link("https://a.test/x".to_owned()), false),
            Reaction::Open("https://a.test/x".to_owned())
        );
        let dir = std::env::temp_dir().join(format!("linkunbound-react-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let page = dir.join("page.html");
        std::fs::write(&page, "<p>x</p>").expect("a page");
        let notes = dir.join("notes.txt");
        std::fs::write(&notes, "x").expect("a note");
        assert!(matches!(
            reaction_to(Event::Document(page.clone()), true),
            Reaction::Open(url) if url.starts_with("file://") && url.ends_with("/page.html")
        ));
        assert_eq!(
            reaction_to(Event::Document(notes), false),
            Reaction::Nothing
        );
        assert_eq!(
            reaction_to(Event::Document(dir.join("missing.html")), false),
            Reaction::Nothing
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_login_item_says_nothing_and_a_launch_by_hand_shows_the_settings() {
        let by_hand = Event::Launched {
            as_login_item: false,
        };
        let at_login = Event::Launched {
            as_login_item: true,
        };
        assert_eq!(reaction_to(by_hand, false), Reaction::Settings);
        assert_eq!(
            reaction_to(
                Event::Launched {
                    as_login_item: false
                },
                true
            ),
            Reaction::Nothing,
            "told to keep quiet"
        );
        assert_eq!(reaction_to(at_login, false), Reaction::Nothing);
        assert_eq!(reaction_to(Event::Reopened, true), Reaction::Settings);
    }

    /// The Mac reads the key rather than the glyph, so with no key event in flight only a
    /// plain digit maps; a shifted one is left to the key code.
    #[test]
    fn a_digit_typed_without_a_key_event_is_still_a_digit() {
        for (typed, digit) in [
            ('1', Some(1)),
            ('9', Some(9)),
            ('0', None),
            ('a', None),
            ('!', None),
        ] {
            assert_eq!(host::digit_behind(typed), digit, "{typed}");
        }
    }
}

pub(crate) fn headless_ui() -> Rc<Ui> {
    i_slint_backend_testing::init_no_event_loop();
    let ui = Rc::new(Ui {
        picker: Picker::new().expect("a window"),
        notice: Notice::new().expect("a notice"),
        mascot: super::mascot::Mascot::new().expect("a mascot"),
        shown: Rc::new(RefCell::new(Shown::default())),
        words: Cell::new(Language::English.strings()),
        firing: RefCell::new(None),
        tray: None,
        watch: slint::Timer::default(),
        countdown: slint::Timer::default(),
        progress_watch: slint::Timer::default(),
        looker: slint::Timer::default(),
        put_away: RefCell::new(None),
        prefs_seen: Cell::new(None),
        held_focus: Cell::new(false),
        launched_at: Cell::new(None),
        light_seen: Cell::new(false),
        taskbar_seen: Cell::new(false),
        shown_over: Cell::new(None),
        put_up: Cell::new(None),
        appeared: Cell::new(None),
        hotkey: RefCell::new(None),
        #[cfg(target_os = "macos")]
        launch_decided: Cell::new(false),
    });
    UI.with_borrow_mut(|slot| *slot = Some(Rc::clone(&ui)));
    ui
}

#[test]
fn the_file_moving_is_what_says_settings_changed_something() {
    let ui = headless_ui();
    let first = std::time::SystemTime::UNIX_EPOCH;
    let later = first + std::time::Duration::from_secs(1);

    assert!(
        ui.moved_since(Some(first)),
        "nothing seen yet, so this is new"
    );
    assert!(
        !ui.moved_since(Some(first)),
        "the same file is not a change"
    );
    assert!(
        ui.moved_since(Some(later)),
        "a shortcut changed with the window open has to reach the resident"
    );
    assert!(!ui.moved_since(Some(later)));
    assert!(
        ui.moved_since(None),
        "a file that cannot be read is not the one already seen"
    );
}

#[test]
fn a_link_over_a_picker_nobody_answers_takes_its_place_and_the_queue_follows() {
    let ui = headless_ui();
    let words = ui.words.get();

    assert!(present(
        &ui.picker,
        &words,
        &ui.shown,
        arrive("https://one.test/")
    ));
    ui.watch_focus();
    assert!(ui.picker.window().is_visible());
    assert_eq!(ui.shown.borrow().url.as_deref(), Some("https://one.test/"));
    assert_eq!(ui.put_up.get().is_some(), host::takes_the_front_back());

    assert!(present(
        &ui.picker,
        &words,
        &ui.shown,
        arrive("https://two.test/")
    ));
    assert_eq!(ui.shown.borrow().url.as_deref(), Some("https://two.test/"));
    assert!(ui.shown.borrow().waiting.is_empty());

    super::look::put_away(&ui.picker);
    ui.shown
        .borrow_mut()
        .waiting
        .push_back(arrive("https://three.test/"));
    show_queued(&ui.picker, &words, &ui.shown, |url, _| {
        assert_eq!(url, "https://three.test/");
        None
    });
    assert!(ui.picker.window().is_visible());
    assert_eq!(
        ui.shown.borrow().url.as_deref(),
        Some("https://three.test/")
    );
    assert!(ui.shown.borrow().waiting.is_empty());
}

#[test]
fn a_choice_in_the_instant_the_picker_appears_is_not_taken() {
    let now = std::time::Instant::now();
    assert!(too_soon(Some(now)));
    assert!(!too_soon(None));
    let earlier = now.checked_sub(INPUT_SETTLES_WITHIN * 2);
    assert!(!too_soon(earlier));
    assert!(INPUT_SETTLES_WITHIN < FRONT_SETTLES_WITHIN);
}

#[test]
fn the_front_is_taken_back_only_in_the_first_moment() {
    let now = std::time::Instant::now();
    assert!(still_settling(Some(now)));
    let stale = now
        .checked_sub(FRONT_SETTLES_WITHIN * 2)
        .expect("uptime beyond a second");
    assert!(!still_settling(Some(stale)), "the moment has passed");
    assert!(
        !still_settling(None),
        "never put up, or a system that never takes it back"
    );
}

/// The screen is measured in physical pixels and the window is described in logical ones, so
/// on a 150% display the untouched number asks for two thirds of the room it needs and the
/// picker opens clipped.
#[test]
fn a_size_is_asked_for_in_the_pixels_the_screen_is_measured_in() {
    assert_eq!(physical(364.0, 1.0), 364);
    assert_eq!(physical(364.0, 1.5), 546);
    assert_eq!(physical(364.0, 2.0), 728);
    assert_eq!(physical(100.4, 1.0), 100, "rounded, never truncated");
    assert_eq!(physical(100.5, 1.0), 101);
}

/// Each row carries the icon of the browser it opens. Paired by anything else, every row
/// shows somebody else's icon and the picker becomes a guess.
#[test]
fn every_row_carries_the_icon_of_its_own_browser() {
    let browsers = vec![
        browser_named("chrome"),
        browser_named("firefox"),
        browser_named("edge"),
    ];
    let listed: Vec<Listed> = browsers.iter().rev().map(|b| row_for(&b.id)).collect();

    let dressed = with_icons(listed, &browsers, |b| Some(format!("{}.png", b.id)));

    for row in &dressed {
        assert_eq!(
            row.icon.as_deref(),
            Some(format!("{}.png", row.browser_id).as_str()),
            "{} was handed another browser's icon",
            row.browser_id
        );
    }
}

/// A row naming a browser that is no longer installed keeps whatever it had rather than
/// taking the first icon that comes along.
#[test]
fn a_row_with_no_browser_behind_it_is_left_as_it_was() {
    let dressed = with_icons(vec![row_for("gone")], &[browser_named("chrome")], |_| {
        Some("chrome.png".to_owned())
    });

    assert_eq!(dressed[0].icon, None);
}

fn row_for(browser_id: &str) -> Listed {
    Listed {
        browser_id: browser_id.to_owned(),
        profile_id: None,
        name: browser_id.to_owned(),
        profile: String::new(),
        icon: None,
        can_private: false,
        key: None,
    }
}
fn browser_named(id: &str) -> linkunbound_core::Browser {
    linkunbound_core::Browser {
        id: id.to_owned(),
        name: id.to_owned(),
        exe: "x.exe".to_owned(),
        profiles: Vec::new(),
        extra_args: Vec::new(),
        private_flag: None,
        icon_path: None,
        custom: false,
        hidden: false,
        keys: Default::default(),
    }
}

fn args(rest: &[&str]) -> Vec<String> {
    std::iter::once("linkunbound.exe")
        .chain(rest.iter().copied())
        .map(str::to_owned)
        .collect()
}

/// The socket accepts a line from any process of this user, so what it
/// carries is as untrusted as a command line and has to cross the same
/// guard. A `file://` reaching a rule would fetch a UNC path and hand over
/// the NTLM hash without a click.
#[test]
fn what_arrives_over_the_socket_meets_the_same_guard_as_the_command_line() {
    for hostile in [
        "file://evil.test/share/payload",
        "--gpu-launcher=calc.exe",
        "javascript:alert(1)",
        "linkunbound://open?url=file%3A%2F%2Fevil.test%2Fshare%2Fpayload",
        "",
    ] {
        assert!(
            normalise(hostile).is_none(),
            "{hostile} must not survive the guard"
        );
    }
    assert_eq!(
        normalise("https://github.com/a").as_deref(),
        Some("https://github.com/a")
    );
}

/// A link arriving over a picker that never got the front, or lost it to the click that sent
/// the link, takes its place: the person clicked past a window that was not answering, and
/// the same link clicked again is that person trying once more, not a program retrying.
#[test]
fn a_link_arriving_over_a_picker_nobody_is_answering_takes_its_place() {
    let mut shown = Shown {
        url: Some("https://behind.test/".to_owned()),
        ..Shown::default()
    };
    let next = claims_the_window(&mut shown, arrive("https://next.test/"), false);
    assert_eq!(next.map(|a| a.url).as_deref(), Some("https://next.test/"));
    assert!(
        shown.waiting.is_empty(),
        "nothing queues behind an unanswered picker"
    );

    let again = claims_the_window(&mut shown, arrive("https://behind.test/"), false);
    assert_eq!(
        again.map(|a| a.url).as_deref(),
        Some("https://behind.test/")
    );
}

/// A link arriving while one is on screen and attended must not redress the
/// window: the user aimed at what they could see. And it must not be dropped
/// either — that click already happened.
#[test]
fn a_link_arriving_over_a_shown_one_waits_instead_of_replacing_it() {
    let mut shown = Shown::default();

    let first = claims_the_window(&mut shown, arrive("https://first.test/a"), false);
    assert_eq!(
        first.map(|a| a.url).as_deref(),
        Some("https://first.test/a"),
        "with no window up, the link is dressed straight away"
    );
    assert!(shown.waiting.is_empty());

    let second = claims_the_window(&mut shown, arrive("https://second.test/b"), true);
    assert!(second.is_none(), "it must not take a window already in use");
    let third = claims_the_window(&mut shown, arrive("https://third.test/c"), true);
    assert!(third.is_none());

    assert_eq!(
        shown.waiting.pop_front().map(|a| a.url).as_deref(),
        Some("https://second.test/b"),
        "queued links are answered in the order they were clicked"
    );
    assert_eq!(shown.waiting.len(), 1, "and none of them is dropped");
}

/// One click retried is one click: a program that re-sends the link every second while the
/// picker is up would otherwise queue it a hundred times, and Esc would show each one.
#[test]
fn the_same_link_waits_once_and_the_queue_has_a_ceiling() {
    let mut shown = Shown {
        url: Some("https://shown.test/".to_owned()),
        ..Shown::default()
    };
    for _ in 0..5 {
        assert!(claims_the_window(&mut shown, arrive("https://again.test/"), true).is_none());
    }
    assert_eq!(shown.waiting.len(), 1);
    assert!(claims_the_window(&mut shown, arrive("https://shown.test/"), true).is_none());
    assert_eq!(
        shown.waiting.len(),
        1,
        "the link on screen is not queued behind itself"
    );

    for n in 0..(WAITING_ROOM * 2) {
        let _ = claims_the_window(&mut shown, arrive(&format!("https://many.test/{n}")), true);
    }
    assert_eq!(shown.waiting.len(), WAITING_ROOM);
    assert!(
        !shown.waiting.iter().any(|a| a.url == "https://again.test/"),
        "the oldest is what goes"
    );
}

fn arrive(url: &str) -> Arrival {
    Arrival {
        url: url.to_owned(),
        source: None,
    }
}

/// Read when it finally reaches the screen, a queued link's origin was whatever the previous
/// answer had just opened, and "always from this app" bound the rule to the browser.
#[test]
fn a_queued_link_keeps_the_app_it_came_from() {
    let mut shown = Shown {
        url: Some("https://shown.test/".to_owned()),
        ..Shown::default()
    };
    let from_slack = Arrival {
        url: "https://queued.test/".to_owned(),
        source: Some(linkunbound_core::Origin::named("slack")),
    };
    assert!(claims_the_window(&mut shown, from_slack.clone(), true).is_none());
    assert_eq!(shown.waiting.pop_front(), Some(from_slack));
}

#[test]
fn a_teams_link_arrives_already_unwrapped() {
    let raw = "microsoft-edge:https://eu01.safelinks.protection.outlook.com/?url=https%3A%2F%2Fgithub.com%2Fa";
    assert_eq!(
        link_from(&args(&["--background", raw])).as_deref(),
        Some("https://github.com/a")
    );
}

#[test]
fn a_link_far_longer_than_four_kilobytes_survives() {
    let padding = "a".repeat(6000);
    let long = format!(
        "https://teams.microsoft.com/l/message/19:meeting@thread.v2?context={padding}&ce=prod"
    );
    assert!(long.len() > 6000);
    assert_eq!(link_from(&args(&[&long])).as_deref(), Some(long.as_str()));
}

#[test]
fn a_long_link_still_unwraps_out_of_its_wrapper() {
    let padding = "b".repeat(5000);
    let inner = format!("https%3A%2F%2Fgithub.com%2F{padding}");
    let wrapped = format!("https://eu01.safelinks.protection.outlook.com/?url={inner}&data=05");
    let out = link_from(&args(&[&wrapped])).expect("should unwrap");
    assert!(out.starts_with("https://github.com/"));
    assert!(out.len() > 5000);
}

#[test]
fn a_link_sent_through_our_own_scheme_arrives_unwrapped() {
    assert_eq!(
        link_from(&args(&[
            "linkunbound://open?url=https%3A%2F%2Fgithub.com%2Fa%3Fb%3D1"
        ]))
        .as_deref(),
        Some("https://github.com/a?b=1")
    );
}

#[test]
fn a_refused_own_scheme_link_is_not_mistaken_for_someone_opening_the_app() {
    for refused in [
        "linkunbound://open?url=javascript%3Aalert(1)",
        "linkunbound://x",
        "linkunbound://open?url=",
    ] {
        let said = args(&[refused]);
        let incoming = link_from(&said);
        assert!(incoming.is_none(), "«{refused}» was read");
        assert!(wants_no_window(&said, incoming.as_deref()), "«{refused}»");
    }
    assert!(!wants_no_window(&args(&[]), None));
    assert!(wants_no_window(&args(&["--hushed"]), None));
}

#[test]
fn switches_never_pass_for_a_link() {
    assert!(link_from(&args(&["--register"])).is_none());
    assert!(link_from(&args(&["--gpu-launcher=calc.exe"])).is_none());
}

#[test]
fn a_link_split_across_arguments_is_put_back_together() {
    let whole = "https://intranet.corp/x?f=activo urgente";
    assert_eq!(
        link_from(&args(&["https://intranet.corp/x?f=activo", "urgente"])).as_deref(),
        Some(whole)
    );
}

#[test]
fn remembering_the_whole_site_for_an_ip_never_captures_a_different_machine() {
    let scope = Reaches::Site
        .scope("https://192.168.1.50/admin", "192.168.1.50")
        .unwrap();
    assert!(scope.matches("https://192.168.1.50/admin", "192.168.1.50"));
    assert!(!scope.matches("https://10.0.1.50/admin", "10.0.1.50"));
}
