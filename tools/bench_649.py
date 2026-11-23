#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Meltype-Flash 649ケース精度・速度ベンチマーク
  - Excelの「ケース別結果」シートから649ケースを読み込み
  - meltype-rs バイナリ（JSON Lines IPC）で1文字ずつ打鍵 → Spaceで確定
  - 精度・p50/p95/p99レイテンシを計測
"""
import argparse
import json
import os
import subprocess
import sys
import time
import statistics
from pathlib import Path

# --- ROMAJI → VK/CH マップ（ASCII printable） ---
def char_to_event(ch: str) -> dict:
    code = ord(ch)
    return {"op": "key", "vk": code, "ch": code}

SPACE_EVENT = {"op": "key", "vk": 32, "ch": 32}
ESCAPE_EVENT = {"op": "key", "vk": 27, "ch": 0}

def run_case(proc, romaji: str, judge: str) -> tuple[str, float]:
    """
    1ケースを打鍵して確定結果と所要時間(ms)を返す。
    judge='部分一致' のときは出力が期待値を含むかチェック用に返す。
    """
    # ESCでリセット
    line = json.dumps(ESCAPE_EVENT, ensure_ascii=False) + "\n"
    proc.stdin.write(line.encode())
    proc.stdin.flush()
    # ESCの応答を読み捨て
    proc.stdout.readline()

    # 文字ごとに打鍵（タイミング計測対象外）
    for ch in romaji:
        ev = char_to_event(ch)
        line = json.dumps(ev, ensure_ascii=False) + "\n"
        proc.stdin.write(line.encode())
        proc.stdin.flush()
        proc.stdout.readline()  # 読み捨て

    # Space で確定（ここだけ計測）
    t0 = time.perf_counter()
    space_line = json.dumps(SPACE_EVENT, ensure_ascii=False) + "\n"
    proc.stdin.write(space_line.encode())
    proc.stdin.flush()
    response_bytes = proc.stdout.readline()
    t1 = time.perf_counter()
    elapsed_ms = (t1 - t0) * 1000.0

    try:
        resp = json.loads(response_bytes)
        # 応答形式を確認してテキストを取り出す
        if isinstance(resp, dict):
            # candidates の最初、またはtextフィールド
            if "text" in resp:
                result_text = resp["text"]
            elif "candidates" in resp and resp["candidates"]:
                result_text = resp["candidates"][0].get("text", "")
            elif "commit" in resp:
                result_text = resp["commit"]
            else:
                result_text = str(resp)
        else:
            result_text = str(resp)
    except Exception:
        result_text = response_bytes.decode(errors="replace").strip()

    return result_text, elapsed_ms


def main():
    parser = argparse.ArgumentParser(description="649ケース精度・速度ベンチマーク")
    parser.add_argument("--binary", type=Path,
                        default=Path(r"C:\opencode-rust\target\release\meltype-rs.exe"),
                        help="meltype-rs.exe のパス")
    parser.add_argument("--excel", type=Path,
                        default=Path(r"C:\Users\藤島\AppData\Local\Temp\Meltype_matrix.xlsx"),
                        help="Meltype_matrix.xlsx のパス")
    parser.add_argument("--sheet", default="ケース別結果")
    parser.add_argument("--header-row", type=int, default=5,
                        help="ヘッダー行番号 (1始まり)")
    parser.add_argument("--output", type=Path, default=None,
                        help="結果JSON出力先 (省略時は標準出力のみ)")
    parser.add_argument("--verbose", action="store_true",
                        help="不正解ケースを逐次表示")
    args = parser.parse_args()

    # Excel読み込み
    try:
        import openpyxl
    except ImportError:
        sys.exit("openpyxl が必要です: pip install openpyxl")

    wb = openpyxl.load_workbook(str(args.excel), read_only=True, data_only=True)
    ws = wb[args.sheet]

    # ヘッダー行を確認して列インデックスを特定
    all_rows = list(ws.iter_rows(values_only=True))
    header = all_rows[args.header_row - 1]  # 0始まり
    print(f"Header: {header[:8]}")

    # 列マップ: B=1, C=2, D=3, E=4
    # all_rows[i][0]=None(A), [1]=set, [2]=romaji, [3]=expected, [4]=judge
    SET_COL = 1
    ROMAJI_COL = 2
    EXPECTED_COL = 3
    JUDGE_COL = 4

    cases = []
    for row in all_rows[args.header_row:]:  # header_row以降がデータ
        set_name = row[SET_COL]
        romaji = row[ROMAJI_COL]
        expected = row[EXPECTED_COL]
        judge = row[JUDGE_COL]
        if set_name and romaji and expected:
            cases.append({
                "set": str(set_name),
                "romaji": str(romaji),
                "expected": str(expected),
                "judge": str(judge) if judge else "完全一致",
            })

    print(f"ケース数: {len(cases)}")

    # バイナリ起動
    if not args.binary.exists():
        sys.exit(f"バイナリが見つかりません: {args.binary}")

    proc = subprocess.Popen(
        [str(args.binary)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )

    # ウォームアップ（3ケース）
    print("ウォームアップ中...")
    for case in cases[:3]:
        run_case(proc, case["romaji"], case["judge"])

    # 本計測
    results = []
    correct_by_set = {}
    total_by_set = {}
    latencies = []

    print(f"計測開始 ({len(cases)} ケース)...")
    for i, case in enumerate(cases):
        set_name = case["set"]
        result_text, elapsed_ms = run_case(proc, case["romaji"], case["judge"])
        expected = case["expected"]
        judge = case["judge"]

        if judge == "部分一致":
            ok = expected in result_text
        else:
            ok = result_text == expected

        latencies.append(elapsed_ms)
        correct_by_set.setdefault(set_name, 0)
        total_by_set.setdefault(set_name, 0)
        total_by_set[set_name] += 1
        if ok:
            correct_by_set[set_name] += 1

        results.append({
            "set": set_name,
            "romaji": case["romaji"],
            "expected": expected,
            "got": result_text,
            "ok": ok,
            "latency_ms": round(elapsed_ms, 3),
        })

        if args.verbose and not ok:
            print(f"  ✗ [{set_name}] {case['romaji']}")
            print(f"      期待: {expected}")
            print(f"      実際: {result_text}")

        if (i + 1) % 100 == 0:
            print(f"  {i+1}/{len(cases)} 完了...")

    proc.terminate()

    # 集計
    total_correct = sum(correct_by_set.values())
    total_cases = len(cases)
    accuracy = total_correct / total_cases * 100

    p50 = statistics.median(latencies)
    latencies_sorted = sorted(latencies)
    p95 = latencies_sorted[int(len(latencies_sorted) * 0.95)]
    p99 = latencies_sorted[int(len(latencies_sorted) * 0.99)]
    avg = statistics.mean(latencies)

    print("\n" + "="*60)
    print("📊 Meltype-Flash 649ケース ベンチマーク結果")
    print("="*60)
    print(f"  総合精度:  {total_correct}/{total_cases} = {accuracy:.1f}%")
    print()
    # セット別
    # セット名のプレフィックスで集約
    set_groups = {}
    for k in total_by_set:
        prefix = k.split("_")[0] if "_" in k else k  # 例: "A", "B", "C", "D"
        set_groups.setdefault(prefix, {"correct": 0, "total": 0})
        set_groups[prefix]["correct"] += correct_by_set.get(k, 0)
        set_groups[prefix]["total"] += total_by_set[k]

    for g in sorted(set_groups):
        sg = set_groups[g]
        pct = sg["correct"] / sg["total"] * 100 if sg["total"] else 0
        print(f"  セット {g}:  {sg['correct']}/{sg['total']} = {pct:.1f}%")

    print()
    print(f"  レイテンシ (Space確定, IPC込み):")
    print(f"    p50  = {p50:.3f} ms")
    print(f"    p95  = {p95:.3f} ms")
    print(f"    p99  = {p99:.3f} ms")
    print(f"    avg  = {avg:.3f} ms")
    print("="*60)

    # 不正解一覧（verbose未指定時もサマリ）
    failures = [r for r in results if not r["ok"]]
    if failures:
        print(f"\n❌ 不正解 {len(failures)} ケース:")
        for r in failures[:30]:
            print(f"  [{r['set']}] {r['romaji']}")
            print(f"    期待: {r['expected']}")
            print(f"    実際: {r['got']}")
        if len(failures) > 30:
            print(f"  ... 他 {len(failures)-30} ケース")

    # JSON出力
    report = {
        "total_correct": total_correct,
        "total_cases": total_cases,
        "accuracy_pct": round(accuracy, 2),
        "per_set": {g: {"correct": v["correct"], "total": v["total"],
                        "pct": round(v["correct"]/v["total"]*100, 1) if v["total"] else 0}
                   for g, v in set_groups.items()},
        "latency": {
            "p50_ms": round(p50, 3),
            "p95_ms": round(p95, 3),
            "p99_ms": round(p99, 3),
            "avg_ms": round(avg, 3),
        },
        "failures": failures,
        "results": results,
    }

    if args.output:
        args.output.write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        print(f"\n詳細結果を保存: {args.output}")

    return 0 if accuracy >= 75.0 else 1


if __name__ == "__main__":
    sys.exit(main())
