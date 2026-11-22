#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""IMEベンチマーク (1万文コーパス) の測定・比較スクリプト。

バージョンの指定方法 (--version / --compare):
  latestrelease      最新の正式リリース
  latestrelease-1    1つ前の正式リリース (latestrelease-2 は2つ前)
  1.1.26 / v1.1.26 / flash-v1.1.26 / flash-v0001.0001.0026  そのバージョン
  HEAD               現在のチェックアウト (開発中の測定用)
  URL形式も可: https://github.com/appipinopi/Meltype-Flash/latestrelease
               https://github.com/appipinopi/Meltype-Flash/latestrelease-1
               https://github.com/appipinopi/Meltype-Flash/releases/tag/flash-v0001.0001.0026

測定は2層:
  Layer A (かな): 読みをローマ字で打ち、コアの確定文字列が読み (ひらがな) と
                  一致するか。どのバージョンでも測定できる。
  Layer B (漢字かな交じり): 変換コマンド (--converter-cmd) が設定された時だけ、
                  expected (漢字かな交じり文) との一致を測る。未設定なら「未測定」。

出力: report.json (機械可読) と report.md (カテゴリ別比較表)。
GitHub Actions では report.md を Step Summary に出す。
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import io
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from kana2romaji import convert as kana_convert  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CORPUS = Path(__file__).parent / "data"  # ディレクトリ内の *.jsonl を結合して読む (GitHubのファイルサイズ制限のため分割保存)
REPO_SLUG = "appipinopi/Meltype-Flash"
RUNNER_EXAMPLE = ROOT / "rust" / "core" / "examples" / "ime_benchmark.rs"

# ---------------- バージョン指定の解決 ----------------

def _version_key(tag):
    """'flash-v0001.0001.0026' -> (1, 1, 26)"""
    m = re.search(r'(\d+)\.(\d+)\.(\d+)', tag or "")
    return tuple(int(x) for x in m.groups()) if m else None


def fetch_releases(repo=REPO_SLUG):
    """GitHub APIから正式リリース (draft/prerelease除く) を新しい順に返す。失敗したら git tag で代替。"""
    url = f"https://api.github.com/repos/{repo}/releases?per_page=100"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "ime-benchmark", "Accept": "application/vnd.github+json"})
        with urllib.request.urlopen(req, timeout=30) as res:
            data = json.load(res)
        rels = [r for r in data if not r.get("draft") and not r.get("prerelease") and _version_key(r.get("tag_name"))]
        rels.sort(key=lambda r: r.get("published_at") or "", reverse=True)
        return [{"tag": r["tag_name"], "version": _version_key(r["tag_name"]), "published_at": r.get("published_at")} for r in rels]
    except Exception:
        pass
    # フォールバック: ローカルの git tag
    try:
        out = subprocess.check_output(["git", "tag", "-l", "flash-v*"], cwd=ROOT, text=True)
        tags = [t.strip() for t in out.splitlines() if _version_key(t.strip())]
        tags.sort(key=_version_key, reverse=True)
        return [{"tag": t, "version": _version_key(t), "published_at": None} for t in tags]
    except Exception:
        return []


def normalize_spec(spec):
    """URLや装飾を取り除き、解決用の素の指定文字列にする。"""
    s = (spec or "").strip()
    if "github.com" in s:
        parts = [p for p in s.split("/") if p]
        # releases/tag/<tag> 形式
        if "tag" in parts:
            return parts[parts.index("tag") + 1]
        s = parts[-1]
    return s


def resolve_spec(spec, releases=None):
    """指定文字列 -> {spec, tag, version, published_at} 。HEAD の場合は tag=None。"""
    raw = normalize_spec(spec)
    if raw.upper() == "HEAD":
        return {"spec": "HEAD", "tag": None, "version": None, "published_at": None}
    if releases is None:
        releases = fetch_releases()
    if not releases:
        raise ValueError("リリース一覧を取得できませんでした (GitHub API / git tag の両方に失敗)")
    m = re.fullmatch(r"latestrelease(?:-(\d+))?", raw, re.IGNORECASE)
    if m:
        idx = int(m.group(1) or 0)
        if idx >= len(releases):
            raise ValueError(f"latestrelease-{idx} は存在しません (リリースは {len(releases)} 件)")
        r = releases[idx]
        return {"spec": raw, **r}
    key = _version_key(raw)
    if key:
        for r in releases:
            if r["version"] == key:
                return {"spec": raw, **r}
        raise ValueError(f"バージョン {raw} のリリースが見つかりません")
    # タグ名そのまま
    for r in releases:
        if r["tag"] == raw:
            return {"spec": raw, **r}
    raise ValueError(f"バージョン指定を解釈できません: {spec!r}")


# ---------------- 採点 ----------------

def edit_distance(a, b):
    if a == b:
        return 0
    previous = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        current = [i]
        for j, cb in enumerate(b, 1):
            current.append(min(current[-1] + 1, previous[j] + 1, previous[j - 1] + (ca != cb)))
        previous = current
    return previous[-1]


