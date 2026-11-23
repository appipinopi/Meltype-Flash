// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import assert from "node:assert/strict";
import test from "node:test";
import {parseZip,safeZipPath,sha256Hex,verifyZip} from "../update.mjs";

function storedZip(entries) {
  const encoder = new TextEncoder();
  const locals = [], centrals = [];
  let offset = 0;
  for (const [name, text] of Object.entries(entries)) {
    const nameBytes = encoder.encode(name), data = encoder.encode(text);
    const local = new Uint8Array(30 + nameBytes.length + data.length);
    const localView = new DataView(local.buffer);
    localView.setUint32(0, 0x04034b50, true);
    localView.setUint16(8, 0, true);
    localView.setUint32(18, data.length, true);
    localView.setUint32(22, data.length, true);
    localView.setUint16(26, nameBytes.length, true);
    local.set(nameBytes, 30); local.set(data, 30 + nameBytes.length);
    const central = new Uint8Array(46 + nameBytes.length);
    const centralView = new DataView(central.buffer);
    centralView.setUint32(0, 0x02014b50, true);
    centralView.setUint16(10, 0, true);
    centralView.setUint32(20, data.length, true);
    centralView.setUint32(24, data.length, true);
    centralView.setUint16(28, nameBytes.length, true);
    centralView.setUint32(42, offset, true);
    central.set(nameBytes, 46);
    locals.push(local); centrals.push(central); offset += local.length;
  }
  const directorySize = centrals.reduce((sum, entry) => sum + entry.length, 0);
  const eocd = new Uint8Array(22);
  const eocdView = new DataView(eocd.buffer);
  eocdView.setUint32(0, 0x06054b50, true);
  eocdView.setUint16(8, centrals.length, true);
  eocdView.setUint16(10, centrals.length, true);
  eocdView.setUint32(12, directorySize, true);
  eocdView.setUint32(16, offset, true);
  const total = new Uint8Array(offset + directorySize + eocd.length);
  let at = 0;
  for (const part of [...locals, ...centrals, eocd]) { total.set(part, at); at += part.length; }
  return total;
}

test("safe zip paths reject traversal and absolute paths", () => {
  assert.equal(safeZipPath("manifest.json"), true);
  assert.equal(safeZipPath("pkg/kanji.bin"), true);
  assert.equal(safeZipPath("../manifest.json"), false);
  assert.equal(safeZipPath("pkg/../../x"), false);
  assert.equal(safeZipPath("/manifest.json"), false);
  assert.equal(safeZipPath("C:/manifest.json"), false);
});

test("stored zip entries are read and sha256 verification rejects mismatches", async () => {
  const buffer = storedZip({"manifest.json": '{"version":"1.1.2"}', "pkg/kanji.bin": "abc"});
  const files = await parseZip(buffer);
  assert.equal(new TextDecoder().decode(files.get("manifest.json")), '{"version":"1.1.2"}');
  assert.equal(new TextDecoder().decode(files.get("pkg/kanji.bin")), "abc");
  const digest = await sha256Hex(buffer);
  await verifyZip(buffer, {zipDigest: `sha256:${digest}`});
  await assert.rejects(verifyZip(buffer, {zipDigest: `sha256:${"0".repeat(64)}`}), /SHA-256/);
});

test("SHA256SUMS.txt is used when the release asset has no digest", async () => {
  const buffer = storedZip({"manifest.json": "{}"});
  const digest = await sha256Hex(buffer);
  const fetchImpl = async () => new Response(`${digest}  Meltype-flash-ChromeOS.zip\n`);
  await verifyZip(buffer, {sumsUrl: "https://example.invalid/SHA256SUMS.txt"}, fetchImpl);
  await assert.rejects(verifyZip(buffer, {sumsUrl: "https://example.invalid/SHA256SUMS.txt"}, async () => new Response("deadbeef  other.zip\n")), /記載がありません/);
});
