// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro, appipinopi
//! Port of MsImeKanjiConverter: same IFELanguage calls and UTF-16 boundaries.
use std::{ffi::c_void, ptr};
use windows::{
    core::{w, BSTR, PCWSTR},
    Win32::{
        System::{Com::*, WinRT::*},
        UI::Input::Ime::{IFELanguage, MORRSLT, WDD},
    },
};

pub(super) struct Apartment(std::marker::PhantomData<std::rc::Rc<()>>);
impl Apartment {
    pub(super) fn enter() -> Option<Self> {
        unsafe {
            RoInitialize(RO_INIT_SINGLETHREADED).ok()?;
        }
        Some(Self(std::marker::PhantomData))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Segment {
    pub reading: Vec<u16>,
    pub text: Vec<u16>,
}
pub(super) const MAX_INPUT: usize = 100;
const MAX_CONTEXT: usize = 10;

pub(super) fn split(text: &[u16], maximum: usize) -> Vec<&[u16]> {
    assert!(maximum > 0);
    let mut result = Vec::new();
    let mut start = 0;
    while text.len() - start > maximum {
        let cut = ["。．！？!?", "、，,", "はがをにでともへやの"]
            .iter()
            .find_map(|marks| {
                (start + maximum / 2 + 1..=start + maximum)
                    .rev()
                    .find(|i| marks.encode_utf16().any(|mark| mark == text[i - 1]))
            })
            .unwrap_or(start + maximum);
        result.push(&text[start..cut]);
        start = cut;
    }
    result.push(&text[start..]);
    result
}

#[derive(Default)]
pub(super) struct System {
    language: Option<IFELanguage>,
    unavailable: bool,
    cache: std::collections::HashMap<Vec<u16>, Vec<u16>>,
}
impl System {
    fn open(&mut self) -> Option<&IFELanguage> {
        if self.language.is_none() && !self.unavailable {
            let opened = unsafe {
                CLSIDFromProgID(w!("MSIME.Japan")).and_then(|clsid| {
                    let language: IFELanguage = CoCreateInstance(&clsid, None, CLSCTX_ALL)?;
                    language.Open()?;
                    Ok(language)
                })
            };
            match opened {
                Ok(language) => self.language = Some(language),
                Err(_) => self.unavailable = true,
            }
        }
        self.language.as_ref()
    }
    pub(super) fn available(&mut self) -> bool {
        self.open().is_some()
    }
    pub(super) fn convert(&mut self, reading: &[u16]) -> Option<Vec<u16>> {
        if reading.is_empty() {
            return None;
        }
        if let Some(cached) = self.cache.get(reading) {
            return Some(cached.clone());
        }
        if reading.len() > MAX_INPUT {
            return Some(
                split(reading, MAX_INPUT)
                    .into_iter()
                    .flat_map(|part| self.convert(part).unwrap_or_else(|| part.to_vec()))
                    .collect(),
            );
        }
        let language = self.open()?;
        let input = BSTR::from_wide(reading).ok()?;
        let mut output = BSTR::new();
        unsafe {
            language.GetConversion(&input, 1, -1, &mut output).ok()?;
        }
        if output.is_empty() {
            None
        } else {
            let text = String::from_utf16_lossy(output.as_wide());
            let normalized = meltype_core::transducer::normalize(&text);
            let result: Vec<u16> = normalized.encode_utf16().collect();
            if self.cache.len() < 16384 {
                self.cache.insert(reading.to_vec(), result.clone());
            }
            Some(result)
        }
    }
    pub(super) fn clauses(&mut self, reading: &[u16], context: &[u16]) -> Option<Vec<Segment>> {
        if reading.is_empty() {
            return None;
        }
        if reading.len() + context.len() > MAX_INPUT {
            let mut result: Vec<Segment> = Vec::new();
            for part in split(reading, MAX_INPUT - MAX_CONTEXT) {
                let previous = if result.is_empty() {
                    context.to_vec()
                } else {
                    result.iter().flat_map(|s| s.text.iter().copied()).collect()
                };
                let context = &previous[previous.len().saturating_sub(MAX_CONTEXT)..];
                let segments = self.clauses(part, context).unwrap_or_else(|| {
                    vec![Segment {
                        reading: part.to_vec(),
                        text: self.convert(part).unwrap_or_else(|| part.to_vec()),
                    }]
                });
                result.extend(segments);
            }
            return Some(result);
        }
        if !context.is_empty() {
            if let Some(segments) = self.morph(reading, context) {
                return Some(segments);
            }
        }
        self.morph(reading, &[])
    }
    fn morph(&mut self, reading: &[u16], context: &[u16]) -> Option<Vec<Segment>> {
        let language = self.open()?;
        let mut input: Vec<_> = context.iter().chain(reading).copied().collect();
        let length = input.len();
        input.push(0);
        let mut info: Vec<u32> = (0..length)
            .map(|i| if i < context.len() { 0x10 } else { 0 })
            .collect();
        let mut raw = ptr::null_mut();
        unsafe {
            let status = language.GetJMorphResult(
                0x10000,
                0x40000000,
                length as i32,
                PCWSTR(input.as_ptr()),
                if context.is_empty() {
                    ptr::null_mut()
                } else {
                    info.as_mut_ptr()
                },
                &mut raw,
            );
            let result = Allocation(raw.cast());
            if status.is_err() || result.0.is_null() {
                return None;
            }
            parse(raw, &input[..length], context.len())
        }
    }
}
impl Drop for System {
    fn drop(&mut self) {
        if let Some(language) = self.language.take() {
            unsafe {
                let _ = language.Close();
            }
        }
    }
}
struct Allocation(*mut c_void);
impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.0));
        }
    }
}