def score(cases, actuals, target_key):
    """cases: [{id, category, ...}], actuals: {id: str} -> カテゴリ別集計"""
    per = {}
    for case in cases:
        cat = case["category"]
        bucket = per.setdefault(cat, {"exact": 0, "total": 0, "chars": 0, "errors": 0})
        expected = case[target_key]
        actual = actuals.get(case["id"], "")
        bucket["total"] += 1
        bucket["exact"] += int(actual == expected)
        bucket["chars"] += len(expected)
        bucket["errors"] += edit_distance(actual, expected)
    for bucket in per.values():
        bucket["exact_accuracy"] = bucket["exact"] / bucket["total"] if bucket["total"] else 0.0
        bucket["character_accuracy"] = 1 - bucket["errors"] / max(bucket["chars"], 1)
    total = {"exact": sum(b["exact"] for b in per.values()), "total": sum(b["total"] for b in per.values()),
             "chars": sum(b["chars"] for b in per.values()), "errors": sum(b["errors"] for b in per.values())}
    total["exact_accuracy"] = total["exact"] / total["total"] if total["total"] else 0.0
    total["character_accuracy"] = 1 - total["errors"] / max(total["chars"], 1)
    return {"per_category": per, "total": total}


# ---------------- 測定 ----------------

def prepare_cases(corpus_path):
    path = Path(corpus_path)
    if path.is_dir():
        lines = []
        for part in sorted(path.glob("*.jsonl")):
            lines.extend(part.read_text(encoding="utf-8").splitlines())
    else:
        lines = path.read_text(encoding="utf-8").splitlines()
    rows = [json.loads(line) for line in lines if line.strip()]
    rows.sort(key=lambda r: r["id"])
    cases = []
    for row in rows:
        typed, kana_target = kana_convert(row["input"])
        cases.append({"id": row["id"], "category": row["category"], "subcategory": row.get("subcategory", ""),
                      "typed": typed, "kana_target": kana_target, "expected": row["expected"], "input": row["input"]})
    return cases


def extract_ref(ref, dest):
    """git archive で対象リビジョンのソースを取り出す (既存ベンチマークと同じ方式)。"""
    dest.mkdir(parents=True)
    archive = subprocess.check_output(["git", "archive", ref], cwd=ROOT)
    with tarfile.open(fileobj=io.BytesIO(archive)) as files:
        files.extractall(dest, filter="data")


def measure(resolved, cases, workdir, converter_cmd=None):
    """1バージョン分を測定し、Layer A (と可能なら Layer B) の集計を返す。"""
    label = resolved["tag"] or "HEAD"
    src = workdir / f"src-{label}"
    if resolved["tag"]:
        extract_ref(resolved["tag"], src)
    else:
        src = ROOT
    # ランナーを対象ソースへ差し込む (旧タグには無いため)
    example_dest = src / "rust" / "core" / "examples" / "ime_benchmark.rs"
    if RUNNER_EXAMPLE.resolve() != example_dest.resolve() and example_dest.parent.exists():
        shutil.copyfile(RUNNER_EXAMPLE, example_dest)
    cases_path = workdir / f"cases-{label}.jsonl"
    with cases_path.open("w", encoding="utf-8") as f:
        for c in cases:
            f.write(json.dumps({"id": c["id"], "typed": c["typed"]}, ensure_ascii=False) + "\n")
    out_path = workdir / f"actual-{label}.jsonl"
    cmd = ["cargo", "run", "--release", "--locked", "-p", "meltype-core", "--example", "ime_benchmark",
           "--", str(cases_path), str(out_path)]
    subprocess.run(cmd, cwd=src, check=True)
    actuals = {}
    for line in out_path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            row = json.loads(line)
            actuals[row["id"]] = row["actual"]
    result = {"version": resolved, "layer_a_kana": score(cases, actuals, "kana_target"), "layer_b_kanji": None}
    if converter_cmd:
        conv_in = workdir / f"conv-in-{label}.jsonl"
        conv_out = workdir / f"conv-out-{label}.jsonl"
        with conv_in.open("w", encoding="utf-8") as f:
            for c in cases:
                f.write(json.dumps({"id": c["id"], "input": c["input"]}, ensure_ascii=False) + "\n")
        with conv_in.open(encoding="utf-8") as fin, conv_out.open("w", encoding="utf-8") as fout:
            subprocess.run(converter_cmd, shell=True, stdin=fin, stdout=fout, check=True)
        conv_actuals = {}
        for line in conv_out.read_text(encoding="utf-8").splitlines():
            if line.strip():
                row = json.loads(line)
                conv_actuals[row["id"]] = row["actual"]
        result["layer_b_kanji"] = score(cases, conv_actuals, "expected")
    return result


# ---------------- 表の生成 ----------------

def _pct(x):
    return f"{x * 100:.2f}%"


