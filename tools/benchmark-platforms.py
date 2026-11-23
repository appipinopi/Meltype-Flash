#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Collect measured Core/native results and explicitly unmeasured OS endpoints."""
import argparse
import datetime
import json
from pathlib import Path

PLATFORMS = ("Windows", "macOS", "Linux", "Android", "iOS", "ChromeOS")
HOSTS = {"Windows": "Windows", "Darwin": "macOS", "Linux": "Linux"}


def collect(directory):
    measurements = []
    for path in sorted(directory.rglob("report.json")):
        report = json.loads(path.read_text(encoding="utf-8"))
        if report.get("comparison_kind") != "rust-versions":
            continue
        if not report.get("accepted_performance_baseline"):
            raise ValueError(f"Integrity verification failed: {path}")
        host = report.get("host_os")
        if host not in HOSTS:
            raise ValueError(f"Unknown measurement OS: {host}")
        matching = [r for r in report["results"] if r["eligible_for_speed_claim"]]
        measurements.append(dict(
            os=HOSTS[host], backend=report.get("backend", "core"),
            machine=report["machine"], measured_at=report["measured_at"],
            old_ref=report["old_ref"], new_ref=report.get("new_commit", report["new_ref"]),
            current_source=report.get("current_source", "not recorded"),
            old_version=report["old_version"], current_version=report["current_version"],
            fixtures_sha256=report["fixtures_sha256"], report=str(path.resolve()),
            cases=len(report["results"]), matching_cases=len(matching),
            faster_cases=sum(r["new_over_old"] < 1 for r in matching),
            medians=[dict(case=r["case"], **r["medians_ms"],
                          eligible=r["eligible_for_speed_claim"]) for r in report["results"]],
            scope=report["scope"],
        ))
    # Different host computers never become one pooled performance distribution.
    coverage = []
    for os_name in PLATFORMS:
        entries = [r for r in measurements if r["os"] == os_name]
        coverage.append(dict(os=os_name,
            core="測定済み" if any(r["backend"] == "core" for r in entries) else "未測定",
            native="測定済み" if any(r["backend"] == "native" for r in entries) else "未測定",
            ime_application="未測定", physical_device_input="未測定"))
    return dict(created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                coverage=coverage, measurements=measurements)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = collect(args.input)
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "coverage.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    rows = ["# Core・各OSのベンチマーク", "",
            "同一入力を前版・現在版に与え、状態が一致したケースだけ速度比較に使います。異なるOS・CPU・CIランナーの値を合算せず、OS間の速さの順位には使いません。", "",
            "|OS|Core|ネイティブ接続|IME全体|実機での入力|", "|---|---|---|---|---|"]
    for row in report["coverage"]:
        rows.append("|" + "|".join(row.values()) + "|")
    rows += ["", "Android・iOS・ChromeOSは利用できる実機がないため未測定。別OS上のCoreの値を、そのOSの結果として表示しません。ネイティブ接続はC ABIまでで、TSF/InputMethodKit/IBus・描画・アプリへの反映は測っていません。", ""]
    for entry in report["measurements"]:
        rows += [f"## {entry['os']} / {entry['backend']}", "",
                 f"Rust {entry['old_version']} → {entry['current_version']}。状態一致 {entry['matching_cases']}/{entry['cases']}、現在版が速い入力 {entry['faster_cases']}/{entry['matching_cases']}。", "",
                 f"環境: {entry['machine']}。現在版: `{entry['new_ref']}`（{entry['current_source']}。作業ツリーの場合は基点のコミット。実際のファイルのSHA-256は測定JSONに記録）。", "",
                 f"[測定JSON]({Path(entry['report']).as_posix()})", ""]
    (args.output / "README.md").write_text("\n".join(rows), encoding="utf-8")
    print("\n".join(rows))


if __name__ == "__main__":
    main()
