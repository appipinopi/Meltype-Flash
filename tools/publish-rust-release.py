#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""全OS・測定値・本文画像を検証して、リリースの下書きを作る。"""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

REPO = "appipinopi/Meltype-Flash"
MEDIA_BRANCH = "release-media"
COMBINED_IMAGES = ("speed-32-64.png", "speed-128-256.png", "speed-short-edit.png", "cpu.png", "ram.png")
RUST_IMAGES = tuple("rust-" + name for name in COMBINED_IMAGES)
IMAGES = RUST_IMAGES + COMBINED_IMAGES
PACKAGES = (
    "Meltype-flash-Windows-x64.zip", "Meltype-flash-Linux-x64.tar.gz",
    "Meltype-flash-macOS-arm64.zip", "Meltype-flash-macOS-x64.zip",
    "Meltype-flash-Android-debug.apk", "Meltype-flash-ChromeOS.zip",
    "Meltype-flash-iOS-unsigned.ipa", "Meltype-flash-iOS-Simulator-arm64.zip",
    "Meltype-flash-iOS-source.zip",
)
DOCUMENTS = ("LICENSE", "THIRD-PARTY-NOTICES.md", "SHA256SUMS.txt")


def gh(*args, payload=None):
    command = ["gh", *args]
    if payload is not None:
        command += ["--input", "-"]
    result = subprocess.run(command, input=json.dumps(payload) if payload is not None else None,
                            text=True, encoding="utf-8", capture_output=True, check=True)
    return result.stdout


def api(path, payload=None, method=None):
    args = ["api", f"repos/{REPO}/{path}"]
    if method:
        args += ["--method", method]
    return json.loads(gh(*args, payload=payload))


def find_release(tag):
    # タグ未作成の下書きは /releases/tags/{tag} では取得できない。
    # 認証済みの一覧を全ページ検索し、既存の下書きも重複作成しない。
    pages = json.loads(gh("api", f"repos/{REPO}/releases?per_page=100", "--paginate", "--slurp"))
    return next((release for page in pages for release in page if release["tag_name"] == tag), None)


def normalize_markdown(text):
    text = text.replace("\r", "")
    text = re.sub(r"(?m)^(\|[^\n]*\|)\n+(?=\|)", r"\1\n", text)
    return re.sub(r"\n{3,}", "\n\n", text).strip() + "\n"


def validate_assets(directory):
    expected = set(PACKAGES + DOCUMENTS)
    if {p.name for p in directory.iterdir()} != expected:
        raise ValueError("配布ファイルが不足、または不要な添付が混入しています")
    if any(not (directory / name).is_file() or not (directory / name).stat().st_size for name in expected):
        raise ValueError("空の配布ファイルがあります")
    sums = dict(line.split(None, 1)[::-1] for line in (directory / "SHA256SUMS.txt").read_text(encoding="utf-8-sig").splitlines())
    if not set(PACKAGES) <= sums.keys():
        raise ValueError("全OSのチェックサムが必要です")
    for name in PACKAGES:
        if hashlib.sha256((directory / name).read_bytes()).hexdigest() != sums[name]:
            raise ValueError(f"配布ファイルのSHA256不一致: {name}")


def validate_benchmark(directory, sha):
    report = json.loads((directory / "report.json").read_text(encoding="utf-8-sig"))
    if report.get("comparison_kind") != "rust-versions" or report.get("new_commit") != sha:
        raise ValueError("ビルド元とベンチマークのコミットが一致しません")
    if not all(report.get(key) is True for key in ("binaries_unchanged", "sources_unchanged", "accepted_performance_baseline")):
        raise ValueError("測定前後の整合性検査が必要です")
    if not report.get("results") or not report.get("process_resources"):
        raise ValueError("ベンチマークの測定値がありません")
    if not re.fullmatch(r"[0-9a-f]{40}", report.get("old_commit", "")):
        raise ValueError("以前の版のコミットSHAが不正です")
    summary = json.loads((directory / "charts/summary.json").read_text(encoding="utf-8-sig"))
    if summary.get("full_report_sha256") != hashlib.sha256((directory / "report.json").read_bytes()).hexdigest():
        raise ValueError("箱ひげ図と測定レポートが一致しません")
    for name in IMAGES:
        if not (directory / "charts" / name).read_bytes().startswith(b"\x89PNG\r\n\x1a\n"):
            raise ValueError(f"箱ひげ図がありません: {name}")
    notes = (directory / "charts/README.md").read_text(encoding="utf-8-sig")
    if "## 全入力の中央値" not in notes or "## CPU・RAMのプロセス中央値" not in notes or "## 全画像" not in notes:
        raise ValueError("測定結果の本文がありません")
    return report, normalize_markdown(notes.split("## 全画像")[0])


