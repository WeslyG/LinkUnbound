use linkunbound_shell::hotkey::Hotkey;
use std::cell::Cell;
use std::process::{Command, ExitCode};
use std::rc::Rc;
use std::time::Duration;

fn claim_in_other_process(shortcut: &str, expected: i32) {
    // Carbon installs one event handler per application. A second process tests
    // the actual system-wide conflict without trying to install that handler twice.
    let status = Command::new(std::env::current_exe().expect("locate the smoke test"))
        .arg(shortcut)
        .status()
        .expect("start the competing hotkey owner");
    assert_eq!(status.code(), Some(expected), "claiming {shortcut}");
}

fn main() -> ExitCode {
    let result = Rc::new(Cell::new(0));
    let exit_code = Rc::clone(&result);
    slint::Timer::single_shot(Duration::ZERO, move || {
        let mut hotkey = Hotkey::new().expect("the OS must provide a global hotkey manager");
        if let Some(shortcut) = std::env::args().nth(1) {
            exit_code.set(u8::from(hotkey.claim(Some(&shortcut)).is_none()));
        } else {
            let first = "Alt+Shift+L";
            let second = "Ctrl+Alt+Shift+F10";

            assert_eq!(hotkey.claim(Some(first)).as_deref(), Some(first));
            claim_in_other_process(first, 1);
            assert_eq!(hotkey.claim(Some(second)).as_deref(), Some(second));
            claim_in_other_process(first, 0);
            drop(hotkey);
            claim_in_other_process(second, 0);
        }
        slint::quit_event_loop().expect("the smoke test must stop the event loop");
    });
    slint::run_event_loop_until_quit().expect("run the native event loop");
    ExitCode::from(result.get())
}
