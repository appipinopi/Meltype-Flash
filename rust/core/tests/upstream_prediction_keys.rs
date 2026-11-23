// SPDX-License-Identifier: GPL-3.0-or-later
use meltype_core::{storage::Store, Session, SessionResult};
use serde_json::{json, Value};

fn projection(result: &Value) -> Value {
    let view = &result["view"];
    json!({"consumed":result["consumed"],"commits":result["commits"],"view":
    if view.is_null() { Value::Null } else {
        json!({"text":view["text"],"converting":view["converting"],
            "predictions":view["predictions"],"selectedPrediction":view["selectedPrediction"]})
    }})
}

#[test]
fn original_prediction_display_navigation_and_phrase_learning_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("data/upstream-prediction-keys-v150.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 110);
    for row in cases {
        let case = &row["case"];
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("state.json"),
            serde_json::to_vec(&json!({"phrases":case["phrases"]})).unwrap(),
        )
        .unwrap();
        let store = Store::new(directory.path());
        let mut s = Session::open(store.clone()).unwrap();
        for name in ["LiveConversion", "ScoredSegmentation", "TabConversion"] {
            s.settings.set(name, json!(false)).unwrap();
        }
        s.settings.set("PredictiveCandidates", json!(true)).unwrap();
        s.settings
            .set("LearningEnabled", case["learning"].clone())
            .unwrap();
        s.settings
            .set("DetectionLevel", case["level"].clone())
            .unwrap();
        let keys = row["keys"].as_array().unwrap();
        let expected = row["expected"].as_array().unwrap();
        assert_eq!(keys.len(), expected.len());
        for (index, (key, expected)) in keys.iter().zip(expected).enumerate() {
            let vk = key[0].as_i64().unwrap() as i32;
            let ch = key[1].as_i64().unwrap() as i32;
            let result: SessionResult = if vk == -1 {
                s.select_prediction(ch as usize)
            } else {
                s.key(vk, ch, key[2].as_i64().unwrap() as i32)
            };
            assert_eq!(
                projection(&serde_json::to_value(result).unwrap()),
                projection(expected),
                "{} key {index}",
                case["name"]
            );
        }
        let mut phrases: Vec<_> = store
            .load()
            .unwrap()
            .phrases
            .into_iter()
            .map(|p| json!({"reading":p.reading,"text":p.text,"count":p.count}))
            .collect();
        phrases.sort_by(|a, b| {
            (a["reading"].as_str(), a["text"].as_str())
                .cmp(&(b["reading"].as_str(), b["text"].as_str()))
        });
        assert_eq!(
            json!(phrases),
            row["phrasesAfter"],
            "{} persisted phrases",
            case["name"]
        );
    }
}