def compose_body(tag, sha, benchmark, media_sha, changes):
    match = re.fullmatch(r"flash-v(\d+\.\d+\.\d+)-preview\.(\d+)", tag)
    if not match:
        raise ValueError("Flashのプレビュータグが必要です")
    title = f"Meltype-flash v{match[1]} Pre{match[2]}"
    rows = [f"# {title}", "", "共通Rustコアを使う開発プレビューです。ビルド成功と各OSの実機確認は別です。", "",
            "## 変更内容", ""]
    # コミット件名は命令として実行せず、コード表記のテキストとして載せる。
    rows += ["- `" + line.replace("`", "'").replace("<", "&lt;").replace(">", "&gt;") + "`" for line in changes.splitlines() if line]
    rows += ["", "## ビルドと確認範囲", "",
             "同じコミットの全OS配布ビルドが成功した後に、この下書きを作成しています。", "",
             "|OS|配布ビルド|実機でのIME入力|", "|---|---|---|"]
    for os_name in ("Windows", "macOS", "Linux", "Android", "ChromeOS", "iOS"):
        rows.append(f"|{os_name}|成功|未実施|")
    rows += ["", "Windows・macOSはCI専用試験、Linuxは隔離IBus、Androidはエミュレーター、ChromeOSは実WASM・模擬IME API、iOSはiOS Simulatorで検証しています。通常アプリ・物理端末でのIME入力確認とは別です。",
             "", "## 配布と導入", "", "|OS|配布ファイル|使い方|", "|---|---|---|"]
    platforms = [
        ("Windows x64", PACKAGES[0:1], "rust/WINDOWS-PACKAGE.md"),
        ("Linux x64", PACKAGES[1:2], "linux/README.md"),
        ("macOS Apple Silicon / Intel", PACKAGES[2:4], "mac/README.md"),
        ("Android", PACKAGES[4:5], "android/README.md"),
        ("ChromeOS", PACKAGES[5:6], "chromeos/README.md"),
        ("iOS", PACKAGES[6:9], "ios/README.md"),
    ]
    for os_name, files, guide in platforms:
        links = "<br>".join(f"[{name}](https://github.com/{REPO}/releases/download/{tag}/{name})" for name in files)
        rows.append(f"|{os_name}|{links}|[導入手順](https://github.com/{REPO}/blob/{sha}/{guide})|")
    rows += ["", "iOSのIPAは未署名です。実機へ入れるには自分のApple IDでの署名が必要です。Simulator ZIPとソースZIPは開発用です。",
             "Androidはdebug署名、macOSはアドホック署名のプレビューです。ChromeOSは通常のデスクトップChrome用IMEではありません。",
             "各配布ファイルのSHA256は添付のSHA256SUMS.txtで確認できます。", "", "## ベンチマーク", ""]
    rows += [re.sub(r"(?m)^(#+) ", r"\1# ", benchmark),
             "GitHub共有Windowsランナー上の共通Coreの測定です。全OSの入力遅延や常駐IME全体の性能を示すものではありません。", "",
             "### 箱ひげ図", ""]
    for heading, names in [("Rust前版・現在版（2者比較）", RUST_IMAGES),
                           ("C#・Rust前版・現在版（3者比較）", COMBINED_IMAGES)]:
        rows += ["#### " + heading, ""]
        for name in names:
            rows += [f"![{name}](https://raw.githubusercontent.com/{REPO}/{media_sha}/{tag}/{name})", ""]
    rows += ["測定レポート・画像・生データのダウンロード用アセットは添付していません。", ""]
    body = normalize_markdown("\n".join(rows))
    if re.search(r"\]\((?!https://)[^)]+\)", body):
        raise ValueError("本文にローカルファイルへのリンクがあります")
    return title, body


