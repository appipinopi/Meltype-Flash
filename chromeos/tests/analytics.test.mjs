// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import assert from "node:assert/strict";
import test from "node:test";
import {eventPayload} from "../analytics.mjs";

test("analytics payload contains only the event and OS/app metadata", () => {
  const now = new Date("2026-11-09T12:34:56.789Z");
  const [event] = eventPayload("keyboard_ready", {
    platform: "cros", version: "1.1.2", locale: "ja", now, session: "session-1",
  });
  assert.equal(event.timestamp, "2026-11-09T12:34:56Z");
  assert.equal(event.sessionId, "session-1");
  assert.equal(event.eventName, "keyboard_ready");
  assert.equal(event.systemProps.osName, "ChromeOS");
  assert.equal(event.systemProps.appVersion, "1.1.2");
  assert.equal(event.systemProps.locale, "ja");
  const names = Object.keys(event.systemProps).sort().join(",");
  assert.equal(names, "appBuildNumber,appVersion,deviceModel,isDebug,locale,osName,osVersion,sdkVersion");
  assert.equal(JSON.stringify(event).includes("hashi"), false);
});

test("unknown platforms are passed through without inventing a version", () => {
  const [event] = eventPayload("options_open", {platform: "linux", version: "1.0.0"});
  assert.equal(event.systemProps.osName, "linux");
  assert.equal(event.systemProps.osVersion, "");
});
