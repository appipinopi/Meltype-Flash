// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! ReadingLattice: 日本語の語・英単語・未知語をノードにしたViterbiで日英の区切りを選ぶ。
//!
//! 上流 v1.5.0 の ReadingLattice.cs の考え方を、Flash の既存 Dictionary / ローマ字単位へ
//! 接続した実装。上流のすべての記号・ドメイン・大文字規則はまだ移植していない。
use super::Dictionary;
use crate::romaji;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    None,
    Japanese,
    English,
}

#[derive(Clone)]
struct Edge {
    text: String,
    particle: bool,
}

#[derive(Clone)]
struct Node {
    cost: f64,
    prev: usize,
    prev_lang: usize,
    edge: Edge,
}

const PARTICLES: &[&str] = &[
    "が", "を", "に", "で", "と", "の", "は", "も", "へ", "や", "から", "まで", "より", "って",
    "だ", "です",
];

fn lang_index(lang: Lang) -> usize {
    match lang {
        Lang::None => 0,
        Lang::Japanese => 1,
        Lang::English => 2,
    }
}

impl Dictionary {
    pub fn composition_lattice(
        &self,
        raw: &str,
        final_input: bool,
        before: &str,
        after: &str,
        level: &str,
    ) -> String {
        if raw.is_empty() || level == "Manual" || !raw.bytes().all(|b| b.is_ascii_alphabetic()) {
            return self.composition(raw, final_input, before, after, level);
        }
        let lower = raw.to_ascii_lowercase();
        let (bounds, pending) = romaji::unit_boundaries(
            &lower,
            |prefix| self.known_english(prefix),
            |prefix| self.known_english(prefix),
        );
        let end = pending.unwrap_or(raw.len());
        let mut offsets = vec![0];
        offsets.extend((1..=end).filter(|i| bounds[*i]));
        if offsets.last().copied() != Some(end) {
            offsets.push(end);
        }
        let n = offsets.len().saturating_sub(1);
        if n == 0 {
            return self.composition(raw, final_input, before, after, level);
        }
        let mut best: Vec<[Option<Node>; 3]> = (0..=n).map(|_| [None, None, None]).collect();
        best[0][0] = Some(Node {
            cost: 0.0,
            prev: 0,
            prev_lang: 0,
            edge: Edge {
                text: String::new(),
                particle: false,
            },
        });
        let max_span = 8;
        for index in 0..n {
            for state in 0..3 {
                let Some(from) = best[index][state].clone() else {
                    continue;
                };
                let unit = &raw[offsets[index]..offsets[index + 1]];
                if !unit.bytes().all(|b| b.is_ascii_alphabetic()) {
                    offer(
                        &mut best,
                        index + 1,
                        Lang::None,
                        from.cost,
                        index,
                        state,
                        Edge {
                            text: unit.to_owned(),
                            particle: false,
                        },
                    );
                    continue;
                }
                // 日本語のノード。
                for next in index + 1..=n.min(index + max_span) {
                    let span = &raw[offsets[index]..offsets[next]];
                    if !span.bytes().all(|b| b.is_ascii_alphabetic()) {
                        break;
                    }
                    let kana = romaji::hiragana(&span.to_ascii_lowercase(), true);
                    if kana.is_empty()
                        || kana
                            .chars()
                            .any(|c| !('ぁ'..='ゖ').contains(&c) && c != 'ー')
                    {
                        continue;
                    }
                    let particle = PARTICLES.contains(&kana.as_str());
                    let known = self.is_japanese(span)
                        || self.is_japanese(&kana)
                        || self
                            .candidates(&kana, "Asia/Tokyo")
                            .iter()
                            .any(|candidate| candidate.chars().any(crate::linguistics::kanji));
                    let cost = if particle {
                        0.8
                    } else if known {
                        1.0
                    } else {
                        3.0 * kana.chars().count() as f64
                    };
                    let text = self.composition(&kana, final_input, before, after, level);
                    offer(
                        &mut best,
                        next,
                        Lang::Japanese,
                        from.cost + cost,
                        index,
                        state,
                        Edge {
                            text: if text.is_empty() { kana } else { text },
                            particle,
                        },
                    );
                }
                // 英単語のノード。
                for next in index + 1..=n.min(index + max_span) {
                    let span = &raw[offsets[index]..offsets[next]];
                    if !span.bytes().all(|b| b.is_ascii_alphabetic()) {
                        break;
                    }
                    let lower_span = span.to_ascii_lowercase();
                    let known =
                        self.known_english(&lower_span) || self.is_common_english(&lower_span);
                    if !known && span.len() < 3 {
                        continue;
                    }
                    let cost = if known { 1.0 } else { 1.5 * span.len() as f64 };
                    offer(
                        &mut best,
                        next,
                        Lang::English,
                        from.cost + cost,
                        index,
                        state,
                        Edge {
                            text: span.to_owned(),
                            particle: false,
                        },
                    );
                }
            }
        }
        let mut chosen: Option<(usize, usize, f64)> = None;
        for state in [1, 0, 2] {
            if let Some(node) = &best[n][state] {
                let tie = if state == 1 { -1e-6 } else { 0.0 };
                let cost = node.cost + tie;
                if chosen.is_none_or(|(_, _, best)| cost < best) {
                    chosen = Some((n, state, cost));
                }
            }
        }
        let Some((mut position, mut state, _)) = chosen else {
            return self.composition(raw, final_input, before, after, level);
        };
        let mut edges = Vec::new();
        while position > 0 {
            let Some(node) = best[position][state].clone() else {
                break;
            };
            edges.push(node.edge);
            position = node.prev;
            state = node.prev_lang;
        }
        edges.reverse();
        let mut output = String::new();
        for edge in edges {
            output.push_str(&edge.text);
        }
        if let Some(rest) = raw.get(end..) {
            output.push_str(rest);
        }
        if output.is_empty() {
            self.composition(raw, final_input, before, after, level)
        } else {
            output
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn offer(
    best: &mut [[Option<Node>; 3]],
    position: usize,
    lang: Lang,
    cost: f64,
    prev: usize,
    prev_state: usize,
    edge: Edge,
) {
    let previous_lang = match prev_state {
        1 => Lang::Japanese,
        2 => Lang::English,
        _ => Lang::None,
    };
    let switch = if previous_lang != Lang::None
        && lang != Lang::None
        && previous_lang != lang
        && !edge.particle
    {
        1.0
    } else {
        0.0
    };
    let adjacent = if previous_lang == Lang::English && lang == Lang::English && prev > 0 {
        1.0
    } else {
        0.0
    };
    let cost = cost + switch + adjacent;
    let slot = &mut best[position][lang_index(lang)];
    if slot.as_ref().is_none_or(|node| cost < node.cost) {
        *slot = Some(Node {
            cost,
            prev,
            prev_lang: prev_state,
            edge,
        });
    }
}
