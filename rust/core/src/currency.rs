// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! 金額と通貨の読みに、別の通貨へ換算した額の候補を出す。
//!
//! レートは同梱した欧州中央銀行 (ECB) の参照レートを使い、使うときにネットへは
//! つながない。`tools/make-exchange-rates.mjs` で作った
//! `dictionaries/exchange-rates.txt` を読む。
use chrono::NaiveDate;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// 通貨の読み → 通貨コード・候補に書く名前。長い読みを先に置く (かなだどる を どる より先に)。
pub const CURRENCIES: &[(&str, &str, &str)] = &[
    ("おーすとらりあどる", "AUD", "豪ドル"),
    ("しんがぽーるどる", "SGD", "シンガポールドル"),
    ("じんみんげん", "CNY", "人民元"),
    ("ほんこんどる", "HKD", "香港ドル"),
    ("すいすふらん", "CHF", "スイスフラン"),
    ("かなだどる", "CAD", "カナダドル"),
    ("ごうどる", "AUD", "豪ドル"),
    ("べいどる", "USD", "米ドル"),
    ("ゆーろ", "EUR", "ユーロ"),
    ("ぽんど", "GBP", "ポンド"),
    ("うぉん", "KRW", "ウォン"),
    ("ばーつ", "THB", "バーツ"),
    ("るぴー", "INR", "ルピー"),
    ("どる", "USD", "ドル"),
    ("げん", "CNY", "元"),
    ("えん", "JPY", "円"),
];

/// 1 ユーロあたりの各通貨の額と、その日付。
#[derive(Clone, Debug)]
pub struct Rates {
    pub date: NaiveDate,
    pub rates: BTreeMap<String, f64>,
}

impl Rates {
    /// `# 日付: yyyy-MM-dd` と `通貨コード<TAB>レート` の行を読む。読めなければ None。
    pub fn parse(text: &str) -> Option<Self> {
        let mut date = None;
        let mut rates = BTreeMap::new();
        for raw in text.lines() {
            let line = raw.trim();
            if let Some(rest) = line.strip_prefix("# 日付:") {
                date = NaiveDate::parse_from_str(rest.trim(), "%Y-%m-%d").ok();
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((code, value)) = line.split_once('\t') else {
                continue;
            };
            if code.len() != 3 {
                continue;
            }
            let Ok(rate) = value.parse::<f64>() else {
                continue;
            };
            if rate > 0.0 {
                rates.insert(code.to_owned(), rate);
            }
        }
        if rates.len() > 1 {
            Some(Self {
                date: date?,
                rates,
            })
        } else {
            None
        }
    }

    /// amount (from の通貨) を to の通貨にした額。どちらかのレートが無ければ None。
    pub fn convert(&self, amount: f64, from: &str, to: &str) -> Option<f64> {
        let from = *self.rates.get(from)?;
        let to = *self.rates.get(to)?;
        (from > 0.0).then(|| amount / from * to)
    }
}

/// 同梱したレート。読めなければ None。
pub fn bundled() -> Option<&'static Rates> {
    static RATES: OnceLock<Option<Rates>> = OnceLock::new();
    RATES
        .get_or_init(|| Rates::parse(crate::system_data::text("exchange-rates.txt")))
        .as_ref()
}

/// 読みから取り出した金額・換算元・換算先 (決めていなければ None)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parsed {
    pub amount: f64,
    pub from: (&'static str, &'static str),
    pub to: Option<(&'static str, &'static str)>,
}

fn currency(reading: &str) -> Option<(&'static str, &'static str)> {
    CURRENCIES
        .iter()
        .find(|(name, _, _)| *name == reading)
        .map(|(_, code, unit)| (*code, *unit))
}

fn digit(c: char) -> Option<char> {
    match c {
        '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32),
        '．' => Some('.'),
        '0'..='9' | '.' => Some(c),
        _ => None,
    }
}

/// 「100どる」「1まんえんをゆーろ」「１２．５ぽんど」のような読みを金額にする。
pub fn parse(reading: &str) -> Option<Parsed> {
    let reading = reading.trim_end_matches('\n');
    let mut number = String::new();
    let mut used = 0;
    for (offset, c) in reading.char_indices() {
        match digit(c) {
            Some(d) => {
                number.push(d);
                used = offset + c.len_utf8();
            }
            None => break,
        }
    }
    let rest = &reading[used..];
    let amount_text = number.as_str();
    let (whole, fraction) = amount_text
        .split_once('.')
        .map_or((amount_text, None), |(a, b)| (a, Some(b)));
    if whole.is_empty()
        || whole.len() > 12
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction.is_some_and(|f| {
            f.is_empty() || f.len() > 4 || !f.bytes().all(|b| b.is_ascii_digit())
        })
    {
        return None;
    }
    let (multiplier, rest) = if let Some(rest) = rest.strip_prefix("まん") {
        (10_000.0, rest)
    } else if let Some(rest) = rest.strip_prefix("おく") {
        (100_000_000.0, rest)
    } else {
        (1.0, rest)
    };
    let (from_part, to_part) = rest.split_once('を').map_or((rest, None), |(a, b)| (a, Some(b)));
    let from = currency(from_part)?;
    let to = match to_part {
        Some(target) => {
            let target = target.strip_suffix('に').unwrap_or(target);
            let target = currency(target)?;
            if target.0 == from.0 {
                return None;
            }
            Some(target)
        }
        None => None,
    };
    let amount = amount_text.parse::<f64>().ok()? * multiplier;
    Some(Parsed { amount, from, to })
}

