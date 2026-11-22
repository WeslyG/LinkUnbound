use linkunbound_shell::hotkey::Hotkey;
use std::process::{Command, ExitCode};

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
    let mut hotkey = Hotkey::new().expect("the OS must provide a global hotkey manager");
    if let Some(shortcut) = std::env::args().nth(1) {
        return ExitCode::from(u8::from(hotkey.claim(Some(&shortcut)).is_none()));
    }

    let first = "Alt+Shift+L";
    let second = "Ctrl+Alt+Shift+F10";
    assert_eq!(hotkey.claim(Some(first)).as_deref(), Some(first));
    claim_in_other_process(first, 1);
    assert_eq!(hotkey.claim(Some(second)).as_deref(), Some(second));
    claim_in_other_process(first, 0);
    drop(hotkey);
    claim_in_other_process(second, 0);
    ExitCode::SUCCESS
}
