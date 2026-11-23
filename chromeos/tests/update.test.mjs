// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import test from "node:test";
import assert from "node:assert/strict";
import {newerVersion, newestChromeRelease, versionParts} from "../update.mjs";

test("versions compare numerically, not as strings", () => {
  assert.deepEqual(versionParts("flash-v1.10.0"), [1, 10, 0]);
  assert.equal(newerVersion("flash-v1.10.0", "1.9.9"), true);
  assert.equal(newerVersion("1.1.10", "1.1.9"), true);
  assert.equal(newerVersion("1.1.0", "1.1.0"), false);
  assert.equal(newerVersion("1.0.9", "1.0.10"), false);
});

test("only the newest published release with a ChromeOS package is chosen", () => {
  const release = newestChromeRelease([
    {tag_name: "flash-v1.2.0", draft: true, assets: [{name: "Meltype-flash-ChromeOS.zip", browser_download_url: "draft"}]},
    {tag_name: "flash-v1.1.2", prerelease: true, assets: [{name: "Meltype-flash-ChromeOS.zip", browser_download_url: "pre"}]},
    {tag_name: "flash-v1.1.0", assets: [{name: "Meltype-flash-Windows-x64.zip", browser_download_url: "win"}]},
    {
      tag_name: "flash-v1.1.1",
      html_url: "https://example.test/1.1.1",
      assets: [
        {name: "Meltype-flash-ChromeOS.zip", browser_download_url: "zip-1.1.1"},
        {name: "Meltype-flash-ChromeOS.crx", browser_download_url: "crx-1.1.1"},
      ],
    },
    {tag_name: "flash-v1.0.10", assets: [{name: "Meltype-flash-ChromeOS.zip", browser_download_url: "zip-1.0.10"}]},
  ]);
  assert.deepEqual(release, {
    tagName: "flash-v1.1.1",
    version: "1.1.1",
    pageUrl: "https://example.test/1.1.1",
    zipUrl: "zip-1.1.1",
    zipDigest: null,
    sumsUrl: null,
    crxUrl: "crx-1.1.1",
  });
});

test("no release with a ChromeOS package returns null", () => {
  assert.equal(newestChromeRelease([{tag_name: "flash-v1.1.0", assets: []}]), null);
  assert.equal(newestChromeRelease([]), null);
});
