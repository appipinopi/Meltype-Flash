// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Android editor application. Counts and comparisons use exact UTF-16 units;
//! a batch groups notifications, but does not promise rollback of partial writes.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug)]
pub(super) struct EditorError;
type Result<T> = std::result::Result<T, EditorError>;
pub(super) trait Editor {
    fn selected(&mut self) -> Result<Option<Vec<u16>>>;
    fn before(&mut self, count: i32) -> Result<Option<Vec<u16>>>;
    fn begin(&mut self) -> Result<()>;
    fn end(&mut self) -> Result<()>;
    fn composing(&mut self, text: &str) -> Result<bool>;
    fn delete(&mut self, count: i32) -> Result<bool>;
    fn commit(&mut self, text: &str) -> Result<bool>;
    fn finish(&mut self) -> Result<bool>;
    fn move_back(&mut self, count: u32) -> Result<bool>;
}
#[derive(Deserialize)]
struct Edit {
    #[serde(default, rename = "deleteBefore")]
    delete: i32,
    text: String,
    expect: Option<String>,
    #[serde(default, rename = "caretBack")]
    caret_back: u32,
}
#[derive(Deserialize)]
struct Reply {
    consumed: bool,
    commits: Vec<Edit>,
    view: Option<Value>,
    #[serde(rename = "selectionExpect")]
    selected: Option<String>,
}
#[derive(Deserialize)]
struct View {
    text: String,
    // Validate all render data before changing the document.
    #[serde(rename = "candidates")]
    _candidates: Vec<String>,
    #[serde(default, rename = "predictions")]
    _predictions: Option<Vec<String>>,
}
#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct Outcome {
    pub consumed: bool,
    pub composing: bool,
    pub composing_text: String,
    pub view: Option<Value>,
    #[serde(skip)]
    pub reset: bool,
}
impl Outcome {
    pub fn failure(changed: bool) -> Self {
        Self {
            // Replaying a key after a partial successful edit can duplicate it.
            consumed: changed,
            composing: false,
            composing_text: String::new(),
            view: None,
            reset: true,
        }
    }
}
fn decode(json: &str) -> Option<(Reply, Option<View>)> {
    let reply: Reply = serde_json::from_str(json).ok()?;
    let view: Option<View> = reply
        .view
        .as_ref()
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .ok()?;
    for edit in &reply.commits {
        if edit.caret_back as usize > edit.text.encode_utf16().count() {
            return None;
        }
        if edit.delete > 0
            && edit.expect.as_ref().map(|s| s.encode_utf16().count()) != Some(edit.delete as usize)
        {
            return None;
        }
    }
    Some((reply, view))
}
pub(super) fn apply(editor: &mut impl Editor, json: &str, pending: &[u16]) -> Outcome {
    let Some((reply, view)) = decode(json) else {
        return Outcome::failure(false);
    };
    if let Some(expected) = &reply.selected {
        let expected: Vec<_> = expected.encode_utf16().collect();
        if editor.selected().ok().flatten().as_ref() != Some(&expected) {
            return Outcome::failure(false);
        }
    }
    if editor.begin().is_err() {
        return Outcome::failure(false);
    }
    let mut changed = false;
    let result = edit_batch(editor, &reply, view.as_ref(), pending, &mut changed);
    if result.is_err() {
        // Keep host text but end a stale composing span after resetting core.
        let _ = editor.finish();
    }
    // Always end a started batch, including rejected guards and Java exceptions.
    let ended = editor.end();
    if result.is_err() || ended.is_err() {
        return Outcome::failure(changed);
    }
    Outcome {
        consumed: reply.consumed,
        composing: view.is_some(),
        composing_text: view.map_or_else(String::new, |v| v.text),
        view: reply.view,
        reset: false,
    }
}
fn edit_batch(
    editor: &mut impl Editor,
    reply: &Reply,
    view: Option<&View>,
    pending: &[u16],
    changed: &mut bool,
) -> Result<()> {
    let mut pending = pending;
    for edit in &reply.commits {
        if edit.delete > 0 {
            if editor.selected()?.is_some_and(|s| !s.is_empty()) {
                return Err(EditorError);
            }
            let mut expected: Vec<_> = edit
                .expect
                .as_ref()
                .ok_or(EditorError)?
                .encode_utf16()
                .collect();
            expected.extend_from_slice(pending);
            let count = i32::try_from(expected.len()).map_err(|_| EditorError)?;
            if editor.before(count)?.as_ref() != Some(&expected) {
                return Err(EditorError);
            }
            if !pending.is_empty() {
                if !write_result(editor.composing(""), changed)? {
                    return Err(EditorError);
                }
                *changed = true;
                pending = &[];
            }
            if !write_result(editor.delete(edit.delete), changed)? {
                return Err(EditorError);
            }
            *changed = true;
        }
        if !edit.text.is_empty() {
            if !write_result(editor.commit(&edit.text), changed)? {
                return Err(EditorError);
            }
            *changed = true;
            pending = &[];
        }
        if edit.caret_back != 0 && !write_result(editor.move_back(edit.caret_back), changed)? {
            return Err(EditorError);
        }
    }
    let applied = if let Some(view) = view {
        write_result(editor.composing(&view.text), changed)?
    } else {
        // A consumed cancellation/deletion with no replacement must remove the
        // old preedit. finishComposingText alone leaves those characters behind.
        if reply.consumed && !pending.is_empty() {
            if !write_result(editor.composing(""), changed)? {
                return Err(EditorError);
            }
            *changed = true;
        }
        editor.finish()?
    };
    if !applied {
        return Err(EditorError);
    }
    Ok(())
}
fn write_result(result: Result<bool>, changed: &mut bool) -> Result<bool> {
    if result.is_err() {
        // A thrown editor exception may follow a partial write. Do not replay
        // the key when the host cannot prove that the document was unchanged.
        *changed = true;
    }
    result
}