def make_table(results):
    """results: [measure() の返り値, ...] (先頭が主対象、残りが比較対象)"""
    lines = []
    names = [(r["version"]["tag"] or "HEAD") for r in results]
    lines.append("## IMEベンチマーク結果 (1万文コーパス)")
    lines.append("")
    lines.append("Layer A: 読みをローマ字入力し、コアの確定文字列が読み (ひらがな) と一致した割合")
    lines.append("")
    header = "| カテゴリ | 件数 | " + " | ".join(f"{n} 完全一致 | {n} 文字精度" for n in names) + (" | Δ完全一致 |" if len(results) > 1 else " |")
    lines.append(header)
    lines.append("|---|---:|" + "---:|" * (len(names) * 2) + ("---:|" if len(results) > 1 else ""))
    cats = list(results[0]["layer_a_kana"]["per_category"].keys())
    for cat in cats + ["__total__"]:
        if cat == "__total__":
            label, row0 = "**全体**", results[0]["layer_a_kana"]["total"]
            cells = []
            for r in results:
                b = r["layer_a_kana"]["total"]
                cells += [_pct(b["exact_accuracy"]), _pct(b["character_accuracy"])]
            delta = ""
            if len(results) > 1:
                d = results[0]["layer_a_kana"]["total"]["exact_accuracy"] - results[1]["layer_a_kana"]["total"]["exact_accuracy"]
                delta = f"**{d * 100:+.2f}pt**"
            lines.append(f"| {label} | {row0['total']} | " + " | ".join(cells) + f" | {delta} |" if delta else f"| {label} | {row0['total']} | " + " | ".join(cells) + " |")
        else:
            row0 = results[0]["layer_a_kana"]["per_category"][cat]
            cells = []
            for r in results:
                b = r["layer_a_kana"]["per_category"].get(cat, {"exact_accuracy": 0, "character_accuracy": 0})
                cells += [_pct(b["exact_accuracy"]), _pct(b["character_accuracy"])]
            delta = ""
            if len(results) > 1:
                d = row0["exact_accuracy"] - results[1]["layer_a_kana"]["per_category"].get(cat, {"exact_accuracy": 0})["exact_accuracy"]
                delta = f"{d * 100:+.2f}pt"
            lines.append(f"| {cat} | {row0['total']} | " + " | ".join(cells) + (f" | {delta} |" if len(results) > 1 else " |"))
    lines.append("")
    # Layer B
    lines.append("Layer B: 漢字かな交じり文 (expected) との完全一致")
    lines.append("")
    if all(r["layer_b_kanji"] for r in results):
        lines.append("| バージョン | 完全一致 | 文字精度 |")
        lines.append("|---|---:|---:|")
        for r in results:
            t = r["layer_b_kanji"]["total"]
            lines.append(f"| {r['version']['tag'] or 'HEAD'} | {_pct(t['exact_accuracy'])} ({t['exact']}/{t['total']}) | {_pct(t['character_accuracy'])} |")
    else:
        lines.append("未測定 (変換コマンド --converter-cmd / 環境変数 IME_BENCH_CONVERTER_CMD が未設定のため)。"
                     "AIモデル等の変換層をコマンドとして接続すると測定できます。")
    lines.append("")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--version", default="latestrelease", help="測定するバージョン指定 (既定: latestrelease)")
    ap.add_argument("--compare", default=None, help="比較するバージョン指定 (例: latestrelease-1)。省略時は比較なし")
    ap.add_argument("--corpus", default=str(DEFAULT_CORPUS))
    ap.add_argument("--output", default="ime-benchmark-output", help="出力ディレクトリ")
    ap.add_argument("--converter-cmd", default=os.environ.get("IME_BENCH_CONVERTER_CMD"), help="Layer B用変換コマンド (JSONL入出力)")
    ap.add_argument("--resolve-only", action="store_true", help="バージョン解決だけ行って表示する")
    ap.add_argument("--github-summary", default=os.environ.get("GITHUB_STEP_SUMMARY"), help="表を追記するファイル")
    args = ap.parse_args()

    releases = fetch_releases()
    main_resolved = resolve_spec(args.version, releases)
    compare_resolved = resolve_spec(args.compare, releases) if args.compare else None
    print(f"version: {args.version} -> {main_resolved['tag'] or 'HEAD'}")
    if compare_resolved:
        print(f"compare: {args.compare} -> {compare_resolved['tag'] or 'HEAD'}")
    if args.resolve_only:
        return

    cases = prepare_cases(args.corpus)
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    workdir = output / "work"
    workdir.mkdir(exist_ok=True)
    results = [measure(main_resolved, cases, workdir, args.converter_cmd)]
    if compare_resolved:
        results.append(measure(compare_resolved, cases, workdir, args.converter_cmd))
    report = {"corpus": str(Path(args.corpus).name), "case_count": len(cases), "results": results}
    (output / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    table = make_table(results)
    (output / "report.md").write_text(table + "\n", encoding="utf-8")
    print(table)
    if args.github_summary:
        with open(args.github_summary, "a", encoding="utf-8") as f:
            f.write(table + "\n")


if __name__ == "__main__":
    main()
