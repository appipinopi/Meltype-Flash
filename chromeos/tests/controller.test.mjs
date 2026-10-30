// SPDX-License-Identifier: GPL-3.0-or-later
import {test} from "node:test";
import assert from "node:assert/strict";
import {ImeController} from "../controller.mjs";
import {translateKey} from "../keys.mjs";
function fixture(loadOverride) {
  const calls = [], errors = [], saved = [];
  const core = {reset() {}, focusIfNeeded() {}, snapshot: () => "snapshot",
    key: () => JSON.stringify({consumed: true, commits: [], view: {text: "😀か", converting: true,
      candidates: ["😀か", "蚊"], notes: [null, "注釈"], selectedIndex: 1}})};
  const ime = Object.fromEntries(["commitText", "setComposition", "clearComposition", "setCandidates",
    "setCandidateWindowProperties", "setCursorPosition"].map(name => [name, async args => {calls.push([name, args]); return true;}]));
  ime.keyEventHandled = (id, handled) => calls.push(["ack", id, handled]);
  const controller = new ImeController({ime, load: loadOverride ?? (async () => core), persist: async s => saved.push(s), report: e => errors.push(e), now: () => 100});
  controller.activate("meltype-rust"); controller.focus({contextID: 7, type: "text", shouldDoLearning: true});
  return {controller, core, ime, calls, errors, saved};
}
const event = {type: "keydown", key: "a", code: "KeyA"};

test("site mode is cached, refreshed without resetting composition, and stale lookups never reach Core", async () => {
  const f = fixture();
  let queries = 0, resets = 0;
  const modes = [];
  f.core.reset = () => resets++;
  f.core.set_code_mode = mode => modes.push(mode);
  f.controller.resolveMode = async () => {queries++; return "code";};
  for (const id of ["first", "second"]) { f.controller.key("meltype-rust", event, id); await f.controller.queue; }
  assert.equal(queries, 1);
  assert.deepEqual(modes, [true, true]);
  let release, started;
  const waiting = new Promise(resolve => {started = resolve;});
  f.controller.resolveMode = () => {started(); return new Promise(resolve => {release = resolve;});};
  f.controller.refreshMode();
  f.controller.key("meltype-rust", event, "stale");
  await waiting;
  f.controller.resolveMode = async () => "normal";
  f.controller.refreshMode(); release("code"); await f.controller.queue;
  assert.deepEqual(f.calls.at(-1), ["ack", "stale", true]);
  assert.deepEqual(modes, [true, true, false]);
  assert.equal(resets, 0);
  f.controller.resolveMode = async () => "normal";
  f.controller.key("meltype-rust", event, "normal"); await f.controller.queue;
  assert.deepEqual(modes, [true, true, false, false]);
});

