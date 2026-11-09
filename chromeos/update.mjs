// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Update release discovery for the ChromeOS extension. The pure helpers are
// testable in Node; the Chrome APIs are touched only by the wrappers.
export const REPO = "appipinopi/Meltype-Flash";
export const RELEASES_API = `https://api.github.com/repos/${REPO}/releases?per_page=20`;
export const ORIGIN = "https://api.github.com/*";
export const DOWNLOAD_ORIGINS = [ORIGIN, "https://github.com/*", "https://release-assets.githubusercontent.com/*"];

export function versionParts(version) {
  return String(version).replace(/^flash-v/, "").split(".").map(value => Number.parseInt(value, 10) || 0);
}

export function newerVersion(candidate, current) {
  const left = versionParts(candidate), right = versionParts(current);
  for (let index = 0; index < Math.max(left.length, right.length); index += 1) {
    if ((left[index] ?? 0) !== (right[index] ?? 0)) return (left[index] ?? 0) > (right[index] ?? 0);
  }
  return false;
}

/** Choose the newest published release that carries the ChromeOS package. */
export function newestChromeRelease(releases) {
  let best = null;
  for (const release of releases ?? []) {
    if (!release || release.draft || release.prerelease) continue;
    const assets = Array.isArray(release.assets) ? release.assets : [];
    const zip = assets.find(asset => asset?.name === "Meltype-flash-ChromeOS.zip");
    if (!zip || typeof release.tag_name !== "string") continue;
    if (!best || newerVersion(release.tag_name, best.tagName)) {
      best = {
        tagName: release.tag_name,
        version: release.tag_name.replace(/^flash-v/, ""),
        pageUrl: release.html_url,
        zipUrl: zip.browser_download_url,
        zipDigest: zip.digest ?? null,
        sumsUrl: assets.find(asset => asset?.name === "SHA256SUMS.txt")?.browser_download_url ?? null,
        crxUrl: assets.find(asset => asset?.name === "Meltype-flash-ChromeOS.crx")?.browser_download_url ?? null,
      };
    }
  }
  return best;
}

export async function checkForUpdate({fetchImpl = fetch, current = globalThis.chrome?.runtime?.getManifest?.().version ?? "0.0.0"} = {}) {
  const response = await fetchImpl(RELEASES_API, {headers: {Accept: "application/vnd.github+json"}});
  if (!response.ok) throw new Error(`更新の確認に失敗しました (${response.status})`);
  const release = newestChromeRelease(await response.json());
  if (!release) throw new Error("ChromeOS版の配布物が見つかりません");
  return {...release, current, update: newerVersion(release.version, current), checkedAt: Date.now()};
}

export async function hasUpdatePermission() {
  return chrome.permissions.contains({origins: DOWNLOAD_ORIGINS});
}

export async function requestUpdatePermission() {
  return chrome.permissions.request({origins: DOWNLOAD_ORIGINS});
}

export async function cachedUpdate() {
  const {updateCheck} = await chrome.storage.local.get("updateCheck");
  return updateCheck ?? null;
}

/** Reject absolute paths, parent traversal, and obvious archive bombs. */
export function safeZipPath(name) {
  const text = String(name).replaceAll("\\", "/");
  if (!text || text.startsWith("/") || /^[A-Za-z]:/.test(text)) return false;
  const parts = text.split("/");
  return parts.every(part => part && part !== "." && part !== "..");
}

const EOCD = 0x06054b50, CENTRAL = 0x02014b50, LOCAL = 0x04034b50;

function u16(view, at) { return view.getUint16(at, true); }
function u32(view, at) { return view.getUint32(at, true); }

