//! Native Windows checks use disposable credentials and synthetic Bulgarian text.

#[test]
#[ignore = "requires the Windows user credential store"]
fn windows_credential_manager_round_trip() {
    let service = format!("app.aidoo.whisper-lite.ci.{}", uuid::Uuid::new_v4());
    let entry = keyring::Entry::new(&service, "disposable-test").unwrap();
    let dummy = "not-an-api-key-проверка";
    entry.set_password(dummy).unwrap();
    let read_back = entry.get_password();
    let deleted = entry.delete_credential();
    assert_eq!(read_back.unwrap(), dummy);
    deleted.unwrap();
    assert!(matches!(entry.get_password(), Err(keyring::Error::NoEntry)));
}

#[test]
#[ignore = "requires the CI foreground test field; changes the clipboard"]
fn windows_paste_into_foreground_test_field() {
    crate::text_insertion::copy("Проверка на диктовка: 123.").unwrap();
    // Dropping and reopening arboard must preserve the Windows clipboard.
    assert_eq!(
        arboard::Clipboard::new().unwrap().get_text().unwrap(),
        "Проверка на диктовка: 123."
    );
    crate::text_insertion::paste().unwrap();
}

#[test]
#[ignore = "requires the Windows desktop keyboard hook"]
fn windows_global_f8_receives_press_and_release() {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use std::{sync::mpsc, time::Duration};
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        rdev::listen(move |event| match event.event_type {
            rdev::EventType::KeyPress(rdev::Key::F8) => {
                let _ = sender.send(true);
            }
            rdev::EventType::KeyRelease(rdev::Key::F8) => {
                let _ = sender.send(false);
            }
            _ => {}
        })
        .unwrap();
    });
    std::thread::sleep(Duration::from_millis(500));
    let mut keyboard = Enigo::new(&Settings::default()).unwrap();
    keyboard.key(Key::F8, Direction::Press).unwrap();
    keyboard.key(Key::F8, Direction::Release).unwrap();
    assert!(receiver.recv_timeout(Duration::from_secs(5)).unwrap());
    assert!(!receiver.recv_timeout(Duration::from_secs(5)).unwrap());
}
