#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""AIモデル一括測定のジョブ分割 (行列) を作る。

model-catalog.json の34モデルを、実測した1文あたりの時間に合わせて
シャードへ分ける。分割数は `ai_bench_plan.json` の `shards` に書く
(無いモデルは1ジョブ)。Shunti は Rust の組み込み実行器なので
2ジョブに3モデルずつまとめる。

  python tools/ime-benchmark/ai_bench_plan.py            # 人が読む形
  python tools/ime-benchmark/ai_bench_plan.py --github-output  # Actions の matrix
"""
import argparse
import json
import os
from pathlib import Path

HERE = Path(__file__).resolve().parent
CATALOG = HERE.parents[1] / "rust" / "ai" / "model-catalog.json"
PLAN = HERE / "ai_bench_plan.json"
SHUNTI_GROUPS = (("a", 3), ("b", 3))


def load_shards():
    if PLAN.exists():
        return json.loads(PLAN.read_text(encoding="utf-8"))
    return {}


def build(catalog, shards, only=None):
    models = catalog["models"]
    if only:
        wanted = [name.strip() for name in only.split(",") if name.strip()]
        unknown = [name for name in wanted if name not in {m["id"] for m in models}]
        if unknown:
            raise SystemExit(f"一覧に無いモデル: {', '.join(unknown)}")
        models = [m for m in models if m["id"] in wanted]
    shunti_models = [m["id"] for m in models if m["family"] == "Shunti"]
    shunti = []
    if shunti_models:
        size = -(-len(shunti_models) // len(SHUNTI_GROUPS))
        for index, (label, _) in enumerate(SHUNTI_GROUPS):
            group = shunti_models[index * size : (index + 1) * size]
            if group:
                shunti.append({"group": label, "variants": " ".join(group)})
    gguf = []
    for model in models:
        if model["family"] == "Shunti":
            continue
        count = int(shards.get(model["id"], 1))
        for shard in range(count):
            gguf.append(
                {
                    "model": model["id"],
                    "shard": shard,
                    "shards": count,
                    "name": f"{model['id']}-s{shard}of{count}",
                }
            )
    return {"shunti": shunti, "gguf": gguf}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", default="", help="モデルIDをカンマ区切りで限定")
    parser.add_argument("--github-output", action="store_true")
    args = parser.parse_args()
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    plan = build(catalog, load_shards(), args.only or None)
    print(f"shunti jobs: {len(plan['shunti'])}")
    for entry in plan["shunti"]:
        print(f"  {entry['group']}: {entry['variants']}")
    print(f"gguf jobs: {len(plan['gguf'])}")
    for entry in plan["gguf"]:
        print(f"  {entry['name']}")
    target = os.environ.get("GITHUB_OUTPUT")
    if args.github_output and target:
        with open(target, "a", encoding="utf-8") as handle:
            handle.write(f"shunti={json.dumps(plan['shunti'])}\n")
            handle.write(f"gguf={json.dumps(plan['gguf'])}\n")


if __name__ == "__main__":
    main()
