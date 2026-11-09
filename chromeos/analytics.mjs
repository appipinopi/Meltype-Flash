// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Usage analytics for the ChromeOS extension. It sends only event names and
// the OS/app version; typed text, input history, dictionaries and settings are
// never included. Consent is required once, and cannot be turned off later.
export const APP_KEY = "A-US-4882313783";
export const API_URL = "https://us.aptabase.com/api/v0/events";
export const CONSENT_VERSION = 1;
const CONSENT_KEY = "analyticsConsentVersion";
const SDK_VERSION = "meltype-flash-web@1";

export async function analyticsAccepted() {
  if (!globalThis.chrome?.storage?.local) return false;
  const {[CONSENT_KEY]: version} = await chrome.storage.local.get(CONSENT_KEY);
  return Number(version) >= CONSENT_VERSION;
}

export async function acceptAnalytics() {
  await chrome.storage.local.set({[CONSENT_KEY]: CONSENT_VERSION});
}

/** Build the Aptabase event batch. Kept pure so tests can inspect every field. */
export function eventPayload(event, {platform = "", version = "0.0.0", locale = "", now = new Date(), session = ""} = {}) {
  return [{
    timestamp: now.toISOString().replace(/\.\d{3}Z$/, "Z"),
    sessionId: session || `${now.getTime()}${Math.floor(Math.random() * 100000)}`,
    eventName: String(event),
    systemProps: {
      isDebug: false,
      locale,
      osName: platform === "cros" ? "ChromeOS" : platform,
      osVersion: "",
      appVersion: version,
      appBuildNumber: version,
      sdkVersion: SDK_VERSION,
      deviceModel: "",
    },
  }];
}

async function sessionId() {
  if (globalThis.crypto?.randomUUID) return globalThis.crypto.randomUUID().replaceAll("-", "").slice(0, 19);
  return `${Date.now()}${Math.floor(Math.random() * 100000)}`;
}

export async function track(event) {
  try {
    if (!(await analyticsAccepted())) return false;
    const platform = await chrome.runtime.getPlatformInfo?.();
    const version = chrome.runtime.getManifest?.().version ?? "0.0.0";
    const locale = chrome.i18n?.getUILanguage?.() ?? "";
    const payload = eventPayload(event, {
      platform: platform?.os ?? "",
      version,
      locale,
      session: await sessionId(),
    });
    const response = await fetch(API_URL, {
      method: "POST",
      headers: {"Content-Type": "application/json", "App-Key": APP_KEY},
      body: JSON.stringify(payload),
    });
    return response.ok;
  } catch {
    // Analytics must never break input or settings.
    return false;
  }
}
