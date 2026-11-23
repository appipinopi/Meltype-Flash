// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// WASM inference runs in a dedicated worker, never on the IME event loop.
const worker = new Worker("ai-worker.mjs", {type: "module"});
const pending = new Map(); let next = 1;
worker.onmessage = ({data}) => {
  const resolve = pending.get(data.id); pending.delete(data.id); resolve?.(data);
};
worker.onerror = () => {
  for (const resolve of pending.values()) resolve({error: "shuntiの実行環境が終了しました"});
  pending.clear();
};
chrome.runtime.onMessage.addListener((message, sender, respond) => {
  if (sender.id !== chrome.runtime.id || sender.url && sender.url !== chrome.runtime.getURL("background.mjs") || message.channel !== "meltype-shunti") return false;
  if (!["prepare", "convert"].includes(message.action)) return false;
  const id = next++;
  pending.set(id, respond); worker.postMessage({...message, id});
  return true;
});
