// SPDX-License-Identifier: GPL-3.0-or-later
use meltype_core::dictionary::Dictionary;
use std::collections::BTreeMap;

fn empty() -> (BTreeMap<String, bool>, BTreeMap<String, Vec<String>>) {
    (BTreeMap::new(), BTreeMap::new())
}

#[test]
fn kana_units_segment_japanese() {
    let (learned, user) = empty();
    let text = Dictionary::bundled()
        .composition_lattice_kana("hs", "はし", true, "", "", "Balanced", &learned, &user);
    assert_eq!(text, "はし");
}

#[test]
fn kana_units_keep_ascii_english_words() {
    let (learned, user) = empty();
    let text = Dictionary::bundled().composition_lattice_kana(
        "google", "google", true, "", "", "Balanced", &learned, &user,
    );
    assert_eq!(text, "google");
}

#[test]
fn kana_units_honor_learned_user_words() {
    let mut learned = BTreeMap::new();
    learned.insert("github".into(), true);
    let user = BTreeMap::new();
    let text = Dictionary::bundled().composition_lattice_kana(
        "github", "github", true, "", "", "Balanced", &learned, &user,
    );
    assert_eq!(text, "github");
}