def validate_rendered(body):
    html = gh("api", "markdown", payload={"text": body, "mode": "gfm", "context": REPO})
    if len(re.findall(r"<table\b", html)) < 3 or len(re.findall(r"<img\b", html)) != len(IMAGES):
        raise ValueError("GitHubで表または箱ひげ図を描画できません")


def publish_images(tag, charts):
    # CIからGitデータAPIで画像専用ブランチへ保存する。
    # コミットSHAのURLを本文へ埋め込み、過去リリースの画像を変更しない。
    refs = api(f"git/matching-refs/heads/{MEDIA_BRANCH}")
    ref = next((r for r in refs if r["ref"] == f"refs/heads/{MEDIA_BRANCH}"), None)
    parent = ref["object"]["sha"] if ref else None
    payload = {"tree": []}
    if parent:
        payload["base_tree"] = api(f"git/commits/{parent}")["tree"]["sha"]
    for name in IMAGES:
        blob = api("git/blobs", {"content": base64.b64encode((charts / name).read_bytes()).decode(), "encoding": "base64"})
        payload["tree"].append({"path": f"{tag}/{name}", "mode": "100644", "type": "blob", "sha": blob["sha"]})
    tree = api("git/trees", payload)
    commit = api("git/commits", {"message": f"docs: {tag}の本文用箱ひげ図", "tree": tree["sha"], "parents": [parent] if parent else []})
    if parent:
        api(f"git/refs/heads/{MEDIA_BRANCH}", {"sha": commit["sha"], "force": False}, "PATCH")
    else:
        api("git/refs", {"ref": f"refs/heads/{MEDIA_BRANCH}", "sha": commit["sha"]})
    return commit["sha"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--sha", required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--benchmark", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--publish", action="store_true", help="検証後、画像と下書きをGitHubへ保存する")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.sha):
        parser.error("完全なコミットSHAが必要です")
    if args.publish:
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], encoding="utf-8").strip()
        version = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
        if head != args.sha or args.tag != "flash-v" + version.replace("-pre.", "-preview."):
            raise ValueError("タグ・ソースの版・実行元コミットが一致しません")
    validate_assets(args.assets)
    report, benchmark = validate_benchmark(args.benchmark, args.sha)
    changes = subprocess.check_output(["git", "log", "--format=%s", f"{report['old_commit']}..{args.sha}"], encoding="utf-8")
    title, body = compose_body(args.tag, args.sha, benchmark, args.sha, changes)
    validate_rendered(body)
    if args.publish:
        # 再実行で公開済みリリースや配布ファイルを上書きしない。
        if find_release(args.tag) is not None:
            raise ValueError("同じタグのリリースが既にあります。上書きしません")
        media_sha = publish_images(args.tag, args.benchmark / "charts")
        title, body = compose_body(args.tag, args.sha, benchmark, media_sha, changes)
        validate_rendered(body)
    args.output.write_text(body, encoding="utf-8", newline="\n")
    # アプリだけでなくライセンスのハッシュも最終一覧へ含める。
    hashes = "".join(f"{hashlib.sha256((args.assets / name).read_bytes()).hexdigest()}  {name}\n"
                     for name in sorted(PACKAGES + DOCUMENTS[:2]))
    (args.assets / "SHA256SUMS.txt").write_text(hashes, encoding="utf-8", newline="\n")
    if args.publish:
        names = [str(args.assets / name) for name in PACKAGES + DOCUMENTS]
        gh("release", "create", args.tag, *names, "--repo", REPO, "--target", args.sha,
           "--title", title, "--notes-file", str(args.output), "--prerelease", "--draft")
        release = find_release(args.tag)
        if release is None:
            raise ValueError("作成した下書きを取得できません")
        uploaded = {asset["name"]: asset["digest"] for asset in release["assets"]}
        expected = {name: "sha256:" + hashlib.sha256((args.assets / name).read_bytes()).hexdigest() for name in PACKAGES + DOCUMENTS}
        if uploaded != expected or normalize_markdown(release["body"]) != body:
            raise ValueError("GitHub上の添付または本文が期待した内容と一致しません")
        print(release["html_url"])


if __name__ == "__main__":
    main()
