// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import {test} from "node:test";
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";

// 設定画面の実モジュールを、DOMとChrome通信の最小限の代替で動かす。
class Element {
  value = ""; checked = false; disabled = false; textContent = "";
  children = []; options = []; dataset = {}; listeners = new Map();
  addEventListener(type, callback) { this.listeners.set(type, callback); }
  append(...items) { this.children.push(...items); this.options.push(...items); }
  replaceChildren(...items) { this.children = items; }
  async fire(type) { await this.listeners.get(type)?.({preventDefault() {}}); }
}
let serial = 0;
async function page(t, {available = true} = {}) {
  const oldDocument = globalThis.document, oldChrome = globalThis.chrome;
  t.after(() => { globalThis.document = oldDocument; globalThis.chrome = oldChrome; });
  const html = readFileSync(new URL("../options.html", import.meta.url), "utf8");
  const elements = new Map([...html.matchAll(/\bid="([^"]+)"/g)].map(([, id]) => [`#${id}`, new Element()]));
  const fields = [...html.matchAll(/<(input|select)\b([^>]*\bdata-setting="([^"]+)"[^>]*)>/g)].map(([, tag, attrs, key]) => {
    const field = new Element(); field.dataset.setting = key;
    field.type = tag === "input" ? "checkbox" : "select-one";
    if (tag === "select") {
      const contents = html.slice(html.indexOf(`<select${attrs}>`) + `<select${attrs}>`.length).split("</select>")[0];
      field.options = [...contents.matchAll(/value="([^"]+)"/g)].map(([, value]) => ({value}));
    }
    return field;
  });
  elements.get("#core-controls").disabled = true;
  globalThis.document = {
    querySelector: selector => { assert(elements.has(selector), selector); return elements.get(selector); },
    querySelectorAll: selector => selector === "[data-setting]" ? fields : [elements.get("#export"), elements.get("#clear")],
    createElement: () => new Element(),
  };
  let saved = {...JSON.parse(readFileSync(new URL("../../rust/core/data/settings-defaults.json", import.meta.url))),
    FutureOption: {keep: true}, AppProfile: "General"};
  const writes = [];
  let failure;
  globalThis.chrome = {runtime: {sendMessage: async message => {
    if (message.op === "site_preferences") return {preferences: {enabled: false, defaultMode: "normal", rules: []}};
    if (message.op === "status") return {available, diagnostic: available ? "ready" : "unavailable"};
    if (message.op === "snapshot") return {snapshot: JSON.stringify({settings: saved})};
    if (message.op === "configure") {
      if (failure) return {error: failure};
      saved = JSON.parse(message.settings); writes.push(structuredClone(saved)); return {saved: true};
    }
    assert.fail(message.op);
  }}};
  await import(`../options.mjs?test=${serial++}`);
  await new Promise(resolve => setImmediate(resolve));
  return {elements, fields: Object.fromEntries(fields.map(field => [field.dataset.setting, field])), writes,
    saved: () => saved, fail: message => { failure = message; }};
}

