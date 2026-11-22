use linkunbound_shell::hotkey::Hotkey;
use std::time::Duration;

fn main() {
    slint::Timer::single_shot(Duration::ZERO, || {
        let mut hotkey = Hotkey::new().expect("the OS must provide a global hotkey manager");
        let mut other = Hotkey::new().expect("the OS must provide a second hotkey manager");
        let first = "Alt+Shift+L";
        let second = "Ctrl+Alt+Shift+F10";

        assert_eq!(hotkey.claim(Some(first)).as_deref(), Some(first));
        assert_eq!(other.claim(Some(first)), None, "the first key is held");
        assert_eq!(hotkey.claim(Some(second)).as_deref(), Some(second));
        assert_eq!(other.claim(Some(first)).as_deref(), Some(first));

        drop(hotkey);
        assert_eq!(other.claim(Some(second)).as_deref(), Some(second));
        slint::quit_event_loop().expect("the smoke test must stop the event loop");
    });
    slint::run_event_loop_until_quit().expect("run the native event loop");
}
