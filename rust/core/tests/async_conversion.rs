// SPDX-License-Identifier: GPL-3.0-or-later
use meltype_core::session::AsyncConversionPhase;
use meltype_core::Session;
use serde_json::json;

fn type_text(session: &mut Session, text: &str) {
    for c in text.chars() {
        session.key(0, c as i32, 0);
    }
}

#[test]
fn completed_request_applies_and_clears_state() {
    let mut session = Session::default();
    type_text(&mut session, "hashi");
    let id = session.begin_async_conversion("はし", "");
    assert_eq!(
        session.async_conversion().unwrap().phase,
        AsyncConversionPhase::Pending
    );
    assert!(session.complete_async_conversion(id, vec!["橋".into(), "箸".into()]));
    assert_eq!(
        session.async_conversion().unwrap().phase,
        AsyncConversionPhase::Ready
    );
    assert!(session.try_apply_async_conversion(
        "はし",
        vec!["橋".into(), "箸".into()]
    ));
    assert!(session.is_converting());
    assert!(session.async_conversion().is_none());
}

#[test]
fn stale_reading_is_rejected_without_replacing_newer_input() {
    let mut session = Session::default();
    type_text(&mut session, "hashi");
    let id = session.begin_async_conversion("はし", "");
    assert!(session.complete_async_conversion(id, vec!["橋".into()]));
    assert!(!session.try_apply_async_conversion("かき", vec!["柿".into()]));
    assert_eq!(
        session.async_conversion().unwrap().phase,
        AsyncConversionPhase::Ready
    );
}

#[test]
fn cancellation_and_timeout_never_apply_results() {
    let mut session = Session::default();
    type_text(&mut session, "hashi");
    let id = session.begin_async_conversion("はし", "");
    assert!(session.complete_async_conversion(id, vec!["橋".into()]));
    session.cancel_async_conversion();
    assert!(session.async_conversion().is_none());
    assert!(!session.try_apply_async_conversion("はし", vec!["橋".into()]));

    session
        .settings
        .set("ImeTimeoutMs", json!(50))
        .unwrap();
    type_text(&mut session, "kaki");
    let id = session.begin_async_conversion("かき", "");
    let requested = session.async_conversion().unwrap().requested_ms;
    assert_eq!(
        session.timeout_async_conversion(requested),
        Some(AsyncConversionPhase::Pending)
    );
    assert_eq!(
        session.timeout_async_conversion(requested + 50),
        Some(AsyncConversionPhase::TimedOut)
    );
    assert!(!session.complete_async_conversion(id, vec!["柿".into()]));
    assert!(!session.try_apply_async_conversion("かき", vec!["柿".into()]));
}
