#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""第2弾コア比較ベンチのジョブ (行列) を作る。

`core_targets.json` の対象一覧から、GitHub Actions の matrix に入れる
`core=[{"target":..,"name":..}, ...]` を組み立てる。ワークフロー
(`ai-benchmark-tmp.yml`) の `mode: core` は、この出力の `core` を
`strategy.matrix.include` にそのまま渡し、各ジョブで
`core_bench_runner.py --target <id> --out bench-out/<name>.json` を実行する。

  python tools/ime-benchmark/core_bench_plan.py              # 人が読む一覧
  python tools/ime-benchmark/core_bench_plan.py --only a,b   # 対象を限定
  python tools/ime-benchmark/core_bench_plan.py --github-output  # Actions の matrix へ書く

`--only` は id・name・label のどれでも指定できる (カンマ区切り)。空なら全対象。
"""
import argparse
import json
import os
from pathlib import Path

HERE = Path(__file__).resolve().parent
TARGETS = HERE / "core_targets.json"


def load_targets(path=TARGETS):
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    targets = data.get("targets") or []
    if not targets:
        raise SystemExit(f"対象がありません: {path}")
    seen = set()
    for target in targets:
        for key in ("id", "name", "repo", "ref", "kind"):
            if not target.get(key):
                raise SystemExit(f"対象 {target.get('id')!r} に {key} がありません")
        if target["id"] in seen:
            raise SystemExit(f"対象 ID が重複しています: {target['id']}")
        seen.add(target["id"])
        if target["kind"] == "csharp" and target.get("generation") not in (1, 2, 3):
            raise SystemExit(f"C# の対象 {target['id']} に generation (1..3) がありません")
    return data


def select(data, only=None):
    """--only の指定を対象のリストにする。指定が無ければ全対象。"""
    targets = data["targets"]
    if not only:
        return targets
    wanted = [item.strip() for item in only.split(",") if item.strip()]
    if not wanted:
        return targets
    by_id = {t["id"]: t for t in targets}
    by_name = {t["name"]: t for t in targets}
    by_label = {t["label"]: t for t in targets}
    chosen, unknown = [], []
    for item in wanted:
        target = by_id.get(item) or by_name.get(item) or by_label.get(item)
        if target is None:
            unknown.append(item)
        elif target not in chosen:
            chosen.append(target)
    if unknown:
        known = ", ".join(t["id"] for t in targets)
        raise SystemExit(f"対象に無い指定: {', '.join(unknown)}\n指定できる ID: {known}")
    return chosen


def build(data, only=None):
    """matrix に渡す配列。要素は target (ID) と name (artifact/ファイル名) だけ。"""
    return [{"target": t["id"], "name": t["name"]} for t in select(data, only)]


def describe(data, chosen):
    lines = []
    counts = {}
    for target in chosen:
        counts[target["kind"]] = counts.get(target["kind"], 0) + 1
    lines.append(f"core jobs: {len(chosen)} ({', '.join(f'{k}={v}' for k, v in sorted(counts.items()))})")
    for target in chosen:
        layer = "A+B" if target.get("layer_b", True) else "A"
        lines.append(f"  {target['id']:<20} {target['name']:<20} {target['repo']}@{target['ref']} [{target['kind']}, {layer}]")
    excluded = data.get("excluded") or []
    if excluded:
        lines.append(f"対象外: {len(excluded)} 件 (理由は core_targets.json の excluded)")
        for item in excluded:
            lines.append(f"  - {item['label']}: {item['reason']}")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--only", default="", help="対象 ID (または name / label) をカンマ区切りで限定 (空=全対象)")
    parser.add_argument("--targets", default=str(TARGETS), help="対象一覧の JSON")
    parser.add_argument("--github-output", action="store_true", help="GITHUB_OUTPUT に core=<matrix JSON> を書く")
    parser.add_argument("--json", action="store_true", help="matrix の JSON を標準出力に出す")
    args = parser.parse_args()

    data = load_targets(args.targets)
    chosen = select(data, args.only)
    matrix = build(data, args.only)

    if args.json:
        print(json.dumps(matrix, ensure_ascii=False))
    else:
        print(describe(data, chosen))

    output = os.environ.get("GITHUB_OUTPUT")
    if args.github_output and output:
        with open(output, "a", encoding="utf-8") as handle:
            handle.write(f"core={json.dumps(matrix, ensure_ascii=False)}\n")


if __name__ == "__main__":
    main()