test("a field change during site lookup never applies the old key or mode", async () => {
  const f = fixture();
  let release, started;
  const waiting = new Promise(resolve => {started = resolve;});
  f.controller.resolveMode = () => {started(); return new Promise(resolve => {release = resolve;});};
  f.core.key = () => assert.fail("old key reached Core");
  f.core.set_code_mode = () => assert.fail("old mode reached Core");
  f.controller.key("meltype-rust", event, "old-field");
  await waiting;
  f.controller.focus({contextID: 8, type: "text"});
  release("code"); await f.controller.queue;
  assert.deepEqual(f.calls, [["ack", "old-field", false]]);
});
test("named/OEM/JIS keys and modifiers map to Core ABI", () => {
  assert.deepEqual(translateKey({...event, shiftKey: true, ctrlKey: true}), {vk: 65, character: 97, modifiers: 3});
  assert.equal(translateKey({key: ";", code: "Semicolon"}).vk, 186);
  assert.equal(translateKey({key: "￥", code: "IntlYen"}).vk, 220);
  assert.equal(translateKey({key: "F10", code: "F10"}).vk, 121);
  assert.equal(translateKey({key: "AudioVolumeUp", code: "AudioVolumeUp"}), null);
});
test("半角/全角とAlt+Backquoteをモード切替として認識し、文字のバッククォートを保つ", () => {
  for (const [key, mode] of [["ZenkakuHankaku", "toggle"], ["KanjiMode", "toggle"],
    ["Hankaku", "direct"], ["Zenkaku", "japanese"]]) {
    for (const code of ["Backquote", "Unidentified", ""]) {
      assert.deepEqual(translateKey({key, code}), {vk: 243, character: 0, modifiers: 0, mode});
    }
  }
  assert.equal(translateKey({key: "Unidentified", code: "Lang5"}).mode, "toggle");
  assert.deepEqual(translateKey({key: "`", code: "Backquote"}), {vk: 192, character: 96, modifiers: 0});
  assert.equal(translateKey({key: "`", code: "Backquote", altKey: true}).mode, "toggle");
  for (const modifiers of [{altgrKey: true}, {altKey: true, altgrKey: true},
    {altKey: true, shiftKey: true}, {altKey: true, ctrlKey: true}, {altKey: true, metaKey: true}]) {
    assert.equal(translateKey({key: "`", code: "Backquote", ...modifiers}).mode, undefined);
  }
});
test("async ack happens once, candidate IDs/UTF16 cursor are correct, keyup is paired", async () => {
  const f = fixture();
  assert.equal(f.controller.key("meltype-rust", event, "one"), undefined);
  await f.controller.queue;
  assert.equal(f.calls.find(c => c[0] === "setComposition")[1].cursor, 3);
  assert.equal(f.calls.find(c => c[0] === "setCursorPosition")[1].candidateID, 1);
  assert.deepEqual(f.calls.filter(c => c[0] === "ack"), [["ack", "one", true]]);
  f.controller.key("meltype-rust", {...event, type: "keyup"}, "two"); await f.controller.queue;
  assert.deepEqual(f.calls.at(-1), ["ack", "two", true]);
});
test("focus change during WASM startup never sends old keys to the new field", async () => {
  let resolve;
  const pending = new Promise(r => {resolve = r;});
  const f = fixture(() => pending);
  f.controller.key("meltype-rust", event, "old");
  await Promise.resolve();
  f.controller.focus({contextID: 8, type: "text"}); resolve(f.core);
  await f.controller.queue;
  assert.deepEqual(f.calls, [["ack", "old", false]]);
});
test("passwords, unknown fields, other engines and injected extension events pass through", () => {
  const f = fixture();
  assert.equal(f.controller.key("other", event, "a"), false);
  assert.equal(f.controller.key("meltype-rust", {...event, extensionId: "foreign"}, "b"), false);
  for (const type of ["password", "null", "number", "tel"]) {
    f.controller.focus({contextID: 7, type});
    assert.equal(f.controller.key("meltype-rust", event, "c"), false);
  }
  assert.equal(f.calls.length, 0);
});
test("failed API and rejected startup still acknowledge the key exactly once", async () => {
  const f = fixture(); f.ime.setComposition = async () => false;
  f.controller.key("meltype-rust", event, "failure"); await f.controller.queue;
  assert.equal(f.errors.length, 1);
  assert.deepEqual(f.calls.filter(c => c[0] === "ack"), [["ack", "failure", true]]);
  const failed = fixture(async () => {throw new Error("corrupt snapshot");});
  failed.controller.key("meltype-rust", event, "startup"); await failed.controller.queue;
  assert.deepEqual(failed.calls, [["ack", "startup", false]]);
});
test("commits are persisted except for contexts that forbid learning", async () => {
  const f = fixture();
  f.core.key = () => JSON.stringify({consumed: true, commits: [{text: "箸", deleteBefore: 0}], view: null});
  f.controller.key("meltype-rust", event, "saved"); await f.controller.queue;
  assert.equal(f.saved.length, 1);
  f.controller.focus({contextID: 7, type: "text", shouldDoLearning: false});
  f.controller.key("meltype-rust", event, "private"); await f.controller.queue;
  assert.equal(f.saved.length, 1);
});

test("new request ID takes priority, legacy ID is acknowledged, missing IDs pass through", async () => {
  const f = fixture();
  f.controller.key("meltype-rust", {...event, requestId: "old"}, "new");
  await f.controller.queue;
  assert.deepEqual(f.calls.filter(c => c[0] === "ack"), [["ack", "new", true]]);
  f.calls.length = 0;
  f.controller.key("meltype-rust", {...event, requestId: "legacy"});
  await f.controller.queue;
  assert.deepEqual(f.calls.filter(c => c[0] === "ack"), [["ack", "legacy", true]]);
  f.calls.length = 0;
  assert.equal(f.controller.key("meltype-rust", event), false);
  assert.deepEqual(f.calls, []);
});

test("prediction rows and clicks use prediction selection instead of conversion selection", async () => {
  const f = fixture();
  const reply = {consumed: true, commits: [], view: {text: "はし", converting: false,
    candidates: ["はし"], predictions: ["橋の近くで待っています"], selectedPrediction: -1}};
  f.core.key = f.core.view = () => JSON.stringify(reply);
  f.core.select_prediction = id => {assert.equal(id, 0); return JSON.stringify({consumed: true, commits: [{text: reply.view.predictions[id]}], view: null});};
  f.core.select = () => assert.fail("prediction click must not select a conversion candidate");
  f.controller.key("meltype-rust", event, "prediction"); await f.controller.queue;
  assert.equal(f.calls.find(c => c[0] === "setCandidates")[1].candidates[0].candidate, reply.view.predictions[0]);
  assert.equal(f.calls.some(c => c[0] === "setCursorPosition"), false);
  f.controller.candidate("meltype-rust", 0, "left"); await f.controller.queue;
  assert.equal(f.calls.find(c => c[0] === "commitText")[1].text, reply.view.predictions[0]);
});
