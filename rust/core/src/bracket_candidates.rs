// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 HarukiKubota, appipinopi
//! Bracket-only candidate families (upstream v1.5.0, #1162).
//!
//! A single bracket or an empty pair converts only within its family: the
//! typed shape and its width variant first, then the other empty pairs of the
//! same family. Ordinary text containing brackets is never handled here.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Round,
    Square,
    Curly,
    Angle,
}

struct Entry {
    pair: &'static str,
    group: Group,
    other_width: Option<&'static str>,
}

const ENTRIES: &[Entry] = &[
    Entry {
        pair: "「」",
        group: Group::Square,
        other_width: Some("｢｣"),
    },
    Entry {
        pair: "『』",
        group: Group::Square,
        other_width: None,
    },
    Entry {
        pair: "（）",
        group: Group::Round,
        other_width: Some("()"),
    },
    Entry {
        pair: "()",
        group: Group::Round,
        other_width: Some("（）"),
    },
    Entry {
        pair: "［］",
        group: Group::Square,
        other_width: Some("[]"),
    },
    Entry {
        pair: "[]",
        group: Group::Square,
        other_width: Some("［］"),
    },
    Entry {
        pair: "｛｝",
        group: Group::Curly,
        other_width: Some("{}"),
    },
    Entry {
        pair: "{}",
        group: Group::Curly,
        other_width: Some("｛｝"),
    },
    Entry {
        pair: "【】",
        group: Group::Square,
        other_width: None,
    },
    Entry {
        pair: "〈〉",
        group: Group::Angle,
        other_width: None,
    },
    Entry {
        pair: "《》",
        group: Group::Angle,
        other_width: None,
    },
    Entry {
        pair: "〔〕",
        group: Group::Square,
        other_width: None,
    },
    Entry {
        pair: "｢｣",
        group: Group::Square,
        other_width: Some("「」"),
    },
    Entry {
        pair: "＜＞",
        group: Group::Angle,
        other_width: Some("<>"),
    },
    Entry {
        pair: "<>",
        group: Group::Angle,
        other_width: Some("＜＞"),
    },
];

/// Candidates for a single left/right bracket or a matched empty pair.
/// Returns `None` for text, whitespace, unmatched or multiple pairs.
pub fn groups(input: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = input.chars().collect();
    if chars.len() > 2 {
        return None;
    }
    let (count, first) = match chars.as_slice() {
        [first] => (1, *first),
        [first, _] => (2, *first),
        _ => return None,
    };
    let entry = ENTRIES.iter().find(|entry| {
        if count == 2 {
            entry.pair == input
        } else {
            entry.pair.contains(first)
        }
    })?;
    let side = if count == 2 {
        None
    } else if entry.pair.starts_with(first) {
        Some(0)
    } else {
        Some(1)
    };
    let mut ordered: Vec<&Entry> = ENTRIES
        .iter()
        .filter(|candidate| candidate.group == entry.group)
        .collect();
    ordered.sort_by_key(|candidate| {
        if std::ptr::eq(*candidate, entry) {
            0
        } else if Some(candidate.pair) == entry.other_width {
            1
        } else {
            2
        }
    });
    Some(
        ordered
            .into_iter()
            .map(|candidate| match side {
                None => candidate.pair.to_owned(),
                Some(index) => candidate.pair.chars().nth(index).unwrap().to_string(),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::groups;

    #[test]
    fn families_keep_the_typed_shape_first() {
        for (input, expected) in [
            ("「」", "「」 ｢｣ 『』 ［］ [] 【】 〔〕"),
            ("「", "「 ｢ 『 ［ [ 【 〔"),
            ("『』", "『』 「」 ［］ [] 【】 〔〕 ｢｣"),
            ("（）", "（） ()"),
            ("()", "() （）"),
            ("［］", "［］ [] 「」 『』 【】 〔〕 ｢｣"),
            ("[]", "[] ［］ 「」 『』 【】 〔〕 ｢｣"),
            ("｛｝", "｛｝ {}"),
            ("{}", "{} ｛｝"),
            ("〈〉", "〈〉 《》 ＜＞ <>"),
            ("《》", "《》 〈〉 ＜＞ <>"),
            ("＜＞", "＜＞ <> 〈〉 《》"),
            ("<>", "<> ＜＞ 〈〉 《》"),
        ] {
            let actual = groups(input).unwrap();
            assert_eq!(actual.join(" "), expected, "{input}");
            let first: String = input.chars().take(1).collect();
            assert_eq!(
                actual[0],
                if input.chars().count() == 2 {
                    input
                } else {
                    &first
                }
            );
        }
    }

    #[test]
    fn only_single_or_matched_empty_pairs_are_handled() {
        for input in [
            "",
            "「文」",
            "(abc)",
            "文「」",
            "「 」",
            " 「」",
            "「」 ",
            "「)",
            "［」",
            ")(",
            "「」()",
            "((",
            "\"\"",
            "“”",
        ] {
            assert!(groups(input).is_none(), "{input:?}");
        }
    }
}