test("入力設定と詳細JSONを同期し、未知の設定・非表示の設定を保存後も保持する", async t => {
  const p = await page(t), get = id => p.elements.get(id);
  assert.equal(Object.keys(p.fields).length, 6);
  assert.equal(get("#core-controls").disabled, false);
  assert.equal(p.fields.DetectionLevel.value, "Balanced");
  assert.equal(p.fields.LearningEnabled.checked, true);
  p.fields.TabConversion.checked = false;
  await p.fields.TabConversion.fire("change");
  p.fields.Punctuation.value = "FullWidthCommaPeriod";
  await p.fields.Punctuation.fire("change");
  assert.equal(JSON.parse(get("#settings").value).TabConversion, false);
  assert.equal(p.writes.length, 0);
  const json = JSON.parse(get("#settings").value);
  json.DetectionLevel = "Conservative";
  json.LearningEnabled = false;
  json.AppProfile = "do not replace hidden settings";
  get("#settings").value = JSON.stringify(json);
  await get("#settings").fire("input");
  assert.equal(p.fields.DetectionLevel.value, "Conservative");
  assert.equal(p.fields.LearningEnabled.checked, false);
  // 画面を開いた後に変更された非表示設定も、保存直前の値を使う。
  p.saved().AppProfile = "Code";
  await get("#settings-form").fire("submit");
  assert.equal(p.writes.length, 1);
  assert.equal(p.saved().AppProfile, "Code");
  assert.deepEqual(p.saved().FutureOption, {keep: true});
  assert.equal(p.saved().Punctuation, "FullWidthCommaPeriod");
  assert.equal(p.saved().TabConversion, false);
  assert.equal("AppProfile" in JSON.parse(get("#settings").value), false);
  p.fields.DetectionLevel.value = "Aggressive";
  await p.fields.DetectionLevel.fire("change");
  await get("#reload").fire("click");
  assert.equal(p.fields.DetectionLevel.value, "Conservative");
  assert.equal(p.fields.LearningEnabled.checked, false);
});

test("不正なJSONと保存エラーで設定を失わず、再試行できる", async t => {
  const p = await page(t), get = id => p.elements.get(id);
  for (const invalid of ["{", "[]", "null"]) {
    get("#settings").value = invalid;
    await get("#settings-form").fire("submit");
    assert.equal(p.writes.length, 0);
    assert.equal(get("#core-controls").disabled, false);
    assert.notEqual(get("#status").textContent, "設定を保存しました。");
  }
  await get("#reload").fire("click");
  p.fields.CorrectTypos.checked = false;
  await p.fields.CorrectTypos.fire("change");
  p.fail("storage failed");
  await get("#settings-form").fire("submit");
  assert.match(get("#status").textContent, /storage failed/);
  assert.equal(JSON.parse(get("#settings").value).CorrectTypos, false);
  assert.equal(get("#core-controls").disabled, false);
  p.fail(undefined);
  await get("#settings-form").fire("submit");
  assert.equal(p.saved().CorrectTypos, false);
});

test("IMEが利用できない環境では入力設定と学習の操作を無効にする", async t => {
  const p = await page(t, {available: false});
  for (const id of ["#core-controls", "#export", "#clear"]) assert.equal(p.elements.get(id).disabled, true);
  assert.equal(p.elements.get("#site-enabled").disabled, false);
  assert.equal(p.writes.length, 0);
});

test("AI変換の設定は詳細JSONから除き、通常の予測と未知の設定は保持する", async t => {
  const p = await page(t), get = id => p.elements.get(id);
  Object.assign(p.saved(), {UseZenzai: true, ZenzaiServerPath: "server", ZenzaiModelPath: "model"});
  p.saved().Profiles = [{Name: "移行", Values: {UseZenzai: true, ZenzaiModelPath: "model", PredictiveCandidates: false}}];
  await get("#reload").fire("click");
  const displayed = JSON.parse(get("#settings").value);
  assert.equal("UseZenzai" in displayed, false);
  assert.equal("ZenzaiModelPath" in displayed.Profiles[0].Values, false);
  Object.assign(displayed, {UseZenzai: true, ZenzaiServerPath: "server", ZenzaiModelPath: "model"});
  displayed.Profiles[0].Values.UseZenzai = true;
  get("#settings").value = JSON.stringify(displayed);
  await get("#settings-form").fire("submit");
  for (const key of ["UseZenzai", "ZenzaiServerPath", "ZenzaiModelPath"]) {
    assert.equal(key in p.saved(), false);
    assert.equal(key in p.saved().Profiles[0].Values, false);
  }
  assert.equal(p.saved().Profiles[0].Values.PredictiveCandidates, false);
  assert.deepEqual(p.saved().FutureOption, {keep: true});
});
