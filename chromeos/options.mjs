// SPDX-License-Identifier: GPL-3.0-or-later
import {validatePreferences} from "./site-profiles.mjs";
const status = document.querySelector("#status"), settings = document.querySelector("#settings");
const siteStatus = document.querySelector("#site-status"), siteEnabled = document.querySelector("#site-enabled");
const rules = document.querySelector("#site-rules"), defaultMode = document.querySelector("#default-mode");
const desktopOnly = new Set(["AppRules", "AppKinds", "AppProfile", "StopInGames"]);
const neuralSettings = ["UseZenzai", "ZenzaiServerPath", "ZenzaiModelPath"];
function withoutNeuralSettings(value) {
  for (const key of neuralSettings) delete value[key];
  if (Array.isArray(value.Profiles)) for (const profile of value.Profiles) {
    if (profile?.Values && typeof profile.Values === "object") {
      for (const key of neuralSettings) delete profile.Values[key];
    }
  }
  return value;
}
let savedSettings = {};
const coreControls = document.querySelector("#core-controls");
const fields = [...document.querySelectorAll("[data-setting]")];
function parseSettings() {
  const value = JSON.parse(settings.value);
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("設定はJSONオブジェクトで入力してください。");
  return withoutNeuralSettings(value);
}
function displayFields(value) {
  for (const field of fields) {
    const current = value[field.dataset.setting];
    if (field.type === "checkbox") field.checked = current === true;
    else {
      // 未知の選択肢も保持し、他の項目を変更しただけで既存設定を失わない。
      if (typeof current === "string" && ![...field.options].some(option => option.value === current)) {
        const option = document.createElement("option"); option.value = current; option.textContent = current; field.append(option);
      }
      field.value = current ?? "";
    }
  }
}
for (const field of fields) field.addEventListener("change", () => {
  try {
    const value = parseSettings();
    value[field.dataset.setting] = field.type === "checkbox" ? field.checked : field.value;
    settings.value = JSON.stringify(value, null, 2);
    status.textContent = "入力設定を変更しました。「入力設定を保存」で反映します。";
  } catch (error) {
    displayFields(savedSettings);
    status.textContent = `詳細設定のJSONを修正するか、保存済み設定へ戻してください。${error}`;
  }
});
settings.addEventListener("input", () => {
  try { displayFields(parseSettings()); } catch { /* 編集途中のJSONは保存時に検証する。 */ }
});
async function loadSettings() {
  const {snapshot} = await request({op: "snapshot"});
  savedSettings = withoutNeuralSettings(JSON.parse(snapshot).settings);
  settings.value = JSON.stringify(Object.fromEntries(Object.entries(savedSettings).filter(([key]) => !desktopOnly.has(key))), null, 2);
  displayFields(savedSettings);
  coreControls.disabled = false;
}
function addRule(rule = {host: "", mode: "code"}) {
  const row = document.createElement("div"); row.className = "site-rule";
  const host = document.createElement("input");
  host.type = "text"; host.value = rule.host; host.placeholder = "例: vscode.dev";
  host.setAttribute("aria-label", "サイトのホスト名またはURL"); host.required = true;
  const mode = document.createElement("select"); mode.setAttribute("aria-label", "このサイトの入力モード");
  for (const [value, label] of [["normal", "通常入力"], ["code", "コード入力"]]) {
    const option = document.createElement("option"); option.value = value; option.textContent = label; mode.append(option);
  }
  mode.value = rule.mode;
  const remove = document.createElement("button"); remove.type = "button"; remove.textContent = "削除";
  remove.addEventListener("click", () => row.remove());
  row.append(host, mode, remove); rules.append(row);
}
function displayPreferences(preferences) {
  siteEnabled.checked = preferences.enabled; defaultMode.value = preferences.defaultMode;
  rules.replaceChildren(); preferences.rules.forEach(addRule);
}
document.querySelector("#site-add").addEventListener("click", () => {
  addRule(); rules.lastElementChild.querySelector("input").focus();
});
siteEnabled.addEventListener("change", async () => {
  if (!siteEnabled.checked) return;
  try {
    // Keep request() in the user gesture; do not await a background message first.
    if (!await chrome.permissions.request({permissions: ["tabs"]})) {
      siteEnabled.checked = false;
      siteStatus.textContent = "許可されませんでした。通常使うモードは権限なしでも利用できます。";
    } else siteStatus.textContent = "タブの参照を許可しました。「入力モードを保存」で反映します。";
  } catch (error) { siteEnabled.checked = false; siteStatus.textContent = String(error); }
});
document.querySelector("#site-form").addEventListener("submit", async event => {
  event.preventDefault();
  try {
    const preferences = validatePreferences({enabled: siteEnabled.checked, defaultMode: defaultMode.value,
      rules: [...rules.children].map(row => ({host: row.querySelector("input").value, mode: row.querySelector("select").value}))});
    const reply = await request({op: "save_site_preferences", preferences});
    displayPreferences(reply.preferences);
    siteStatus.textContent = "入力モードを保存しました。入力中の文字列がある場合は、確定後に切り替わります。";
  } catch (error) { siteStatus.textContent = String(error); }
});
async function request(message) {
  const reply = await chrome.runtime.sendMessage(message);
  if (reply?.error) throw new Error(reply.error);
  return reply;
}
async function show(work) {
  try { await work(); } catch (error) { status.textContent = String(error); }
}
async function reload() {
  const {preferences} = await request({op: "site_preferences"});
  displayPreferences(preferences);
  const state = await request({op: "status"});
  status.textContent = state.diagnostic;
  if (!state.available) {
    coreControls.disabled = true;
    document.querySelectorAll("#export, #clear").forEach(element => { element.disabled = true; });
    return;
  }
  await loadSettings();
}
document.querySelector("#reload").addEventListener("click", () => show(async () => {
  await loadSettings(); status.textContent = "保存済みの入力設定へ戻しました。";
}));
document.querySelector("#settings-form").addEventListener("submit", event => {
  event.preventDefault();
  return show(async () => {
    const value = parseSettings();
    coreControls.disabled = true;
    try {
      const {snapshot} = await request({op: "snapshot"});
      const latest = JSON.parse(snapshot).settings;
      const hidden = Object.fromEntries(Object.entries(latest).filter(([key]) => desktopOnly.has(key)));
      await request({op: "configure", settings: JSON.stringify({...value, ...hidden})});
      await loadSettings();
    } finally { coreControls.disabled = false; }
    status.textContent = "設定を保存しました。";
  });
});
document.querySelector("#clear").addEventListener("click", () => show(async () => {
  if (!confirm("学習履歴を削除します。ユーザー辞書と設定は保持します。")) return;
  await request({op: "clear_learning"});
  status.textContent = "学習を削除しました。";
}));
document.querySelector("#export").addEventListener("click", () => show(async () => {
  const {snapshot} = await request({op: "snapshot"});
  const url = URL.createObjectURL(new Blob([snapshot], {type: "application/json"}));
  const link = document.createElement("a");
  link.href = url; link.download = "meltype-chromeos-snapshot.json"; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  status.textContent = "書き出しには個人の辞書・学習を含みます。";
}));
show(reload);
