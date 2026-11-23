// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import {test} from "node:test";
import assert from "node:assert/strict";
import {SiteProfiles, defaultPreferences, validatePreferences, normalizeHost, matchMode} from "../site-profiles.mjs";

function fixture() {
  let saved = {sitePreferences: {...defaultPreferences(), enabled: true}}, queries = 0;
  const api = {storage: {local: {get: async () => saved, set: async value => {saved = value;}}},
    permissions: {contains: async () => true}, windows: {getLastFocused: async () => ({id: 4, focused: true, type: "normal"})},
    tabs: {query: async query => {assert.deepEqual(query, {active: true, windowId: 4}); queries++; return [{active: true, url: "https://vscode.dev/project"}];}}};
  return {api, profiles: new SiteProfiles(api), queries: () => queries};
}
test("site defaults are opt-in and URLs normalize to hosts without retaining paths", () => {
  assert.equal(defaultPreferences().enabled, false);
  assert.equal(normalizeHost("https://VSCODE.dev/project?secret=1#file"), "vscode.dev");
  assert.equal(normalizeHost("https://vscode.dev?secret=1#file"), "vscode.dev");
  assert.equal(normalizeHost("INSIDERS.vscode.dev."), "insiders.vscode.dev");
  for (const host of ["", "*.vscode.dev", "https://user:pass@vscode.dev", "file:///tmp/a", "javascript://alert(1)"]) {
    assert.throws(() => normalizeHost(host));
  }
  assert.throws(() => validatePreferences({...defaultPreferences(), rules: [{host: "VSCODE.dev", mode: "code"}, {host: "vscode.dev", mode: "normal"}]}));
  assert.throws(() => validatePreferences({...defaultPreferences(), rules: [{host: "vscode.dev", mode: "game"}]}));
});
test("host boundaries and most-specific overrides protect unrelated sites", () => {
  const preferences = defaultPreferences();
  preferences.rules.push({host: "docs.vscode.dev", mode: "normal"});
  for (const url of ["https://vscode.dev/", "https://insiders.vscode.dev", "https://github.dev/repo", "https://vscode.com/"]) {
    assert.equal(matchMode(preferences, url), "code");
  }
  for (const url of ["https://evilvscode.dev/", "https://vscode.dev.evil.example/", "https://docs.vscode.dev/", "chrome://settings", undefined]) {
    assert.equal(matchMode(preferences, url), "normal");
  }
});
test("only the foreground non-private browser editor uses site rules", async () => {
  const f = fixture();
  assert.equal(await f.profiles.resolve({type: "text", shouldDoLearning: false}), "code");
  assert.equal(f.queries(), 1);
  for (const type of ["url", "search", "email"]) assert.equal(await f.profiles.resolve({type}), "normal");
  assert.equal(f.queries(), 1);
  f.api.windows.getLastFocused = async () => ({id: 4, type: "normal", focused: false});
  assert.equal(await f.profiles.resolve({type: "text"}), "normal");
  assert.equal(f.queries(), 1);
  f.api.windows.getLastFocused = async () => ({id: 4, type: "normal", focused: true});
  for (const flags of [{incognito: true}, {pendingUrl: "https://other.example"}, {status: "loading"}, {active: false}]) {
    f.api.tabs.query = async () => [{active: true, url: "https://vscode.dev/", ...flags}];
    assert.equal(await f.profiles.resolve({type: "text"}), "normal");
  }
});
test("permission denial, revocation, API errors and missing APIs safely use default mode", async () => {
  const f = fixture();
  f.api.permissions.contains = async () => false;
  assert.equal(await f.profiles.resolve({type: "text"}), "normal");
  await assert.rejects(() => f.profiles.save({...defaultPreferences(), enabled: true}));
  await f.profiles.save({...defaultPreferences(), defaultMode: "code"});
  assert.equal(await f.profiles.resolve({type: "text"}), "code");
  f.api.permissions.contains = async () => true;
  await f.profiles.save({...defaultPreferences(), enabled: true});
  f.api.windows.getLastFocused = async () => {throw new Error("window closed");};
  assert.equal(await f.profiles.resolve({type: "text"}), "normal");
  delete f.api.windows;
  assert.equal(await f.profiles.resolve({type: "text"}), "normal");
});
test("saved preferences are reloaded after invalidation and malformed storage is preserved", async () => {
  const f = fixture();
  assert.equal(await f.profiles.resolve({type: "text"}), "code");
  f.api.storage.local.get = async () => ({sitePreferences: {...defaultPreferences(), defaultMode: "code"}});
  f.profiles.invalidate();
  assert.equal(await f.profiles.resolve({type: "email"}), "code");
  f.api.storage.local.get = async () => ({sitePreferences: {corrupt: true}});
  f.profiles.invalidate();
  await assert.rejects(() => f.profiles.preferences());
});
