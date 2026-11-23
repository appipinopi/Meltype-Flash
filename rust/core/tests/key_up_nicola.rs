// SPDX-License-Identifier: GPL-3.0-or-later
use meltype_core::Session;
use serde_json::json;

fn nicola() -> Session {
    let mut session = Session::default();
    session.settings.set("InputStyle", json!("Kana")).unwrap();
    session.settings.set("KanaLayout", json!("Nicola")).unwrap();
    session
        .settings
        .set("LiveConversion", json!(false))
        .unwrap();
    session
}

#[test]
fn thumb_pressed_before_character_release_restrokes() {
    let mut session = nicola();
    session.key(0x51, 'q' as i32, 0);
    assert_eq!(session.display(false), "。");
    // The character key is still held, so the thumb upgrades it in place.
    session.key(0x1D, 0, 0);
    assert_eq!(session.display(false), "ぁ");
    session.key_up(0x1D, 0, 0);
    session.key_up(0x51, 'q' as i32, 0);
}

#[test]
fn thumb_after_character_release_does_not_restroke() {
    let mut session = nicola();
    session.key(0x51, 'q' as i32, 0);
    session.key_up(0x51, 'q' as i32, 0);
    session.key(0x1D, 0, 0);
    assert_eq!(session.display(false), "。");
    session.key_up(0x1D, 0, 0);
}

#[test]
fn thumb_tap_starts_conversion() {
    let mut session = nicola();
    session.key(0x51, 'q' as i32, 0);
    session.key(0x1C, 0, 0);
    session.key_up(0x1C, 0, 0);
    assert!(session.is_converting());
}

#[test]
fn replay_preserves_order_based_chording() {
    let mut session = nicola();
    session.key(0x51, 'q' as i32, 0);
    session.key(0x1D, 0, 0);
    session.key_up(0x1D, 0, 0);
    session.key_up(0x51, 'q' as i32, 0);
    let expected = session.display(false);
    session.replay_key_events();
    assert_eq!(session.display(false), expected);
}
