// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Update release discovery for the ChromeOS extension. The pure helpers are
// testable in Node; the Chrome APIs are touched only by the wrappers.
export const REPO = "appipinopi/Meltype-Flash";
export const RELEASES_API = `https://api.github.com/repos/${REPO}/releases?per_page=20`;
export const ORIGIN = "https://api.github.com/*";

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
  return chrome.permissions.contains({origins: [ORIGIN]});
}

export async function requestUpdatePermission() {
  return chrome.permissions.request({origins: [ORIGIN]});
}

export async function cachedUpdate() {
  const {updateCheck} = await chrome.storage.local.get("updateCheck");
  return updateCheck ?? null;
}