/** Parse a ZIP archive. Supports stored and deflate entries; rejects unsafe names. */
export async function parseZip(buffer) {
  if (!(buffer instanceof ArrayBuffer)) {
    buffer = buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength);
  }
  const view = new DataView(buffer);
  let eocd = -1;
  for (let at = buffer.byteLength - 22; at >= Math.max(0, buffer.byteLength - 65557); at -= 1) {
    if (u32(view, at) === EOCD) { eocd = at; break; }
  }
  if (eocd < 0) throw new Error("ZIPの終端が見つかりません");
  const count = u16(view, eocd + 10), directoryOffset = u32(view, eocd + 16);
  const files = new Map();
  let at = directoryOffset, total = 0;
  for (let index = 0; index < count; index += 1) {
    if (u32(view, at) !== CENTRAL) throw new Error("ZIPの目次が壊れています");
    const method = u16(view, at + 10);
    const compressedSize = u32(view, at + 20), uncompressedSize = u32(view, at + 24);
    const nameLength = u16(view, at + 28), extraLength = u16(view, at + 30), commentLength = u16(view, at + 32);
    const localOffset = u32(view, at + 42);
    const name = new TextDecoder().decode(new Uint8Array(buffer, at + 46, nameLength));
    at += 46 + nameLength + extraLength + commentLength;
    if (name.endsWith("/")) continue;
    if (!safeZipPath(name)) throw new Error(`安全でないパスが含まれています: ${name}`);
    total += uncompressedSize;
    if (total > 256 * 1024 * 1024 || compressedSize > 128 * 1024 * 1024) throw new Error("ZIPが大きすぎます");
    if (u32(view, localOffset) !== LOCAL) throw new Error(`ZIPの項目が壊れています: ${name}`);
    const localName = u16(view, localOffset + 26), localExtra = u16(view, localOffset + 28);
    const start = localOffset + 30 + localName + localExtra;
    const raw = buffer.slice(start, start + compressedSize);
    if (method === 0) files.set(name, new Uint8Array(raw));
    else if (method === 8) {
      const stream = new Blob([raw]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
      files.set(name, new Uint8Array(await new Response(stream).arrayBuffer()));
    } else throw new Error(`未対応のZIP圧縮方式です: ${method}`);
  }
  return files;
}

export async function sha256Hex(buffer) {
  const digest = await globalThis.crypto.subtle.digest("SHA-256", buffer);
  return [...new Uint8Array(digest)].map(byte => byte.toString(16).padStart(2, "0")).join("");
}

/** Verify the ZIP against the release asset digest or SHA256SUMS.txt. */
export async function verifyZip(buffer, update, fetchImpl = fetch) {
  const expected = String(update?.zipDigest ?? "").replace(/^sha256:/i, "").toLowerCase();
  if (expected) {
    if (await sha256Hex(buffer) !== expected) throw new Error("更新ファイルのSHA-256が一致しません");
    return;
  }
  if (!update?.sumsUrl) throw new Error("更新ファイルのハッシュを確認できません");
  const response = await fetchImpl(update.sumsUrl);
  if (!response.ok) throw new Error(`SHA256SUMS.txt を取得できません (${response.status})`);
  const line = (await response.text()).split(/\r?\n/).find(row => /(^|\s)Meltype-flash-ChromeOS\.zip$/.test(row.trim()));
  const wanted = line?.trim().split(/\s+/)[0]?.toLowerCase();
  if (!wanted || wanted.length !== 64) throw new Error("SHA256SUMS.txt にChromeOS版の記載がありません");
  if (await sha256Hex(buffer) !== wanted) throw new Error("更新ファイルのSHA-256が一致しません");
}

/** Download and verify the release ZIP, then return its files. */
export async function downloadUpdate(update, fetchImpl = fetch) {
  if (!update?.zipUrl) throw new Error("ChromeOS版のZIPがありません");
  const response = await fetchImpl(update.zipUrl);
  if (!response.ok) throw new Error(`ZIPを取得できません (${response.status})`);
  const buffer = await response.arrayBuffer();
  await verifyZip(buffer, update, fetchImpl);
  return parseZip(buffer);
}

/** Write the extracted files into the user-selected extension folder. */
export async function writeUpdate(files, directory) {
  for (const [path, bytes] of files) {
    const parts = path.split("/");
    let folder = directory;
    for (const part of parts.slice(0, -1)) folder = await folder.getDirectoryHandle(part, {create: true});
    const file = await folder.getFileHandle(parts.at(-1), {create: true});
    const writable = await file.createWritable();
    await writable.write(bytes);
    await writable.close();
  }
}