fn group(value: u64) -> String {
    let text = value.to_string();
    let mut out = String::with_capacity(text.len() + text.len() / 3);
    for (index, c) in text.chars().enumerate() {
        if index > 0 && (text.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 円・ウォンは整数、ほかは小数 2 桁まで。3 桁ごとに , を入れる。
fn format_value(value: f64, code: &str) -> String {
    if matches!(code, "JPY" | "KRW") {
        return group(value.round().max(0.0) as u64);
    }
    let cents = (value * 100.0).round().max(0.0) as u64;
    let (whole, fraction) = (cents / 100, cents % 100);
    if fraction == 0 {
        group(whole)
    } else if fraction % 10 == 0 {
        format!("{}.{}", group(whole), fraction / 10)
    } else {
        format!("{}.{:02}", group(whole), fraction)
    }
}

/// 換算した額の候補 (レートの日付つきと、額だけ)。
/// 換算先を決めていなければ、円以外は円に、円はドルとユーロにする。
pub fn candidates_with(
    amount: f64,
    from: (&str, &str),
    to: Option<(&str, &str)>,
    rates: &Rates,
) -> Vec<String> {
    let targets: Vec<(&str, &str)> = match to {
        Some(target) => vec![target],
        None if from.0 == "JPY" => vec![("USD", "ドル"), ("EUR", "ユーロ")],
        None => vec![("JPY", "円")],
    };
    let date = rates.date.format("%Y/%m/%d").to_string();
    let mut values = Vec::new();
    for (code, unit) in targets {
        let Some(value) = rates.convert(amount, from.0, code) else {
            continue;
        };
        let text = format!("約{}{}", format_value(value, code), unit);
        let dated = format!("{text}（{date} のレート）");
        for candidate in [dated, text] {
            if !values.contains(&candidate) {
                values.push(candidate);
            }
        }
    }
    values
}

/// 同梱のレートで換算した候補。
pub fn candidates(reading: &str) -> Vec<String> {
    let Some(parsed) = parse(reading) else {
        return Vec::new();
    };
    let Some(rates) = bundled() else {
        return Vec::new();
    };
    candidates_with(parsed.amount, parsed.from, parsed.to, rates)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "\
# Meltype 通貨の換算に使うレート (1 ユーロあたりの各通貨の額)
# 日付: 2026-11-11
EUR\t1
USD\t1.1127
JPY\t177.18
GBP\t0.83618
KRW\t1600.5
";

    fn rates() -> Rates {
        Rates::parse(TABLE).unwrap()
    }

    #[test]
    fn parses_the_bundled_table() {
        let rates = rates();
        assert_eq!(rates.date, NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
        assert_eq!(rates.rates["EUR"], 1.0);
        assert_eq!(rates.rates["JPY"], 177.18);
        assert!(Rates::parse("EUR\t1\nJPY\t177.18\n").is_none());
        assert!(bundled().is_some_and(|rates| {
            rates.rates.contains_key("JPY") && rates.rates.contains_key("USD")
        }));
    }

    #[test]
    fn converts_with_the_rate_date() {
        let rates = rates();
        assert_eq!(
            candidates_with(100.0, ("USD", "ドル"), None, &rates),
            ["約15,923円（2026/11/11 のレート）", "約15,923円"]
        );
        assert_eq!(
            candidates_with(10_000.0, ("JPY", "円"), None, &rates),
            [
                "約62.8ドル（2026/11/11 のレート）",
                "約62.8ドル",
                "約56.44ユーロ（2026/11/11 のレート）",
                "約56.44ユーロ"
            ]
        );
        assert_eq!(
            candidates_with(10_000.0, ("JPY", "円"), Some(("EUR", "ユーロ")), &rates),
            ["約56.44ユーロ（2026/11/11 のレート）", "約56.44ユーロ"]
        );
        assert!(candidates_with(100.0, ("THB", "バーツ"), None, &rates).is_empty());
    }

    #[test]
    fn parses_amount_and_currencies() {
        assert_eq!(
            parse("100どる"),
            Some(Parsed {
                amount: 100.0,
                from: ("USD", "ドル"),
                to: None
            })
        );
        assert_eq!(
            parse("1まんえんをゆーろ"),
            Some(Parsed {
                amount: 10_000.0,
                from: ("JPY", "円"),
                to: Some(("EUR", "ユーロ"))
            })
        );
        assert_eq!(parse("10000えんをどるに").unwrap().to, Some(("USD", "ドル")));
        assert_eq!(parse("5かなだどる").unwrap().from, ("CAD", "カナダドル"));
        assert_eq!(
            parse("１２．５ぽんど"),
            Some(Parsed {
                amount: 12.5,
                from: ("GBP", "ポンド"),
                to: None
            })
        );
        assert!(parse("100にん").is_none());
        assert!(parse("どる").is_none());
        assert!(parse("100えんをえん").is_none());
    }
}