/// Owned engine memory must remain valid throughout parsing. SDK records are
/// packed to one byte on both ABIs; copy them before inspecting their fields.
unsafe fn parse(pointer: *const MORRSLT, input: &[u16], context: usize) -> Option<Vec<Segment>> {
    let result = pointer.read_unaligned();
    let output_pointer = result.pwchOutput;
    let reading_pointer = result.Anonymous1.pwchRead;
    let read_count = result.Anonymous2.cchRead as usize;
    let output_count = result.cchOutput as usize;
    if output_pointer.is_null() || result.pWDD.is_null() || result.cWDD <= 0
        || result.cWDD as usize > input.len() * 2 || output_count == 0
        || output_count > input.len() * 4 || read_count > input.len() * 4
        // The original C# structure excludes the SDK's trailing BLKBuff[1].
        || result.dwSize < (std::mem::size_of::<MORRSLT>() - 2) as u32
        || result.dwSize > 1 << 20 || context > input.len()
    {
        return None;
    }
    let output = std::slice::from_raw_parts(output_pointer.0, output_count);
    let reading = if reading_pointer.is_null() {
        input
    } else {
        std::slice::from_raw_parts(reading_pointer.0, read_count)
    };
    let suffix = &input[context..];
    if !reading.ends_with(suffix) {
        return None;
    }
    let context = reading.len() - suffix.len();
    let mut clauses: Vec<(usize, usize, usize, usize)> = Vec::new();
    for i in 0..result.cWDD as usize {
        let word: WDD = result.pWDD.add(i).read_unaligned();
        let start = word.Anonymous1.wReadPos as usize;
        let end = start + word.Anonymous2.cchRead as usize;
        let display = word.wDispPos as usize;
        let display_end = display + word.cchDisp as usize;
        if end <= context {
            continue;
        }
        if start < context {
            return None;
        }
        if clauses.is_empty() || word.nPos != 0 {
            clauses.push((start, end, display, display_end));
        } else {
            let last = clauses.last_mut().unwrap();
            last.1 = last.1.max(end);
            last.3 = last.3.max(display_end);
        }
    }
    if clauses.first()?.0 != context || clauses.last()?.1 != reading.len() {
        return None;
    }
    clauses
        .into_iter()
        .map(|(start, end, display, display_end)| {
            if end <= start || display_end < display {
                return None;
            }
            Some(Segment {
                reading: reading.get(start..end)?.to_vec(),
                text: output.get(display..display_end)?.to_vec(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use windows::{
        core::PWSTR,
        Win32::UI::Input::Ime::{MORRSLT_0, MORRSLT_1, WDD_0, WDD_1},
    };
    #[test]
    fn original_split_and_packed_morph_cases_match_on_this_abi() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../tests/data/upstream-windows-converter.json"
        ))
        .unwrap();
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 27);
        for row in fixture["cases"].as_array().unwrap() {
            let case = &row["case"];
            let input: Vec<u16> = case["input"].as_str().unwrap().encode_utf16().collect();
            let actual = if case["kind"] == "split" {
                json!(split(&input, case["maximum"].as_u64().unwrap() as usize)
                    .into_iter()
                    .map(String::from_utf16_lossy)
                    .collect::<Vec<_>>())
            } else {
                let mut output: Vec<u16> =
                    case["output"].as_str().unwrap().encode_utf16().collect();
                let reading = case["reading"].as_str();
                let mut reading_units: Vec<u16> = reading.unwrap_or("").encode_utf16().collect();
                let mut words: Vec<WDD> = case["words"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|word| {
                        let n = |key: &str| word[key].as_u64().unwrap_or(0) as u16;
                        WDD {
                            wDispPos: n("dispStart"),
                            Anonymous1: WDD_0 {
                                wReadPos: n("readStart"),
                            },
                            cchDisp: n("dispLength"),
                            Anonymous2: WDD_1 {
                                cchRead: n("readLength"),
                            },
                            nPos: n("pos"),
                            _bitfield: n("flags"),
                            ..Default::default()
                        }
                    })
                    .collect();
                let header = MORRSLT {
                    dwSize: ((std::mem::size_of::<MORRSLT>() - 2) as i64
                        + case["sizeDelta"].as_i64().unwrap_or(0))
                        as u32,
                    pwchOutput: PWSTR(if case["nullOutput"] == true {
                        ptr::null_mut()
                    } else {
                        output.as_mut_ptr()
                    }),
                    cchOutput: output.len() as u16,
                    Anonymous1: MORRSLT_0 {
                        pwchRead: PWSTR(if reading.is_some() {
                            reading_units.as_mut_ptr()
                        } else {
                            ptr::null_mut()
                        }),
                    },
                    Anonymous2: MORRSLT_1 {
                        cchRead: reading_units.len() as u16,
                    },
                    pWDD: if case["nullWords"] == true {
                        ptr::null_mut()
                    } else {
                        words.as_mut_ptr()
                    },
                    cWDD: case["wordCount"].as_i64().unwrap_or(words.len() as i64) as i32,
                    ..Default::default()
                };
                let result = unsafe {
                    parse(
                        &header,
                        &input,
                        case["contextLength"].as_u64().unwrap() as usize,
                    )
                };
                json!(result.map(|segments|segments.into_iter().map(|s|json!({"reading":String::from_utf16_lossy(&s.reading),"text":String::from_utf16_lossy(&s.text)})).collect::<Vec<_>>()))
            };
            assert_eq!(actual, row["expected"], "{}", case["name"]);
        }
    }
}
