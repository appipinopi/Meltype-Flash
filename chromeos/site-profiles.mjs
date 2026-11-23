// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
const modes = new Set(["normal", "code"]);
export const defaultPreferences = () => ({enabled: false, defaultMode: "normal", rules:
  ["vscode.com", "vscode.dev", "github.dev"].map(host => ({host, mode: "code"}))});

export function normalizeHost(value) {
  if (typeof value !== "string" || !value.trim() || value.length > 2048) throw new Error("サイトのホスト名を入力してください。");
  const input = value.trim();
  let url;
  try { url = new URL(input.includes("://") ? input : `https://${input}`); }
  catch { throw new Error("サイトのホスト名またはhttps URLを入力してください。"); }
  if (!["http:", "https:"].includes(url.protocol) || url.username || url.password
      || !url.hostname || url.hostname.includes("*") || /[\s\\]/.test(input)) {
    throw new Error("http/httpsのサイトを指定してください。ワイルドカードや認証情報は使えません。");
  }
  return url.hostname.toLowerCase().replace(/\.$/, "");
}

export function validatePreferences(value) {
  if (!value || typeof value.enabled !== "boolean" || !modes.has(value.defaultMode)
      || !Array.isArray(value.rules) || value.rules.length > 100) throw new Error("入力モードの設定が不正です（サイトは100件まで）。");
  const seen = new Set();
  const rules = value.rules.map(rule => {
    const host = normalizeHost(rule.host);
    if (!modes.has(rule.mode)) throw new Error("入力モードを選択してください。");
    if (seen.has(host)) throw new Error(`サイトが重複しています: ${host}`);
    seen.add(host);
    return {host, mode: rule.mode};
  });
  return {enabled: value.enabled, defaultMode: value.defaultMode, rules};
}

export function matchMode(preferences, url) {
  let parsed;
  try { parsed = new URL(url); } catch { return preferences.defaultMode; }
  if (!["http:", "https:"].includes(parsed.protocol)) return preferences.defaultMode;
  const host = parsed.hostname.toLowerCase().replace(/\.$/, "");
  const rule = preferences.rules.filter(rule => host === rule.host || host.endsWith(`.${rule.host}`))
    .sort((a, b) => b.host.length - a.host.length)[0];
  return rule?.mode ?? preferences.defaultMode;
}

export class SiteProfiles {
  constructor(api) { this.api = api; }
  invalidate() { this.loading = null; }
  preferences() {
    this.loading ??= this.api.storage.local.get("sitePreferences").then(saved =>
      saved.sitePreferences === undefined ? defaultPreferences() : validatePreferences(saved.sitePreferences));
    return this.loading;
  }
  async save(value) {
    const preferences = validatePreferences(value);
    if (preferences.enabled && !await this.api.permissions?.contains({permissions: ["tabs"]})) {
      throw new Error("サイトごとの自動切替には、設定画面でタブの参照を許可してください。");
    }
    await this.api.storage.local.set({sitePreferences: preferences});
    this.invalidate();
    return preferences;
  }
  async resolve(context) {
    const preferences = await this.preferences();
    // URL/search/email fields and private contexts must never inherit editor rules.
    if (!preferences.enabled || context?.type !== "text"
        || this.api.extension?.inIncognitoContext || !this.api.windows || !this.api.tabs) return preferences.defaultMode;
    try {
      if (!await this.api.permissions?.contains({permissions: ["tabs"]})) return preferences.defaultMode;
      const window = await this.api.windows.getLastFocused();
      if (!window.focused || !["normal", "popup"].includes(window.type)) return preferences.defaultMode;
      const [tab] = await this.api.tabs.query({active: true, windowId: window.id});
      if (!tab?.active || tab.incognito || tab.pendingUrl || tab.status === "loading") return preferences.defaultMode;
      return matchMode(preferences, tab.url);
    } catch { return preferences.defaultMode; }
  }
}
